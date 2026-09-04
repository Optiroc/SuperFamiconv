//! Palette generation: building subpalettes from an image's tiles.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::string::ToString;

use crate::binpack::{Effort, pack};
use crate::color::{self, CandidateColor, NormalizedColor, ReducedColor, summed_distance};
use crate::image::Image;
use crate::mode::{Mode, color::ModeColor};

pub const fn palette_size_at_bpp(bpp: u32) -> u32 {
    1 << bpp
}

#[derive(Debug, Clone)]
pub struct Palette {
    pub mode: Mode,
    pub max_subpalettes: usize,
    pub max_colors_per_subpalette: usize,
    subpalettes: Vec<Subpalette>,
    color_zero: ReducedColor,
    color_zero_is_shared: bool,
}

#[derive(Debug, Clone)]
pub struct Subpalette {
    pub mode: Mode,
    pub colors: Vec<ReducedColor>,
    max_colors: usize,
    colors_set: HashSet<ReducedColor>,
}

impl Subpalette {
    pub fn new(
        mode: Mode,
        max_colors: usize,
    ) -> Self {
        Subpalette {
            mode,
            max_colors,
            colors: Vec::new(),
            colors_set: HashSet::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.colors.len() == self.max_colors
    }

    pub fn normalized_colors(&self) -> Vec<NormalizedColor> {
        self.colors.iter().map(|&c| self.mode.normalize_color(c)).collect()
    }

    /// Count of `new_colors` not already present in subpalette.
    pub fn diff(
        &self,
        new_colors: &BTreeSet<ReducedColor>,
    ) -> usize {
        new_colors.iter().filter(|c| !self.colors_set.contains(c)).count()
    }

    /// Adds one (reduced-space) color.
    pub fn add(
        &mut self,
        color: ReducedColor,
        add_duplicates: bool,
    ) -> Result<(), String> {
        let should_push = add_duplicates || !self.colors_set.contains(&color);
        if should_push {
            if self.is_full() {
                return Err("Colors don't fit in palette".into());
            }
            self.colors.push(color);
        }
        self.colors_set.insert(color);
        Ok(())
    }

    /// Adds several (reduced-space) colors.
    pub fn add_all(
        &mut self,
        colors: impl IntoIterator<Item = ReducedColor>,
        add_duplicates: bool,
    ) -> Result<(), String> {
        for c in colors {
            self.add(c, add_duplicates)?;
        }
        Ok(())
    }

    /// Returns a copy padded with transparent/black entries up to `max_colors`,
    /// for outputs that expect fixed-size palettes (native binary, .act).
    pub fn padded(&self) -> Subpalette {
        let mut sp = self.clone();
        while sp.colors.len() < sp.max_colors {
            sp.add(ReducedColor::TRANSPARENT, true).unwrap();
        }
        sp
    }

    /// Aesthetically pleasing color sorting.
    /// - 9 groups of near-grays + 8 hue bands, sorted by perceived luma.
    /// - If `lock_color_zero`, index 0 is left in place;
    pub fn sort(
        &mut self,
        lock_color_zero: bool,
    ) {
        if self.colors.len() < 3 {
            return;
        }
        let (zero, start) = if lock_color_zero {
            (Some(self.colors[0]), 1)
        } else {
            (None, 0)
        };
        let mut sorted = self.colors[start..].to_vec();
        sorted.sort_by(|&a, &b| {
            visual_sort_key(a, self.mode)
                .partial_cmp(&visual_sort_key(b, self.mode))
                .unwrap()
        });
        self.colors = zero.into_iter().chain(sorted).collect();
    }

    /// If a duplicate of color-zero exists elsewhere in this subpalette,
    /// clears color-zero's alpha (marking it transparent) and returns true.
    ///
    /// Used when loading a Palette from native bytes.
    fn fix_color_zero_duplicates(&mut self) -> bool {
        if self.colors.len() <= 1 {
            return false;
        }
        let cz = self.colors[0];
        if self.colors[1..].contains(&cz) {
            self.colors[0] = ReducedColor::new(cz.r, cz.g, cz.b, 0);
            self.colors_set = self.colors.iter().copied().collect();
            true
        } else {
            false
        }
    }
}

#[allow(clippy::float_cmp)]
fn visual_sort_key(
    color: ReducedColor,
    mode: Mode,
) -> (f32, f32, f32) {
    let color = mode.normalize_color(color);
    let r = (f32::from(color.r)) / f32::from(u8::MAX);
    let g = (f32::from(color.g)) / f32::from(u8::MAX);
    let b = (f32::from(color.b)) / f32::from(u8::MAX);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);

