use std::collections::BTreeSet;

use crate::color::ReducedColor;

/// Optimizes palettes using first-fit-decreasing bin packing.
///
/// `requirements` must already be deduplicated and free of subsets.
pub fn pack(
    requirements: &[BTreeSet<ReducedColor>],
    capacity: usize,
) -> Vec<BTreeSet<ReducedColor>> {
    let mut sets: Vec<BTreeSet<ReducedColor>> = requirements.to_vec();
    sets.sort_by_key(|s| std::cmp::Reverse(s.len()));

    let mut bins: Vec<BTreeSet<ReducedColor>> = Vec::new();
    for set in sets.drain(..) {
        let fit = bins.iter().position(|bin| bin.union(&set).count() <= capacity);
        match fit {
            Some(i) => bins[i] = bins[i].union(&set).copied().collect(),
            None => bins.push(set),
        }
    }

    bins.sort_by_key(|b| std::cmp::Reverse(b.len()));
    bins
}
