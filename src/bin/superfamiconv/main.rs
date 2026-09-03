//! SuperFamiconv command line tool.

mod cli;
mod resolve_args;

use clap::{CommandFactory, Parser};
use superfamiconv::logger::Logger;
use superfamiconv::operation::{convert, map, palette, tiles};

use cli::{Cli, Command};
use resolve_args::*;

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if raw_args.is_empty() {
        Cli::command().print_help().ok();
        return;
    }

    if raw_args.len() == 1 {
        let mut top = Cli::command();
        top.build();
        if let Some(sub) = top.find_subcommand_mut(&raw_args[0]) {
            sub.print_help().ok();
            println!();
            return;
        }
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Command::Convert(args) => resolve_convert(args).and_then(convert::execute),
        Command::Palette(args) => resolve_palette(args).and_then(palette::execute),
        Command::Tiles(args) => resolve_tiles(args).and_then(tiles::execute),
        Command::Map(args) => resolve_map(args).and_then(map::execute),
    };

    if let Err(err) = result {
        Logger::error(format!("Error: {err}"));
        std::process::exit(1);
    }
}
