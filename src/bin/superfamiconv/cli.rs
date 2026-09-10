//! SuperFamiconv command line interface.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use superfamiconv::color::{self, NormalizedColor};
use superfamiconv::dither::Dither;
use superfamiconv::mode::Mode;

#[derive(Parser, Debug)]
#[command(name = "superfamiconv", version, about, long_about = None, args_override_self = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Convert an image to palette, tile and/or map data.
    Convert(ConvertArgs),
    /// Convert an image to palette data.
    Palette(PaletteArgs),
    /// Convert an image and palette (or native tile data) to tile data.
    Tiles(TilesArgs),
    /// Convert an image, palette and tileset to map data.
    Map(MapArgs),
}

/// Arguments for `superfamiconv convert`.
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input: image(s)
    #[arg(short = 'i', long)]
    pub in_image: Vec<PathBuf>,
    /// Input: priority attribute map image
    #[arg(short = 'a', long)]
    pub in_attribute_map: Option<PathBuf>,

    /// Output: palette data
    #[arg(short = 'p', long)]
    pub out_palette: Option<PathBuf>,
    /// Output: tile data
    #[arg(short = 't', long)]
    pub out_tiles: Option<PathBuf>,
    /// Output: map data
    #[arg(short = 'm', long)]
    pub out_map: Option<PathBuf>,
    /// Output: palette image
    #[arg(long)]
    pub out_palette_image: Option<PathBuf>,
    /// Output: photoshop palette
    #[arg(long)]
    pub out_palette_act: Option<PathBuf>,
    /// Output: tiles image
    #[arg(long)]
    pub out_tiles_image: Option<PathBuf>,
    /// Output: preview image
    #[arg(long)]
    pub out_preview_image: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Settings")]
    pub mode: Mode,
    /// Bits per pixel [default: mode-dependent]
    #[arg(short = 'B', long, help_heading = "Settings")]
    pub bpp: Option<u32>,
    /// Number of subpalettes [default: mode-dependent]
    #[arg(short = 'N', long, help_heading = "Settings")]
    pub palettes: Option<u32>,
    /// Colors per subpalette [default: mode-dependent]
    #[arg(short = 'C', long, help_heading = "Settings")]
    pub colors: Option<u32>,
    /// Tile width [default: mode-dependent]
    #[arg(short = 'W', long, help_heading = "Settings")]
    pub tile_width: Option<u32>,
    /// Tile height [default: mode-dependent]
    #[arg(short = 'H', long, help_heading = "Settings")]
    pub tile_height: Option<u32>,
    /// Don't remap colors
    #[arg(short = 'R', long, help_heading = "Settings")]
    pub no_remap: bool,
    /// Don't deduplicate redundant tiles
    #[arg(short = 'D', long, help_heading = "Settings")]
    pub no_discard: bool,
    /// Don't deduplicate using tile flipping
    #[arg(short = 'F', long, help_heading = "Settings")]
    pub no_flip: bool,
    /// Maximum number of tiles [default: mode-dependent]
    #[arg(short = 'T', long, help_heading = "Settings")]
    pub max_tiles: Option<u32>,
    /// Apply sprite output settings
    #[arg(short = 'S', long, help_heading = "Settings")]
    pub sprite_mode: bool,
    /// Set color #0 (6 or 8 character hex string)
    #[arg(short = 'Z', long, value_parser = color::from_hexstring, help_heading = "Settings")]
    pub color_zero: Option<NormalizedColor>,
    /// Quantize colors and tiles to fit target palette settings
    #[arg(short = 'Q', long, help_heading = "Settings")]
    pub quantize: bool,
    /// Dithering to apply if quantizing
    #[arg(long, value_enum, default_value_t = Dither::Bayer4x4, help_heading = "Settings")]
    pub dither: Dither,
    /// Round instead of truncate when reducing colors to mode-native precision
    #[arg(long, help_heading = "Settings")]
    pub round: bool,
    /// Tile base offset for map data
    #[arg(long, default_value_t = 0, help_heading = "Settings")]
    pub tile_base_offset: i32,
    /// Palette base offset for map data
    #[arg(long, default_value_t = 0, help_heading = "Settings")]
    pub palette_base_offset: i32,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// Arguments for `superfamiconv palette`.
