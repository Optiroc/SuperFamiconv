//! Tile-aware palette quantization.

use quantette::PaletteSize;
use quantette::color_space::{oklab_to_srgb8, srgb8_to_oklab};
use quantette::deps::palette::{Oklab, Srgb};
use quantette::kmeans::{Kmeans, KmeansOptions};
use quantette::wu::{BinnerF32x3, WuF32x3};

use super::dither::{Dither, Ditherer};
use crate::color::{CandidateColor, NormalizedColor, ReducedColor, oklab_sqdist};
use crate::image::Image;
use crate::mode::{Mode, color::ModeColor};
use crate::palette::Palette;

const MAX_ITERATIONS: usize = 32;

/// Creates a palette and a matching quantized image for `image` using tile-aware k-means clustering.
#[allow(clippy::too_many_arguments)]
pub fn quantize_palette(
    image: &Image,
    mode: Mode,
    max_subpalettes: usize,
    capacity: usize,
    color_zero: Option<NormalizedColor>,
    tile_width: u32,
    tile_height: u32,
    dither: Dither,
) -> Result<(Palette, Image), String> {
    let max_subpalettes = max_subpalettes.max(1);
    let capacity = capacity.max(1);
    let color_zero_reduced = color_zero.map(|c| mode.reduce_color(c));

    // Slice image into tiles and map to Oklab color space, ignoring color-zero and transparent pixels
    let crops = image.crops(tile_width, tile_height, mode);
    let tiles_colors: Vec<Vec<Oklab>> = crops
        .iter()
        .map(|crop| get_oklab_colors(crop, mode, color_zero_reduced))
        .collect();

    // Initialize k centroids:
    // - Exclude tiles fully ignored above from seeding (they will always have a matching palette)
    // - Cluster remaining tiles into `max_subpalettes` groups
    let tiles_avg: Vec<Option<Oklab>> = tiles_colors.iter().map(|t| average_oklab(t)).collect();
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
    let mut palette = Palette::new(mode, max_subpalettes, raw_max_colors);
    if let Some(color_zero) = color_zero {
        palette.set_color_zero(color_zero);
    }
    let mut group_candidates: Vec<Vec<CandidateColor>> = Vec::with_capacity(max_subpalettes);

    for group_palette in &group_palettes {
        let mut reduced: Vec<ReducedColor> = Vec::new();
        for srgb in oklab_to_srgb8(group_palette) {
            let normalized = NormalizedColor::new(srgb.red, srgb.green, srgb.blue, 0xff);
            let r = mode.reduce_color(normalized);
            if !r.is_transparent() && !reduced.contains(&r) {
                reduced.push(r);
            }
        }
        reduced.truncate(capacity);
        if let Some(cz) = color_zero_reduced {
            reduced.retain(|&c| !eq_rgb(c, cz));
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
        &crops,
        &group_of,
        &group_candidates,
        mode,
        dither,
        color_zero_reduced,
    );

    Ok((palette, output))
}

/// The `image`'s colors mapped to `Oklab`.
/// - Transparent and color-zero values are ignored.
fn get_oklab_colors(
    image: &Image,
    mode: Mode,
    color_zero: Option<ReducedColor>,
) -> Vec<Oklab> {
    let srgb: Vec<Srgb<u8>> = image
        .color_data()
        .into_iter()
        .filter(|&c| {
            let r = mode.reduce_color(c);
            !r.is_transparent() && !color_zero.is_some_and(|cz| eq_rgb(r, cz))
        })
        .map(|c| Srgb::new(c.r, c.g, c.b))
        .collect();
    srgb8_to_oklab(&srgb)
}

fn average_oklab(colors: &[Oklab]) -> Option<Oklab> {
    if colors.is_empty() {
        return None;
    }
    let n = colors.len() as f32;
    let (mut l, mut a, mut b) = (0.0f32, 0.0f32, 0.0f32);
    for c in colors {
        l += c.l;
        a += c.a;
        b += c.b;
    }
    Some(Oklab::new(l / n, a / n, b / n))
}

fn nearest_index(
    palette: &[Oklab],
    color: Oklab,
) -> usize {
    palette
        .iter()
        .enumerate()
        .map(|(i, &c)| (i, oklab_sqdist(c, color)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(0, |(i, _)| i)
}

fn eq_rgb(
    a: ReducedColor,
    b: ReducedColor,
) -> bool {
    a.r == b.r && a.g == b.g && a.b == b.b
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
                .map(|&c| oklab_sqdist(c, s))
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

/// Seeds a palette with Wu quantization and refine it with k-means.
fn fit_palette<const B1: usize, const B2: usize, const B3: usize>(
    colors: &[Oklab],
    capacity: usize,
    binner: BinnerF32x3<B1, B2, B3>,
) -> Result<Vec<Oklab>, String> {
    if colors.is_empty() {
        return Ok(Vec::new());
    }
    let k = PaletteSize::from_usize_clamped(capacity);
    let seeds = WuF32x3::run_slice(colors, binner)
        .map_err(|e| e.to_string())?
        .palette(k);
    let palette = Kmeans::run_slice(colors, seeds, KmeansOptions::new())
        .map_err(|e| e.to_string())?
        .into_palette();
    Ok(palette.into_vec())
}

/// Remaps every tile's pixels to its assigned group's colors, optionally dithered.
fn make_output_image(
    image: &Image,
    crops: &[Image],
    group_of: &[usize],
    group_colors: &[Vec<CandidateColor>],
    mode: Mode,
    dither: Dither,
    color_zero: Option<ReducedColor>,
) -> Image {
    let mut data = vec![0u8; (image.width * image.height * 4) as usize];

    for (crop, &group_idx) in crops.iter().zip(group_of) {
        let palette = &group_colors[group_idx];
        if palette.is_empty() {
            continue;
        }
        let w = crop.width.min(image.width.saturating_sub(crop.src_x));
        let h = crop.height.min(image.height.saturating_sub(crop.src_y));
        let mut ditherer = Ditherer::new(dither, crop.src_x, crop.src_y, w, h);
        for row in 0..h {
            for col in 0..w {
                let nc = crop.color_at((row * crop.width + col) as usize);
                let rc = mode.reduce_color(nc);
                if rc.is_transparent() {
                    continue;
                }
                let (tx, ty) = (crop.src_x + col, crop.src_y + row);
                let offset = ((ty * image.width + tx) * 4) as usize;
                // Don't apply dither if (reduced) source color == color_zero
                let dc = if color_zero.is_some_and(|cz| eq_rgb(rc, cz)) {
                    rc
                } else {
                    ditherer.color_at(tx, ty, nc, palette)
                };
                data[offset..offset + 4].copy_from_slice(&mode.normalize_color(dc).to_bytes());
            }
        }
    }

    let colors = data
        .chunks_exact(4)
        .map(|c| NormalizedColor::new(c[0], c[1], c[2], c[3]))
        .collect();

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
