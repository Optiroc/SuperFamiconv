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
use crate::map::Map;
use crate::mode::{
    Mode,
    color::{ColorRounding, ModeColor},
};
use crate::palette::Palette;
use crate::quantize::quantize_palette;
use crate::tileset::Tileset;

struct MapPaths<'a> {
    pub in_attribute_map: Option<&'a PathBuf>,
    pub out_preview: Option<&'a PathBuf>,
    pub out_data: Option<&'a PathBuf>,
    pub out_json: Option<&'a PathBuf>,
    pub out_palette_map: Option<&'a PathBuf>,
    pub out_tile_map: Option<&'a PathBuf>,
    pub out_attribute_map: Option<&'a PathBuf>,
    pub out_mode7_data: Option<&'a PathBuf>,
}

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

fn check_dimensions(
    mode: Mode,
    image: &Image,
    tile_width: u32,
    tile_height: u32,
) -> Result<(), String> {
    match mode {
        Mode::PceSprite => {
            if !image.width.is_multiple_of(tile_width) || !image.height.is_multiple_of(tile_height) {
                Err(format!(
                    "Image dimensions must be a multiple of the sprite size ({}x{}) for mode '{}'",
                    tile_width, tile_height, mode
                ))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
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

fn make_map(
    image: &Image,
    tileset: &Tileset,
    palette: &Palette,
    mode: Mode,
    tile_width: u32,
    tile_height: u32,
    max_tile_count: u32,
    bpp: u32,
    quantize: bool,
    dither: Dither,
    rounding: ColorRounding,
    logger: Logger,
) -> Result<Map, String> {
    let map_width = image.width.div_ceil(tile_width);
    let map_height = image.height.div_ceil(tile_height);
    let mut map = Map::new(
        mode,
        map_width,
        map_height,
        tile_width,
        tile_height,
        max_tile_count,
        quantize,
        dither,
        rounding,
    );

    let slices = image.sliced(tile_width, tile_height, mode);
    let slice_count = slices.len();
    logger.verbose(format!(
        "Mapping {slice_count} {tile_width}x{tile_height}px tiles from image"
    ));

    let mut unmatched = 0u32;
    for (i, slice) in slices.enumerate() {
        let i = i as u32;
        if !map.add(&slice, tileset, palette, bpp, i % map_width, i / map_width)? {
            unmatched += 1;
        }
    }

    if unmatched > 0 {
        let hint = if quantize {
            "\n> With --quantize, make sure to use the same dithering setting for both tileset and map"
        } else {
            ""
        };
        Logger::error(format!(
            "> {unmatched} of {slice_count} tiles had no match in the tileset{hint}"
        ));
    }

    Ok(map)
}

fn finalize_map(
    map: &mut Map,
    tileset: &Tileset,
    palette: &Palette,
    mode: Mode,
    split_width: u32,
    split_height: u32,
    column_order: bool,
    tile_base_offset: i32,
    palette_base_offset: i32,
    paths: MapPaths,
    logger: Logger,
) -> Result<(), String> {
    if let Some(path) = paths.in_attribute_map {
        if mode.priority_map_is_supported() {
            let priorities = load_priority_map(
                path,
                mode,
                map.width(),
                map.height(),
                map.tile_width(),
                map.tile_height(),
                logger,
            )?;
            map.set_priorities(&priorities);
            logger.verbose(format!("Loaded attribute map from '{}'", path.display()));
        } else {
            Logger::error(format!("Attribute map not supported for mode '{mode}'"));
        }
    }

    if let Some(path) = paths.out_preview {
        map.preview(tileset, palette)?.save_rgba(path)?;
        logger.verbose(format!("Saved map image to '{}'", path.display()));
    }

    let desc = map.description(split_width, split_height, column_order);
    logger.verbose(format!("Map laid out in {desc}"));
    if tile_base_offset != 0 {
        logger.verbose(format!("Tile base offset: {tile_base_offset}"));
    }
    if palette_base_offset != 0 {
        logger.verbose(format!("Palette base offset: {palette_base_offset}"));
    }
    if tile_base_offset != 0 {
        map.add_base_offset(tile_base_offset);
    }
    if palette_base_offset != 0 {
        map.add_palette_base_offset(palette_base_offset);
    }
    if let Some(warning) = map.get_tile_count_warning() {
        Logger::error(warning);
    }

    if let Some(path) = paths.out_data {
        let data = map.to_native_data(split_width, split_height, column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native map data to '{}'", path.display()));
    }
    if let Some(path) = paths.out_json {
        let json = map.to_json(split_width, split_height, column_order);
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved JSON map data to '{}'", path.display()));
    }
    if let Some(path) = paths.out_palette_map {
        let data = map.get_palette_map(split_width, split_height, column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved palette map to '{}'", path.display()));
    }
    if let Some(path) = paths.out_tile_map {
        let data = map.get_tile_map(split_width, split_height, column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved tile map to '{}'", path.display()));
    }
    if let Some(path) = paths.out_attribute_map {
        let data = map.get_attribute_map(split_width, split_height, column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved attribute map to '{}'", path.display()));
    }
    if let Some(path) = paths.out_mode7_data {
        if mode == Mode::SnesMode7 {
            let data = map.get_snes_mode7_interleaved_data(tileset)?;
            std::fs::write(path, data).map_err(|e| e.to_string())?;
            logger.verbose(format!("Saved interleaved data to '{}'", path.display()));
        } else {
            Logger::error(format!("Warning: --out-mode7-data not supported for mode '{mode}'"));
        }
    }

    Ok(())
}