#[derive(Args, Debug)]
#[command(args_override_self = true)]
pub struct PaletteArgs {
    /// Input: image(s)
    #[arg(short = 'i', long)]
    pub in_image: Vec<PathBuf>,

    /// Output: native data
    #[arg(short = 'd', long)]
    pub out_data: Option<PathBuf>,
    /// Output: adobe color table
    #[arg(short = 'a', long)]
    pub out_act: Option<PathBuf>,
    /// Output: json
    #[arg(short = 'j', long)]
    pub out_json: Option<PathBuf>,
    /// Output: image
    #[arg(short = 'o', long)]
    pub out_image: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Settings")]
    pub mode: Mode,
    /// Number of subpalettes [default: mode-dependent]
    #[arg(short = 'N', long, help_heading = "Settings")]
    pub palettes: Option<u32>,
    /// Colors per subpalette [default: mode-dependent]
    #[arg(short = 'C', long, help_heading = "Settings")]
    pub colors: Option<u32>,
    /// Tile width [default: mode-dependent]
    #[arg(short = 'W', long, help_heading = "Settings")]
    pub tile_width: Option<u32>,
    /// Tile height [default: mode-dependent]
    #[arg(short = 'H', long, help_heading = "Settings")]
    pub tile_height: Option<u32>,
    /// Don't remap colors
    #[arg(short = 'R', long, help_heading = "Settings")]
    pub no_remap: bool,
    /// Apply sprite output settings
    #[arg(short = 'S', long, help_heading = "Settings")]
    pub sprite_mode: bool,
    /// Set color #0 (6 or 8 character hex string)
    #[arg(short = 'Z', long, value_parser = color::from_hexstring, help_heading = "Settings")]
    pub color_zero: Option<NormalizedColor>,
    /// Quantize colors to fit target palette settings
    #[arg(short = 'Q', long, help_heading = "Settings")]
    pub quantize: bool,
    /// Round instead of truncate when reducing colors to mode-native precision
    #[arg(long, help_heading = "Settings")]
    pub round: bool,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// Arguments for `superfamiconv tiles`.
#[derive(Args, Debug)]
#[command(args_override_self = true)]
pub struct TilesArgs {
    /// Input: image(s)
    #[arg(short = 'i', long)]
    pub in_image: Option<Vec<PathBuf>>,
    /// Input: native data
    #[arg(short = 'n', long)]
    pub in_data: Option<PathBuf>,
    /// Input: palette (native/json)
    #[arg(short = 'p', long)]
    pub in_palette: Option<PathBuf>,

    /// Output: native data
    #[arg(short = 'd', long)]
    pub out_data: Option<PathBuf>,
    /// Output: image
    #[arg(short = 'o', long)]
    pub out_image: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Settings")]
    pub mode: Mode,
    /// Bits per pixel [default: mode-dependent]
    #[arg(short = 'B', long, help_heading = "Settings")]
    pub bpp: Option<u32>,
    /// Tile width [default: mode-dependent]
    #[arg(short = 'W', long, help_heading = "Settings")]
    pub tile_width: Option<u32>,
    /// Tile height [default: mode-dependent]
    #[arg(short = 'H', long, help_heading = "Settings")]
    pub tile_height: Option<u32>,
    /// Don't remap colors
    #[arg(short = 'R', long, help_heading = "Settings")]
    pub no_remap: bool,
    /// Don't deduplicate redundant tiles
    #[arg(short = 'D', long, help_heading = "Settings")]
    pub no_discard: bool,
    /// Don't deduplicate using tile flipping
    #[arg(short = 'F', long, help_heading = "Settings")]
    pub no_flip: bool,
    /// Maximum number of tiles [default: mode-dependent]
    #[arg(short = 'T', long, help_heading = "Settings")]
    pub max_tiles: Option<u32>,
    /// Apply sprite output settings
    #[arg(short = 'S', long, help_heading = "Settings")]
    pub sprite_mode: bool,
    /// Quantize (match tiles to the closest subpalette and color)
    #[arg(short = 'Q', long, help_heading = "Settings")]
    pub quantize: bool,
    /// Dithering to apply if quantizing
    #[arg(long, value_enum, default_value_t = Dither::Bayer4x4, help_heading = "Settings")]
    pub dither: Dither,
    /// Round instead of truncate when reducing colors to mode-native precision
    #[arg(long, help_heading = "Settings")]
    pub round: bool,
    /// Width of output tileset image
    #[arg(long, help_heading = "Settings")]
    pub out_image_width: Option<u32>,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// Arguments for `superfamiconv map`.
#[derive(Args, Debug)]
#[command(args_override_self = true)]
pub struct MapArgs {
    /// Input: image
    #[arg(short = 'i', long)]
    pub in_image: Option<PathBuf>,
    /// Input: native data
    #[arg(short = 'n', long)]
    pub in_data: Option<PathBuf>,
    /// Input: palette (json/native)
    #[arg(short = 'p', long)]
    pub in_palette: Option<PathBuf>,
    /// Input: tiles (native)
    #[arg(short = 't', long)]
    pub in_tiles: Option<PathBuf>,
    /// Input: priority attribute map image
    #[arg(short = 'a', long)]
    pub in_attribute_map: Option<PathBuf>,

