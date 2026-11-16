//! SuperFamiconv library.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod binpack;
pub mod cli;
pub mod color;
pub mod dither;
pub mod image;
pub mod logger;
pub mod map;
pub mod mode;
pub mod operation;
pub mod palette;
pub mod prng;
pub mod quant;
pub mod tile;
pub mod tileset;
