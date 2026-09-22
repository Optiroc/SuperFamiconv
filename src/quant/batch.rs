//! Batch k-means based tile-aware palette quantization.

use quantette::PaletteSize;
use quantette::color_space::srgb8_to_oklab;
use quantette::deps::palette::{Oklab, Srgb};
use quantette::kmeans::{Kmeans, KmeansOptions};
use quantette::wu::{BinnerF32x3, WuF32x3};

use super::{dedup_reduced, fit_palette, grow_to_capacity};
use crate::color::{self, CandidateColor, NormalizedColor, ReducedColor};
use crate::dither::{self, Dither, Ditherer};
use crate::image::{self, Image};
use crate::mode::{
    Mode,
    color::{ColorRounding, ModeColor},
};
use crate::palette::{Palette, Subpalette};

const MAX_ITERATIONS: usize = 10;
const DITHER_2ND_SAMPLE_WEIGHT: f32 = 0.2;

/// Maps every pixel of `image` to its closest color in `subpalette`.
///
/// Returns palette indices and the matching full precision colors.
pub fn remap(
    image: &Image,
    subpalette: &Subpalette,
    dither: Dither,
    rounding: ColorRounding,
) -> (Vec<u8>, Vec<NormalizedColor>) {
    dither::quantize_pixels(
        subpalette.mode,
        &subpalette.colors,
        image.width,
        image.height,
        dither,
        rounding,
        |i| image.color_at(i),
    )
}

