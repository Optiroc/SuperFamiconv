//! Bin packing algorithms for palette optimization.

mod first_fit_decreasing;
mod overload_and_remove;
mod prng;

use std::collections::BTreeSet;

use clap::ValueEnum;

use crate::color::ReducedColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum Effort {
    Low,
    Medium,
    High,
}

/// Optimizes palettes into as few subpalettes as possible.
///
/// Tries several algorithms and picks the best result.
/// - `requirements` should be deduplicated and free of subsets.
pub fn pack(
    requirements: &[BTreeSet<ReducedColor>],
    capacity: usize,
    effort: Effort,
) -> Vec<BTreeSet<ReducedColor>> {
    // Run optimizers, keeping only the best-scoring result
    let mut result = first_fit_decreasing::pack(requirements, capacity);

    let mut consider = |candidate: Vec<BTreeSet<ReducedColor>>| {
        if score(&candidate) < score(&result) {
            result = candidate;
        }
    };

    consider(overload_and_remove::pack(requirements, capacity, None));

    let seeds = match effort {
        Effort::Low => 0,
        Effort::Medium => 16,
        Effort::High => 64,
    };
    for seed in 0..seeds {
        consider(overload_and_remove::pack(requirements, capacity, Some(seed)));
    }

    result
}

fn score(bins: &[BTreeSet<ReducedColor>]) -> (usize, usize) {
    let subpalettes = bins.len();
    let colors = bins.iter().map(BTreeSet::len).sum();
    (subpalettes, colors)
}
