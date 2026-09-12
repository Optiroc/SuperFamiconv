//! Map operation: settings and execution.

use std::path::PathBuf;

use crate::dither::Dither;
use crate::logger::Logger;
use crate::map::Map;
use crate::mode::{Mode, color::ColorRounding};
use crate::palette::{Palette, palette_size_at_bpp};
use crate::tileset::Tileset;

#[derive(Debug, PartialEq, Eq)]
pub struct MapSettings {
    pub in_image: Option<PathBuf>,
    pub in_data: Option<PathBuf>,
    pub in_palette: PathBuf,
    pub in_tiles: PathBuf,
    pub in_attribute_map: Option<PathBuf>,
    pub out_data: Option<PathBuf>,
    pub out_json: Option<PathBuf>,
    pub out_image: Option<PathBuf>,
    pub out_palette_map: Option<PathBuf>,
    pub out_tile_map: Option<PathBuf>,
    pub out_attribute_map: Option<PathBuf>,
    pub out_mode7_data: Option<PathBuf>,

    pub mode: Mode,
    pub bpp: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub no_flip: bool,
    pub quantize: bool,
    pub dither: Dither,
    pub rounding: ColorRounding,
    pub map_width: Option<u32>,
    pub map_height: Option<u32>,
    pub split_width: u32,
    pub split_height: u32,
    pub tile_base_offset: i32,
    pub palette_base_offset: i32,
    pub column_order: bool,

    pub logger: Logger,
}

pub fn execute(settings: MapSettings) -> Result<(), String> {
    let logger = settings.logger;
    logger.verbose(format!("Performing map operation (mode: {})", settings.mode));

    let colors_per_subpalette = palette_size_at_bpp(settings.bpp) as usize;
    let palette = Palette::load(
        &settings.in_palette,
        colors_per_subpalette,
        settings.mode,
        settings.rounding,
    )?;
    if palette.size() < 1 {
        return Err("Input palette size is zero".into());
    }
    logger.verbose(format!(
        "Loaded palette from '{}' ({})",
        settings.in_palette.display(),
        palette
    ));

    let tile_bytes = std::fs::read(&settings.in_tiles)
        .map_err(|e| format!("File '{}' could not be opened: {e}", settings.in_tiles.display()))?;
    let tileset = Tileset::from_native_data(
        &tile_bytes,
        settings.mode,
        settings.bpp,
        settings.tile_width,
        settings.tile_height,
        settings.no_flip,
    )?;
    logger.verbose(format!(
        "Loaded tiles from '{}' ({} entries)",
        settings.in_tiles.display(),
        tileset.size()
    ));

    let mut map = if let Some(in_data) = &settings.in_data {
        let map_width = settings
            .map_width
            .ok_or("Map width required when reading native map data")?;
        let map_height = settings
            .map_height
            .ok_or("Map height required when reading native map data")?;
        let bytes =
            std::fs::read(in_data).map_err(|e| format!("File '{}' could not be opened: {e}", in_data.display()))?;
        let map = Map::from_native_data(
            &bytes,
            settings.mode,
            map_width,
            map_height,
            settings.tile_width,
            settings.tile_height,
            settings.split_width,
            settings.split_height,
            settings.column_order,
        )?;
        logger.verbose(format!(
            "Loaded map from '{}' ({map_width}x{map_height} tiles)",
            in_data.display()
        ));
        map
    } else {
        let in_image = settings.in_image.as_ref().expect("in_image or in_data required");
        let mut image = super::load_image(in_image, settings.logger)?;

        let map_width = settings
            .map_width
            .unwrap_or_else(|| image.width.div_ceil(settings.tile_width));
        let map_height = settings
            .map_height
            .unwrap_or_else(|| image.height.div_ceil(settings.tile_height));

        if map_width * settings.tile_width != image.width || map_height * settings.tile_height != image.height {
            image = image.slice(
                0,
                0,
                map_width * settings.tile_width,
                map_height * settings.tile_height,
                settings.mode,
            );
        }

        let dimensions = image.slice_dimensions(settings.tile_width, settings.tile_height);
        let slice_count = dimensions.0 * dimensions.1;
        logger.verbose(format!(
            "Mapping {slice_count} {}x{}px tiles from image",
            settings.tile_width, settings.tile_height
        ));

        let mut map = Map::new(
            settings.mode,
            map_width,
            map_height,
            settings.tile_width,
            settings.tile_height,
            settings.quantize,
            settings.dither,
            settings.rounding,
        );

        let slices = image.sliced(settings.tile_width, settings.tile_height, settings.mode);
        let mut unmatched = 0u32;
        for (i, slice) in slices.enumerate() {
            let i = i as u32;
            if !map.add(&slice, &tileset, &palette, settings.bpp, i % map_width, i / map_width)? {
                unmatched += 1;
            }
        }
        if unmatched > 0 {
            let hint = if settings.quantize {
                "\n> With --quantize, make sure to use the same dithering setting for both tileset and map"
            } else {
                ""
            };
            Logger::error(format!(
                "> {unmatched} of {slice_count} tiles had no match in the tileset{hint}"
            ));
        }
        map
    };

    if let Some(path) = &settings.in_attribute_map {
        if settings.mode.priority_map_is_supported() {
            let priorities = super::load_priority_map(
                path,
                settings.mode,
                map.width(),
                map.height(),
                settings.tile_width,
                settings.tile_height,
                logger,
            )?;
            map.set_priorities(&priorities);
            logger.verbose(format!("Loaded attribute map from '{}'", path.display()));
        } else {
            Logger::error(format!("Attribute map not supported for mode '{}'", settings.mode));
        }
    }

    if settings.tile_base_offset != 0 {
        map.add_base_offset(settings.tile_base_offset);
    }
    if settings.palette_base_offset != 0 {
        map.add_palette_base_offset(settings.palette_base_offset);
    }

    let desc = map.description(settings.split_width, settings.split_height, settings.column_order);
    logger.verbose(format!("Map laid out in {desc}"));
    if settings.tile_base_offset != 0 {
        logger.verbose(format!("  Tile base offset: {}", settings.tile_base_offset));
    }
    if settings.palette_base_offset != 0 {
        logger.verbose(format!("  Palette base offset: {}", settings.palette_base_offset));
    }

    if let Some(path) = &settings.out_data {
        let data = map.to_native_data(settings.split_width, settings.split_height, settings.column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native map data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_json {
        let json = map.to_json(settings.split_width, settings.split_height, settings.column_order);
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved JSON map data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_image {
        map.preview(&tileset, &palette)?.save_rgba(path)?;
        logger.verbose(format!("Saved map image to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_palette_map {
        let data = map.get_palette_map(settings.split_width, settings.split_height, settings.column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved palette map to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_tile_map {
        let data = map.get_tile_map(settings.split_width, settings.split_height, settings.column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved tile map to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_attribute_map {
        let data = map.get_attribute_map(settings.split_width, settings.split_height, settings.column_order);
        std::fs::write(path, data).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved attribute map to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_mode7_data {
        if settings.mode == Mode::SnesMode7 {
            let data = map.get_snes_mode7_interleaved_data(&tileset)?;
            std::fs::write(path, data).map_err(|e| e.to_string())?;
            logger.verbose(format!("Saved interleaved data to '{}'", path.display()));
        } else {
            Logger::error(format!(
                "Warning: --out-mode7-data not supported for mode '{}'",
                settings.mode
            ))
        }
    }

    Ok(())
}