/// Creates a palette and a matching quantized image for `image`.
pub fn quantize_image(
    image: &Image,
    mode: Mode,
    max_subpalettes: usize,
    capacity: usize,
    color_zero: Option<NormalizedColor>,
    tile_width: u32,
    tile_height: u32,
    dither: Dither,
    rounding: ColorRounding,
) -> Result<(Palette, Image), String> {
    // Slice image into tiles and map to Oklab color space.
    let slices: Vec<Image> = image.sliced(tile_width, tile_height, mode).collect();

    // All colors present in the image, used for ditherer-friendly clustering samples.
    let candidates: Vec<CandidateColor> = if dither == Dither::Off {
        Vec::new()
    } else {
        dither::candidate_colors_in(image, mode, rounding)
    };

    let color_zero_reduced = color_zero.map(|c| mode.reduce_color(c, rounding));
    let tiles_colors: Vec<Vec<Oklab>> = slices
        .iter()
        .map(|slice| get_oklab_colors(slice, mode, rounding, color_zero_reduced, &candidates, capacity))
        .collect();

    // Initialize k centroids:
    // - Exclude tiles fully ignored above from seeding (they will always have a matching palette)
    // - Cluster remaining tiles into `max_subpalettes` groups
    let tiles_avg: Vec<Option<Oklab>> = tiles_colors
        .iter()
        .map(|t| color::mean_oklab(t.iter().copied()))
        .collect();
    let present_avg: Vec<Oklab> = tiles_avg.iter().filter_map(|&t| t).collect();
    let binner = BinnerF32x3::oklab_from_srgb8();
    let palette_size = PaletteSize::from_usize_clamped(max_subpalettes);

    let seed_centroids: Vec<Oklab> = if present_avg.is_empty() {
        Vec::new()
    } else {
        let seeds = WuF32x3::run_slice(&present_avg, binner)
            .map_err(|e| e.to_string())?
            .palette(palette_size);
        Kmeans::run_slice(&present_avg, seeds, KmeansOptions::new())
            .map_err(|e| e.to_string())?
            .into_palette()
            .into_vec()
    };

    // Assign each tile to its nearest seed centroid
    let mut group_of: Vec<usize> = tiles_avg
        .iter()
        .map(|avg| avg.map_or(0, |avg| nearest_index(&seed_centroids, avg)))
        .collect();
    // Fit initial per-group palette from the tiles assigned to each group
    let mut group_palettes = fit_group_palettes(&tiles_colors, &group_of, max_subpalettes, capacity, binner)?;

    // Reassign each tile to current best fit palette, refit palettes, repeat until happy.
    for iteration in 0..MAX_ITERATIONS {
        let mut group_error = vec![0.0f32; max_subpalettes];
        let mut group_tile_count = vec![0usize; max_subpalettes];
        let mut new_group_of = vec![0usize; tiles_colors.len()];

        for (i, colors) in tiles_colors.iter().enumerate() {
            if colors.is_empty() {
                new_group_of[i] = group_of[i];
                continue;
            }
            let (best_group, best_error) = (0..max_subpalettes)
                .map(|g| (g, tile_error(colors, &group_palettes[g])))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap(); // max_subpalettes is at least 1
            new_group_of[i] = best_group;
            group_error[best_group] += best_error;
            group_tile_count[best_group] += 1;
        }

        // Exit early if converged
        let assignment_changed = new_group_of != group_of;
        let has_empty_group = group_tile_count.contains(&0);
        group_of = new_group_of;
        if !assignment_changed && !has_empty_group {
            break;
        }

        // Re-seed empty groups with half of the worst-scoring group's tiles
        // (Except on the final iteration so all tiles end up assigned by nearest palette)
        if has_empty_group && iteration + 1 < MAX_ITERATIONS {
            let worst_group = group_error
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap(); // max_subpalettes is at least 1
            for (g, &count) in group_tile_count.iter().enumerate() {
                if count == 0 && g != worst_group {
                    let worst_tiles: Vec<usize> = group_of
                        .iter()
                        .enumerate()
                        .filter(|&(_, &gi)| gi == worst_group)
                        .map(|(i, _)| i)
                        .collect();
                    for &i in worst_tiles.iter().take(worst_tiles.len() / 2) {
                        group_of[i] = g;
                    }
                }
            }
        }

        group_palettes = fit_group_palettes(&tiles_colors, &group_of, max_subpalettes, capacity, binner)?;
    }

    // Finalize palette
    let raw_max_colors = capacity + usize::from(color_zero_reduced.is_some());
    let mut palette = Palette::new(mode, max_subpalettes, raw_max_colors, rounding);
    if let Some(color_zero) = color_zero {
        palette.set_color_zero(color_zero);
    }
    let mut group_candidates: Vec<Vec<CandidateColor>> = Vec::with_capacity(max_subpalettes);

    for (g, group_palette) in group_palettes.iter().enumerate() {
        let mut reduced = dedup_reduced(group_palette, mode, rounding);
        reduced.truncate(capacity);

        if reduced.len() < capacity {
            // Some fitted colors mode-reduce to the same color, attempt to fill up.
            let group_colors: Vec<Oklab> = tiles_colors
                .iter()
                .zip(&group_of)
                .filter(|&(_, &gi)| gi == g)
                .flat_map(|(colors, _)| colors.iter().copied())
                .collect();
            reduced = grow_to_capacity(reduced, &group_colors, capacity, mode, rounding, binner)?;
        }

        if let Some(cz) = color_zero_reduced {
            reduced.retain(|&c| !color::eq_rgb(c, cz));
            reduced.insert(0, cz);
        }

        let candidates: Vec<CandidateColor> = reduced
            .iter()
            .map(|&r| CandidateColor::new(r, mode.normalize_color(r)))
            .collect();
        palette.add_subpalette_with(&reduced)?;
        group_candidates.push(candidates);
    }
    palette.sort();

    let output = make_output_image(
        image,
        &slices,
        &group_of,
        &group_candidates,
        mode,
        dither,
        rounding,
        color_zero_reduced,
    );

    Ok((palette, output))
}

