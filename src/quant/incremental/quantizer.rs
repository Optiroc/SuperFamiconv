//! Incremental k-means quantization implementation.

use quantette::deps::palette::Oklab;

use super::palette::{Palette, Subpalette};
use super::stats::{argmax, argmin};
use super::{Sample, Settings, TileData};
use crate::color::mean_oklab;
use crate::prng::Prng;

/// Number of weak-entry replace iterations.
const REPLACE_ITERATIONS: usize = 10;
/// Number of k-means refinement passes after quantization.
const REFINEMENT_ITERATIONS: usize = 4;
/// Fraction of pixels sampled per iteration.
const FRACTION_OF_PIXELS: f32 = 0.1;
/// Max allowed error share for a palette's worst color candidate.
const MIN_COLOR_FACTOR: f32 = 0.5;
/// Max allowed error share of the worst subpalette for a color candidate.
const MIN_PALETTE_FACTOR: f32 = 0.5;

/// Nudge fraction.
const REPLACE_ALPHA: f32 = 0.3;
/// Nudge fraction for final round.
const FINAL_ALPHA: f32 = 0.5;
/// Nudge fraction if dithering.
const REPLACE_ALPHA_D: f32 = 0.1;
/// Nudge fraction for final round if dithering.
const FINAL_ALPHA_D: f32 = 0.2;

/// Cycles through `samples` in random order, reshuffling once exhausted.
struct SampleCycle {
    order: Vec<usize>,
    index: usize,
    prng: Prng,
}

impl SampleCycle {
    fn new(
        len: usize,
        seed: u64,
    ) -> Self {
        let mut prng = Prng::new(seed);
        let mut order: Vec<usize> = (0..len).collect();
        prng.shuffle(&mut order);
        SampleCycle { order, index: 0, prng }
    }

    fn next(&mut self) -> usize {
        if self.index >= self.order.len() {
            self.prng.shuffle(&mut self.order);
            self.index = 0;
        }
        let i = self.order[self.index];
        self.index += 1;
        i
    }
}

pub(super) fn run(
    tiles: &[TileData],
    samples: &[Sample],
    max_subpalettes: usize,
    capacity: usize,
    settings: Settings,
    seed: u64,
) -> Vec<Vec<Oklab>> {
    let mut cycle = SampleCycle::new(samples.len(), seed);
    let base_iterations = (FRACTION_OF_PIXELS * samples.len() as f32) as usize;
    let (iterations, alpha, final_alpha) = if settings.is_dithered() {
        (base_iterations / 5, REPLACE_ALPHA_D, FINAL_ALPHA_D)
    } else {
        (base_iterations, REPLACE_ALPHA, FINAL_ALPHA)
    };

    let mut palette = seed_palette(tiles, samples, &mut cycle, max_subpalettes, alpha, iterations, settings);
    for _ in 1..capacity {
        grow_palette(&mut palette, tiles, samples, &mut cycle, alpha, iterations);
    }

    let mut min_mse = palette.mse(tiles);
    let mut min_palette = palette.clone();
    for _ in 0..REPLACE_ITERATIONS {
        palette = replace_weakest(&palette, tiles, MIN_COLOR_FACTOR, MIN_PALETTE_FACTOR);
        nudge(&mut palette, tiles, samples, &mut cycle, alpha, iterations);
        let mse = palette.mse(tiles);
        if mse < min_mse {
            min_mse = mse;
            min_palette = palette.clone();
        }
    }
    palette = min_palette;

    if !settings.is_dithered() {
        palette.quantize();
    }

    nudge(&mut palette, tiles, samples, &mut cycle, final_alpha, iterations * 10);

    // Refine palettes by classic k-means clustering only if dithered is disabled
    if !settings.is_dithered() {
        palette.quantize();
        for _ in 0..REFINEMENT_ITERATIONS {
            palette = refine(&palette, tiles);
        }
    }

    palette.quantize();
    palette.colors()
}

/// Assigns every tile to its best-fit trained palette.
pub(super) fn assign_subpalettes(
    colors: &[Vec<Oklab>],
    tiles: &[TileData],
    settings: Settings,
) -> Vec<usize> {
    let palette = Palette::from_colors(colors, settings);

    tiles
        .iter()
        .map(|tile| {
            if tile.is_empty(settings) {
                0
            } else {
                palette.best_fit(tile).0
            }
        })
        .collect()
}

/// Seeds single-color palettes by repeatedly splitting off the palette
/// with the highest total error and re-train.
fn seed_palette(
    tiles: &[TileData],
    samples: &[Sample],
    cycle: &mut SampleCycle,
    max_subpalettes: usize,
    alpha: f32,
    iterations: usize,
    settings: Settings,
) -> Palette {
    let mean = mean_oklab(samples.iter().map(|s| s.color)).expect("samples must not be empty");
    let mut palette = Palette::seeded(mean, settings);
    let mut split_index = 0usize;

    for _ in 1..max_subpalettes {
        palette.duplicate_palette(split_index);
        nudge(&mut palette, tiles, samples, cycle, alpha, iterations);

        let mut distances = vec![0.0f32; palette.len()];
        for tile in tiles {
            if tile.is_empty(settings) {
                continue;
            }
            let (sp_index, dist) = palette.best_fit(tile);
            distances[sp_index] += dist;
        }
        split_index = argmax(&distances);
    }
    palette
}