    let delta = max - min;
    let hue = if delta <= 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let luma = perceived_luma(color);
    let sat = if max <= 0.0 { 0.0 } else { delta / (max + min) };

    // Group hue into n_bands + near grayscale
    let n_bands = 8.0;
    let degrees_per_band = 360.0 / n_bands;
    let hue_rot = (hue + degrees_per_band / 2.0) % 360.0;
    let hue_grouped = if sat < 0.005 {
        -1.0
    } else {
        (hue_rot / degrees_per_band).round()
    };
    (hue_grouped, luma, max)
}

/// Perceived luma in range 0..=1.
fn perceived_luma(color: NormalizedColor) -> f32 {
    let r = srgb_to_linear(f32::from(color.r) / f32::from(u8::MAX));
    let g = srgb_to_linear(f32::from(color.g) / f32::from(u8::MAX));
    let b = srgb_to_linear(f32::from(color.b) / f32::from(u8::MAX));
    let pr = 0.299;
    let pg = 0.587;
    let pb = 0.114;
    (r * r * pr + g * g * pg + b * b * pb).sqrt()
}

/// Converts a single sRGB-encoded channel value (0.0-1.0) to linear light.
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

impl Palette {
    pub fn new(
        mode: Mode,
        max_subpalettes: usize,
        max_colors_per_subpalette: usize,
    ) -> Self {
        Palette {
            mode,
            max_subpalettes,
            max_colors_per_subpalette,
            subpalettes: Vec::new(),
            color_zero: ReducedColor::TRANSPARENT,
            color_zero_is_shared: false,
        }
    }

    // The total number of colors in all subpalettes.
    pub fn size(&self) -> usize {
        self.subpalettes.iter().map(|sp| sp.colors.len()).sum()
    }

    // The mode-reduced colors of all subpalettes.
    pub fn colors(&self) -> Vec<Vec<ReducedColor>> {
        self.subpalettes.iter().map(|sp| sp.colors.clone()).collect()
    }

    // The full precision colors of all subpalettes.
    pub fn normalized_colors(&self) -> Vec<Vec<NormalizedColor>> {
        self.subpalettes.iter().map(Subpalette::normalized_colors).collect()
    }

    /// The subpalette at `index`, if any.
    pub fn subpalette_at(
        &self,
        index: usize,
    ) -> Option<&Subpalette> {
        self.subpalettes.get(index)
    }

    /// Index of `subpalette`.
    pub fn index_of(
        &self,
        subpalette: &Subpalette,
    ) -> Option<usize> {
        self.subpalettes.iter().position(|sp| std::ptr::eq(sp, subpalette))
    }

    /// Sorts all subpalettes.
    pub fn sort(&mut self) {
        for sp in &mut self.subpalettes {
            sp.sort(self.color_zero_is_shared);
        }
    }

    /// Sets the color to be used at index 0 of every subsequently created subpalette.
    pub fn set_color_zero(
        &mut self,
        color: NormalizedColor,
    ) {
        let reduced = self.mode.reduce_color(color);
        self.color_zero = if reduced.is_transparent() {
            ReducedColor::TRANSPARENT
        } else {
            reduced
        };
        self.color_zero_is_shared = true;
    }

