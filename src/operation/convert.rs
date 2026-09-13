//! Convert operation: settings and execution.

use std::path::PathBuf;

use crate::color::NormalizedColor;
use crate::dither::Dither;
use crate::logger::Logger;
use crate::mode::Mode;
use crate::mode::color::ColorRounding;
use crate::tileset::Tileset;

#[derive(Debug, PartialEq, Eq)]
pub struct ConvertSettings {
    pub in_image: Vec<PathBuf>,
    pub in_attribute_map: Option<PathBuf>,
    pub out_palette: Option<PathBuf>,
    pub out_tiles: Option<PathBuf>,
    pub out_map: Option<PathBuf>,
    pub out_palette_map: Option<PathBuf>,
    pub out_tile_map: Option<PathBuf>,
    pub out_attribute_map: Option<PathBuf>,
    pub out_mode7_data: Option<PathBuf>,
    pub out_palette_image: Option<PathBuf>,
    pub out_tile_image: Option<PathBuf>,
    pub out_preview_image: Option<PathBuf>,
    pub out_palette_act: Option<PathBuf>,

    pub mode: Mode,
    pub bpp: u32,
    pub palettes: u32,
    pub colors: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub no_remap: bool,
    pub no_discard: bool,
    pub no_flip: bool,
    pub max_tiles: u32,
    pub color_zero: Option<NormalizedColor>,
    pub quantize: bool,
    pub dither: Dither,
    pub rounding: ColorRounding,
    pub tile_base_offset: i32,
    pub palette_base_offset: i32,

    pub logger: Logger,
}

pub fn execute(settings: ConvertSettings) -> Result<(), String> {
    let logger = settings.logger;
    logger.verbose(format!("Performing convert operation (mode: {})", settings.mode));

    let image = super::load_images(settings.in_image, settings.logger)?;
    super::check_dimensions(settings.mode, &image, settings.tile_width, settings.tile_height)?;
    let color_zero = super::resolve_color_zero(settings.mode, settings.color_zero, &image, settings.rounding);

    let (palette, image) = super::make_palette(
        &image,
        settings.mode,
        settings.palettes,
        settings.colors,
        settings.tile_width,
        settings.tile_height,
        settings.no_remap,
        color_zero,
        settings.quantize,
        settings.dither,
        settings.rounding,
        settings.logger,
    )?;

    if let Some(path) = &settings.out_palette {
        std::fs::write(path, palette.native_data()?).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native palette data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_palette_act {
        std::fs::write(path, palette.act_data()).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved ACT palette to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_palette_image {
        palette.preview()?.save_rgba(path)?;
        logger.verbose(format!("Saved palette image to '{}'", path.display()));
    }

    let mut tileset = Tileset::new(
        settings.mode,
        settings.bpp,
        settings.tile_width,
        settings.tile_height,
        settings.no_discard,
        settings.no_flip,
        false, // In convert mode no-remap only applies to palette
        false,
        Dither::Off,
        settings.rounding,
        settings.max_tiles,
    );
    for slice in image.sliced(settings.tile_width, settings.tile_height, settings.mode) {
        tileset.add(&slice, Some(&palette))?;
    }
    if tileset.is_full() {
        return Err(format!(
            "Tileset exceeds maximum size ({} entries generated, {} maximum)",
            tileset.size(),
            tileset.max()
        ));
    }
    if settings.no_discard {
        logger.verbose(format!("Created tileset with {} entries", tileset.size()));
    } else {
        logger.verbose(format!(
            "Created tileset with {} entries ({} tiles deduplicated)",
            tileset.size(),
            tileset.discarded_tiles
        ));
    }

    if let Some(path) = &settings.out_tiles {
        std::fs::write(path, tileset.to_native_data()?).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native tile data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_tile_image {
        tileset.preview(None)?.save_rgba(path)?;
        logger.verbose(format!("Saved tileset image to '{}'", path.display()));
    }

    if settings.mode.map_generation_is_supported() {
        // Skip map generation if no map or preview outputs
        let no_map_output = settings.out_map.is_none()
            && settings.out_palette_map.is_none()
            && settings.out_tile_map.is_none()
            && settings.out_attribute_map.is_none()
            && settings.out_mode7_data.is_none()
            && settings.out_preview_image.is_none();
        if no_map_output {
            return Ok(());
        }

        let split_size = settings.mode.default_map_size().unwrap();

        let mut map = super::make_map(
            &image,
            &tileset,
            &palette,
            settings.mode,
            settings.tile_width,
            settings.tile_height,
            settings.max_tiles,
            settings.bpp,
            false,
            Dither::Off,
            settings.rounding,
            logger,
        )?;

        let paths = super::MapPaths {
            in_attribute_map: settings.in_attribute_map.as_ref(),
            out_preview: settings.out_preview_image.as_ref(),
            out_data: settings.out_map.as_ref(),
            out_json: None,
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
            split_size,
            split_size,
            false,
            settings.tile_base_offset,
            settings.palette_base_offset,
            paths,
            logger,
        )?;
    } else {
        let no_map_output = settings.out_map.is_none()
            && settings.out_palette_map.is_none()
            && settings.out_tile_map.is_none()
            && settings.out_attribute_map.is_none()
            && settings.out_mode7_data.is_none();
        if !no_map_output {
            Logger::error(format!("Map output not supported for mode '{}'", settings.mode));
        }
        if settings.out_preview_image.is_some() {
            Logger::error(format!("Preview image not supported for mode '{}'", settings.mode));
        }
    }

    Ok(())
}
