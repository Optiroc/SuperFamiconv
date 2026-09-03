//! Basic logger.

use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Verbosity {
    #[default]
    Normal,
    Verbose,
    VeryVerbose,
}

impl From<u8> for Verbosity {
    fn from(count: u8) -> Self {
        match count {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::VeryVerbose,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Logger {
    verbosity: Verbosity,
}

impl Logger {
    pub fn new(verbosity: Verbosity) -> Self {
        Self { verbosity }
    }

    /// Logs a message at `-v` and above.
    pub fn verbose(
        self,
        message: impl Display,
    ) {
        if self.verbosity >= Verbosity::Verbose {
            println!("{message}");
        }
    }

    /// Logs a message at `-vv` and above.
    pub fn very_verbose(
        self,
        message: impl Display,
    ) {
        if self.verbosity >= Verbosity::VeryVerbose {
            println!("{message}");
        }
    }

    /// Logs a message to stderr, regardless of verbosity.
    pub fn error(message: impl Display) {
        eprintln!("{message}");
    }
}
