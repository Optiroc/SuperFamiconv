//! Basic logger.
//!
//! Messages go to stdout and stderr, or into a buffer while `capture` runs on the same thread.

use std::cell::RefCell;
use std::fmt::Display;

thread_local! {
    static CAPTURE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Runs `f`, collecting what it logs on this thread instead of printing it.
/// - Returns `f`'s result and the messages, one per line.
/// - The library logs only from the calling thread, so nothing escapes.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, String) {
    let outer = CAPTURE.with(|c| c.replace(Some(String::new())));
    let result = f();
    let log = CAPTURE.with(|c| c.replace(outer)).unwrap_or_default();
    (result, log)
}

/// Appends `message` to the capture buffer, or returns false when none is active.
fn captured(message: &dyn Display) -> bool {
    CAPTURE.with(|c| match c.borrow_mut().as_mut() {
        Some(log) => {
            log.push_str(&format!("{message}\n"));
            true
        }
        None => false,
    })
}

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
        if self.verbosity >= Verbosity::Verbose && !captured(&message) {
            println!("{message}");
        }
    }

    /// Logs a message at `-vv` and above.
    pub fn very_verbose(
        self,
        message: impl Display,
    ) {
        if self.verbosity >= Verbosity::VeryVerbose && !captured(&message) {
            println!("{message}");
        }
    }

    /// Logs a message to stderr, regardless of verbosity.
    pub fn error(message: impl Display) {
        if !captured(&message) {
            eprintln!("{message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_collects_errors_and_verbose_lines() {
        let ((), log) = capture(|| {
            Logger::error("first");
            Logger::new(Verbosity::Verbose).verbose("second");
            Logger::new(Verbosity::Normal).verbose("hidden");
        });
        assert_eq!(log, "first\nsecond\n");
    }
}
