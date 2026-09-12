//! SuperFamiconv command line interface.

use std::path::PathBuf;

use clap::builder::styling::{self, Style};
use clap::{Args, Parser, Subcommand};
use superfamiconv::color::{self, NormalizedColor};
use superfamiconv::dither::Dither;
use superfamiconv::mode::Mode;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(Style::new().bold())
    .usage(Style::new().bold())
    .literal(Style::new());

#[derive(Parser, Debug)]
#[command(
    about,
    version,
    long_about = None,
    args_override_self = false,
    disable_help_flag = true,
    disable_version_flag = true,
    hide_possible_values = true,
    styles = STYLES,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Print help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help_heading = "Info")]
    help: Option<bool>,
    /// Print version
    #[arg(short = 'V', long = "version", action = clap::ArgAction::Version, help_heading = "Info")]
    version: Option<bool>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Convert an image to palette, tile and/or map data.
    #[command(alias = "c")]
    Convert(ConvertArgs),
    /// Convert an image to palette data.
    #[command(alias = "p")]
    Palette(PaletteArgs),
    /// Convert an image and palette (or native tile data) to tile data.
    #[command(alias = "t")]
    Tiles(TilesArgs),
    /// Convert an image, palette and tileset to map data.
    #[command(alias = "m")]
    Map(MapArgs),
}

/// Arguments for `superfamiconv convert`.
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Source image(s)
    #[arg(short = 'i', long, value_name = "FILE", help_heading = "Input files")]
    pub in_image: Vec<PathBuf>,
    /// Priority attribute map image
    #[arg(short = 'a', long, value_name = "FILE", help_heading = "Input files")]
    pub in_attribute_map: Option<PathBuf>,

    /// Native palette data
    #[arg(short = 'p', long, value_name = "FILE", help_heading = "Output files")]
    pub out_palette: Option<PathBuf>,
    /// Native tile data
    #[arg(short = 't', long, value_name = "FILE", help_heading = "Output files")]
    pub out_tiles: Option<PathBuf>,
    /// Native map data
    #[arg(short = 'm', long, value_name = "FILE", help_heading = "Output files")]
    pub out_map: Option<PathBuf>,
    /// Palette map
    #[arg(long, visible_alias = "pm", value_name = "FILE", help_heading = "Output files")]
    pub out_palette_map: Option<PathBuf>,
    /// Tile map
    #[arg(long, visible_alias = "tm", value_name = "FILE", help_heading = "Output files")]
    pub out_tile_map: Option<PathBuf>,
    /// Attribute map
    #[arg(long, visible_alias = "am", value_name = "FILE", help_heading = "Output files")]
    pub out_attribute_map: Option<PathBuf>,
    /// Interleaved map/tile data [snes_mode7]
    #[arg(long, visible_alias = "m7", value_name = "FILE", help_heading = "Output files")]
    pub out_mode7_data: Option<PathBuf>,
    /// Palette image
    #[arg(long, visible_alias = "pi", value_name = "FILE", help_heading = "Output files")]
    pub out_palette_image: Option<PathBuf>,
    /// Tile image
    #[arg(long, visible_alias = "ti", value_name = "FILE", help_heading = "Output files")]
    pub out_tile_image: Option<PathBuf>,
    /// Preview image
    #[arg(long, visible_alias = "img", value_name = "FILE", help_heading = "Output files")]
    pub out_preview_image: Option<PathBuf>,
    /// Adobe color table
    #[arg(long, visible_alias = "act", value_name = "FILE", help_heading = "Output files")]
    pub out_palette_act: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Options")]
    pub mode: Mode,
    /// Bits per pixel
    #[arg(short = 'B', long, help_heading = "Options")]
    pub bpp: Option<u32>,
    /// Number of subpalettes
    #[arg(short = 'N', long, value_name = "N", help_heading = "Options")]
    pub palettes: Option<u32>,
    /// Colors per subpalette
    #[arg(short = 'C', long, value_name = "N", help_heading = "Options")]
    pub colors: Option<u32>,
    /// Tile width
    #[arg(short = 'W', long, value_name = "W", help_heading = "Options")]
    pub tile_width: Option<u32>,
    /// Tile height
    #[arg(short = 'H', long, value_name = "H", help_heading = "Options")]
    pub tile_height: Option<u32>,
    /// Do not remap colors
    #[arg(short = 'R', long, help_heading = "Options")]
    pub no_remap: bool,
    /// Do not deduplicate identical tiles
    #[arg(short = 'D', long, help_heading = "Options")]
    pub no_discard: bool,
    /// Do not deduplicate via tile flipping
    #[arg(short = 'F', long, help_heading = "Options")]
    pub no_flip: bool,
    /// Maximum number of tiles
    #[arg(short = 'T', long, value_name = "N", help_heading = "Options")]
    pub max_tiles: Option<u32>,
    /// Apply sprite output settings
    #[arg(short = 'S', long, help_heading = "Options")]
    pub sprite_mode: bool,
    /// Set color zero
    #[arg(short = 'Z', long, value_name = "COLOR", value_parser = color::from_hexstring, help_heading = "Options")]
    pub color_zero: Option<NormalizedColor>,
    /// Quantize colors and tiles to fit target palette
    #[arg(short = 'Q', long, help_heading = "Options")]
    pub quantize: bool,
    /// Dithering to apply if quantizing
    #[arg(long, value_enum, default_value_t = Dither::Bayer4x4, help_heading = "Options")]
    pub dither: Dither,
    /// Round colors instead of truncating
    #[arg(long, help_heading = "Options")]
    pub round: bool,
    /// Tile base offset for map data
    #[arg(long, default_value_t = 0, value_name = "N", help_heading = "Options")]
    pub tile_base_offset: i32,
    /// Palette base offset for map data
    #[arg(long, default_value_t = 0, value_name = "N", help_heading = "Options")]
    pub palette_base_offset: i32,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help_heading = "Info")]
    pub verbose: u8,
    /// Print help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help_heading = "Info")]
    help: Option<bool>,
}

