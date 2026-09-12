//! Resolve CLI arguments into library settings.

use superfamiconv::logger::{Logger, Verbosity};
use superfamiconv::mode::color::ColorRounding;
use superfamiconv::operation;
use superfamiconv::operation::convert::ConvertSettings;
use superfamiconv::operation::map::MapSettings;
use superfamiconv::operation::palette::PaletteSettings;
use superfamiconv::operation::tiles::TilesSettings;
use superfamiconv::palette::palette_size_at_bpp;

use crate::cli::{ConvertArgs, MapArgs, PaletteArgs, TilesArgs};

pub fn resolve_convert(args: ConvertArgs) -> Result<ConvertSettings, String> {
    let (mode, sprite_mode) = operation::resolve_sprite_mode(args.mode, args.sprite_mode);
    let bpp = args.bpp.unwrap_or_else(|| mode.default_bpp());
    let palettes = args.palettes.unwrap_or_else(|| mode.default_palette_count());
    let colors = args.colors.unwrap_or_else(|| palette_size_at_bpp(bpp));
    let tile_width = args.tile_width.unwrap_or_else(|| mode.default_tile_size());
    let tile_height = args.tile_height.unwrap_or_else(|| mode.default_tile_size());
    let mut no_flip = operation::resolve_no_flip(args.no_flip, mode);
    let mut no_discard = args.no_discard;
    let max_tiles = args.max_tiles.unwrap_or_else(|| mode.max_tile_count());

    if !mode.tile_width_is_allowed(tile_width) {
        return Err(format!("tile-width={tile_width} is not allowed for mode '{mode}'"));
    }
    if !mode.tile_height_is_allowed(tile_height) {
        return Err(format!("tile-height={tile_height} is not allowed for mode '{mode}'"));
    }
    if sprite_mode {
        no_discard = true;
        no_flip = true;
    }

    Ok(ConvertSettings {
        in_image: args.in_image,
        in_attribute_map: args.in_attribute_map,
        out_palette: args.out_palette,
        out_tiles: args.out_tiles,
        out_map: args.out_map,
        out_palette_map: args.out_palette_map,
        out_tile_map: args.out_tile_map,
        out_attribute_map: args.out_attribute_map,
        out_mode7_data: args.out_mode7_data,
        out_palette_image: args.out_palette_image,
        out_tile_image: args.out_tile_image,
        out_preview_image: args.out_preview_image,
        out_palette_act: args.out_palette_act,
        mode,
        bpp,
        palettes,
        colors,
        tile_width,
        tile_height,
        no_remap: args.no_remap,
        no_discard,
        no_flip,
        max_tiles,
        color_zero: args.color_zero,
        quantize: args.quantize,
        dither: args.dither,
        rounding: ColorRounding::from(args.round),
        tile_base_offset: args.tile_base_offset,
        palette_base_offset: args.palette_base_offset,
        logger: Logger::new(Verbosity::from(args.verbose)),
    })
}

pub fn resolve_palette(args: PaletteArgs) -> Result<PaletteSettings, String> {
    let mode = args.mode;
    let palettes = args.palettes.unwrap_or_else(|| mode.default_palette_count());
    let colors = args.colors.unwrap_or_else(|| palette_size_at_bpp(mode.default_bpp()));
    let tile_width = args.tile_width.unwrap_or_else(|| mode.default_tile_size());
    let tile_height = args.tile_height.unwrap_or_else(|| mode.default_tile_size());

    if !mode.tile_width_is_allowed(tile_width) {
        return Err(format!("tile-width={tile_width} is not allowed for mode '{mode}'"));
    }
    if !mode.tile_height_is_allowed(tile_height) {
        return Err(format!("tile-height={tile_height} is not allowed for mode '{mode}'"));
    }

    Ok(PaletteSettings {
        in_image: args.in_image,
        out_data: args.out_data,
        out_image: args.out_image,
        out_json: args.out_json,
        out_act: args.out_act,
        mode,
        palettes,
        colors,
        tile_width,
        tile_height,
        no_remap: args.no_remap,
        color_zero: args.color_zero,
        quantize: args.quantize,
        rounding: ColorRounding::from(args.round),
        logger: Logger::new(Verbosity::from(args.verbose)),
    })
}

