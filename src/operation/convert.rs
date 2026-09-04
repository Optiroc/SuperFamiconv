//! Convert operation: settings and execution.

use std::path::PathBuf;

use crate::binpack::Effort;
use crate::color::NormalizedColor;
use crate::dither::Dither;
use crate::image::Image;
use crate::logger::Logger;
use crate::map::Map;
use crate::mode::Mode;
use crate::tileset::Tileset;

#[derive(Debug, PartialEq, Eq)]
pub struct ConvertSettings {
    pub in_image: PathBuf,
    pub out_palette: Option<PathBuf>,
    pub out_tiles: Option<PathBuf>,
    pub out_map: Option<PathBuf>,
    pub out_palette_image: Option<PathBuf>,
    pub out_palette_act: Option<PathBuf>,
    pub out_tiles_image: Option<PathBuf>,
    pub out_preview_image: Option<PathBuf>,

    pub mode: Mode,
    pub bpp: u32,
    pub palettes: u32,
    pub colors: u32,
    pub effort: Effort,
    pub tile_width: u32,
    pub tile_height: u32,
    pub no_remap: bool,
    pub no_discard: bool,
    pub no_flip: bool,
    pub max_tiles: u32,
    pub sprite_mode: bool,
    pub color_zero: Option<NormalizedColor>,
    pub quantize: bool,
    pub dither: Dither,
    pub tile_base_offset: i32,
    pub palette_base_offset: i32,

    pub logger: Logger,
}

pub fn execute(settings: ConvertSettings) -> Result<(), String> {
    let logger = settings.logger;
    logger.verbose(format!("Performing convert operation (mode: {})", settings.mode));

    let image = super::load_image(&settings.in_image, settings.logger)?;
    let color_zero = super::resolve_color_zero(settings.mode, settings.color_zero, &image, settings.sprite_mode);

    if settings.mode == Mode::PceSprite && (image.width % 16 != 0 || image.height % 16 != 0) {
        return Err("pce_sprite mode requires image dimensions to be a multiple of 16".into());
    }

    let (palette, image) = super::make_palette(
        &image,
        settings.mode,
        settings.palettes,
        settings.colors,
        settings.tile_width,
        settings.tile_height,
        settings.effort,
        settings.no_remap,
        color_zero,
        settings.quantize,
        settings.dither,
        settings.logger,
    )?;

    if let Some(path) = &settings.out_preview_image {
        image.save_quantized(path, settings.mode)?;
        logger.verbose(format!("Saved preview image to '{}'", path.display()));
    }

    if let Some(path) = &settings.out_palette {
        std::fs::write(path, palette.native_data()?).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native palette data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_palette_act {
        std::fs::write(path, palette.act_data()).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved ACT palette to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_palette_image {
        Image::from_palette(&palette)?.save(path)?;
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
        settings.max_tiles,
    );
    for crop in image.crops(settings.tile_width, settings.tile_height, settings.mode) {
        tileset.add(&crop, Some(&palette))?;
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
    if let Some(path) = &settings.out_tiles_image {
        Image::from_tileset(&tileset, None)?.save(path)?;
        logger.verbose(format!("Saved tileset image to '{}'", path.display()));
    }

    if let Some(path) = &settings.out_map {
        if settings.mode.map_generation_is_supported() {
            let map_width = image.width.div_ceil(settings.tile_width);
            let map_height = image.height.div_ceil(settings.tile_height);
            let split_size = settings.mode.default_map_size().unwrap();

            let crops = image.crops(settings.tile_width, settings.tile_height, settings.mode);
            logger.verbose(format!(
                "Mapping {} {}x{}px tiles from image",
                crops.len(),
                settings.tile_width,
                settings.tile_height
            ));

            let mut map = Map::new(
                settings.mode,
                map_width,
                map_height,
                settings.tile_width,
                settings.tile_height,
                false,
                Dither::Off,
            );
            for (i, crop) in crops.iter().enumerate() {
                let i = i as u32;
                map.add(crop, &tileset, &palette, settings.bpp, i % map_width, i / map_width)?;
            }

            if settings.tile_base_offset != 0 {
                map.add_base_offset(settings.tile_base_offset);
            }
            if settings.palette_base_offset != 0 {
                map.add_palette_base_offset(settings.palette_base_offset);
            }

            let desc = map.description(split_size, split_size, false);
            logger.verbose(format!("Map laid out in {desc}"));

            let data = map.to_native_data(0, 0, false);
            std::fs::write(path, data).map_err(|e| e.to_string())?;

            logger.verbose(format!("Saved native map data to '{}'", path.display()));
            if settings.tile_base_offset != 0 {
                logger.verbose(format!("  Tile base offset: {}", settings.tile_base_offset));
            }
            if settings.palette_base_offset != 0 {
                logger.verbose(format!("  Palette base offset: {}", settings.palette_base_offset));
            }
        } else {
            Logger::error(format!("Map output not supported for mode '{}'", settings.mode));
        }
    }

    Ok(())
}