    /// Finds the first subpalette that includes all colors in `image`.
    ///
    /// Errors if the tile needs more colors than fit in a single subpalette, or if no subpalette matches.
    pub fn subpalette_matching(
        &self,
        image: &Image,
    ) -> Result<&Subpalette, String> {
        let mut required: BTreeSet<ReducedColor> = image
            .color_data()
            .into_iter()
            .map(|c| self.mode.reduce_color(c))
            .collect();
        required.remove(&ReducedColor::TRANSPARENT);

        if required.len() > self.max_colors_per_subpalette {
            return Err(format!(
                "Tile with too many ({} > {}) unique colors at ({}, {}) in source image",
                required.len(),
                self.max_colors_per_subpalette,
                image.src_x,
                image.src_y
            ));
        }

        self.subpalettes
            .iter()
            .find(|sp| sp.diff(&required) == 0)
            .ok_or_else(|| {
                format!(
                    "No matching palette for tile at ({}, {}) in source image",
                    image.src_x, image.src_y
                )
            })
    }

    /// Finds all subpalettes that includes all colors in `image`.
    pub fn subpalettes_matching(
        &self,
        image: &Image,
    ) -> Result<Vec<&Subpalette>, String> {
        let mut required: BTreeSet<ReducedColor> = image
            .color_data()
            .into_iter()
            .map(|c| self.mode.reduce_color(c))
            .collect();
        required.remove(&ReducedColor::TRANSPARENT);

        if required.len() > self.max_colors_per_subpalette {
            return Err(format!(
                "Tile with too many ({} > {}) unique colors at ({}, {}) in source image",
                required.len(),
                self.max_colors_per_subpalette,
                image.src_x,
                image.src_y
            ));
        }

        Ok(self.subpalettes.iter().filter(|sp| sp.diff(&required) == 0).collect())
    }

    /// Subpalettes ordered by ascending summed color distance to colors in `image`.
    pub fn subpalettes_by_distance(
        &self,
        image: &Image,
    ) -> Vec<&Subpalette> {
        let colors: Vec<NormalizedColor> = image
            .color_data()
            .into_iter()
            .filter(|&c| !self.mode.reduce_color(c).is_transparent())
            .collect();

        let mut scored: Vec<(&Subpalette, f32)> = self
            .subpalettes
            .iter()
            .map(|sp| {
                let candidates: Vec<CandidateColor> = sp
                    .colors
                    .iter()
                    .map(|&r| CandidateColor::new(r, self.mode.normalize_color(r)))
                    .collect();
                let distance = if candidates.is_empty() {
                    f32::INFINITY
                } else {
                    summed_distance(colors.iter().copied(), &candidates)
                };
                (sp, distance)
            })
            .collect();
        scored.sort_by(|a, b| a.1.total_cmp(&b.1));
        scored.into_iter().map(|(sp, _)| sp).collect()
    }

    /// Adds empty subpalette.
    fn add_subpalette(&mut self) -> Result<&mut Subpalette, String> {
        if self.subpalettes.len() >= self.max_subpalettes {
            return Err("Colors do not fit in available palettes".into());
        }
        self.subpalettes
            .push(Subpalette::new(self.mode, self.max_colors_per_subpalette));
        Ok(self.subpalettes.last_mut().unwrap())
    }

    /// Adds subpalette containing `colors`.
    pub fn add_subpalette_with(
        &mut self,
        colors: &[ReducedColor],
    ) -> Result<(), String> {
        let sp = self.add_subpalette()?;
        sp.add_all(colors.iter().copied(), false)
    }

    /// Adds `colors` as one or more subpalette.
    /// - Used for `--no-remap` and when loading an existing palette.
    pub fn add_colors(
        &mut self,
        colors: &[ReducedColor],
    ) -> Result<(), String> {
        for chunk in colors.chunks(self.max_colors_per_subpalette.max(1)) {
            let sp = self.add_subpalette()?;
            sp.add_all(chunk.iter().copied(), true)?;
        }
        Ok(())
    }

