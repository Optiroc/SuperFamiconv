//! Palette and color types for the incremental k-means quantization method.
//!
//! This is where most optimizations compared to TiledPaletteQuant are implemented:
//! - Matching starts from each tile's last winning subpalette, since it's usually still best.
//! - A subpalette's summed distance to a tile stops as soon as it can't beat the current best.
//! - Dithered candidate matches are computed once per tile color and cached, not once per pixel.

use quantette::deps::palette::Oklab;

use crate::color::{NormalizedColor, oklab_sqdist};
use crate::mode::Mode;
use crate::mode::color::{ColorRounding, ModeColor};
use quantette::color_space::oklab_to_srgb8;

use super::dither::{DITHER_WEIGHT, IncrementalDither, MAX_CANDIDATES};
use super::stats::smallest_two;
use super::{Sample, TileData};

#[derive(Clone, Copy)]
pub(super) struct Settings {
    pub mode: Mode,
    pub rounding: ColorRounding,
    pub dither: Option<IncrementalDither>,
}

impl Settings {
    pub(super) fn is_dithered(self) -> bool {
        self.dither.is_some()
    }

    /// Reduces `color` to the precision of the target mode and back.
    pub(super) fn quantize_color(
        self,
        color: Oklab,
    ) -> Oklab {
        let srgb = oklab_to_srgb8(std::slice::from_ref(&color))[0];
        let normalized = NormalizedColor::new(srgb.red, srgb.green, srgb.blue, 0xff);
        self.mode.quantize_color(normalized, self.rounding).to_oklab()
    }
}

/// Candidate matches of one color as `(index, squared distance, biased color, brightness)`.
type Candidates = [(usize, f32, Oklab, f32); MAX_CANDIDATES];

/// Outcome of a tie between equal distances.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tie {
    Win,
    Lose,
}

#[derive(Clone)]
pub(super) struct Palette {
    subpalettes: Vec<Subpalette>,
    settings: Settings,
}

impl Palette {
    pub(super) fn new(
        subpalettes: Vec<Subpalette>,
        settings: Settings,
    ) -> Self {
        Palette { subpalettes, settings }
    }

    pub(super) fn seeded(
        color: Oklab,
        settings: Settings,
    ) -> Self {
        Palette::new(vec![Subpalette::seeded(color, settings)], settings)
    }

    pub(super) fn from_colors(
        colors: &[Vec<Oklab>],
        settings: Settings,
    ) -> Self {
        let subpalettes = colors
            .iter()
            .map(|c| Subpalette::from_colors(c.clone(), settings))
            .collect();
        Palette::new(subpalettes, settings)
    }

    pub(super) fn len(&self) -> usize {
        self.subpalettes.len()
    }

    pub(super) fn settings(&self) -> Settings {
        self.settings
    }

    pub(super) fn subpalettes(&self) -> &[Subpalette] {
        &self.subpalettes
    }

    pub(super) fn colors(self) -> Vec<Vec<Oklab>> {
        self.subpalettes.into_iter().map(|sp| sp.colors).collect()
    }

    pub(super) fn duplicate_palette(
        &mut self,
        index: usize,
    ) {
        self.subpalettes.push(self.subpalettes[index].clone());
    }

    pub(super) fn duplicate_color(
        &mut self,
        palette: usize,
        color_index: usize,
    ) {
        self.subpalettes[palette].duplicate_color(color_index);
    }

    pub(super) fn copy_subpalette(
        &mut self,
        from: usize,
        to: usize,
    ) {
        self.subpalettes[to] = self.subpalettes[from].clone();
    }

    pub(super) fn copy_color(
        &mut self,
        palette: usize,
        from: usize,
        to: usize,
    ) {
        self.subpalettes[palette].copy_color(from, to);
    }

    /// Index and distance of the best-fitting subpalette for `tile`, and the distance of the second.
    pub(super) fn best_two(
        &self,
        tile: &TileData,
    ) -> (usize, f32, f32) {
        let dither = self.settings.dither;
        let hint = tile.sp_hint.get().min(self.subpalettes.len() - 1);
        let mut best = (hint, self.subpalettes[hint].view().distance(tile, dither));
        let mut second = f32::INFINITY;

        for (i, sp) in self.subpalettes.iter().enumerate() {
            if i == hint {
                continue;
            }
            // Anything over the runner-up can't change the outcome
            let Some(d) = sp.view().distance_within(tile, dither, second, Tie::Win) else {
                continue;
            };
            if d < best.1 || (d == best.1 && i < best.0) {
                second = best.1;
                best = (i, d);
            } else {
                second = second.min(d);
            }
        }

        tile.sp_hint.set(best.0);
        (best.0, best.1, second)
    }

