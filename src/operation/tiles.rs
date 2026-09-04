//! Tiles operation: settings and execution.

use std::path::PathBuf;

use crate::dither::Dither;
use crate::image::Image;
use crate::logger::Logger;
use crate::mode::Mode;
use crate::palette::{Palette, palette_size_at_bpp};
use crate::tileset::Tileset;

#[derive(Debug, PartialEq, Eq)]
pub struct TilesSettings {
    pub in_image: Option<PathBuf>,
    pub in_data: Option<PathBuf>,
    pub in_palette: Option<PathBuf>,
    pub out_data: Option<PathBuf>,
    pub out_image: Option<PathBuf>,

    pub mode: Mode,
    pub bpp: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub no_remap: bool,
    pub no_discard: bool,
    pub no_flip: bool,
    pub max_tiles: u32,
    pub sprite_mode: bool,
    pub quantize: bool,
    pub dither: Dither,
    pub out_image_width: Option<u32>,

    pub logger: Logger,
}

pub fn execute(settings: TilesSettings) -> Result<(), String> {
    let logger = settings.logger;
    logger.verbose(format!("Performing tiles operation (mode: {})", settings.mode));

    let tileset = if let Some(in_data) = &settings.in_data {
        let bytes =
            std::fs::read(in_data).map_err(|e| format!("File '{}' could not be opened: {e}", in_data.display()))?;
        let ts = Tileset::from_native_data(
            &bytes,
            settings.mode,
            settings.bpp,
            settings.tile_width,
            settings.tile_height,
            settings.no_flip,
        )?;
        logger.verbose(format!(
            "Loaded tiles from '{}' ({} tiles)",
            in_data.display(),
            ts.size()
        ));
        ts
    } else {
        let in_image = settings.in_image.as_ref().expect("in_image or in_data required");
        let image = super::load_image(in_image, settings.logger)?;

        if settings.mode == Mode::PceSprite && (image.width % 16 != 0 || image.height % 16 != 0) {
            return Err("Mode 'pce_sprite' requires image dimensions to be a multiple of 16".into());
        }

        let crops = image.crops(settings.tile_width, settings.tile_height, settings.mode);
        logger.very_verbose(format!(
            "Image sliced into {} {}x{}px tiles",
            crops.len(),
            settings.tile_width,
            settings.tile_height
        ));

        let mut tileset = Tileset::new(
            settings.mode,
            settings.bpp,
            settings.tile_width,
            settings.tile_height,
            settings.no_discard,
            settings.no_flip,
            settings.no_remap,
            settings.quantize,
            settings.dither,
            settings.max_tiles,
        );

        let palette = if settings.no_remap {
            if image.palette_size() == 0 {
                return Err("Indexed color image required for no-remap".into());
            }
            logger.verbose("Creating tile data straight from color indices");
            None
        } else {
            let in_palette = settings
                .in_palette
                .as_ref()
                .ok_or("Input palette required (except in --no-remap mode)")?;
            let colors_per_subpalette = palette_size_at_bpp(settings.bpp) as usize;
            let pal = Palette::load(in_palette, colors_per_subpalette, settings.mode)?;
            if pal.size() < 1 {
                return Err("Input palette size is zero".into());
            }
            logger.verbose(format!(
                "Remapping tile data from palette '{}' ({})",
                in_palette.display(),
                pal
            ));
            Some(pal)
        };

        for crop in &crops {
            tileset.add(crop, palette.as_ref())?;
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

        tileset
    };

    if let Some(path) = &settings.out_data {
        std::fs::write(path, tileset.to_native_data()?).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native tile data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_image {
        let preview = Image::from_tileset(&tileset, settings.out_image_width)?;
        if settings.in_data.is_some() {
            preview.save_indexed(path)?;
        } else {
            preview.save(path)?;
        }
        logger.verbose(format!("Saved tileset image to '{}'", path.display()));
    }

    Ok(())
}
