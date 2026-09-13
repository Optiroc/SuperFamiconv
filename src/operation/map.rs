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
        // Make map from native data
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
        // Make map from image
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

        super::make_map(
            &image,
            &tileset,
            &palette,
            settings.mode,
            settings.tile_width,
            settings.tile_height,
            settings.mode.max_tile_count(),
            settings.bpp,
            settings.quantize,
            settings.dither,
            settings.rounding,
            logger,
        )?
    };

    let paths = super::MapPaths {
        in_attribute_map: settings.in_attribute_map.as_ref(),
        out_preview: settings.out_image.as_ref(),
        out_data: settings.out_data.as_ref(),
        out_json: settings.out_json.as_ref(),
        out_palette_map: settings.out_palette_map.as_ref(),
        out_tile_map: settings.out_tile_map.as_ref(),
        out_attribute_map: settings.out_attribute_map.as_ref(),
        out_mode7_data: settings.out_mode7_data.as_ref(),
    };

    super::finalize_map(
        &mut map,
        &tileset,
        &palette,
        settings.mode,
        settings.split_width,
        settings.split_height,
        settings.column_order,
        settings.tile_base_offset,
        settings.palette_base_offset,
        paths,
        logger,
    )
}
