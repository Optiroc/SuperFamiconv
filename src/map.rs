//! Map representation.

use crate::dither::Dither;
use crate::image::Image;
use crate::mode::{Mode, map::ModeMap};
use crate::palette::Palette;
use crate::tile::Tile;
use crate::tileset::{self, Tileset};

#[derive(Debug)]
pub struct Map {
    mode: Mode,
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
    quantize: bool,
    dither: Dither,
    entries: Vec<Mapentry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mapentry {
    pub tile_index: u32,
    pub palette_index: u32,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl Mapentry {
    pub fn new(
        tile_index: u32,
        palette_index: u32,
        flip_h: bool,
        flip_v: bool,
    ) -> Self {
        Mapentry {
            tile_index,
            palette_index,
            flip_h,
            flip_v,
        }
    }
}

impl Map {
    pub fn new(
        mode: Mode,
        width: u32,
        height: u32,
        tile_width: u32,
        tile_height: u32,
        quantize: bool,
        dither: Dither,
    ) -> Self {
        Map {
            mode,
            width,
            height,
            tile_width,
            tile_height,
            quantize,
            dither,
            entries: vec![Mapentry::default(); (width * height) as usize],
        }
    }

    /// Finds entry in `tileset` matching `image`, searching every viable subpalette.
    /// - If match found: add entry to map and return true.
    /// - If no match or matched index > max tile count: print message to stderr, add blank entry and return false.
    pub fn add(
        &mut self,
        image: &Image,
        tileset: &Tileset,
        palette: &Palette,
        bpp: u32,
        pos_x: u32,
        pos_y: u32,
    ) -> Result<bool, String> {
        let index = (pos_y * self.width + pos_x) as usize;
        if index >= self.entries.len() {
            return Err("Map entry out of bounds".into());
        }

        // Search all viable palette mappings of image in tileset
        let mut status = true;
        let mut found: Option<(usize, usize, Tile)> = None;
        let candidates = if self.quantize {
            palette.subpalettes_by_distance(image)
        } else {
            palette.subpalettes_matching(image)?
        };
        for candidate in candidates {
            let remapped_image = if self.quantize {
                image.remapped_quantized(candidate, self.dither)?
            } else {
                image.remapped(candidate)?
            };
            let remapped_tile = Tile::from_image(&remapped_image, self.mode, bpp, true)?;
            if let Some(tileset_index) = tileset.index_of(&remapped_tile) {
                let palette_index = palette.index_of(candidate).unwrap();
                found = Some((tileset_index, palette_index, remapped_tile));
                break;
            }
        }

        self.entries[index] = match found {
            None => {
                eprintln!("> No matching tile for position ({}, {})", image.src_x, image.src_y);
                status = false;
                Mapentry::default()
            }
            Some((tileset_index, _, _)) if tileset_index >= self.mode.max_tile_count() as usize => {
                eprintln!(
                    "> Mapped tile exceeds allowed map index at position ({}, {})",
                    image.src_x, image.src_y
                );
                status = false;
                Mapentry::default()
            }
            Some((tileset_index, palette_index, matched_tile)) => {
                let flip = tileset.tiles()[tileset_index]
                    .matching_flip(matched_tile.data())
                    .unwrap();
                Mapentry::new(tileset_index as u32, palette_index as u32, flip.h, flip.v)
            }
        };

        Ok(status)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn tile_width(&self) -> u32 {
        self.tile_width
    }

    pub fn tile_height(&self) -> u32 {
        self.tile_height
    }

    pub fn add_base_offset(
        &mut self,
        offset: i32,
    ) {
        for e in &mut self.entries {
            e.tile_index = e.tile_index.saturating_add_signed(offset);
        }
    }

    pub fn add_palette_base_offset(
        &mut self,
        offset: i32,
    ) {
        for e in &mut self.entries {
            e.palette_index = e.palette_index.saturating_add_signed(offset);
        }
    }

    /// The entry at `(x, y)` in "logical tiles" coordinate space.
    pub fn entry_at(
        &self,
        x: u32,
        y: u32,
    ) -> Mapentry {
        self.entries[(y * self.width + x) as usize]
    }

    /// The entry at `(x, y)`, with its tile index converted from "logical tiles"
    /// coordinate space to the tile index used in native map data.
    fn native_entry_at(
        &self,
        x: u32,
        y: u32,
    ) -> Mapentry {
        if x >= self.width || y >= self.height {
            return Mapentry::default();
        }
        let mut entry = self.entries[(y * self.width + x) as usize];
        entry.tile_index = tileset::native_tile_index(entry.tile_index, self.mode, self.tile_width, self.tile_height);
        entry
    }

    /// Groups entries into `split_w` * `split_h` chunks.
    fn collect_entries(
        &self,
        split_w: u32,
        split_h: u32,
        column_order: bool,
    ) -> Vec<Vec<Mapentry>> {
        let split_w = if split_w == 0 || split_w > self.width {
            self.width
        } else {
            split_w
        };
        let split_h = if split_h == 0 || split_h > self.height {
            self.height
        } else {
            split_h
        };

        let mut groups: Vec<Vec<Mapentry>> = if split_w == self.width && split_h == self.height {
            vec![
                (0..self.height)
                    .flat_map(|y| (0..self.width).map(move |x| (x, y)))
                    .map(|(x, y)| self.native_entry_at(x, y))
                    .collect(),
            ]
        } else {
            let columns = self.width.div_ceil(split_w).max(1);
            let rows = self.height.div_ceil(split_h).max(1);
            let mut groups = Vec::with_capacity((columns * rows) as usize);
            for col in 0..columns {
                for row in 0..rows {
                    let group: Vec<Mapentry> = (0..split_w * split_h)
                        .map(|pos| self.native_entry_at(col * split_w + pos % split_w, row * split_h + pos / split_w))
                        .collect();
                    groups.push(group);
                }
            }
            groups
        };

        if column_order {
            for group in &mut groups {
                let original = group.clone();
                for (pos, entry) in group.iter_mut().enumerate() {
                    let pos = pos as u32;
                    let src = (pos % split_h) * split_w + pos / split_h;
                    *entry = original[src as usize];
                }
            }
        }
        groups
    }

    pub fn to_native_data(
        &self,
        split_w: u32,
        split_h: u32,
        column_order: bool,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        for group in self.collect_entries(split_w, split_h, column_order) {
            for entry in group {
                data.extend(self.mode.pack_mapentry(entry));
            }
        }
        data
    }

    pub fn from_native_data(
        data: &[u8],
        mode: Mode,
        width: u32,
        height: u32,
        tile_width: u32,
        tile_height: u32,
        split_width: u32,
        split_height: u32,
        column_order: bool,
    ) -> Result<Map, String> {
        let entry_size = mode.mapentry_size();
        if entry_size == 0 {
            return Err(format!("Map data can't be read for mode '{mode}'"));
        }

        let split_w = if split_width == 0 || split_width > width {
            width
        } else {
            split_width
        };
        let split_h = if split_height == 0 || split_height > height {
            height
        } else {
            split_height
        };

        let (group_count, group_len) = if split_w == width && split_h == height {
            (1, width * height)
        } else {
            let columns = width.div_ceil(split_w).max(1);
            let rows = height.div_ceil(split_h).max(1);
            (columns * rows, split_w * split_h)
        };

        let expected_len = (group_count * group_len) as usize * entry_size;
        if data.len() != expected_len {
            return Err(format!(
                "Map data can't be deserialized (got {} bytes, expected {expected_len})",
                data.len()
            ));
        }

        // Unpack entries
        let raw_entries: Vec<Mapentry> = data.chunks_exact(entry_size).map(|c| mode.unpack_mapentry(c)).collect();

        // Place entries
        let mut entries = vec![Mapentry::default(); (width * height) as usize];
        for (group_idx, chunk) in raw_entries.chunks_exact(group_len as usize).enumerate() {
            let group_idx = group_idx as u32;
            let mut current_group = vec![Mapentry::default(); group_len as usize];

            if column_order {
                for (pos, &entry) in chunk.iter().enumerate() {
                    let pos = pos as u32;
                    let src = (pos % split_h) * split_w + pos / split_h;
                    current_group[src as usize] = entry;
                }
            } else {
                current_group.copy_from_slice(chunk);
            }

            let (col, row) = if group_count == 1 {
                (0, 0)
            } else {
                let rows = height.div_ceil(split_h).max(1);
                (group_idx / rows, group_idx % rows)
            };

            for (pos, &entry) in current_group.iter().enumerate() {
                let pos = pos as u32;
                let (x, y) = if group_count == 1 {
                    (pos % width, pos / width)
                } else {
                    (col * split_w + pos % split_w, row * split_h + pos / split_w)
                };
                if x >= width || y >= height {
                    continue; // padding, keep default Mapentry
                }
                let mut entry = entry;
                entry.tile_index = tileset::logical_tile_index(entry.tile_index, mode, tile_width, tile_height);
                entries[(y * width + x) as usize] = entry;
            }
        }

        Ok(Map {
            mode,
            width,
            height,
            tile_width,
            tile_height,
            quantize: false,
            dither: Dither::Off,
            entries,
        })
    }

    pub fn get_snes_mode7_interleaved_data(
        &self,
        tileset: &Tileset,
    ) -> Result<Vec<u8>, String> {
        let map_data = self.to_native_data(0, 0, false);
        let tile_data = tileset.to_native_data()?;

        let size = map_data.len().max(tile_data.len());
        let mut data = vec![0u8; size * 2];
        for (i, &b) in map_data.iter().enumerate() {
            data[i << 1] = b;
        }
        for (i, &b) in tile_data.iter().enumerate() {
            data[(i << 1) + 1] = b;
        }
        Ok(data)
    }

    pub fn get_gbc_banked_data(&self) -> Result<Vec<u8>, String> {
        if !self.width.is_multiple_of(32) || !self.height.is_multiple_of(32) {
            return Err("gbc/out-gbc-bank requires map dimensions to be multiples of 32".into());
        }
        let linear_data = self.to_native_data(0, 0, false);
        let half = linear_data.len() / 2;
        let mut banked_data = vec![0u8; linear_data.len()];
        for i in 0..half {
            banked_data[i] = linear_data[i << 1];
            banked_data[i + half] = linear_data[(i << 1) + 1];
        }
        Ok(banked_data)
    }

    pub fn get_palette_map(
        &self,
        split_w: u32,
        split_h: u32,
        column_order: bool,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        for group in self.collect_entries(split_w, split_h, column_order) {
            for entry in group {
                data.push((entry.palette_index & 0xff) as u8);
                data.push((entry.palette_index >> 8) as u8);
            }
        }
        data
    }

    pub fn to_json(
        &self,
        split_w: u32,
        split_h: u32,
        column_order: bool,
    ) -> String {
        let flip_allowed = self.mode.tile_flipping_is_allowed();
        let multi_palette = self.mode.default_palette_count() > 1;

        let entry_json = |m: &Mapentry| -> serde_json::Value {
            match (flip_allowed, multi_palette) {
                (true, true) => serde_json::json!({
                    "tile": m.tile_index,
                    "palette": m.palette_index,
                    "flip_h": u8::from(m.flip_h),
                    "flip_v": u8::from(m.flip_v),
                }),
                (true, false) => serde_json::json!({
                    "tile": m.tile_index,
                    "flip_h": u8::from(m.flip_h),
                    "flip_v": u8::from(m.flip_v),
                }),
                (false, true) => serde_json::json!({
                    "tile": m.tile_index,
                    "palette": m.palette_index,
                }),
                (false, false) => serde_json::json!({ "tile": m.tile_index }),
            }
        };

        let groups: Vec<Vec<serde_json::Value>> = self
            .collect_entries(split_w, split_h, column_order)
            .iter()
            .map(|g| g.iter().map(entry_json).collect())
            .collect();

        let json = if groups.len() > 1 {
            serde_json::json!({ "maps": groups })
        } else {
            serde_json::json!({ "map": groups.into_iter().next().unwrap_or_default() })
        };
        serde_json::to_string_pretty(&json).unwrap()
    }

    pub fn description(
        &self,
        split_width: u32,
        split_height: u32,
        column_order: bool,
    ) -> String {
        let (w, split_w) = if split_width == 0 || split_width > self.width {
            (self.width, self.width)
        } else {
            (self.width, split_width)
        };
        let (h, split_h) = if split_height == 0 || split_height > self.height {
            (self.height, self.height)
        } else {
            (self.height, split_height)
        };

        if split_w == w && split_h == h {
            format!("single group, {w}x{h} entries")
        } else {
            let cols = w.div_ceil(split_w).max(1);
            let rows = h.div_ceil(split_h).max(1);
            format!(
                "{cols}x{rows} groups, {split_w}x{split_h} entries each{}",
                if column_order { ", column-major" } else { "" }
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::NormalizedColor;
    use crate::mode::color::ModeColor;
    use crate::palette::Palette;

    /// A Map with increasing tile indices.
    fn mock_map(
        mode: Mode,
        width: u32,
        height: u32,
    ) -> Map {
        let mut map = Map::new(mode, width, height, 8, 8, false, Dither::Off);
        for (i, e) in map.entries.iter_mut().enumerate() {
            e.tile_index = i as u32;
        }
        map
    }

    /// An 8x8 Image with `colors` repeated vertically.
    fn mock_image(colors: &[NormalizedColor; 8]) -> Image {
        let row: Vec<u8> = colors.iter().flat_map(|c| c.to_bytes()).collect();
        let pixels: Vec<u8> = row.repeat(8);
        Image::from_rgba_data(8, 8, pixels)
    }

    fn gray_gradient() -> [NormalizedColor; 8] {
        std::array::from_fn(|i| NormalizedColor::new((i as u8) * 16, (i as u8) * 16, (i as u8) * 16, 255))
    }

    fn palette_for(
        mode: Mode,
        colors: &[NormalizedColor],
    ) -> Palette {
        let mut pal = Palette::new(mode, 8, 8);
        let reduced: Vec<_> = colors.iter().map(|&c| mode.reduce_color(c)).collect();
        pal.add_colors(&reduced).unwrap();
        pal
    }

    #[test]
    fn add_no_flip() {
        let colors = gray_gradient();
        let pal = palette_for(Mode::Snes, &colors);
        let image = mock_image(&colors);

        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, true, false, false, false, Dither::Off, 0);
        ts.add(&image, Some(&pal)).unwrap();

        let mut map = Map::new(Mode::Snes, 1, 1, 8, 8, false, Dither::Off);
        assert!(map.add(&image, &ts, &pal, 4, 0, 0).unwrap());
        assert_eq!(map.entries[0], Mapentry::new(0, 0, false, false));
    }

    #[test]
    fn add_hflip() {
        let colors = gray_gradient();
        let pal = palette_for(Mode::Snes, &colors);
        let image = mock_image(&colors);

        let mut flipped_colors = colors;
        flipped_colors.reverse();
        let flipped_image = mock_image(&flipped_colors);

        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, true, false, false, false, Dither::Off, 0);
        ts.add(&image, Some(&pal)).unwrap();

        let mut map = Map::new(Mode::Snes, 1, 1, 8, 8, false, Dither::Off);
        assert!(map.add(&flipped_image, &ts, &pal, 4, 0, 0).unwrap());
        assert_eq!(map.entries[0], Mapentry::new(0, 0, true, false));
    }

    #[test]
    fn add_no_matching_tile() {
        let colors = gray_gradient();
        let pal = palette_for(Mode::Snes, &colors);
        let image = mock_image(&colors);

        let ts = Tileset::new(Mode::Snes, 4, 8, 8, true, false, true, false, Dither::Off, 0);
        let mut map = Map::new(Mode::Snes, 1, 1, 8, 8, false, Dither::Off);
        assert!(!map.add(&image, &ts, &pal, 4, 0, 0).unwrap());
        assert_eq!(map.entries[0], Mapentry::default());
    }

    #[test]
    fn add_out_of_bounds() {
        let colors = gray_gradient();
        let pal = palette_for(Mode::Snes, &colors);
        let image = mock_image(&colors);

        let ts = Tileset::new(Mode::Snes, 4, 8, 8, true, false, true, false, Dither::Off, 0);
        let mut map = Map::new(Mode::Snes, 1, 1, 8, 8, false, Dither::Off);
        assert!(map.add(&image, &ts, &pal, 4, 5, 5).is_err());
    }

    #[test]
    fn native_tile_index_16x16() {
        let mut map = Map::new(Mode::Snes, 2, 1, 16, 16, false, Dither::Off);
        map.entries[0].tile_index = 0;
        map.entries[1].tile_index = 1;
        let groups = map.collect_entries(0, 0, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0][0].tile_index,
            tileset::native_tile_index(0, Mode::Snes, 16, 16)
        );
        assert_eq!(
            groups[0][1].tile_index,
            tileset::native_tile_index(1, Mode::Snes, 16, 16)
        );
        assert_eq!(groups[0][1].tile_index, 2);
    }

    #[test]
    fn collect_entries() {
        let map = mock_map(Mode::Gb, 4, 4);
        let groups = map.collect_entries(0, 0, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 16);
    }

    #[test]
    fn collect_entries_splits() {
        let map = mock_map(Mode::Gb, 4, 4);
        let groups = map.collect_entries(2, 2, false);
        assert_eq!(groups.len(), 4);
        let tiles: Vec<Vec<u32>> = groups
            .iter()
            .map(|g| g.iter().map(|e| e.tile_index).collect())
            .collect();
        assert_eq!(tiles[0], vec![0, 1, 4, 5]); // top-left 2x2
        assert_eq!(tiles[1], vec![8, 9, 12, 13]); // bottom-left 2x2
        assert_eq!(tiles[2], vec![2, 3, 6, 7]); // top-right 2x2
        assert_eq!(tiles[3], vec![10, 11, 14, 15]); // bottom-right 2x2
    }

    #[test]
    fn collect_entries_padding() {
        let map = mock_map(Mode::Gb, 3, 1);
        let groups = map.collect_entries(2, 1, false);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0][0].tile_index, 0);
        assert_eq!(groups[0][1].tile_index, 1);
        assert_eq!(groups[1][0].tile_index, 2);
        assert_eq!(groups[1][1], Mapentry::default()); // padded
    }

    #[test]
    fn collect_entries_column_order() {
        // 2x3 map, values 0..5 -> 0,2,4,1,3,5.
        let map = mock_map(Mode::Gb, 2, 3);
        let groups = map.collect_entries(0, 0, true);
        assert_eq!(groups.len(), 1);
        let tiles: Vec<u32> = groups[0].iter().map(|e| e.tile_index).collect();
        assert_eq!(tiles, vec![0, 2, 4, 1, 3, 5]);
    }

    #[test]
    fn to_native_data() {
        let mut map = Map::new(Mode::Gb, 2, 1, 8, 8, false, Dither::Off);
        map.entries[0].tile_index = 1;
        map.entries[1].tile_index = 2;
        assert_eq!(map.to_native_data(0, 0, false), vec![1, 2]);
    }

    #[test]
    fn from_native_data_roundtrip_single_group() {
        let map = mock_map(Mode::Gb, 4, 4);
        let data = map.to_native_data(0, 0, false);
        let from_native = Map::from_native_data(&data, Mode::Gb, 4, 4, 8, 8, 0, 0, false).unwrap();
        assert_eq!(from_native.entries, map.entries);
    }

    #[test]
    fn from_native_data_splits() {
        let map = mock_map(Mode::Gb, 4, 4);
        let data = map.to_native_data(2, 2, false);
        let from_native = Map::from_native_data(&data, Mode::Gb, 4, 4, 8, 8, 2, 2, false).unwrap();
        assert_eq!(from_native.entries, map.entries);
    }

    #[test]
    fn from_native_data_padded() {
        let map = mock_map(Mode::Gb, 3, 1);
        let data = map.to_native_data(2, 1, false);
        let from_native = Map::from_native_data(&data, Mode::Gb, 3, 1, 8, 8, 2, 1, false).unwrap();
        assert_eq!(from_native.entries, map.entries);
    }

    #[test]
    fn from_native_data_16x16() {
        let mut map = Map::new(Mode::Snes, 2, 1, 16, 16, false, Dither::Off);
        map.entries[0].tile_index = 0;
        map.entries[1].tile_index = 1;
        let data = map.to_native_data(0, 0, false);
        let from_native = Map::from_native_data(&data, Mode::Snes, 2, 1, 16, 16, 0, 0, false).unwrap();
        assert_eq!(from_native.entries, map.entries);
    }

    #[test]
    fn get_palette_map_16bit_le_entries() {
        let mut map = Map::new(Mode::Snes, 1, 1, 8, 8, false, Dither::Off);
        map.entries[0].palette_index = 0x0102;
        assert_eq!(map.get_palette_map(0, 0, false), vec![0x02, 0x01]);
    }

    #[test]
    fn get_snes_mode7_interleaved_data() {
        let mut map = Map::new(Mode::SnesMode7, 2, 1, 8, 8, false, Dither::Off);
        map.entries[0].tile_index = 0xaa;
        map.entries[1].tile_index = 0xbb;

        let ts = Tileset::new(Mode::SnesMode7, 8, 8, 8, true, true, true, false, Dither::Off, 0);
        let data = map.get_snes_mode7_interleaved_data(&ts).unwrap();
        assert_eq!(data[0], 0xaa);
        assert_eq!(data[1], 0x00);
        assert_eq!(data[2], 0xbb);
        assert_eq!(data[3], 0x00);
        assert!(data.len() >= 4);
    }

    #[test]
    fn get_gbc_banked_data() {
        let mut map = Map::new(Mode::Gbc, 32, 32, 8, 8, false, Dither::Off);
        map.entries[0] = Mapentry::new(0x0100, 3, false, false); // 2nd byte non-zero
        let banked = map.get_gbc_banked_data().unwrap();
        let linear = map.to_native_data(0, 0, false);
        assert_eq!(banked[0], linear[0]);
        assert_eq!(banked[linear.len() / 2], linear[1]);
    }
}
