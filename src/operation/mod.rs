//! Operation: settings and execution of high level
//! convert, palette, tiles and map operations.

pub mod convert;
pub mod map;
pub mod palette;
pub mod tiles;

use crate::binpack::Effort;
use crate::color::{NormalizedColor, ReducedColor};
use crate::dither::Dither;
use crate::image::Image;
use crate::logger::Logger;
use crate::mode::{Mode, color::ModeColor};
use crate::palette::Palette;
use crate::quantize::quantize_palette;

pub fn resolve_sprite_mode(
    mode: Mode,
    sprite_mode: bool,
) -> (Mode, bool) {
    let mode = if sprite_mode && mode == Mode::Pce {
        Mode::PceSprite
    } else {
        mode
    };
    let sprite_mode = sprite_mode || mode == Mode::PceSprite;
    (mode, sprite_mode)
}

pub fn resolve_no_flip(
    explicit: bool,
    mode: Mode,
) -> bool {
    explicit || !mode.tile_flipping_is_allowed()
}

fn resolve_color_zero(
    mode: Mode,
    color_zero: Option<NormalizedColor>,
    image: &Image,
    sprite_mode: bool,
) -> Option<NormalizedColor> {
    if sprite_mode {
        Some(NormalizedColor::TRANSPARENT)
    } else if color_zero.is_some() || mode.color_zero_is_shared() {
        Some(color_zero.unwrap_or_else(|| image.infer_color_zero(mode)))
    } else {
        None
    }
}

fn load_image(
    path: &std::path::Path,
    logger: Logger,
) -> Result<Image, String> {
    let image = Image::load(path)?;
    logger.verbose(format!("Loaded image from '{}' ({image})", path.display()));
    Ok(image)
}

#[allow(clippy::too_many_arguments)]
fn make_palette(
    image: &Image,
    mode: Mode,
    max_subpalettes: u32,
    max_colors_per_subpalette: u32,
    tile_width: u32,
    tile_height: u32,
    effort: Effort,
    no_remap: bool,
    color_zero: Option<NormalizedColor>,
    quantize: bool,
    dither: Dither,
    logger: Logger,
) -> Result<(Palette, Image), String> {
    let mut palette;
    let out_image;

    if no_remap {
        // No remap: map colors from image.palette directly
        if image.palette_size() == 0 {
            return Err("no-remap requires indexed color image".into());
        }
        logger.verbose("Mapping palette straight from indexed color image");
        palette = Palette::new(mode, max_subpalettes as usize, max_colors_per_subpalette as usize);
        let colors: Vec<ReducedColor> = image.palette.iter().map(|&c| mode.reduce_color(c)).collect();
        palette.add_colors(&colors)?;
        out_image = image.clone();
    } else if quantize {
        // Quantize: create best-effort palette and matching image
        logger.verbose(format!(
            "Quantizing palette with at most {max_subpalettes}x{max_colors_per_subpalette} entries"
        ));

        let capacity: usize;
        if let Some(color_zero) = color_zero {
            if color_zero.is_transparent() {
                logger.verbose("Locking color zero to transparent");
            } else {
                logger.verbose(format!("Locking color zero to {}", color_zero.to_hexstring(true)));
            }
            capacity = (max_colors_per_subpalette as usize).saturating_sub(1)
        } else {
            capacity = max_colors_per_subpalette as usize;
        }

        (palette, out_image) = quantize_palette(
            image,
            mode,
            max_subpalettes as usize,
            capacity,
            color_zero,
            tile_width,
            tile_height,
            dither,
        )?;
    } else {
        // Default: lossless palette packing
        logger.verbose(format!(
            "Mapping palette with at most {max_subpalettes}x{max_colors_per_subpalette} entries"
        ));
        palette = Palette::new(mode, max_subpalettes as usize, max_colors_per_subpalette as usize);

        if let Some(color_zero) = color_zero {
            if color_zero.is_transparent() {
                logger.verbose("Locking color zero to transparent");
            } else {
                logger.verbose(format!("Locking color zero to {}", color_zero.to_hexstring(true)));
            }
            palette.set_color_zero(color_zero);
        }

        palette.add_colors_from_tiles(&image.crops(tile_width, tile_height, mode), effort)?;
        out_image = image.clone();
    }

    logger.verbose(format!("Created palette with {palette}"));
    if !no_remap {
        palette.sort();
    }

    Ok((palette, out_image))
}