/// Arguments for `superfamiconv palette`.
#[derive(Args, Debug)]
#[command(args_override_self = true)]
pub struct PaletteArgs {
    /// Source image(s)
    #[arg(short = 'i', long, value_name = "FILE", help_heading = "Input files")]
    pub in_image: Vec<PathBuf>,

    /// Native palette data
    #[arg(short = 'd', long, value_name = "FILE", help_heading = "Output files")]
    pub out_data: Option<PathBuf>,
    /// Palette image
    #[arg(short = 'o', long, value_name = "FILE", help_heading = "Output files")]
    pub out_image: Option<PathBuf>,
    /// Palette json
    #[arg(short = 'j', long, value_name = "FILE", help_heading = "Output files")]
    pub out_json: Option<PathBuf>,
    /// Adobe color table
    #[arg(long, visible_alias = "act", value_name = "FILE", help_heading = "Output files")]
    pub out_act: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Options")]
    pub mode: Mode,
    /// Number of subpalettes
    #[arg(short = 'N', long, value_name = "N", help_heading = "Options")]
    pub palettes: Option<u32>,
    /// Colors per subpalette
    #[arg(short = 'C', long, value_name = "N", help_heading = "Options")]
    pub colors: Option<u32>,
    /// Tile width
    #[arg(short = 'W', long, value_name = "W", help_heading = "Options")]
    pub tile_width: Option<u32>,
    /// Tile height
    #[arg(short = 'H', long, value_name = "H", help_heading = "Options")]
    pub tile_height: Option<u32>,
    /// Do not remap colors
    #[arg(short = 'R', long, help_heading = "Options")]
    pub no_remap: bool,
    /// Set color zero
    #[arg(short = 'Z', long, value_name = "COLOR", value_parser = color::from_hexstring, help_heading = "Options")]
    pub color_zero: Option<NormalizedColor>,
    /// Quantize colors to fit target palette
    #[arg(short = 'Q', long, help_heading = "Options")]
    pub quantize: bool,
    /// Round colors instead of truncating
    #[arg(long, help_heading = "Options")]
    pub round: bool,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help_heading = "Info")]
    pub verbose: u8,
    /// Print help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help_heading = "Info")]
    help: Option<bool>,
}

