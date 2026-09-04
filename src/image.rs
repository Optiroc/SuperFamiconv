//! Image representation.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use png::{BitDepth, ColorType, Transformations};

use crate::color::{NormalizedColor, Rgba8888};
use crate::dither;
use crate::map::Map;
use crate::mode::{Mode, color::ModeColor};
use crate::palette::{Palette, Subpalette};
use crate::tileset::Tileset;

#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub src_x: u32,
    pub src_y: u32,
    /// RGBA8 pixel data, `width * height * 4` bytes.
    pub data: Vec<u8>,
    /// Palette index per pixel.
    /// - Empty unless source image was indexed color.
    pub indexed_data: Vec<u8>,
    /// The source image palette.
    /// - Empty unless source image was indexed color.
    pub palette: Vec<NormalizedColor>,
    /// Distinct colors in image.
    pub colors: BTreeSet<NormalizedColor>,
}

impl Image {
    /// Creates an Image from PNG file at `path`.
    pub fn load(path: &Path) -> Result<Image, String> {
        /// Expands color indices to u8.
        fn expand(
            raw: &[u8],
            width: u32,
            height: u32,
            bit_depth: u8,
        ) -> Vec<u8> {
            if bit_depth == 8 {
                return raw.to_vec();
            }
            let (width, height) = (width as usize, height as usize);
            let row_size = (width * bit_depth as usize).div_ceil(8);
            let mask: u8 = (1u16 << bit_depth) as u8 - 1;
            let mut out = vec![0u8; width * height];
            for y in 0..height {
                let row = &raw[(y * row_size)..(y * row_size + row_size)];
                for x in 0..width {
                    let bit_offset = x * bit_depth as usize;
                    let byte_index = bit_offset / 8;
                    let shift = 8 - bit_depth as usize - (bit_offset % 8);
                    out[y * width + x] = (row[byte_index] >> shift) & mask;
                }
            }
            out
        }

        let open = |p: &Path| File::open(p).map_err(|e| format!("File '{}' could not be opened: {e}", p.display()));
        let mut decoder = png::Decoder::new(BufReader::new(open(path)?));
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;

        let width = reader.info().width;
        let height = reader.info().height;
        let bit_depth = reader.info().bit_depth;
        let color_type = reader.info().color_type;
        let is_indexed = color_type == ColorType::Indexed;

        let (data, indexed_data, palette): (Vec<u8>, Vec<u8>, Vec<NormalizedColor>) = if is_indexed {
            // Read color indices and expand to u8
            let mut raw = vec![0u8; reader.output_buffer_size().ok_or("Image too large")?];
            reader.next_frame(&mut raw).map_err(|e| e.to_string())?;
            let indices = expand(&raw, width, height, bit_depth as u8);
            let palette_size = *indices.iter().max().ok_or("Indexed PNG contains no pixel data")? as usize + 1;
            // Read palette
            let plte = reader.info().palette.as_ref().ok_or("Indexed PNG missing PLTE chunk")?;
            let trns = reader.info().trns.as_deref();
            if plte.len() / 3 < palette_size {
                Err("Indexed PNG has too few colors in PLTE chunk")?;
            }
            let mut pal = Vec::with_capacity(palette_size);
            for i in 0..palette_size {
                let a = trns.and_then(|t| t.get(i)).copied().unwrap_or(255);
                pal.push(NormalizedColor::new(plte[i * 3], plte[i * 3 + 1], plte[i * 3 + 2], a));
            }
            // Map to RGBA data
            let data = indices.iter().flat_map(|&i| pal[i as usize].to_bytes()).collect();
            (data, indices, pal)
        } else {
            // Get new decoder configured for 8-bit Rgba/GrayscaleAlpha conversion
            decoder = png::Decoder::new(BufReader::new(open(path)?));
            let transformations = Transformations::ALPHA | Transformations::EXPAND | Transformations::STRIP_16;
            decoder.set_transformations(transformations);
            reader = decoder.read_info().map_err(|e| e.to_string())?;
            // Read pixel data
            let mut raw = vec![0u8; reader.output_buffer_size().ok_or("Image too large")?];
            reader.next_frame(&mut raw).map_err(|e| e.to_string())?;
            let (out_color_type, _) = reader.output_color_type();
            match out_color_type {
                ColorType::Rgba => (raw, Vec::new(), Vec::new()),
                ColorType::GrayscaleAlpha => {
                    let rgba = raw.chunks_exact(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect();
                    (rgba, Vec::new(), Vec::new())
                }
                _ => unreachable!(),
            }
        };

        let colors = colors_in(&data);
        Ok(Image {
            width,
            height,
            src_x: 0,
            src_y: 0,
            data,
            indexed_data,
            palette,
            colors,
        })
    }

    /// Creates an Image from indexed pixel data and palette.
    #[cfg(test)]
    pub fn from_indexed_data(
        width: u32,
        height: u32,
        indexed_data: Vec<u8>,
        palette: Vec<NormalizedColor>,
    ) -> Image {
        let data: Vec<u8> = indexed_data
            .iter()
            .flat_map(|&i| palette[i as usize].to_bytes())
            .collect();
        let colors = colors_in(&data);
        Image {
            width,
            height,
            src_x: 0,
            src_y: 0,
            data,
            indexed_data,
            palette,
            colors,
        }
    }

    /// Creates an Image from RGBA pixel data as an array of u8.
    #[cfg(test)]
    pub fn from_rgba_data(
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) -> Image {
        let colors = colors_in(&data);
        Image {
            width,
            height,
            src_x: 0,
            src_y: 0,
            data,
            indexed_data: Vec::new(),
            palette: Vec::new(),
            colors,
        }
    }

    /// Creates an Image from an array of colors.
    #[cfg(test)]
    pub fn from_colors(colors: &[NormalizedColor]) -> Image {
        let width = colors.len() as u32;
        let height = 1;
        let data: Vec<u8> = colors.iter().flat_map(|&c| c.to_bytes()).collect();
        Self::from_rgba_data(width, height, data)
    }

    pub fn is_indexed(&self) -> bool {
        !self.indexed_data.is_empty()
    }

    pub fn palette_size(&self) -> usize {
        self.palette.len()
    }

    pub fn color_data(&self) -> Vec<NormalizedColor> {
        self.data
            .chunks_exact(4)
            .map(|c| NormalizedColor(Rgba8888::from_bytes(c.try_into().unwrap())))
            .collect()
    }

    pub fn color_at(
        &self,
        index: usize,
    ) -> NormalizedColor {
        let i = index * 4;
        NormalizedColor::new(self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3])
    }