    /// Index and distance of the best-fitting subpalette for `tile`.
    pub(super) fn best_fit(
        &self,
        tile: &TileData,
    ) -> (usize, f32) {
        let dither = self.settings.dither;

        // Start with the previous winner, it's usually still best.
        let hint = tile.sp_hint.get().min(self.subpalettes.len() - 1);
        let mut best = (hint, self.subpalettes[hint].view().distance(tile, dither));

        for (i, sp) in self.subpalettes.iter().enumerate() {
            if i == hint {
                continue;
            }
            let tie = if i < best.0 { Tie::Win } else { Tie::Lose };
            if let Some(d) = sp.view().distance_within(tile, dither, best.1, tie) {
                best = (i, d);
            }
        }

        tile.sp_hint.set(best.0);
        best
    }

    /// Nudges the color closest to `sample` in `tile`'s best-fitting subpalette towards it.
    pub(super) fn nudge(
        &mut self,
        tile: &TileData,
        sample: &Sample,
        alpha: f32,
    ) {
        let (subpalette_index, _) = self.best_fit(tile);
        let view = self.subpalettes[subpalette_index].view();
        let (color_index, target) = if let Some(pattern) = self.settings.dither {
            let (color_index, _, target) = view.nearest_dithered(pattern, sample.x, sample.y, sample.color, None);
            (color_index, target)
        } else {
            (view.nearest(sample.color).0, sample.color)
        };
        self.subpalettes[subpalette_index].nudge(color_index, target, alpha);
    }

    /// Mean squared quantization error of `tiles`, matched to its best-fitting subpalette.
    pub(super) fn mse(
        &self,
        tiles: &[TileData],
    ) -> f32 {
        let mut total = 0.0f64;
        let mut count = 0u64;

        for tile in tiles {
            if tile.is_empty(self.settings) {
                continue;
            }
            let (sp_idx, _) = self.best_fit(tile);
            let sp_view = self.subpalettes[sp_idx].view();

            sp_view.for_each_match(tile, self.settings.dither, |_, d, c| {
                total += f64::from(d) * f64::from(c);
                count += u64::from(c);
                Some(())
            });
        }

        if count == 0 { 0.0 } else { (total / count as f64) as f32 }
    }

    pub(super) fn quantize(&mut self) {
        for subpalette in &mut self.subpalettes {
            subpalette.quantize();
        }
    }
}

#[derive(Clone)]
pub(super) struct Subpalette {
    colors: Vec<Oklab>,
    quantized: Vec<Oklab>,
    settings: Settings,
}

impl Subpalette {
    fn seeded(
        color: Oklab,
        settings: Settings,
    ) -> Self {
        Subpalette::from_colors(vec![color], settings)
    }

    pub(super) fn from_colors(
        colors: Vec<Oklab>,
        settings: Settings,
    ) -> Self {
        let quantized = if settings.is_dithered() {
            colors.iter().map(|&c| settings.quantize_color(c)).collect()
        } else {
            colors.clone()
        };
        Subpalette {
            colors,
            quantized,
            settings,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.colors.len()
    }

    pub(super) fn colors(&self) -> &[Oklab] {
        &self.colors
    }

    pub(super) fn view(&self) -> SubpaletteView<'_> {
        SubpaletteView {
            colors: &self.colors,
            quantized: &self.quantized,
        }
    }

    fn nudge(
        &mut self,
        index: usize,
        target: Oklab,
        alpha: f32,
    ) {
        let color = &mut self.colors[index];
        color.l = (1.0 - alpha) * color.l + alpha * target.l;
        color.a = (1.0 - alpha) * color.a + alpha * target.a;
        color.b = (1.0 - alpha) * color.b + alpha * target.b;
        if self.settings.is_dithered() {
            self.quantized[index] = self.settings.quantize_color(self.colors[index]);
        }
    }

