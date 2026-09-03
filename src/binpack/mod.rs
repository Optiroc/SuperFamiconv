//! Bin packing algorithms for palette optimization.

mod first_fit_decreasing;
mod overload_and_remove;
mod prng;

use std::collections::BTreeSet;

use crate::color::ReducedColor;

const MAX_ITERATIONS: u64 = 32;

/// Optimizes palettes into as few subpalettes as possible.
///
/// Tries several algorithms and picks the best result.
/// - `requirements` should be deduplicated and free of subsets.
pub fn pack(
    requirements: &[BTreeSet<ReducedColor>],
    capacity: usize,
) -> Vec<BTreeSet<ReducedColor>> {
    // Run optimizers, keeping only the best-scoring result
    let mut result = first_fit_decreasing::pack(requirements, capacity);

    let mut consider = |candidate: Vec<BTreeSet<ReducedColor>>| {
        if score(&candidate) < score(&result) {
            result = candidate;
        }
    };

    consider(overload_and_remove::pack(requirements, capacity, None));

    for seed in 0..MAX_ITERATIONS {
        consider(overload_and_remove::pack(requirements, capacity, Some(seed)));
    }

    result
}

fn score(bins: &[BTreeSet<ReducedColor>]) -> (usize, usize) {
    let subpalettes = bins.len();
    let colors = bins.iter().map(BTreeSet::len).sum();
    (subpalettes, colors)
}