/// Arguments for `superfamiconv tiles`.
#[derive(Args, Debug)]
#[command(args_override_self = true)]
pub struct TilesArgs {
    /// Source image (multiple allowed)
    #[arg(short = 'i', long, value_name = "FILE", help_heading = "Input files")]
    pub in_image: Option<Vec<PathBuf>>,
    /// Native tile data
    #[arg(short = 'n', long, value_name = "FILE", help_heading = "Input files")]
    pub in_data: Option<PathBuf>,
    /// Palette (native or json)
    #[arg(short = 'p', long, value_name = "FILE", help_heading = "Input files")]
    pub in_palette: Option<PathBuf>,

    /// Native tile data
    #[arg(short = 'd', long, value_name = "FILE", help_heading = "Output files")]
    pub out_data: Option<PathBuf>,
    /// Tile image
    #[arg(short = 'o', long, value_name = "FILE", help_heading = "Output files")]
    pub out_image: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Options")]
    pub mode: Mode,
    /// Bits per pixel
    #[arg(short = 'B', long, help_heading = "Options")]
    pub bpp: Option<u32>,
    /// Tile width
    #[arg(short = 'W', long, value_name = "W", help_heading = "Options")]
    pub tile_width: Option<u32>,
    /// Tile height
    #[arg(short = 'H', long, value_name = "H", help_heading = "Options")]
    pub tile_height: Option<u32>,
    /// Do not remap colors
    #[arg(short = 'R', long, help_heading = "Options")]
    pub no_remap: bool,
    /// Do not deduplicate identical tiles
    #[arg(short = 'D', long, help_heading = "Options")]
    pub no_discard: bool,
    /// Do not deduplicate via tile flipping
    #[arg(short = 'F', long, help_heading = "Options")]
    pub no_flip: bool,
    /// Maximum number of tiles
    #[arg(short = 'T', long, value_name = "N", help_heading = "Options")]
    pub max_tiles: Option<u32>,
    /// Apply sprite output settings
    #[arg(short = 'S', long, help_heading = "Options")]
    pub sprite_mode: bool,
    /// Quantize (match tiles to the closest subpalette)
    #[arg(short = 'Q', long, help_heading = "Options")]
    pub quantize: bool,
    /// Dithering to apply if quantizing
    #[arg(long, value_enum, default_value_t = Dither::Bayer4x4, help_heading = "Options")]
    pub dither: Dither,
    /// Round colors instead of truncating
    #[arg(long, help_heading = "Options")]
    pub round: bool,
    /// Width of output tile image
    #[arg(long, value_name = "W", help_heading = "Options")]
    pub out_image_width: Option<u32>,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help_heading = "Info")]
    pub verbose: u8,
    /// Print help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help_heading = "Info")]
    help: Option<bool>,
}

/// Arguments for `superfamiconv map`.
#[derive(Args, Debug)]
#[command(args_override_self = true)]
pub struct MapArgs {
    /// Source image(s)
    #[arg(short = 'i', long, value_name = "FILE", help_heading = "Input files")]
    pub in_image: Option<PathBuf>,
    /// Native map data
    #[arg(short = 'n', long, value_name = "FILE", help_heading = "Input files")]
    pub in_data: Option<PathBuf>,
    /// Palette (native or json)
    #[arg(short = 'p', long, value_name = "FILE", help_heading = "Input files")]
    pub in_palette: Option<PathBuf>,
    /// Native tile data
    #[arg(short = 't', long, value_name = "FILE", help_heading = "Input files")]
    pub in_tiles: Option<PathBuf>,
    /// Priority attribute map image
    #[arg(short = 'a', long, value_name = "FILE", help_heading = "Input files")]
    pub in_attribute_map: Option<PathBuf>,

