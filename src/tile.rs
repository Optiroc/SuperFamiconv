//! Tile type.

use crate::color::NormalizedColor;
use crate::image::{self, Image};
use crate::mode::{Mode, tile::ModeTile};
use crate::palette::palette_size_at_bpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TileFlip {
    pub h: bool,
    pub v: bool,
}

#[derive(Debug, Clone)]
pub struct Tile {
    mode: Mode,
    bpp: u32,
    width: u32,
    height: u32,
    data: Vec<u8>,
    /// H/V/HV-flipped variants of the tile data.
    mirrors: Vec<Vec<u8>>,
    /// Palette used when creating preview image.
    palette: Vec<NormalizedColor>,
}

impl Tile {
    pub fn with_mirrors(
        mode: Mode,
        bpp: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        palette: Vec<NormalizedColor>,
        no_flip: bool,
    ) -> Tile {
        let mirrors = if no_flip {
            Vec::new()
        } else {
            let h = flipped_h(&data, width);
            let v = flipped_v(&data, width);
            let hv = flipped_h(&v, width);
            vec![h, v, hv]
        };
        Tile {
            mode,
            bpp,
            width,
            height,
            data,
            mirrors,
            palette,
        }
    }

    /// Creates a Tile from an indexed color image.
    pub fn from_image(
        image: &Image,
        mode: Mode,
        bpp: u32,
        no_flip: bool,
    ) -> Result<Tile, String> {
        if !image.is_indexed() {
            return Err("Can't create tile from non-indexed image".into());
        }
        let mask = bitmask_at_bpp(bpp);
        let data: Vec<u8> = image.indexed_data.iter().map(|&i| i & mask).collect();
        Ok(Tile::with_mirrors(
            mode,
            bpp,
            image.width,
            image.height,
            data,
            image.palette.clone(),
            no_flip,
        ))
    }

    /// Creates a Tile by unpacking native bytes.
    ///
    /// The preview palette is a grayscale gradient, since there's no
    /// color information stored in the native data.
    pub fn from_native(
        data: &[u8],
        mode: Mode,
        bpp: u32,
        no_flip: bool,
        width: u32,
        height: u32,
    ) -> Result<Tile, String> {
        let unpacked = mode.unpack_tile(data, bpp, width, height)?;
        let palette = image::default_palette(palette_size_at_bpp(bpp) as usize);
        Ok(Tile::with_mirrors(mode, bpp, width, height, unpacked, palette, no_flip))
    }

    /// Creates a blank Tile (all index 0).
    pub fn blank(
        mode: Mode,
        bpp: u32,
        width: u32,
        height: u32,
    ) -> Tile {
        Tile {
            mode,
            bpp,
            width,
            height,
            data: vec![0u8; (width * height) as usize],
            mirrors: Vec::new(),
            palette: vec![NormalizedColor::TRANSPARENT; palette_size_at_bpp(bpp) as usize],
        }
    }

    /// Creates a Tile ("logical tile") from grid of native cells ("metatile").
    pub fn from_metatile(
        cells: &[Tile],
        no_flip: bool,
        width: u32,
        height: u32,
    ) -> Result<Tile, String> {
        let first = cells.first().ok_or("Can't build a tile from zero cells")?;
        let cell_dim = first.width;
        let cells_per_row = width / cell_dim;

        let mut data = vec![0u8; (width * height) as usize];
        for (i, cell) in cells.iter().enumerate() {
            let i = i as u32;
            let (cell_x, cell_y) = ((i % cells_per_row) * cell_dim, (i / cells_per_row) * cell_dim);
            for row in 0..cell_dim {
                let dst = (((cell_y + row) * width) + cell_x) as usize;
                let src = (row * cell_dim) as usize;
                data[dst..dst + cell_dim as usize].copy_from_slice(&cell.data[src..src + cell_dim as usize]);
            }
        }

        Ok(Tile::with_mirrors(
            first.mode,
            first.bpp,
            width,
            height,
            data,
            first.palette.clone(),
            no_flip,
        ))
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn palette(&self) -> &[NormalizedColor] {
        &self.palette
    }

    pub fn native_data(&self) -> Result<Vec<u8>, String> {
        self.mode.pack_tile(&self.data, self.bpp, self.width, self.height)
    }

    pub fn rgba_data(&self) -> Vec<NormalizedColor> {
        self.data.iter().map(|&i| self.palette[i as usize]).collect()
    }

    pub fn matches(
        &self,
        other_data: &[u8],
    ) -> bool {
        self.matching_flip(other_data).is_some()
    }

    pub fn matching_flip(
        &self,
        other_data: &[u8],
    ) -> Option<TileFlip> {
        if self.data == other_data {
            return Some(TileFlip::default());
        }
        if self.mirrors.first().is_some_and(|m| m == other_data) {
            return Some(TileFlip { h: true, v: false });
        }
        if self.mirrors.get(1).is_some_and(|m| m == other_data) {
            return Some(TileFlip { h: false, v: true });
        }
        if self.mirrors.get(2).is_some_and(|m| m == other_data) {
            return Some(TileFlip { h: true, v: true });
        }
        None
    }

    fn crop(
        &self,
        x: u32,
        y: u32,
        crop_width: u32,
        crop_height: u32,
    ) -> Tile {
        let mut data = vec![0u8; (crop_width * crop_height) as usize];
        if x <= self.width && y <= self.height {
            let w = crop_width.min(self.width.saturating_sub(x));
            let h = crop_height.min(self.height.saturating_sub(y));
            for row in 0..h {
                let src = (((y + row) * self.width) + x) as usize;
                let dst = (row * crop_width) as usize;
                data[dst..dst + w as usize].copy_from_slice(&self.data[src..src + w as usize]);
            }
        }
        Tile::with_mirrors(
            self.mode,
            self.bpp,
            crop_width,
            crop_height,
            data,
            self.palette.clone(),
            self.mirrors.is_empty(),
        )
    }

    /// Slices the tile into a row-major grid of `tile_width * tile_height` crops.
    pub fn crops(
        &self,
        tile_width: u32,
        tile_height: u32,
    ) -> Vec<Tile> {
        let mut out = Vec::new();
        let mut y = 0;
        while y < self.height {
            let mut x = 0;
            while x < self.width {
                out.push(self.crop(x, y, tile_width, tile_height));
                x += tile_width;
            }
            y += tile_height;
        }
        out
    }
}

const fn bitmask_at_bpp(bpp: u32) -> u8 {
    ((1u16 << bpp) - 1) as u8
}

pub fn flipped_h(
    source: &[u8],
    width: u32,
) -> Vec<u8> {
    source
        .chunks(width as usize)
        .flat_map(|row| row.iter().rev().copied())
        .collect()
}

fn flipped_v(
    source: &[u8],
    width: u32,
) -> Vec<u8> {
    source.chunks(width as usize).rev().flatten().copied().collect()
}

#[cfg(test)]
pub mod tests {
    use crate::color::ReducedColor;
    use crate::mode::color::ModeColor;
    use crate::palette::Subpalette;
    use std::path::Path;