    /// Crops a `crop_width x crop_height` region starting at `(x, y)`.
    ///
    /// Regions extending past the source are padded with transparent color,
    /// except in `gb` mode, which pads opaque black.
    pub fn crop(
        &self,
        x: u32,
        y: u32,
        crop_width: u32,
        crop_height: u32,
        mode: Mode,
    ) -> Image {
        let fill = if mode == Mode::Gb {
            NormalizedColor::new(0, 0, 0, 0xff)
        } else {
            NormalizedColor::TRANSPARENT
        }
        .to_bytes();

        let mut data = vec![0u8; (crop_width * crop_height * 4) as usize];
        for px in data.chunks_exact_mut(4) {
            px.copy_from_slice(&fill);
        }

        let mut indexed_data = if self.is_indexed() {
            vec![0u8; (crop_width * crop_height) as usize]
        } else {
            Vec::new()
        };

        if x <= self.width && y <= self.height {
            let w = crop_width.min(self.width.saturating_sub(x));
            let h = crop_height.min(self.height.saturating_sub(y));

            for row in 0..h {
                let src_row_start = (((y + row) * self.width) + x) as usize;
                let dst_row_start = (row * crop_width) as usize;

                let src_px = &self.data[src_row_start * 4..(src_row_start + w as usize) * 4];
                data[dst_row_start * 4..(dst_row_start + w as usize) * 4].copy_from_slice(src_px);

                if self.is_indexed() {
                    let src_idx = &self.indexed_data[src_row_start..src_row_start + w as usize];
                    indexed_data[dst_row_start..dst_row_start + w as usize].copy_from_slice(src_idx);
                }
            }
        }

        let colors = colors_in(&data);
        Image {
            width: crop_width,
            height: crop_height,
            src_x: x,
            src_y: y,
            data,
            indexed_data,
            palette: self.palette.clone(),
            colors,
        }
    }