    /// Native map data
    #[arg(short = 'd', long, value_name = "FILE", help_heading = "Output files")]
    pub out_data: Option<PathBuf>,
    /// JSON map data
    #[arg(short = 'j', long, value_name = "FILE", help_heading = "Output files")]
    pub out_json: Option<PathBuf>,
    /// Map image
    #[arg(short = 'o', long, value_name = "FILE", help_heading = "Output files")]
    pub out_image: Option<PathBuf>,
    /// Palette map
    #[arg(long, visible_alias = "pm", value_name = "FILE", help_heading = "Output files")]
    pub out_palette_map: Option<PathBuf>,
    /// Tile map
    #[arg(long, visible_alias = "tm", value_name = "FILE", help_heading = "Output files")]
    pub out_tile_map: Option<PathBuf>,
    /// Attribute map
    #[arg(long, visible_alias = "am", value_name = "FILE", help_heading = "Output files")]
    pub out_attribute_map: Option<PathBuf>,
    /// Interleaved native map/tile data [snes_mode7]
    #[arg(long, visible_alias = "m7", value_name = "FILE", help_heading = "Output files")]
    pub out_mode7_data: Option<PathBuf>,

    /// Mode
    #[arg(short = 'M', long, value_enum, default_value_t = Mode::Snes, help_heading = "Options")]
    pub mode: Mode,
    /// Bits per pixel
    #[arg(short = 'B', long, help_heading = "Options")]
    pub bpp: Option<u32>,
    /// Tile width
    #[arg(short = 'W', long, value_name = "W", help_heading = "Options")]
    pub tile_width: Option<u32>,
    /// Tile height
    #[arg(short = 'H', long, value_name = "H", help_heading = "Options")]
    pub tile_height: Option<u32>,
    /// Do not allow tile flipping
    #[arg(short = 'F', long, help_heading = "Options")]
    pub no_flip: bool,
    /// Quantize (match tiles to the closest subpalette)
    #[arg(short = 'Q', long, help_heading = "Options")]
    pub quantize: bool,
    /// Dithering to apply if quantizing
    #[arg(long, value_enum, default_value_t = Dither::Bayer4x4, help_heading = "Options")]
    pub dither: Dither,
    /// Round colors instead of truncating
    #[arg(long, help_heading = "Options")]
    pub round: bool,
    /// Map width (in tiles)
    #[arg(long, visible_alias = "mw", value_name = "W", value_parser = clap::value_parser!(u32).range(1..), help_heading = "Options")]
    pub map_width: Option<u32>,
    /// Map height (in tiles)
    #[arg(long, visible_alias = "mh", value_name = "H", value_parser = clap::value_parser!(u32).range(1..), help_heading = "Options")]
    pub map_height: Option<u32>,
    /// Split output into columns of <tiles> width
    #[arg(long, visible_alias = "sw", value_name = "W", value_parser = clap::value_parser!(u32).range(1..), help_heading = "Options")]
    pub split_width: Option<u32>,
    /// Split output into rows of <tiles> height
    #[arg(long, visible_alias = "sh", value_name = "H", value_parser = clap::value_parser!(u32).range(1..), help_heading = "Options")]
    pub split_height: Option<u32>,
    /// Tile base offset for map data
    #[arg(long, value_name = "N", default_value_t = 0, help_heading = "Options")]
    pub tile_base_offset: i32,
    /// Palette base offset for map data
    #[arg(long, value_name = "N", default_value_t = 0, help_heading = "Options")]
    pub palette_base_offset: i32,
    /// Output data in column-major order
    #[arg(long, help_heading = "Options")]
    pub column_order: bool,

    /// Verbose logging (-vv for extra verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help_heading = "Info")]
    pub verbose: u8,
    /// Print help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help_heading = "Info")]
    help: Option<bool>,
}