    use super::*;

    pub fn make_tile(
        mode: Mode,
        bpp: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        mirrors: Vec<Vec<u8>>,
        palette: Vec<NormalizedColor>,
    ) -> Tile {
        Tile {
            mode,
            bpp,
            width,
            height,
            data,
            mirrors,
            palette,
        }
    }

    /// Creates an 8x8 indexed image filled with `color`.
    pub fn solid_image(
        mode: Mode,
        color: ReducedColor,
    ) -> Image {
        let mut sp = Subpalette::new(mode, 1);
        sp.add(color, false).unwrap();
        let fill_rgba = mode.normalize_color(color).to_bytes();
        let img = Image::from_rgba_data(8, 8, fill_rgba.repeat(64));
        img.remapped(&sp).unwrap()
    }

    /// Creates an 8x8 indexed image with pixel indices == column number.
    pub fn column_index_image() -> Image {
        let indices: Vec<u8> = (0..8).cycle().take(64).collect();
        let palette: Vec<NormalizedColor> = (0..8)
            .map(|i| NormalizedColor::new(i * 16, i * 16, i * 16, 255))
            .collect();
        Image::from_indexed_data(8, 8, indices, palette)
    }

    #[test]
    fn from_image_requires_indexed_data() {
        let path = Path::new("test_data/basic/rgba_red.png");
        let img = Image::load(path).unwrap();
        assert!(Tile::from_image(&img, Mode::Snes, 4, false).is_err());
    }

    #[test]
    fn bitmask_at_bpp_values() {
        assert_eq!(bitmask_at_bpp(1), 0x01);
        assert_eq!(bitmask_at_bpp(2), 0x03);
        assert_eq!(bitmask_at_bpp(4), 0x0f);
        assert_eq!(bitmask_at_bpp(8), 0xff);
    }

    #[test]
    fn matching_flip_and_matches() {
        let img = column_index_image();
        let tile = Tile::from_image(&img, Mode::Snes, 4, false).unwrap();
        let flipped_data = flipped_h(tile.data(), 8);
        assert_eq!(tile.matching_flip(&flipped_data), Some(TileFlip { h: true, v: false }));
        assert!(tile.matches(&flipped_data));
    }

    #[test]
    fn matching_flip_identity() {
        let img = column_index_image();
        let tile = Tile::from_image(&img, Mode::Snes, 4, false).unwrap();
        assert_eq!(tile.matching_flip(tile.data()), Some(TileFlip::default()));
    }

    #[test]
    fn matching_flip_none() {
        let img = column_index_image();
        let tile = Tile::from_image(&img, Mode::Snes, 4, false).unwrap();
        let other = vec![0xffu8; tile.data().len()];
        assert_eq!(tile.matching_flip(&other), None);
    }

    #[test]
    fn no_flip() {
        let img = column_index_image();
        let tile = Tile::from_image(&img, Mode::Snes, 4, true).unwrap();
        let flipped_data = flipped_h(tile.data(), 8);
        assert_ne!(tile.data(), flipped_data.as_slice());
        assert!(!tile.matches(&flipped_data));
    }
}
