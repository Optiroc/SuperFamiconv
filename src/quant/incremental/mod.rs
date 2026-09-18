//! Incremental k-means based tile-aware palette quantization.
//!
//! Based on the approach used in "TiledPaletteQuant" by Rilden:
//! - <https://github.com/rilden/tiledpalettequant>

mod dither;
mod palette;
mod quantizer;
mod stats;

use std::cell::Cell;

use quantette::deps::palette::Oklab;
use quantette::wu::BinnerF32x3;

use self::dither::IncrementalDither;
use self::palette::{Settings, SubpaletteView};
use super::{dedup_reduced, grow_to_capacity};
use crate::color::{CandidateColor, NormalizedColor, ReducedColor, eq_rgb};
use crate::dither::Dither;
use crate::image::{self, Image};
use crate::mode::{
    Mode,
    color::{ColorRounding, ModeColor},
};

/// A tile's unique colors, with counts and pixel data.
pub(super) struct TileData {
    pub colors: Vec<Oklab>,
    pub counts: Vec<u32>,
    pub pixels: Vec<TilePixel>,
    /// Previous best-fit subpalette.
    pub sp_hint: Cell<usize>,
}

impl TileData {
    fn is_empty(
        &self,
        settings: Settings,
    ) -> bool {
        if settings.is_dithered() {
            self.pixels.is_empty()
        } else {
            self.colors.is_empty()
        }
    }
}

/// A pixel in a tile.
#[derive(Clone, Copy)]
pub(super) struct TilePixel {
    pub x: u32,
    pub y: u32,
    pub color: usize,
}

/// A training sample referencing one pixel in a tile.
struct Sample {
    pub tile: usize,
    pub x: u32,
    pub y: u32,
    pub color: Oklab,
}

/// Errors if `dither` isn't supported by the incremental quantizer.
pub(super) fn dither_available(dither: Dither) -> Result<(), String> {
    IncrementalDither::for_dither(dither)?;
    Ok(())
}

/// Creates a palette and a matching quantized image for `image`.
pub(super) fn quantize_image(
    image: &Image,
    mode: Mode,
    max_subpalettes: usize,
    capacity: usize,
    color_zero: Option<NormalizedColor>,
    tile_width: u32,
    tile_height: u32,
    dither: Dither,
    rounding: ColorRounding,
) -> Result<(crate::palette::Palette, Image), String> {
    let max_subpalettes = max_subpalettes.max(1);
    let capacity = capacity.max(1);
    let color_zero_reduced = color_zero.map(|c| mode.reduce_color(c, rounding));
    let settings = Settings {
        mode,
        rounding,
        dither: IncrementalDither::for_dither(dither)?,
    };

    let slices: Vec<Image> = image.sliced(tile_width, tile_height, mode).collect();
    let (tiles, samples) = extract(&slices, mode, rounding, color_zero_reduced);

    let subpalettes: Vec<Vec<Oklab>> = if samples.is_empty() {
        vec![Vec::new(); max_subpalettes]
    } else {
        quantizer::run(&tiles, &samples, max_subpalettes, capacity, settings, 0)
    };

    // Assign each tile to its best-fit subpalette
    let subpalette_of = quantizer::assign_subpalettes(&subpalettes, &tiles, settings);

    // Finalize palette
    let max_colors = capacity + usize::from(color_zero_reduced.is_some());
    let mut palette = crate::palette::Palette::new(mode, max_subpalettes, max_colors, rounding);
    if let Some(color_zero) = color_zero {
        palette.set_color_zero(color_zero);
    }

    let binner = BinnerF32x3::oklab_from_srgb8();
    let mut palette_candidates: Vec<Vec<CandidateColor>> = Vec::with_capacity(max_subpalettes);

    for (sp_idx, sp_oklab) in subpalettes.iter().enumerate() {
        let mut sp_reduced = dedup_reduced(sp_oklab, mode, rounding);
        sp_reduced.truncate(capacity);

        if sp_reduced.len() < capacity {
            let tile_oklab: Vec<Oklab> = tiles
                .iter()
                .zip(&subpalette_of)
                .filter(|&(_, &i)| i == sp_idx)
                .flat_map(|(tile, _)| tile.colors.iter().copied())
                .collect();
            sp_reduced = grow_to_capacity(sp_reduced, &tile_oklab, capacity, mode, rounding, binner)?;
        }

        if let Some(cz) = color_zero_reduced {
            sp_reduced.retain(|&c| !eq_rgb(c, cz));
            sp_reduced.insert(0, cz);
        }

        let candidates: Vec<CandidateColor> = sp_reduced
            .iter()
            .map(|&r| CandidateColor::new(r, mode.normalize_color(r)))
            .collect();
        palette.add_subpalette_with(&sp_reduced)?;
        palette_candidates.push(candidates);
    }
    palette.sort();

    let output = make_output_image(
        image,
        &slices,
        &subpalette_of,
        &palette_candidates,
        mode,
        settings,
        color_zero_reduced,
    );

    Ok((palette, output))
}

