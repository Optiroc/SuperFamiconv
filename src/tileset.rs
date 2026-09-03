//! Tileset type.

use crate::dither::Dither;
use crate::image::Image;
use crate::mode::Mode;
use crate::palette::Palette;
use crate::tile::Tile;

#[derive(Debug)]
pub struct Tileset {
    mode: Mode,
    bpp: u32,
    tile_width: u32,
    tile_height: u32,
    no_discard: bool,
    no_flip: bool,
    no_remap: bool,
    quantize: bool,
    dither: Dither,
    max_tiles: u32,
    tiles: Vec<Tile>,
    pub discarded_tiles: u32,
}

impl Tileset {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: Mode,
        bpp: u32,
        tile_width: u32,
        tile_height: u32,
        no_discard: bool,
        no_flip: bool,
        no_remap: bool,
        quantize: bool,
        dither: Dither,
        max_tiles: u32,
    ) -> Tileset {
        Tileset {
            mode,
            bpp,
            tile_width,
            tile_height,
            no_discard,
            no_flip,
            no_remap,
            quantize,
            dither,
            max_tiles,
            tiles: Vec::new(),
            discarded_tiles: 0,
        }
    }

    pub fn tile_width(&self) -> u32 {
        self.tile_width
    }

    pub fn tile_height(&self) -> u32 {
        self.tile_height
    }

    pub fn size(&self) -> usize {
        self.tiles.len()
    }

    pub fn max(&self) -> u32 {
        self.max_tiles
    }

    pub fn is_full(&self) -> bool {
        self.max_tiles > 0 && self.tiles.len() as u32 > self.max_tiles
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    /// Index of the first stored tile that matches `tile`.
    pub fn index_of(
        &self,
        tile: &Tile,
    ) -> Option<usize> {
        self.tiles.iter().position(|t| t.matches(tile.data()))
    }

    /// Takes `image` and converts to a Tile.
    ///
    /// Maps color indices straight from indexed data in `image` if `no_remap`, else
    /// from `palette`, then it is added if an identical tile is not already present.
    pub fn add(
        &mut self,
        image: &Image,
        palette: Option<&Palette>,
    ) -> Result<(), String> {
        let tile = if self.no_remap {
            Tile::from_image(image, self.mode, self.bpp, self.no_flip)?
        } else {
            let palette = palette.ok_or("Can't remap tile without palette")?;
            let remapped = if self.quantize {
                let subpalette = palette
                    .subpalettes_by_distance(image)
                    .first()
                    .copied()
                    .ok_or("Palette has no subpalettes")?;
                image.remapped_quantized(subpalette, self.dither)?
            } else {
                let subpalette = palette.subpalette_matching(image)?;
                image.remapped(subpalette)?
            };
            Tile::from_image(&remapped, self.mode, self.bpp, self.no_flip)?
        };

        if self.no_discard {
            self.tiles.push(tile);
        } else if self.tiles.iter().any(|t| t.matches(tile.data())) {
            self.discarded_tiles += 1;
        } else {
            self.tiles.push(tile);
        }
        Ok(())
    }

    pub fn to_native_data(&self) -> Result<Vec<u8>, String> {
        let owned;
        let tiles: &[Tile] = if self.mode != Mode::PceSprite && (self.tile_width != 8 || self.tile_height != 8) {
            owned = remap_tiles_for_output(&self.tiles, self.mode, self.bpp, self.tile_width, self.tile_height);
            &owned
        } else {
            &self.tiles
        };

        let mut data = Vec::new();
        for tile in tiles {
            data.extend(tile.native_data()?);
        }
        Ok(data)
    }

    pub fn from_native_data(
        data: &[u8],
        mode: Mode,
        bpp: u32,
        tile_width: u32,
        tile_height: u32,
        no_flip: bool,
    ) -> Result<Tileset, String> {
        if mode == Mode::PceSprite {
            return Err("Reading 'pce_sprite' native tile data is not implemented".into());
        }

        let bytes_per_cell = (bpp * 8) as usize;
        if bytes_per_cell == 0 || !data.len().is_multiple_of(bytes_per_cell) {
            return Err("Tile data can't be deserialized (size doesn't match bpp setting)".into());
        }

        let cells: Vec<Tile> = data
            .chunks_exact(bytes_per_cell)
            .map(|chunk| Tile::from_native(chunk, mode, bpp, no_flip, 8, 8))
            .collect::<Result<_, _>>()?;

        let tiles = if tile_width != 8 || tile_height != 8 {
            remap_tiles_for_input(&cells, mode, tile_width, tile_height, no_flip)?
        } else {
            cells
        };

        Ok(Tileset {
            mode,
            bpp,
            tile_width,
            tile_height,
            no_discard: false,
            no_flip,
            no_remap: false,
            quantize: false,
            dither: Dither::Off,
            max_tiles: 0,
            tiles,
            discarded_tiles: 0,
        })
    }
}