    /// Slices the image into a row-major grid of `tile_width * tile_height` crops,
    /// covering the full image (partial tiles at the right/bottom edge are padded).
    pub fn crops(
        &self,
        tile_width: u32,
        tile_height: u32,
        mode: Mode,
    ) -> Vec<Image> {
        let mut out = Vec::new();
        let mut y = 0;
        while y < self.height {
            let mut x = 0;
            while x < self.width {
                out.push(self.crop(x, y, tile_width, tile_height, mode));
                x += tile_width;
            }
            y += tile_height;
        }
        out
    }

    /// Creates a new image with each pixel mapped to its color index within `subpalette`.
    /// - Transparent pixels are always mapped to index 0.
    pub fn remapped(
        &self,
        subpalette: &Subpalette,
    ) -> Result<Image, String> {
        let mode = subpalette.mode;
        let palette: Vec<NormalizedColor> = subpalette.normalized_colors();
        if palette.is_empty() {
            return Err("No colors".into());
        }

        let size = (self.width * self.height) as usize;
        let mut indexed_data = vec![0u8; size];
        let mut data = vec![0u8; size * 4];

        for i in 0..size {
            let quantized = mode.quantize_color(self.color_at(i));
            if quantized.is_transparent() {
                continue; // indexed_data/data already zeroed
            }
            let Some(index) = palette.iter().position(|&p| p == quantized) else {
                return Err("Color not in palette".into());
            };
            indexed_data[i] = index as u8;
            data[i * 4..i * 4 + 4].copy_from_slice(&quantized.to_bytes());
        }

        let colors = colors_in(&data);
        Ok(Image {
            width: self.width,
            height: self.height,
            src_x: self.src_x,
            src_y: self.src_y,
            data,
            indexed_data,
            palette,
            colors,
        })
    }

    /// Creates a new image with each pixel mapped to its closest color in `subpalette`.
    pub fn remapped_quantized(
        &self,
        subpalette: &Subpalette,
        dither: dither::Dither,
    ) -> Result<Image, String> {
        if subpalette.colors.is_empty() {
            return Err("No colors".into());
        }

        let (indexed_data, data) = dither::quantize_image(
            subpalette.mode,
            &subpalette.colors,
            self.width,
            self.height,
            dither,
            |i| self.color_at(i),
        );

        let colors = colors_in(&data);
        Ok(Image {
            width: self.width,
            height: self.height,
            src_x: self.src_x,
            src_y: self.src_y,
            data,
            indexed_data,
            palette: subpalette.normalized_colors(),
            colors,
        })
    }

    /// Returns the full precision color suitable for use as color-zero.
    /// - Current "heuristic" returns the color which occurs in the longest continuous run.
    pub fn infer_color_zero(
        &self,
        mode: Mode,
    ) -> NormalizedColor {
        let mut best_color = NormalizedColor::new(0, 0, 0, 0xff);
        let mut best_len = 0usize;
        let mut run_color = best_color;
        let mut run_len = best_len;

        for c in self.color_data() {
            let qc = mode.normalize_color(mode.reduce_color(c));
            if qc == run_color {
                run_len += 1;
            } else {
                run_color = qc;
                run_len = 1;
            }
            if run_len > best_len {
                best_len = run_len;
                best_color = qc;
            }
        }
        best_color
    }