/// Maps every pixel of `image` to its closest color in `subpalette`.
///
/// Returns palette indices and the matching full precision colors.
pub(super) fn remap(
    image: &Image,
    subpalette: &crate::palette::Subpalette,
    dither: Dither,
    rounding: ColorRounding,
) -> Result<(Vec<u8>, Vec<NormalizedColor>), String> {
    let pattern = IncrementalDither::for_dither(dither)?;
    let mode = subpalette.mode;
    let normalized = subpalette.normalized_colors();
    let oklabs: Vec<Oklab> = normalized.iter().map(|&c| c.to_oklab()).collect();
    let view = SubpaletteView::exact(&oklabs);

    let size = (image.width * image.height) as usize;
    let mut indexed_data = vec![0u8; size];
    let mut data = vec![NormalizedColor::TRANSPARENT; size];

    for i in 0..size {
        let nc = image.color_at(i);
        if mode.reduce_color(nc, rounding).is_transparent() {
            continue;
        }

        let x = (i as u32) % image.width;
        let y = (i as u32) / image.width;
        let index = view.index_at(pattern, x, y, nc.to_oklab());
        indexed_data[i] = index as u8;
        data[i] = normalized[index];
    }

    Ok((indexed_data, data))
}

/// Extract training samples from `slices`.
fn extract(
    slices: &[Image],
    mode: Mode,
    rounding: ColorRounding,
    color_zero: Option<ReducedColor>,
) -> (Vec<TileData>, Vec<Sample>) {
    let mut tiles = Vec::with_capacity(slices.len());
    let mut samples = Vec::new();

    for (i, slice) in slices.iter().enumerate() {
        let mut reduced_colors: Vec<ReducedColor> = Vec::new();
        let mut colors: Vec<Oklab> = Vec::new();
        let mut counts: Vec<u32> = Vec::new();
        let mut pixels: Vec<TilePixel> = Vec::new();

        for row in 0..slice.height {
            for col in 0..slice.width {
                let nc = slice.color_at((row * slice.width + col) as usize);
                let rc = mode.reduce_color(nc, rounding);
                if rc.is_transparent() || color_zero.is_some_and(|cz| eq_rgb(rc, cz)) {
                    continue;
                }

                let oklab = mode.normalize_color(rc).to_oklab();
                let (x, y) = (slice.src_x + col, slice.src_y + row);
                let color = match reduced_colors.iter().position(|&c| c == rc) {
                    Some(pos) => {
                        counts[pos] += 1;
                        pos
                    }
                    None => {
                        reduced_colors.push(rc);
                        colors.push(oklab);
                        counts.push(1);
                        colors.len() - 1
                    }
                };
                pixels.push(TilePixel { x, y, color });
                samples.push(Sample {
                    tile: i,
                    x,
                    y,
                    color: oklab,
                });
            }
        }
        tiles.push(TileData {
            colors,
            counts,
            pixels,
            sp_hint: Cell::new(0),
        });
    }

    (tiles, samples)
}

/// Remaps every tile's pixels to its assigned subpalette's colors, optionally dithered.
fn make_output_image(
    image: &Image,
    slices: &[Image],
    subpalette_of: &[usize],
    subpalette_candidates: &[Vec<CandidateColor>],
    mode: Mode,
    settings: Settings,
    color_zero: Option<ReducedColor>,
) -> Image {
    let mut data = vec![NormalizedColor::TRANSPARENT; (image.width * image.height) as usize];

    for (slice, &sp_idx) in slices.iter().zip(subpalette_of) {
        let candidates = &subpalette_candidates[sp_idx];
        if candidates.is_empty() {
            continue;
        }
        let candidates_oklab: Vec<Oklab> = candidates.iter().map(|c| c.oklab).collect();
        let sp = SubpaletteView::exact(&candidates_oklab);
        let w = slice.width.min(image.width.saturating_sub(slice.src_x));
        let h = slice.height.min(image.height.saturating_sub(slice.src_y));

        for row in 0..h {
            for col in 0..w {
                let nc = slice.color_at((row * slice.width + col) as usize);
                let rc = mode.reduce_color(nc, settings.rounding);
                if rc.is_transparent() {
                    continue;
                }

                let (tx, ty) = (slice.src_x + col, slice.src_y + row);
                let offset = (ty * image.width + tx) as usize;
                // Don't apply dither if (reduced) source color == color_zero
                let dc = if color_zero.is_some_and(|cz| eq_rgb(rc, cz)) {
                    rc
                } else {
                    let oklab = nc.to_oklab();
                    let index = sp.index_at(settings.dither, tx, ty, oklab);
                    candidates[index].reduced
                };
                data[offset] = mode.normalize_color(dc);
            }
        }
    }

    let colors = image::colors_in(&data);

    Image {
        width: image.width,
        height: image.height,
        src_x: image.src_x,
        src_y: image.src_y,
        data,
        indexed_data: Vec::new(),
        palette: Vec::new(),
        colors,
    }
}
