//! Tile- and dithering-aware palette quantization.

pub(crate) mod batch;
pub(crate) mod incremental;

use clap::ValueEnum;
use quantette::PaletteSize;
use quantette::color_space::oklab_to_srgb8;
use quantette::deps::palette::Oklab;
use quantette::kmeans::{Kmeans, KmeansOptions};
use quantette::wu::{BinnerF32x3, WuF32x3};

use crate::color::{NormalizedColor, ReducedColor};
use crate::dither::{self, Dither};
use crate::image::Image;
use crate::mode::{
    Mode,
    color::{ColorRounding, ModeColor},
};
use crate::palette::{Palette, Subpalette};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum Method {
    /// No quantization.
    #[default]
    #[value(skip)]
    Off,
    /// Batch k-means.
    Batch,
    /// Incremental k-means.
    #[value(alias = "inc")]
    Incremental,
}

impl std::fmt::Display for Method {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let m = match self {
            Method::Off => "none",
            Method::Batch => "batch",
            Method::Incremental => "incremental",
        };
        write!(f, "{m}")
    }
}

/// Creates a palette and a matching quantized image for `image` using `method`.
pub fn quantize_palette(
    image: &Image,
    mode: Mode,
    max_subpalettes: usize,
    capacity: usize,
    color_zero: Option<NormalizedColor>,
    tile_width: u32,
    tile_height: u32,
    quant_method: Method,
    dither: Dither,
    rounding: ColorRounding,
) -> Result<(Palette, Image), String> {
    let max_subpalettes = max_subpalettes.max(1);
    let capacity = capacity.max(1);

    if quant_method == Method::Incremental {
        incremental::dither_available(dither)?;
    }

    // Try lossless binpacking on dithered tile palettes first
    if let Some(result) = try_lossless(
        image,
        mode,
        max_subpalettes,
        capacity,
        color_zero,
        tile_width,
        tile_height,
        dither,
        rounding,
    ) {
        return Ok(result);
    }

    let quantize_fn = match quant_method {
        Method::Off => unreachable!("quantize_palette called with Method::Off"),
        Method::Batch => batch::quantize_image,
        Method::Incremental => incremental::quantize_image,
    };
    quantize_fn(
        image,
        mode,
        max_subpalettes,
        capacity,
        color_zero,
        tile_width,
        tile_height,
        dither,
        rounding,
    )
}

/// Maps every pixel of `image` to its closest color in `subpalette`.
///
/// Returns palette indices and the matching full precision colors.
pub fn remap(
    image: &Image,
    subpalette: &Subpalette,
    quant_method: Method,
    dither: Dither,
    rounding: ColorRounding,
) -> Result<(Vec<u8>, Vec<NormalizedColor>), String> {
    match quant_method {
        Method::Off => Err("Remapping to a quantized palette requires a quantization method".into()),
        Method::Batch => Ok(batch::remap(image, subpalette, dither, rounding)),
        Method::Incremental => incremental::remap(image, subpalette, dither, rounding),
    }
}

/// Attempts to binpack palettes dithered tiles.
fn try_lossless(
    image: &Image,
    mode: Mode,
    max_subpalettes: usize,
    capacity: usize,
    color_zero: Option<NormalizedColor>,
    tile_width: u32,
    tile_height: u32,
    dither: Dither,
    rounding: ColorRounding,
) -> Option<(Palette, Image)> {
    let color_zero_reduced = color_zero.map(|c| mode.reduce_color(c, rounding));
    let max_colors_per_subpalette = capacity + usize::from(color_zero.is_some());
    let budget = dither::TileBudget {
        width: tile_width,
        height: tile_height,
        max_colors: max_colors_per_subpalette,
    };
    let image = dither::dither_to_mode(image, mode, dither, rounding, color_zero_reduced, budget)?;
    let mut palette = Palette::new(mode, max_subpalettes, max_colors_per_subpalette, rounding);
    if let Some(color_zero) = color_zero {
        palette.set_color_zero(color_zero);
    }

    let slices: Vec<Image> = image.sliced(tile_width, tile_height, mode).collect();
    palette.add_colors_from_tiles(&slices).ok()?;
    palette.sort();

    Some((palette, image.clone()))
}

/// Seeds a palette with Wu quantization and refines it with k-means.
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

/// Attempt to fill up `current` to `capacity` by refitting `colors`.
fn grow_to_capacity<const B1: usize, const B2: usize, const B3: usize>(
    mut current: Vec<ReducedColor>,
    colors: &[Oklab],
    capacity: usize,
    mode: Mode,
    rounding: ColorRounding,
    binner: BinnerF32x3<B1, B2, B3>,
) -> Result<Vec<ReducedColor>, String> {
    const REFIT_ATTEMPTS: usize = 4;
    let mut k = capacity;
    for _ in 0..REFIT_ATTEMPTS {
        if current.len() >= capacity || k >= colors.len() {
            break;
        }
        k += capacity - current.len();
        let mut next = dedup_reduced(&fit_palette(colors, k, binner)?, mode, rounding);
        next.truncate(capacity);
        if next.len() > current.len() {
            current = next;
        }
    }
    Ok(current)
}

/// Colors in `palette`, mode-reduced and deduplicated.
fn dedup_reduced(
    palette: &[Oklab],
    mode: Mode,
    rounding: ColorRounding,
) -> Vec<ReducedColor> {
    let mut reduced: Vec<ReducedColor> = Vec::new();
    for srgb in oklab_to_srgb8(palette) {
        let normalized = NormalizedColor::new(srgb.red, srgb.green, srgb.blue, 0xff);
        let r = mode.reduce_color(normalized, rounding);
        if !r.is_transparent() && !reduced.contains(&r) {
            reduced.push(r);
        }
    }
    reduced
}