    fn duplicate_color(
        &mut self,
        index: usize,
    ) {
        self.colors.push(self.colors[index]);
        self.quantized.push(self.quantized[index]);
    }

    fn copy_color(
        &mut self,
        from: usize,
        to: usize,
    ) {
        self.colors[to] = self.colors[from];
        self.quantized[to] = self.quantized[from];
    }

    fn quantize(&mut self) {
        for color in &mut self.colors {
            *color = self.settings.quantize_color(*color);
        }
        self.quantized = self.colors.clone();
    }
}

#[derive(Clone, Copy)]
pub(super) struct SubpaletteView<'a> {
    colors: &'a [Oklab],
    quantized: &'a [Oklab],
}

impl<'a> SubpaletteView<'a> {
    /// View of colors that are already mode-reduced (a finished palette).
    pub(super) fn exact(colors: &'a [Oklab]) -> Self {
        SubpaletteView {
            colors,
            quantized: colors,
        }
    }

    /// Index and squared distance of the nearest color to `color`.
    pub(super) fn nearest(
        &self,
        color: Oklab,
    ) -> (usize, f32) {
        self.nearest_exclude(color, None)
    }

    /// Index and squared distance of the nearest color to `color`, optionally ignoring one index.
    fn nearest_exclude(
        &self,
        color: Oklab,
        exclude: Option<usize>,
    ) -> (usize, f32) {
        let mut best = (0usize, f32::INFINITY);
        for (i, &c) in self.colors.iter().enumerate() {
            if Some(i) == exclude {
                continue;
            }
            let d = oklab_sqdist(c, color);
            if d < best.1 {
                best = (i, d);
            }
        }
        best
    }

    /// Index and distances of the two colors nearest to `color`.
    pub(super) fn nearest_two(
        &self,
        color: Oklab,
    ) -> (usize, f32, f32) {
        smallest_two(self.colors.iter().map(|&c| oklab_sqdist(c, color)))
    }

    /// Index of the closest color to `color` at `(x, y)`.
    pub(super) fn index_at(
        &self,
        dither: Option<IncrementalDither>,
        x: u32,
        y: u32,
        color: Oklab,
    ) -> usize {
        match dither {
            Some(pattern) => self.nearest_dithered(pattern, x, y, color, None).0,
            None => self.nearest(color).0,
        }
    }

    /// Index, squared distance and dithered target color of the closest match to
    /// `color` at position `(x, y)`, optionally ignoring one index.
    pub(super) fn nearest_dithered(
        &self,
        pattern: IncrementalDither,
        x: u32,
        y: u32,
        color: Oklab,
        exclude: Option<usize>,
    ) -> (usize, f32, Oklab) {
        let (index, dist, biased, _) = self.candidates(pattern, color, exclude)[pattern.rank(x, y)];
        (index, dist, biased)
    }

    /// Calls `f(index, dist, count)` for every match between `tile` and this subpalette.
    /// - Once per pixel if dithered.
    /// - Once per unique color otherwise.
    /// - Stops early if `f` returns `None`.
    pub(super) fn for_each_match(
        &self,
        tile: &TileData,
        dither: Option<IncrementalDither>,
        mut f: impl FnMut(usize, f32, u32) -> Option<()>,
    ) -> Option<()> {
        if let Some(pattern) = dither {
            let mut matcher = self.dither_matcher(pattern, tile);
            for pixel in 0..tile.pixels.len() {
                let (index, dist) = matcher.best(pixel);
                f(index, dist, 1)?;
            }
        } else {
            for (&color, &count) in tile.colors.iter().zip(&tile.counts) {
                let (index, dist) = self.nearest(color);
                f(index, dist, count)?;
            }
        }
        Some(())
    }

    /// Calls `f(index, dist, second_dist, count)` for every match between `tile` and this subpalette.
    /// - Like `for_each_match`, but never stops early.
    pub(super) fn for_each_match_two(
        &self,
        tile: &TileData,
        dither: Option<IncrementalDither>,
        mut f: impl FnMut(usize, f32, f32, u32),
    ) {
        if let Some(pattern) = dither {
            let mut matcher = self.dither_matcher(pattern, tile);
            for pixel in 0..tile.pixels.len() {
                let (index, dist) = matcher.best(pixel);
                let second_dist = matcher.second_dist(pixel);
                f(index, dist, second_dist, 1);
            }
        } else {
            for (&color, &count) in tile.colors.iter().zip(&tile.counts) {
                let (index, dist, second_dist) = self.nearest_two(color);
                f(index, dist, second_dist, count);
            }
        }
    }