    /// Adds subpalettes satisfying every tile's required colors.
    pub fn add_colors_from_tiles(
        &mut self,
        tiles: &[Image],
        effort: Effort,
    ) -> Result<(), String> {
        let capacity = if self.color_zero_is_shared {
            // Shared color_zero will be discarded before packing, adjust max colors
            self.max_colors_per_subpalette.saturating_sub(1)
        } else {
            self.max_colors_per_subpalette
        };

        // Collect required colors
        let mut required_colors: Vec<BTreeSet<ReducedColor>> = Vec::with_capacity(tiles.len());
        for tile in tiles {
            let mut colors: BTreeSet<ReducedColor> =
                tile.colors.iter().map(|&rgba| self.mode.reduce_color(rgba)).collect();
            if self.color_zero_is_shared {
                // Discard shared color_zero from required_colors
                colors.remove(&self.color_zero);
            }
            if colors.len() > capacity {
                return Err(format!(
                    "Tile with too many ({} > {}) unique colors at ({}, {}) in source image",
                    colors.len(),
                    capacity,
                    tile.src_x,
                    tile.src_y
                ));
            }
            if !colors.is_empty() && !required_colors.contains(&colors) {
                required_colors.push(colors);
            }
        }

        // Filter out subsets to improve bin packing performance
        let required_colors: Vec<BTreeSet<ReducedColor>> = required_colors
            .iter()
            .filter(|s| !required_colors.iter().any(|other| other != *s && s.is_subset(other)))
            .cloned()
            .collect();

        let optimized = pack(&required_colors, capacity, effort);
        if optimized.len() > self.max_subpalettes {
            return Err("Colors do not fit in available palettes".into());
        }

        for mut set in optimized {
            if self.color_zero_is_shared {
                set.insert(self.color_zero);
            }
            let mut cv: Vec<ReducedColor> = set.into_iter().collect();
            if self.color_zero_is_shared {
                // Prepend shared color_zero to subpalettes if discarded earlier
                if let Some(pos) = cv.iter().position(|&c| c == self.color_zero) {
                    cv.swap(0, pos);
                }
            }
            let sp = self.add_subpalette()?;
            sp.add_all(cv, false)?;
        }
        Ok(())
    }

    // Creates native palette data.
    pub fn native_data(&self) -> Result<Vec<u8>, String> {
        let mut data = Vec::new();
        for sp in &self.subpalettes {
            data.extend(self.mode.pack_colors(&sp.padded().colors)?);
        }
        Ok(data)
    }

    /// Creates Adobe Color Table (.act) data.
    pub fn act_data(&self) -> Vec<u8> {
        let mut data = vec![0u8; (256 * 3) + 4];
        let mut count = 0usize;
        'outer: for sp in &self.subpalettes {
            for c in sp.padded().normalized_colors() {
                if count >= 256 {
                    break 'outer;
                }
                data[count * 3] = c.r;
                data[count * 3 + 1] = c.g;
                data[count * 3 + 2] = c.b;
                count += 1;
            }
        }
        data[0x300] = 0x00;
        data[0x301] = (count & 0xff) as u8;
        data[0x302] = 0xff;
        data[0x303] = 0xff;
        data
    }

    /// Creates JSON representation.
    pub fn to_json(&self) -> String {
        let hex_palettes: Vec<Vec<String>> = self
            .normalized_colors()
            .iter()
            .map(|p| p.iter().map(|c| c.to_hexstring(false)).collect())
            .collect();

        let native_rgb: Vec<Vec<[u8; 3]>> = self
            .colors()
            .iter()
            .map(|p| p.iter().map(|c| [c.r, c.g, c.b]).collect())
            .collect();

        let json = serde_json::json!({
            "palettes": hex_palettes,
            "palettes_native_rgb": native_rgb,
        });
        serde_json::to_string_pretty(&json).unwrap()
    }

    /// Loads a palette from `path`.
    ///
    /// Loads as JSON if parsable and has a `palettes` array (as written by `to_json`),
    /// otherwise as native binary data (as written by `save`).
    pub fn load(
        path: &Path,
        colors_per_subpalette: usize,
        mode: Mode,
    ) -> Result<Palette, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("File '{}' could not be opened: {e}", path.display()))?;

        let mut palette = Palette::new(mode, 64, colors_per_subpalette);

        let subpalettes_json = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|j| j.get("palettes").cloned());

        if let Some(serde_json::Value::Array(subpalettes)) = subpalettes_json {
            for sp in subpalettes {
                let mut colors = Vec::new();
                if let serde_json::Value::Array(entries) = sp {
                    for entry in entries {
                        if let Some(hex) = entry.as_str() {
                            let normalized = color::from_hexstring(hex)?;
                            colors.push(mode.reduce_color(normalized));
                        }
                    }
                }
                if colors.len() > colors_per_subpalette {
                    return Err("Palette in JSON doesn't match color depth / colors per subpalette".into());
                }
                palette.add_colors(&colors)?;
            }
        } else {
            let colors = mode.unpack_colors(&bytes)?;
            palette.add_colors(&colors)?;
            palette.fix_color_zero_duplicates();
        }

        if palette.subpalettes.is_empty() {
            return Err("No palette data found".into());
        }

        Ok(palette)
    }

    /// Checks if any subpalette has a duplicate of color-zero, and if so clear the duplicates
    /// alpha (marking it as transparent).
    ///
    /// Returns whether any subpalette was fixed.
    fn fix_color_zero_duplicates(&mut self) -> bool {
        if !self.mode.color_zero_is_shared() {
            return false;
        }
        #[allow(clippy::unnecessary_fold)] // Every subpalette must be checked
        self.subpalettes
            .iter_mut()
            .fold(false, |fixed, sp| sp.fix_color_zero_duplicates() || fixed)
    }
}

