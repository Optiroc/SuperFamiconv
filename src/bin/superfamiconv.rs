//! SuperFamiconv command line tool.

use clap::{CommandFactory, Parser};
use superfamiconv::cli::{self, Cli};
use superfamiconv::logger::Logger;

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

    if let Err(err) = cli::execute(Cli::parse()) {
        Logger::error(format!("Error: {err}"));
        std::process::exit(1);
    }
}