pub fn resolve_tiles(args: TilesArgs) -> Result<TilesSettings, String> {
    if args.in_image.is_none() && args.in_data.is_none() {
        return Err("Input image or native data required".into());
    }

    let (mode, sprite_mode) = operation::resolve_sprite_mode(args.mode, args.sprite_mode);
    let bpp = args.bpp.unwrap_or_else(|| mode.default_bpp());
    let tile_width = args.tile_width.unwrap_or_else(|| mode.default_tile_size());
    let tile_height = args.tile_height.unwrap_or_else(|| mode.default_tile_size());
    let mut no_flip = operation::resolve_no_flip(args.no_flip, mode);
    let mut no_discard = args.no_discard;
    let max_tiles = args.max_tiles.unwrap_or_else(|| mode.max_tile_count());

    if !mode.tile_width_is_allowed(tile_width) {
        return Err(format!("tile-width={tile_width} is not allowed for mode '{mode}'"));
    }
    if !mode.tile_height_is_allowed(tile_height) {
        return Err(format!("tile-height={tile_height} is not allowed for mode '{mode}'"));
    }
    if !mode.bpp_is_allowed(bpp) {
        return Err(format!("bpp={bpp} is not allowed for mode '{mode}'"));
    }
    if sprite_mode {
        no_discard = true;
        no_flip = true;
    }

    Ok(TilesSettings {
        in_image: args.in_image,
        in_data: args.in_data,
        in_palette: args.in_palette,
        out_data: args.out_data,
        out_image: args.out_image,
        mode,
        bpp,
        tile_width,
        tile_height,
        no_remap: args.no_remap,
        no_discard,
        no_flip,
        quantize: args.quantize,
        dither: args.dither,
        rounding: ColorRounding::from(args.round),
        max_tiles,
        out_image_width: args.out_image_width,
        logger: Logger::new(Verbosity::from(args.verbose)),
    })
}

pub fn resolve_map(args: MapArgs) -> Result<MapSettings, String> {
    let mode = args.mode;

    let default_size = mode
        .default_map_size()
        .ok_or_else(|| format!("Map output is not available for mode '{mode}'"))?;

    let bpp = args.bpp.unwrap_or_else(|| mode.default_bpp());
    if !mode.bpp_is_allowed(bpp) {
        return Err(format!("bpp={bpp} is not allowed for mode '{mode}'"));
    }

    if args.in_image.is_none() && args.in_data.is_none() {
        return Err("Input image or native data required".into());
    }

    if args.in_data.is_some() && (args.map_width.is_none() || args.map_height.is_none()) {
        return Err("map-width and map-height are required when reading native map data".into());
    }

    let in_palette = args.in_palette.ok_or("Input palette required")?;
    let in_tiles = args.in_tiles.ok_or("Input tileset required")?;

    let tile_width = args.tile_width.unwrap_or_else(|| mode.default_tile_size());
    let tile_height = args.tile_height.unwrap_or_else(|| mode.default_tile_size());
    if !mode.tile_width_is_allowed(tile_width) {
        return Err(format!("tile-width={tile_width} is not allowed for mode '{mode}'"));
    }
    if !mode.tile_height_is_allowed(tile_height) {
        return Err(format!("tile-height={tile_height} is not allowed for mode '{mode}'"));
    }

    let split_width = args.split_width.unwrap_or(default_size);
    let split_height = args.split_height.unwrap_or(default_size);

    Ok(MapSettings {
        in_image: args.in_image,
        in_data: args.in_data,
        in_palette,
        in_tiles,
        in_attribute_map: args.in_attribute_map,
        out_data: args.out_data,
        out_json: args.out_json,
        out_image: args.out_image,
        out_palette_map: args.out_palette_map,
        out_tile_map: args.out_tile_map,
        out_attribute_map: args.out_attribute_map,
        out_mode7_data: args.out_mode7_data,
        mode,
        bpp,
        tile_width,
        tile_height,
        no_flip: operation::resolve_no_flip(args.no_flip, mode),
        quantize: args.quantize,
        dither: args.dither,
        rounding: ColorRounding::from(args.round),
        map_width: args.map_width,
        map_height: args.map_height,
        split_width,
        split_height,
        tile_base_offset: args.tile_base_offset,
        palette_base_offset: args.palette_base_offset,
        column_order: args.column_order,
        logger: Logger::new(Verbosity::from(args.verbose)),
    })
}