    /// Creates an Image from `palette`.
    pub fn from_palette(palette: &Palette) -> Result<Image, String> {
        let rows = palette.normalized_colors();
        if rows.first().is_none_or(Vec::is_empty) {
            return Err("No colors".into());
        }

        let width = palette.max_colors_per_subpalette as u32;
        let height = rows.len() as u32;
        let mut data = vec![0u8; (width * height * 4) as usize];
        for (y, row) in rows.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                let o = (y * width as usize + x) * 4;
                data[o..o + 4].copy_from_slice(&c.to_bytes());
            }
        }

        let colors = colors_in(&data);
        Ok(Image {
            width,
            height,
            src_x: 0,
            src_y: 0,
            data,
            indexed_data: Vec::new(),
            palette: Vec::new(),
            colors,
        })
    }

    /// Creates an Image from `tileset`.
    pub fn from_tileset(
        tileset: &Tileset,
        image_width: Option<u32>,
    ) -> Result<Image, String> {
        let tiles = tileset.tiles();
        let first = tiles.first().ok_or("No tiles")?;

        let width = image_width.unwrap_or(128).max(1);
        let (tile_width, tile_height) = (tileset.tile_width(), tileset.tile_height());
        let tiles_per_row = width.div_ceil(tile_width);
        let height = (tiles.len() as u32).div_ceil(tiles_per_row) * tile_height;

        let mut data = vec![0u8; (width * height * 4) as usize];
        let mut indexed_data = vec![0u8; (width * height) as usize];

        for (i, tile) in tiles.iter().enumerate() {
            let i = i as u32;
            let (base_x, base_y) = ((i % tiles_per_row) * tile_width, (i / tiles_per_row) * tile_height);
            let tile_rgba = tile.rgba_data();
            for row in 0..tile_height {
                for col in 0..tile_width {
                    let (x, y) = (base_x + col, base_y + row);
                    if x >= width || y >= height {
                        continue;
                    }
                    let src = (row * tile_width + col) as usize;
                    let dst = ((y * width + x) * 4) as usize;
                    data[dst..dst + 4].copy_from_slice(&tile_rgba[src].to_bytes());
                    indexed_data[(y * width + x) as usize] = tile.data()[src];
                }
            }
        }

        let colors = colors_in(&data);
        Ok(Image {
            width,
            height,
            src_x: 0,
            src_y: 0,
            data,
            indexed_data,
            palette: first.palette().to_vec(),
            colors,
        })
    }

    /// Creates an Image from `map`, fetching each entry's tile from `tileset`
    /// and its colors from `palette`.
    pub fn from_map(
        map: &Map,
        tileset: &Tileset,
        palette: &Palette,
    ) -> Result<Image, String> {
        let (tile_width, tile_height) = (tileset.tile_width(), tileset.tile_height());
        let (map_width, map_height) = (map.width(), map.height());
        let width = map_width * tile_width;
        let height = map_height * tile_height;

        let tiles = tileset.tiles();
        let subpalettes = palette.normalized_colors();

        let mut data = vec![0u8; (width * height * 4) as usize];

        for my in 0..map_height {
            for mx in 0..map_width {
                let entry = map.entry_at(mx, my);
                let tile = tiles.get(entry.tile_index as usize).ok_or_else(|| {
                    format!(
                        "Map references tile {} but tileset only has {} tiles",
                        entry.tile_index,
                        tiles.len()
                    )
                })?;
                let colors = subpalettes.get(entry.palette_index as usize).ok_or_else(|| {
                    format!(
                        "Map references palette {} but palette only has {} subpalettes",
                        entry.palette_index,
                        subpalettes.len()
                    )
                })?;

                for row in 0..tile_height {
                    for col in 0..tile_width {
                        let src_col = if entry.flip_h { tile_width - 1 - col } else { col };
                        let src_row = if entry.flip_v { tile_height - 1 - row } else { row };
                        let color_idx = tile.data()[(src_row * tile_width + src_col) as usize];
                        let color = colors[color_idx as usize];

                        let (x, y) = (mx * tile_width + col, my * tile_height + row);
                        let dst = ((y * width + x) * 4) as usize;
                        data[dst..dst + 4].copy_from_slice(&color.to_bytes());
                    }
                }
            }
        }

        let colors = colors_in(&data);
        Ok(Image {
            width,
            height,
            src_x: 0,
            src_y: 0,
            data,
            indexed_data: Vec::new(),
            palette: Vec::new(),
            colors,
        })
    }

    /// Writes the RGBA pixel data to PNG.
    pub fn save(
        &self,
        path: &Path,
    ) -> Result<(), String> {
        write_png(path, self.width, self.height, &self.data, |_| {})
    }

    /// Writes the RGBA pixel data quantized to `mode`'s color precision to PNG.
    pub fn save_quantized(
        &self,
        path: &Path,
        mode: Mode,
    ) -> Result<(), String> {
        let bytes: Vec<u8> = self
            .color_data()
            .into_iter()
            .flat_map(|c| mode.quantize_color(c).to_bytes())
            .collect();
        write_png(path, self.width, self.height, &bytes, |_| {})
    }

    /// Writes the indexed color pixel data to PNG.
    pub fn save_indexed(
        &self,
        path: &Path,
    ) -> Result<(), String> {
        let palette = if self.palette.is_empty() {
            default_palette(256)
        } else {
            self.palette.clone()
        };
        if self.indexed_data.len() != (self.width * self.height) as usize {
            return Err("Image has no indexed pixel data to save".into());
        }

        let mut plte = Vec::with_capacity(palette.len() * 3);
        let mut trns = Vec::with_capacity(palette.len());
        for c in &palette {
            plte.extend_from_slice(&[c.r, c.g, c.b]);
            trns.push(c.a);
        }

        write_png(path, self.width, self.height, &self.indexed_data, |e| {
            e.set_color(ColorType::Indexed);
            e.set_depth(BitDepth::Eight);
            e.set_palette(plte);
            e.set_trns(trns);
        })
    }
}