/// Grows every subpalette by one color, splitting off each palette's currently
/// weakest-fit color, and re-train.
fn grow_palette(
    palette: &mut Palette,
    tiles: &[TileData],
    samples: &[Sample],
    cycle: &mut SampleCycle,
    alpha: f32,
    iterations: usize,
) {
    let num_colors = palette.subpalettes()[0].len() + 1;
    let mut split_indexes = vec![0usize; palette.len()];

    if num_colors > 2 {
        let mut total_color_distances = vec![vec![0.0f32; num_colors - 1]; palette.len()];
        for tile in tiles {
            if tile.colors.is_empty() {
                continue;
            }
            let (sp_index, _) = palette.best_fit(tile);
            let view = palette.subpalettes()[sp_index].view();
            view.for_each_match(tile, palette.settings().dither, |i, d, c| {
                total_color_distances[sp_index][i] += d * c as f32;
                Some(())
            });
        }
        for (idx, distances) in total_color_distances.iter().enumerate() {
            split_indexes[idx] = argmax(distances);
        }
    }

    for (idx, &split_idx) in split_indexes.iter().enumerate() {
        palette.duplicate_color(idx, split_idx);
    }

    nudge(palette, tiles, samples, cycle, alpha, iterations);
}

/// Finds the weakest-fit color of each subpalette and the weakest whole subpalette,
/// and replaces them with a copy of the best one.
fn replace_weakest(
    palette: &Palette,
    tiles: &[TileData],
    min_color_factor: f32,
    min_palette_factor: f32,
) -> Palette {
    let settings = palette.settings();
    let n = palette.len();
    let mut nearest_palette_of = vec![0usize; tiles.len()];
    let mut total_palette_mse = vec![0.0f32; n];
    let mut removed_palette_mse = vec![0.0f32; n];
    let (mut max_palette_index, mut min_palette_index) = (0usize, 0usize);

    if n > 1 {
        for (j, tile) in tiles.iter().enumerate() {
            let (index, min_dist, second_dist) = palette.best_two(tile);
            total_palette_mse[index] += min_dist;
            removed_palette_mse[index] += second_dist;
            nearest_palette_of[j] = index;
        }
        max_palette_index = argmax(&total_palette_mse);
        min_palette_index = argmin(&removed_palette_mse);
    }

    let mut replaced = palette.clone();

    if palette.subpalettes()[0].len() > 1 {
        let mut total_color_mse: Vec<Vec<f32>> =
            palette.subpalettes().iter().map(|sp| vec![0.0f32; sp.len()]).collect();
        let mut second_color_mse: Vec<Vec<f32>> =
            palette.subpalettes().iter().map(|sp| vec![0.0f32; sp.len()]).collect();

        for (j, tile) in tiles.iter().enumerate() {
            let subpalette_index = nearest_palette_of[j];
            let view = palette.subpalettes()[subpalette_index].view();

            view.for_each_match_two(tile, settings.dither, |i, dist, second_dist, c| {
                total_color_mse[subpalette_index][i] += dist * c as f32;
                second_color_mse[subpalette_index][i] += second_dist * c as f32;
            });
        }

        for palette_index in 0..n {
            let max_color_index = argmax(&total_color_mse[palette_index]);
            let min_color_index = argmin(&second_color_mse[palette_index]);
            let should_replace = min_color_index != max_color_index
                && second_color_mse[palette_index][min_color_index]
                    < min_color_factor * total_color_mse[palette_index][max_color_index];

            if should_replace {
                replaced.copy_color(palette_index, max_color_index, min_color_index);
            }
        }
    }

    if n > 1
        && min_palette_index != max_palette_index
        && removed_palette_mse[min_palette_index] < min_palette_factor * total_palette_mse[max_palette_index]
    {
        replaced.copy_subpalette(max_palette_index, min_palette_index);
    }

    replaced
}

/// Draws `count` random samples and for each nudges the nearest color in its
/// best-fit subpalette a fraction `alpha` towards it.
fn nudge(
    palette: &mut Palette,
    tiles: &[TileData],
    samples: &[Sample],
    cycle: &mut SampleCycle,
    alpha: f32,
    count: usize,
) {
    for _ in 0..count {
        let sample = &samples[cycle.next()];
        palette.nudge(&tiles[sample.tile], sample, alpha);
    }
}

/// Recenters every subpalette color to the mean of the samples currently assigned.
fn refine(
    palette: &Palette,
    tiles: &[TileData],
) -> Palette {
    let settings = palette.settings();
    let mut counts: Vec<Vec<u32>> = palette.subpalettes().iter().map(|sp| vec![0u32; sp.len()]).collect();
    let mut sums: Vec<Vec<(f32, f32, f32)>> = palette
        .subpalettes()
        .iter()
        .map(|sp| vec![(0.0, 0.0, 0.0); sp.len()])
        .collect();

    for tile in tiles {
        if tile.is_empty(settings) {
            continue;
        }
        let (subpalette_index, _) = palette.best_fit(tile);
        let view = palette.subpalettes()[subpalette_index].view();
        for (&color, &count) in tile.colors.iter().zip(&tile.counts) {
            let (color_index, _) = view.nearest(color);
            counts[subpalette_index][color_index] += count;
            let s = &mut sums[subpalette_index][color_index];
            s.0 += color.l * count as f32;
            s.1 += color.a * count as f32;
            s.2 += color.b * count as f32;
        }
    }

    let subpalettes = palette
        .subpalettes()
        .iter()
        .enumerate()
        .map(|(spi, sp)| {
            let colors = sp
                .colors()
                .iter()
                .enumerate()
                .map(|(ci, &current)| {
                    let n = counts[spi][ci];
                    if n == 0 {
                        current
                    } else {
                        let (l, a, b) = sums[spi][ci];
                        Oklab::new(l / n as f32, a / n as f32, b / n as f32)
                    }
                })
                .collect();
            Subpalette::from_colors(colors, settings)
        })
        .collect();

    Palette::new(subpalettes, settings)
}