/// The native tile index a Map entry references for the nth "logical tile"
/// in a tileset laid out by `remap_tiles_for_output`.
pub fn native_tile_index(
    n: u32,
    mode: Mode,
    tile_width: u32,
    tile_height: u32,
) -> u32 {
    if tile_width == 8 && tile_height == 8 {
        return n;
    }
    let cells_per_tile_h = tile_width / 8;
    let cells_per_tile_v = tile_height / 8;
    let cells_per_row: u32 = if mode == Mode::Snes { 16 } else { 1 };
    let tiles_per_row = cells_per_row / cells_per_tile_h;
    ((n / tiles_per_row) * cells_per_tile_v * cells_per_row) + ((n % tiles_per_row) * cells_per_tile_h)
}

/// Inverse of `native_tile_index`: the "logical tile index" corresponding to
/// the nth native tile index.
pub fn logical_tile_index(
    n: u32,
    mode: Mode,
    tile_width: u32,
    tile_height: u32,
) -> u32 {
    if tile_width == 8 && tile_height == 8 {
        return n;
    }
    let cells_per_tile_h = tile_width / 8;
    let cells_per_tile_v = tile_height / 8;
    let cells_per_row: u32 = if mode == Mode::Snes { 16 } else { 1 };
    let tiles_per_row = cells_per_row / cells_per_tile_h;
    let metatile_row = (n / cells_per_row) / cells_per_tile_v;
    let metatile_col = (n % cells_per_row) / cells_per_tile_h;
    metatile_row * tiles_per_row + metatile_col
}

/// Lays out "logical tiles" as a list of native 8x8 "cells".
fn remap_tiles_for_output(
    tiles: &[Tile],
    mode: Mode,
    bpp: u32,
    tile_width: u32,
    tile_height: u32,
) -> Vec<Tile> {
    let cells_per_tile_h = tile_width / 8;
    let cells_per_tile_v = tile_height / 8;
    let cells_per_row: u32 = if mode == Mode::Snes { 16 } else { 1 };
    let tiles_per_row = cells_per_row / cells_per_tile_h;
    let cell_rows = (tiles.len() as u32).div_ceil(tiles_per_row) * cells_per_tile_v;

    let mut grid: Vec<Tile> = (0..(cells_per_row * cell_rows))
        .map(|_| Tile::blank(mode, bpp, 8, 8))
        .collect();

    for (i, tile) in tiles.iter().enumerate() {
        let base = native_tile_index(i as u32, mode, tile_width, tile_height);
        let cells = tile.crops(8, 8);
        for cy in 0..cells_per_tile_v {
            for cx in 0..cells_per_tile_h {
                grid[(base + (cy * cells_per_row) + cx) as usize] =
                    cells[(cy * cells_per_tile_h + cx) as usize].clone();
            }
        }
    }
    grid
}