    /// Output: native data
    #[arg(short = 'd', long)]
    pub out_data: Option<PathBuf>,
    /// Output: json
    #[arg(short = 'j', long)]
    pub out_json: Option<PathBuf>,
    /// Output: interleaved map/tile data (snes_mode7)
    #[arg(short = '7', long)]
    pub out_m7_data: Option<PathBuf>,
    /// Output: banked map data (gbc)
    #[arg(long)]
    pub out_gbc_bank: Option<PathBuf>,
    /// Output: palette map (native 16-bit LE)
    #[arg(long)]
    pub out_pal_map: Option<PathBuf>,
    /// Output: image
    #[arg(short = 'o', long)]
    pub out_image: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Settings")]
    pub mode: Mode,
    /// Bits per pixel [default: mode-dependent]
    #[arg(short = 'B', long, help_heading = "Settings")]
    pub bpp: Option<u32>,
    /// Tile width [default: mode-dependent]
    #[arg(short = 'W', long, help_heading = "Settings")]
    pub tile_width: Option<u32>,
    /// Tile height [default: mode-dependent]
    #[arg(short = 'H', long, help_heading = "Settings")]
    pub tile_height: Option<u32>,
    /// Don't use flipped tiles
    #[arg(short = 'F', long, help_heading = "Settings")]
    pub no_flip: bool,
    /// Quantize (match tiles to the closest subpalette and color)
    #[arg(short = 'Q', long, help_heading = "Settings")]
    pub quantize: bool,
    /// Dithering to apply if quantizing
    #[arg(long, value_enum, default_value_t = Dither::Bayer4x4, help_heading = "Settings")]
    pub dither: Dither,
    /// Round instead of truncate when reducing colors to mode-native precision
    #[arg(long, help_heading = "Settings")]
    pub round: bool,
    /// Map width (in tiles) [default: image width]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..), help_heading = "Settings")]
    pub map_width: Option<u32>,
    /// Map height (in tiles) [default: image height]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..), help_heading = "Settings")]
    pub map_height: Option<u32>,
    /// Split output into columns of <tiles> width [default: mode-dependent]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..), help_heading = "Settings")]
    pub split_width: Option<u32>,
    /// Split output into rows of <tiles> height [default: mode-dependent]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..), help_heading = "Settings")]
    pub split_height: Option<u32>,
    /// Output data in column-major order [default: row-major]
    #[arg(long, help_heading = "Settings")]
    pub column_order: bool,
    /// Tile base offset for map data
    #[arg(long, default_value_t = 0, help_heading = "Settings")]
    pub tile_base_offset: i32,
    /// Palette base offset for map data
    #[arg(long, default_value_t = 0, help_heading = "Settings")]
    pub palette_base_offset: i32,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