    /// Dither matcher for this subpalette against `tile`.
    fn dither_matcher(
        self,
        pattern: IncrementalDither,
        tile: &'a TileData,
    ) -> LazyDitherMatcher<'a> {
        LazyDitherMatcher {
            view: self,
            pattern,
            tile,
            candidates: vec![None; tile.colors.len()],
            second: Vec::new(),
        }
    }

    /// Candidate matches for `color`, ordered by brightness.
    fn candidates(
        &self,
        pattern: IncrementalDither,
        color: Oklab,
        exclude: Option<usize>,
    ) -> Candidates {
        let mut error = (0.0f32, 0.0f32, 0.0f32);
        let mut candidates: Candidates = [(0usize, 0.0f32, Oklab::new(0.0, 0.0, 0.0), 0.0f32); MAX_CANDIDATES];

        for candidate in &mut candidates[..pattern.n_candidates()] {
            let biased = Oklab::new(
                color.l + error.0 * DITHER_WEIGHT,
                color.a + error.1 * DITHER_WEIGHT,
                color.b + error.2 * DITHER_WEIGHT,
            );
            let (index, dist) = self.nearest_exclude(biased, exclude);
            let brightness = self.colors[index].l;
            *candidate = (index, dist, biased, brightness);

            let reduced = self.quantized[index];
            error.0 += color.l - reduced.l;
            error.1 += color.a - reduced.a;
            error.2 += color.b - reduced.b;
        }

        candidates[..pattern.n_candidates()].sort_by(|a, b| a.3.total_cmp(&b.3));
        candidates
    }

    /// Summed distance from `tile` to this subpalette.
    fn distance(
        &self,
        tile: &TileData,
        dither: Option<IncrementalDither>,
    ) -> f32 {
        self.distance_within(tile, dither, f32::INFINITY, Tie::Win)
            .unwrap_or(f32::INFINITY)
    }

    /// Summed distance from `tile` to this subpalette, or `None` if it is above `limit`.
    fn distance_within(
        &self,
        tile: &TileData,
        dither: Option<IncrementalDither>,
        limit: f32,
        tie: Tie,
    ) -> Option<f32> {
        let mut sum = 0.0f32;
        self.for_each_match(tile, dither, |_, dist, count| {
            sum += dist * count as f32;
            let over = sum > limit || (sum == limit && tie == Tie::Lose);
            (!over).then_some(())
        })?;
        Some(sum)
    }
}

/// Matches a tile's pixels against one subpalette when dithering.
pub(super) struct LazyDitherMatcher<'a> {
    view: SubpaletteView<'a>,
    pattern: IncrementalDither,
    tile: &'a TileData,
    /// Dither candidates, lazily computed per color.
    candidates: Vec<Option<Candidates>>,
    /// Distance with best match excluded, lazily computed per color and rank.
    second: Vec<[Option<f32>; MAX_CANDIDATES]>,
}

impl LazyDitherMatcher<'_> {
    /// Index and squared distance of the best candidate for pixel.
    pub(super) fn best(
        &mut self,
        pixel: usize,
    ) -> (usize, f32) {
        let p = self.tile.pixels[pixel];
        let (view, pattern, color) = (self.view, self.pattern, self.tile.colors[p.color]);
        let candidates = self.candidates[p.color].get_or_insert_with(|| view.candidates(pattern, color, None));
        let (index, dist, _, _) = candidates[pattern.rank(p.x, p.y)];
        (index, dist)
    }

    /// Squared distance of pixel to its second best candidate.
    pub(super) fn second_dist(
        &mut self,
        pixel: usize,
    ) -> f32 {
        let p = self.tile.pixels[pixel];
        let (index, _) = self.best(pixel);
        if self.second.is_empty() {
            self.second = vec![[None; MAX_CANDIDATES]; self.tile.colors.len()];
        }

        let (view, pattern, color) = (self.view, self.pattern, self.tile.colors[p.color]);
        let rank = pattern.rank(p.x, p.y);
        *self.second[p.color][rank].get_or_insert_with(|| view.candidates(pattern, color, Some(index))[rank].1)
    }
}