/// The `image`'s colors mapped to `Oklab`.
///
/// If a color is close enough to its second candidate for the ditherer to pick
/// it with some probability (`DITHER_2ND_SAMPLE_WEIGHT`), add it as an extra sample.
///
/// - Transparent and color-zero values are ignored.
/// - `capacity` is the per-subpalette capacity.
fn get_oklab_colors(
    image: &Image,
    mode: Mode,
    rounding: ColorRounding,
    color_zero: Option<ReducedColor>,
    candidates: &[CandidateColor],
    capacity: usize,
) -> Vec<Oklab> {
    let pixels: Vec<NormalizedColor> = image
        .data
        .iter()
        .copied()
        .filter(|&c| {
            let r = mode.reduce_color(c, rounding);
            !r.is_transparent() && !color_zero.is_some_and(|cz| color::eq_rgb(r, cz))
        })
        .collect();

    let srgb: Vec<Srgb<u8>> = pixels.iter().map(|c| Srgb::new(c.r, c.g, c.b)).collect();
    let mut colors = srgb8_to_oklab(&srgb);

    if !candidates.is_empty() {
        let bracket_width = dither::bracket_width(mode, capacity);
        for &pixel in &pixels {
            let (a, b) = dither::nearest_two(pixel, candidates, bracket_width);
            let Some(b) = b else { continue };
            if dither::lerp(a.normalized, b.normalized, pixel, true) >= DITHER_2ND_SAMPLE_WEIGHT {
                colors.push(b.oklab);
            }
        }
    }
    colors
}

// Index of the closest palette to `color`.
fn nearest_index(
    palette: &[Oklab],
    color: Oklab,
) -> usize {
    palette
        .iter()
        .enumerate()
        .map(|(i, &c)| (i, color::oklab_sqdist(c, color)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(0, |(i, _)| i)
}

/// Total distance from `colors` to their nearest color in `palette`.
fn tile_error(
    colors: &[Oklab],
    palette: &[Oklab],
) -> f32 {
    if palette.is_empty() {
        return f32::INFINITY;
    }
    colors
        .iter()
        .map(|&s| {
            palette
                .iter()
                .map(|&c| color::oklab_sqdist(c, s))
                .fold(f32::INFINITY, f32::min)
        })
        .sum()
}

/// Fits each group's color palette from the colors of its currently-assigned tiles.
fn fit_group_palettes<const B1: usize, const B2: usize, const B3: usize>(
    tile_colors: &[Vec<Oklab>],
    group_of: &[usize],
    max_subpalettes: usize,
    capacity: usize,
    binner: BinnerF32x3<B1, B2, B3>,
) -> Result<Vec<Vec<Oklab>>, String> {
    (0..max_subpalettes)
        .map(|group| {
            let colors: Vec<Oklab> = tile_colors
                .iter()
                .zip(group_of)
                .filter(|&(_, &gi)| gi == group)
                .flat_map(|(s, _)| s.iter().copied())
                .collect();
            fit_palette(&colors, capacity, binner)
        })
        .collect()
}

/// Remaps every tile's pixels to its assigned group's colors, optionally dithered.
fn make_output_image(
    image: &Image,
    slices: &[Image],
    group_of: &[usize],
    group_colors: &[Vec<CandidateColor>],
    mode: Mode,
    dither: Dither,
    rounding: ColorRounding,
    color_zero: Option<ReducedColor>,
) -> Image {
    let mut data = vec![NormalizedColor::TRANSPARENT; (image.width * image.height) as usize];

    for (slice, &group_idx) in slices.iter().zip(group_of) {
        let palette = &group_colors[group_idx];
        if palette.is_empty() {
            continue;
        }
        let w = slice.width.min(image.width.saturating_sub(slice.src_x));
        let h = slice.height.min(image.height.saturating_sub(slice.src_y));
        let bracket_width = dither::bracket_width(mode, palette.len());
        let mut ditherer = Ditherer::new(dither, slice.src_x, slice.src_y, w, h, bracket_width);
        for row in 0..h {
            for col in 0..w {
                let nc = slice.color_at((row * slice.width + col) as usize);
                let rc = mode.reduce_color(nc, rounding);
                if rc.is_transparent() {
                    continue;
                }
                let (tx, ty) = (slice.src_x + col, slice.src_y + row);
                let offset = (ty * image.width + tx) as usize;
                // Don't apply dither if (reduced) source color == color_zero
                let dc = if color_zero.is_some_and(|cz| color::eq_rgb(rc, cz)) {
                    rc
                } else {
                    ditherer.color_at(tx, ty, nc, palette)
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
