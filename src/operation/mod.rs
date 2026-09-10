//! Operation: settings and execution of high level
//! convert, palette, tiles and map operations.

pub mod convert;
pub mod map;
pub mod palette;
pub mod tiles;

use std::path::PathBuf;

use crate::color::{NormalizedColor, ReducedColor};
use crate::dither::Dither;
use crate::image::Image;
use crate::logger::Logger;
use crate::mode::{
    Mode,
    color::{ColorRounding, ModeColor},
};
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
    explicit || !mode.tile_flipping_is_supported()
}

fn resolve_color_zero(
    mode: Mode,
    color_zero: Option<NormalizedColor>,
    image: &Image,
    rounding: ColorRounding,
) -> Option<NormalizedColor> {
    if color_zero.is_some() || mode.color_zero_is_shared() {
        Some(color_zero.unwrap_or_else(|| image.infer_color_zero(mode, rounding)))
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

fn load_images(
    paths: Vec<PathBuf>,
    logger: Logger,
) -> Result<Image, String> {
    match paths.len() {
        0 => Err("Input image required".into()),
        1 => load_image(paths.first().unwrap(), logger),
        _ => {
            let image = Image::load_many(paths)?;
            logger.verbose(format!("Loaded multiple images into {image}"));
            Ok(image)
        }
    }
}

fn load_priority_map(
    path: &std::path::Path,
    mode: Mode,
    map_width: u32,
    map_height: u32,
    tile_width: u32,
    tile_height: u32,
    logger: Logger,
) -> Result<Vec<bool>, String> {
    let image = load_image(path, logger)?;
    let expected_width = map_width * tile_width;
    let expected_height = map_height * tile_height;
    if image.width != expected_width || image.height != expected_height {
        return Err(format!(
            "Attribute map '{}' ({}x{}) doesn't match image size ({expected_width}x{expected_height})",
            path.display(),
            image.width,
            image.height,
        ));
    }

    Ok(image
        .sliced(tile_width, tile_height, mode)
        .map(|tile| {
            tile.data
                .iter()
                .any(|c| !c.is_transparent() && (c.r, c.g, c.b) != (0, 0, 0))
        })
        .collect())
}

fn make_palette(
    image: &Image,
    mode: Mode,
    max_subpalettes: u32,
    max_colors_per_subpalette: u32,
    tile_width: u32,
    tile_height: u32,
    no_remap: bool,
    color_zero: Option<NormalizedColor>,
    quantize: bool,
    dither: Dither,
    rounding: ColorRounding,
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
        palette = Palette::new(
            mode,
            max_subpalettes as usize,
            max_colors_per_subpalette as usize,
            rounding,
        );
        let colors: Vec<ReducedColor> = image.palette.iter().map(|&c| mode.reduce_color(c, rounding)).collect();
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
            rounding,
        )?;
    } else {
        // Default: lossless palette packing
        logger.verbose(format!(
            "Mapping palette with at most {max_subpalettes}x{max_colors_per_subpalette} entries"
        ));
        palette = Palette::new(
            mode,
            max_subpalettes as usize,
            max_colors_per_subpalette as usize,
            rounding,
        );

        if let Some(color_zero) = color_zero {
            if color_zero.is_transparent() {
                logger.verbose("Locking color zero to transparent");
            } else {
                logger.verbose(format!("Locking color zero to {}", color_zero.to_hexstring(true)));
            }
            palette.set_color_zero(color_zero);
        }

        let slices: Vec<Image> = image.sliced(tile_width, tile_height, mode).collect();
        palette.add_colors_from_tiles(&slices)?;
        out_image = image.clone();
    }

    logger.verbose(format!("Created palette with {palette}"));
    if !no_remap {
        palette.sort();
    }

    Ok((palette, out_image))
}
