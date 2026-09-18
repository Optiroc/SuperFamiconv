//! Selection helpers.

/// Index and values of the smallest and second smallest of `values`.
pub(super) fn smallest_two(values: impl IntoIterator<Item = f32>) -> (usize, f32, f32) {
    let (mut best_i, mut best_v) = (0usize, f32::INFINITY);
    let mut second_v = f32::INFINITY;
    for (i, v) in values.into_iter().enumerate() {
        if v < best_v {
            second_v = best_v;
            best_v = v;
            best_i = i;
        } else if v < second_v {
            second_v = v;
        }
    }
    (best_i, best_v, second_v)
}

pub(super) fn argmax(values: &[f32]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(i, _)| i)
}

pub(super) fn argmin(values: &[f32]) -> usize {
    values
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(i, _)| i)
}