impl std::fmt::Display for Image {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let kind = if self.palette.is_empty() { "RGB" } else { "indexed" };
        write!(f, "{}x{}px {kind}", self.width, self.height)
    }
}

/// Returns the unique colors in an array of raw RGBA bytes.
fn colors_in(rgba_data: &[u8]) -> BTreeSet<NormalizedColor> {
    rgba_data
        .chunks_exact(4)
        .map(|c| NormalizedColor(Rgba8888::from_bytes(c.try_into().unwrap())))
        .collect()
}

/// Returns a grayscale gradient with `indices` steps.
pub fn default_palette(indices: usize) -> Vec<NormalizedColor> {
    (0..indices)
        .map(|i| {
            let v = ((0x100 / indices) * i) as u8;
            NormalizedColor::new(v, v, v, 0xff)
        })
        .collect()
}

pub fn write_png(
    path: &Path,
    width: u32,
    height: u32,
    data: &[u8],
    configure: impl FnOnce(&mut png::Encoder<BufWriter<File>>),
) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    configure(&mut encoder);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(data).map_err(|e| e.to_string())
}

#[allow(dead_code)]
fn write_indexed_png(
    path: &Path,
    width: u32,
    height: u32,
    bit_depth: BitDepth,
    data: &[u8],
    palette: &[(u8, u8, u8, u8)],
) {
    let plte: Vec<u8> = palette.iter().flat_map(|&(r, g, b, _)| [r, g, b]).collect();
    let trns: Vec<u8> = palette.iter().map(|&(_, _, _, a)| a).collect();

    let bits_per_row = width as usize * bit_depth as u8 as usize;
    let row_bytes = bits_per_row.div_ceil(8);
    let mut packed = vec![0u8; row_bytes * height as usize];
    let ppb = 8 / bit_depth as u8 as usize;
    for (i, &idx) in data.iter().enumerate() {
        let row = i / width as usize;
        let col = i % width as usize;
        let byte_index = row * row_bytes + col / ppb;
        let shift = 8 - bit_depth as u8 as usize - (col % ppb) * bit_depth as u8 as usize;
        packed[byte_index] |= idx << shift;
    }

    write_png(path, width, height, &packed, |e| {
        e.set_color(ColorType::Indexed);
        e.set_depth(bit_depth);
        e.set_palette(plte);
        e.set_trns(trns);
    })
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::color::ReducedColor;

    fn nc(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> NormalizedColor {
        NormalizedColor::new(r, g, b, a)
    }

    #[test]
    fn load_rgba_png() {
        let path = Path::new("test_data/basic/rgba_red_transparent.png");
        let img = Image::load(path).unwrap();
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(img.color_at(0), nc(255, 0, 0, 255));
        assert_eq!(img.color_at(1), NormalizedColor::TRANSPARENT);
        assert!(img.palette.is_empty());
        assert_eq!(format!("{img}"), "2x1px RGB");
    }

    #[test]
    fn load_indexed_png() {
        let path = Path::new("test_data/basic/indexed_rgb.png");
        let img = Image::load(path).unwrap();
        assert_eq!(img.palette_size(), 3);
        assert_eq!(img.palette[1], nc(0, 255, 0, 128));
        assert_eq!(img.color_at(0), nc(255, 0, 0, 255));
        assert_eq!(img.color_at(1), nc(0, 255, 0, 128));
        assert_eq!(format!("{img}"), "3x1px indexed");
    }

    #[test]
    fn load_2bit_indexed_png() {
        let path = Path::new("test_data/basic/indexed_2bit.png");
        let img = Image::load(path).unwrap();
        assert_eq!(img.color_at(0), nc(0, 0, 0, 255));
        assert_eq!(img.color_at(1), nc(64, 64, 64, 255));
        assert_eq!(img.color_at(2), nc(128, 128, 128, 255));
        assert_eq!(img.color_at(3), nc(255, 255, 255, 255));
        assert_eq!(img.color_at(4), nc(128, 128, 128, 255));
        assert_eq!(img.color_at(5), nc(64, 64, 64, 255));
    }

    #[test]
    fn crop() {
        let path = Path::new("test_data/basic/rgba_rgbw.png");
        let img = Image::load(path).unwrap();
        let crop = img.crop(1, 0, 1, 1, Mode::Snes);
        assert_eq!((crop.width, crop.height), (1, 1));
        assert_eq!((crop.src_x, crop.src_y), (1, 0));
        assert_eq!(crop.color_at(0), nc(0, 255, 0, 255));
    }

    #[test]
    fn crop_past_edge() {
        let path = Path::new("test_data/basic/rgba_rgbw.png");
        let img = Image::load(path).unwrap();
        let crop = img.crop(0, 0, 3, 3, Mode::Snes);
        assert_eq!(crop.color_at(0), nc(255, 0, 0, 255));
        assert_eq!(crop.color_at(2), NormalizedColor::TRANSPARENT);
        assert_eq!(crop.color_at(6), NormalizedColor::TRANSPARENT);
    }

    #[test]
    fn crop_past_edge_gb() {
        let path = Path::new("test_data/basic/rgba_red.png");
        let img = Image::load(path).unwrap();
        let crop = img.crop(0, 0, 2, 1, Mode::Gb);
        assert_eq!(crop.color_at(1), nc(0, 0, 0, 0xff));
    }

    #[test]
    fn crops() {
        let path = Path::new("test_data/basic/indexed_rgb.png");
        let img = Image::load(path).unwrap();
        let tiles = img.crops(2, 2, Mode::Snes);
        assert_eq!(tiles.len(), 2);
        assert_eq!((tiles[0].src_x, tiles[0].src_y), (0, 0));
        assert_eq!((tiles[1].src_x, tiles[1].src_y), (2, 0));
    }

    #[test]
    fn remapped_palette_indices() {
        // 2x1: red, transparent
        let path = Path::new("test_data/basic/rgba_red_transparent.png");
        let img = Image::load(path).unwrap();

        let mut sp = Subpalette::new(Mode::Snes, 4);
        sp.add(ReducedColor::new(0, 0, 0, 0x00), false).unwrap();
        sp.add(ReducedColor::new(0, 0, 31, 0xff), false).unwrap();
        sp.add(ReducedColor::new(0, 0, 31, 0xff), false).unwrap();
        sp.add(ReducedColor::new(31, 0, 0, 0xff), false).unwrap();
        sp.add(ReducedColor::new(0, 31, 0, 0xff), false).unwrap();

        let remapped = img.remapped(&sp).unwrap();
        assert_eq!(remapped.palette_size(), 4);
        assert_eq!(remapped.indexed_data.len(), 2);
        assert_eq!(remapped.color_at(0), nc(255, 0, 0, 255));
        assert_eq!(remapped.color_at(1), NormalizedColor::TRANSPARENT);
        assert_eq!(remapped.indexed_data[0], 2);
        assert_eq!(remapped.indexed_data[1], 0);
    }

    #[test]
    fn remapped_missing_color() {
        let path = Path::new("test_data/basic/indexed_rgb.png");
        let img = Image::load(path).unwrap();
        let mut sp = Subpalette::new(Mode::Snes, 4);
        sp.add(ReducedColor::new(31, 0, 0, 0xff), false).unwrap();
        assert!(img.remapped(&sp).is_err());
    }
}

/// Load and sanity check indexed, rgb and grayscale samples from PngSuite.
#[cfg(test)]
mod pngsuite_test {
    use super::*;

    #[test]
    fn load_png_basic_indexed() {
        // basn3p01 - 1 bit (2 color) paletted
        let basn3p01 = Image::load(Path::new("test_data/pngsuite/basn3p01.png")).unwrap();
        assert_eq!(basn3p01.is_indexed(), true);
        assert_eq!(basn3p01.colors.len(), 2);
        // basn3p02 - 2 bit (4 color) paletted
        let basn3p02 = Image::load(Path::new("test_data/pngsuite/basn3p02.png")).unwrap();
        assert_eq!(basn3p02.is_indexed(), true);
        assert_eq!(basn3p02.colors.len(), 4);
        // basn3p04 - 4 bit (16 color, 15 present in sample) paletted
        let basn3p04 = Image::load(Path::new("test_data/pngsuite/basn3p04.png")).unwrap();
        assert_eq!(basn3p04.is_indexed(), true);
        assert_eq!(basn3p04.colors.len(), 15);
        // basn3p08 - 8 bit (256 color) paletted
        let basn3p08 = Image::load(Path::new("test_data/pngsuite/basn3p08.png")).unwrap();
        assert_eq!(basn3p08.is_indexed(), true);
        assert_eq!(basn3p08.colors.len(), 256);
    }

    #[test]
    fn load_png_basic_rgb() {
        // basn2c08 - 3x8 bits rgb color
        let basn2c08 = Image::load(Path::new("test_data/pngsuite/basn2c08.png")).unwrap();
        assert_eq!(basn2c08.is_indexed(), false);
        // basn2c16 - 3x16 bits rgb color
        let basn2c16 = Image::load(Path::new("test_data/pngsuite/basn2c16.png")).unwrap();
        assert_eq!(basn2c16.is_indexed(), false);
        // basn6a08 - 3x8 bits rgb color + 8 bit alpha-channel
        let basn6a08 = Image::load(Path::new("test_data/pngsuite/basn6a08.png")).unwrap();
        assert_eq!(basn6a08.is_indexed(), false);
        // basn6a16 - 3x16 bits rgb color + 16 bit alpha-channel
        let basn6a16 = Image::load(Path::new("test_data/pngsuite/basn6a16.png")).unwrap();
        assert_eq!(basn6a16.is_indexed(), false);
    }

    #[test]
    fn load_png_basic_grayscale() {
        // basn0g01 - black & white
        let basn0g01 = Image::load(Path::new("test_data/pngsuite/basn0g01.png")).unwrap();
        assert_eq!(basn0g01.is_indexed(), false);
        assert_eq!(basn0g01.colors.len(), 2);
        // basn0g02 - 2 bit (4 level) grayscale
        let basn0g02 = Image::load(Path::new("test_data/pngsuite/basn0g02.png")).unwrap();
        assert_eq!(basn0g02.is_indexed(), false);
        assert_eq!(basn0g02.colors.len(), 4);
        // basn0g04 - 4 bit (16 level, 15 present in sample) grayscale
        let basn0g04 = Image::load(Path::new("test_data/pngsuite/basn0g04.png")).unwrap();
        assert_eq!(basn0g04.is_indexed(), false);
        assert_eq!(basn0g04.colors.len(), 15);
        // basn0g08 - 8 bit (256 level) grayscale
        let basn0g08 = Image::load(Path::new("test_data/pngsuite/basn0g08.png")).unwrap();
        assert_eq!(basn0g08.is_indexed(), false);
        assert_eq!(basn0g08.colors.len(), 256);
        // basn4a08 - 8 bit grayscale + 8 bit alpha-channel
        let basn4a08 = Image::load(Path::new("test_data/pngsuite/basn4a08.png")).unwrap();
        assert_eq!(basn4a08.is_indexed(), false);
        assert_eq!(basn4a08.colors.len(), 1024);
    }

    #[test]
    fn load_png_corrupted() {
        for p in ["xcrn0g04", "xd3n2c08", "xs1n0g01", "xs2n0g01", "xs4n0g01", "xs7n0g01"] {
            let s = format!("test_data/pngsuite/{p}.png");
            let path = Path::new(&s);
            let img = Image::load(path);
            assert!(img.is_err());
        }
    }
}