impl std::fmt::Display for Palette {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let counts: Vec<usize> = self.subpalettes.iter().map(|sp| sp.colors.len()).collect();
        let total: usize = counts.iter().sum();
        if total == 0 {
            return write!(f, "zero colors");
        }
        if counts.len() == 1 {
            write!(f, "{total} colors")
        } else {
            let list = counts.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
            write!(f, "{total} colors [{list}]")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::binpack::Effort::*;

    use super::*;

    fn c(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> ReducedColor {
        ReducedColor::new(r, g, b, a)
    }

    fn palette_from_tiles(
        mode: Mode,
        max_subpalettes: usize,
        max_colors: usize,
        tiles: &[&[NormalizedColor]],
    ) -> Result<Palette, String> {
        let mut palette = Palette::new(mode, max_subpalettes, max_colors);
        let images: Vec<Image> = tiles.iter().map(|colors| Image::from_colors(colors)).collect();
        palette.add_colors_from_tiles(&images, Medium)?;
        Ok(palette)
    }

    #[test]
    fn subpalette_add_dedup() {
        let mut sp = Subpalette::new(Mode::Snes, 2);
        sp.add(c(1, 1, 1, 0xff), false).unwrap();
        sp.add(c(1, 1, 1, 0xff), false).unwrap();
        assert_eq!(sp.colors.len(), 1);
    }

    #[test]
    fn subpalette_add_no_dedup() {
        let mut sp = Subpalette::new(Mode::Snes, 2);
        sp.add(c(1, 1, 1, 0xff), true).unwrap();
        sp.add(c(1, 1, 1, 0xff), true).unwrap();
        assert_eq!(sp.colors.len(), 2);
    }

    #[test]
    fn subpalette_add_full() {
        let mut sp = Subpalette::new(Mode::Snes, 2);
        sp.add(c(1, 1, 1, 0xff), false).unwrap();
        sp.add(c(2, 2, 2, 0xff), false).unwrap();
        assert!(sp.add(c(3, 3, 3, 0xff), false).is_err());
    }

    #[test]
    fn subpalette_padded() {
        let mut sp = Subpalette::new(Mode::Snes, 4);
        sp.add(c(1, 1, 1, 0xff), false).unwrap();
        let padded = sp.padded();
        assert_eq!(padded.colors.len(), 4);
        assert!(padded.colors[1].is_transparent());
        assert!(padded.colors[3].is_transparent());
    }

    #[test]
    fn from_tiles_merging() {
        let red = NormalizedColor::new(255, 0, 0, 0xff);
        let green = NormalizedColor::new(0, 255, 0, 0xff);
        let palette = palette_from_tiles(Mode::Snes, 8, 16, &[&[red], &[green]]).unwrap();
        assert_eq!(palette.colors().len(), 1);
        assert_eq!(palette.colors()[0].len(), 2);
    }

    #[test]
    fn from_tiles_exceed_max_subpalettes() {
        let colors: Vec<NormalizedColor> = (0..6).map(|i| NormalizedColor::new(i * 40, 0, 0, 0xff)).collect();
        let tiles: Vec<&[NormalizedColor]> = colors.iter().map(std::slice::from_ref).collect();
        let result = palette_from_tiles(Mode::Snes, 1, 4, &tiles);
        assert!(result.is_err());
    }

    #[test]
    fn add_colors_no_optimization() {
        let mut palette = Palette::new(Mode::Snes, 8, 2);
        let colors = [c(1, 1, 1, 0xff), c(1, 1, 1, 0xff), c(2, 2, 2, 0xff)];
        palette.add_colors(&colors).unwrap();
        assert_eq!(palette.colors().len(), 2);
        assert_eq!(palette.colors()[0], vec![c(1, 1, 1, 0xff), c(1, 1, 1, 0xff)]);
        assert_eq!(palette.colors()[1], vec![c(2, 2, 2, 0xff)]);
    }

    #[test]
    fn subpalette_matching_finds_subpalette() {
        let red = NormalizedColor::new(255, 0, 0, 0xff);
        let green = NormalizedColor::new(0, 255, 0, 0xff);
        let mut palette = Palette::new(Mode::Snes, 8, 16);
        palette
            .add_colors_from_tiles(&[Image::from_colors(&[red]), Image::from_colors(&[green])], Medium)
            .unwrap();

        let image = Image::from_colors(&[red]);
        let found = palette.subpalette_matching(&image).unwrap();
        assert_eq!(found.diff(&[Mode::Snes.reduce_color(red)].into_iter().collect()), 0);
    }

    #[test]
    fn subpalettes_matching_finds_subpalettes() {
        let red = NormalizedColor::new(255, 0, 0, 0xff);
        let green = NormalizedColor::new(0, 255, 0, 0xff);
        let mut palette = Palette::new(Mode::Snes, 8, 16);
        palette
            .add_colors(&[Mode::Snes.reduce_color(red), Mode::Snes.reduce_color(green)])
            .unwrap();
        palette
            .add_colors(&[
                Mode::Snes.reduce_color(red),
                Mode::Snes.reduce_color(NormalizedColor::new(0, 0, 255, 0xff)),
            ])
            .unwrap();

        let image = Image::from_colors(&[red]);
        let found = palette.subpalettes_matching(&image).unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn subpalette_matching_no_match() {
        let red = NormalizedColor::new(255, 0, 0, 0xff);
        let blue = NormalizedColor::new(0, 0, 255, 0xff);
        let mut palette = Palette::new(Mode::Snes, 8, 16);
        palette
            .add_colors_from_tiles(&[Image::from_colors(&[red])], Medium)
            .unwrap();

        let image = Image::from_colors(&[blue]);
        assert!(palette.subpalette_matching(&image).is_err());
    }

    #[test]
    fn subpalettes_matching_too_many_colors() {
        let colors: Vec<NormalizedColor> = (0..3).map(|i| NormalizedColor::new(i * 40, 0, 0, 0xff)).collect();
        let palette = Palette::new(Mode::Snes, 8, 2);
        let image = Image::from_colors(&colors);
        assert!(palette.subpalettes_matching(&image).is_err());
    }

    #[test]
    fn subpalettes_matching_ignores_transparency() {
        let red = NormalizedColor::new(255, 0, 0, 0xff);
        let transparent = NormalizedColor::TRANSPARENT;
        let mut palette = Palette::new(Mode::Snes, 8, 16);
        palette.add_colors(&[Mode::Snes.reduce_color(red)]).unwrap();

        let image = Image::from_colors(&[red, transparent]);
        let found = palette.subpalettes_matching(&image).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn color_zero_snes_transparent_and_shared() {
        let mut palette = Palette::new(Mode::Snes, 8, 16);
        palette.set_color_zero(NormalizedColor::TRANSPARENT);
        assert!(palette.color_zero.is_transparent());
        assert!(palette.color_zero_is_shared);
    }

    #[test]
    fn fix_color_zero_duplicates_non_shared_modes() {
        let mut palette = Palette::new(Mode::Gb, 8, 4);
        palette
            .add_colors(&[c(1, 1, 1, 255), c(2, 2, 2, 255), c(1, 1, 1, 255)])
            .unwrap();
        assert!(!palette.fix_color_zero_duplicates());
    }

    #[test]
    fn native_data_padding() {
        let mut palette = Palette::new(Mode::Snes, 8, 4);
        palette
            .add_colors(&[Mode::Snes.reduce_color(NormalizedColor::new(255, 0, 0, 0xff))])
            .unwrap();
        let bytes = palette.native_data().unwrap();
        assert_eq!(bytes.len(), 4 * 2);
    }

    #[test]
    fn to_json_and_load() {
        let mut palette = Palette::new(Mode::Snes, 8, 16);
        let colors = [
            Mode::Snes.reduce_color(NormalizedColor::new(255, 0, 0, 0xff)),
            Mode::Snes.reduce_color(NormalizedColor::new(0, 255, 0, 0xff)),
        ];
        palette.add_colors(&colors).unwrap();

        let path = std::env::temp_dir().join("to_json_and_load.json");
        std::fs::write(&path, palette.to_json()).unwrap();
        let loaded = Palette::load(&path, 16, Mode::Snes).unwrap();
        assert_eq!(loaded.colors(), palette.colors());
    }

    #[test]
    fn load_native_binary_if_not_json() {
        let mut palette = Palette::new(Mode::Snes, 8, 4);
        let colors = [
            Mode::Snes.reduce_color(NormalizedColor::new(255, 0, 0, 0xff)),
            Mode::Snes.reduce_color(NormalizedColor::new(0, 255, 0, 0xff)),
        ];
        palette.add_colors(&colors).unwrap();

        let path = std::env::temp_dir().join("load_native_binary_if_not_json.bin");
        std::fs::write(&path, palette.native_data().unwrap()).unwrap();

        let loaded = Palette::load(&path, 4, Mode::Snes).unwrap();
        let black = ReducedColor::new(0, 0, 0, 0xff);
        assert_eq!(
            loaded.colors(),
            vec![vec![c(31, 0, 0, 0xff), c(0, 31, 0, 0xff), black, black]]
        );
    }

    #[test]
    fn tricky_binpacking() -> Result<(), String> {
        for (mode, n_sp, n_colors, path) in [
            (Mode::Gbc, 2, 4, "test_data/tricky_palette_packing/gbc1_max_2x4.png"),
            (Mode::Gbc, 3, 4, "test_data/tricky_palette_packing/gbc2_max_3x4.png"),
            (Mode::Gbc, 4, 4, "test_data/tricky_palette_packing/gbc3_max_4x4.png"),
            (Mode::Snes, 7, 16, "test_data/tricky_palette_packing/snes1_max_7x16.png"),
            (Mode::Snes, 7, 16, "test_data/tricky_palette_packing/snes2_max_7x16.png"),
        ] {
            let image = Image::load(Path::new(path)).unwrap();
            let mut palette = Palette::new(mode, n_sp, n_colors);
            if mode.color_zero_is_shared() {
                palette.set_color_zero(image.infer_color_zero(mode));
            }
            palette
                .add_colors_from_tiles(&image.crops(8, 8, mode), Medium)
                .map_err(|e| format!("pack_tricky_palettes failed for {path}: {e}"))?;
            assert!(
                palette.subpalettes.len() <= n_sp,
                "pack_tricky_palettes failed for {path}"
            );
        }
        Ok(())
    }
}
