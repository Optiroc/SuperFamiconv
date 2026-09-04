//! Palette operation: settings and execution.

use std::path::PathBuf;

use crate::binpack::Effort;
use crate::color::NormalizedColor;
use crate::dither::Dither;
use crate::image::Image;
use crate::logger::Logger;
use crate::mode::Mode;

#[derive(Debug, PartialEq, Eq)]
pub struct PaletteSettings {
    pub in_image: PathBuf,
    pub out_data: Option<PathBuf>,
    pub out_act: Option<PathBuf>,
    pub out_json: Option<PathBuf>,
    pub out_image: Option<PathBuf>,

    pub mode: Mode,
    pub palettes: u32,
    pub colors: u32,
    pub effort: Effort,
    pub tile_width: u32,
    pub tile_height: u32,
    pub no_remap: bool,
    pub sprite_mode: bool,
    pub color_zero: Option<NormalizedColor>,
    pub quantize: bool,

    pub logger: Logger,
}

pub fn execute(settings: PaletteSettings) -> Result<(), String> {
    let logger = settings.logger;
    logger.verbose(format!("Performing palette operation (mode: {})", settings.mode));

    let image = super::load_image(&settings.in_image, settings.logger)?;
    let color_zero = super::resolve_color_zero(settings.mode, settings.color_zero, &image, settings.sprite_mode);

    let (palette, _image) = super::make_palette(
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
        Dither::Off,
        settings.logger,
    )?;

    if let Some(path) = &settings.out_data {
        std::fs::write(path, palette.native_data()?).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved native palette data to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_act {
        std::fs::write(path, palette.act_data()).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved ACT palette to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_image {
        Image::from_palette(&palette)?.save(path)?;
        logger.verbose(format!("Saved palette image to '{}'", path.display()));
    }
    if let Some(path) = &settings.out_json {
        std::fs::write(path, palette.to_json()).map_err(|e| e.to_string())?;
        logger.verbose(format!("Saved JSON data to '{}'", path.display()));
    }

    Ok(())
}