/// Inverse of `remap_tiles_for_output`: groups a list of native 8x8 "cells" into "logical tiles".
fn remap_tiles_for_input(
    cells: &[Tile],
    mode: Mode,
    tile_width: u32,
    tile_height: u32,
    no_flip: bool,
) -> Result<Vec<Tile>, String> {
    let cells_per_tile_h = tile_width / 8;
    let cells_per_tile_v = tile_height / 8;
    let cells_per_row: u32 = if mode == Mode::Snes { 16 } else { 1 };

    let mut tiles = Vec::new();
    let mut i = 0u32;
    loop {
        let base = native_tile_index(i, mode, tile_width, tile_height);
        let metatile: Option<Vec<Tile>> = (0..cells_per_tile_v)
            .flat_map(|cy| (0..cells_per_tile_h).map(move |cx| base + cy * cells_per_row + cx))
            .map(|idx| cells.get(idx as usize).cloned())
            .collect();

        let Some(metatile) = metatile else { break };
        tiles.push(Tile::from_metatile(&metatile, no_flip, tile_width, tile_height)?);
        i += 1;
    }
    Ok(tiles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ReducedColor;
    use crate::tile;
    use crate::tile::tests::{column_index_image, make_tile, solid_image};

    #[test]
    fn tileset_add_discard() {
        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, false, false, true, false, Dither::Off, 0);
        let img = solid_image(Mode::Snes, ReducedColor::new(31, 0, 0, 0xff));
        ts.add(&img, None).unwrap();
        ts.add(&img, None).unwrap();
        assert_eq!(ts.size(), 1);
        assert_eq!(ts.discarded_tiles, 1);
    }

    #[test]
    fn tileset_add_no_discard() {
        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, true, false, true, false, Dither::Off, 0);
        let img = solid_image(Mode::Snes, ReducedColor::new(31, 0, 0, 0xff));
        ts.add(&img, None).unwrap();
        ts.add(&img, None).unwrap();
        assert_eq!(ts.size(), 2);
        assert_eq!(ts.discarded_tiles, 0);
    }

    #[test]
    fn tileset_no_remap_requires_no_palette() {
        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, false, false, true, false, Dither::Off, 0);
        let img = solid_image(Mode::Snes, ReducedColor::new(31, 0, 0, 0xff));
        assert!(ts.add(&img, None).is_ok());
    }

    #[test]
    fn tileset_remap_requires_palette() {
        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, false, false, false, false, Dither::Off, 0);
        let img = column_index_image();
        assert!(ts.add(&img, None).is_err());
    }

    #[test]
    fn tileset_index_of_finds_flip_aware_match() {
        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, true, false, true, false, Dither::Off, 0);
        let base = solid_image(Mode::Snes, ReducedColor::new(31, 0, 0, 0xff));
        ts.add(&base, None).unwrap();

        let column = column_index_image();
        let tile = Tile::from_image(&column, Mode::Snes, 4, true).unwrap();
        assert_eq!(ts.index_of(&tile), None);

        let mut ts2 = Tileset::new(Mode::Snes, 4, 8, 8, true, false, true, false, Dither::Off, 0);
        ts2.add(&column, None).unwrap();
        let flipped_data = tile::flipped_h(tile.data(), 8);
        let flipped_tile = Tile::with_mirrors(Mode::Snes, 4, 8, 8, flipped_data, vec![], true);
        assert_eq!(ts2.index_of(&flipped_tile), Some(0));
    }

    #[test]
    fn tileset_native_data_roundtrip() {
        let mut ts = Tileset::new(Mode::Snes, 4, 8, 8, true, true, true, false, Dither::Off, 0);
        ts.add(&column_index_image(), None).unwrap();
        let native = ts.to_native_data().unwrap();

        let loaded = Tileset::from_native_data(&native, Mode::Snes, 4, 8, 8, true).unwrap();
        assert_eq!(loaded.size(), 1);
        assert_eq!(loaded.tiles()[0].data(), ts.tiles()[0].data());
    }

    #[test]
    fn logical_tile_index_is_inverse_of_native_tile_index() {
        for (mode, tile_width, tile_height) in [
            (Mode::Snes, 8, 8),
            (Mode::Snes, 16, 16),
            (Mode::Gb, 8, 8),
            (Mode::Gba, 8, 8),
        ] {
            for n in 0..40u32 {
                let native = native_tile_index(n, mode, tile_width, tile_height);
                assert_eq!(logical_tile_index(native, mode, tile_width, tile_height), n, "n={n}");
            }
        }
    }

    #[test]
    fn snes_16x16_tile_roundtrip() {
        // 3 16x16 "logical tiles"
        let mut logical_tiles = Vec::new();
        for t in 0..3u8 {
            let mut cells = Vec::new();
            for c in 0..4u8 {
                let data = vec![t * 4 + c; 64];
                cells.push(make_tile(Mode::Snes, 4, 8, 8, data, Vec::new(), vec![]));
            }
            logical_tiles.push(Tile::from_metatile(&cells, true, 16, 16).unwrap());
        }

        // Map to 8x8 output tiles
        let output = remap_tiles_for_output(&logical_tiles, Mode::Snes, 4, 16, 16);

        // Output is padded to two 16-cells-wide rows:
        // 0123456789abcdef
        // 001122----------
        // 001122----------
        assert_eq!(output.len(), 16 * 2);

        // Map output back to 16x16 tiles, which will include the padding
        let reconstructed = remap_tiles_for_input(&output, Mode::Snes, 16, 16, true).unwrap();
        assert_eq!(reconstructed.len(), 8);

        for (org, rec) in logical_tiles.iter().zip(reconstructed.iter()) {
            assert_eq!(org.data(), rec.data());
        }
    }
}
