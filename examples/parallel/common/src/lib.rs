//! What the parallel example's guests compute, shared with the host, which checks their output
//! against the same functions run natively.

use core::ops::Range;

use serde::{Deserialize, Serialize};

/// The input of every guest of the example.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Params {
    /// How many values.
    pub len: u64,
    /// How many tasks the guest splits the work into.
    pub tasks: u64,
    /// The seed of the values.
    pub seed: u64,
}

/// The bins of a histogram ([`bin_counts`], [`histogram`]): one per value of the top 4 bits.
pub const BINS: usize = 16;

/// The `i`-th pseudo-random value from `seed` (splitmix64): each value depends on its index
/// only, so a task can generate its own slice of them.
pub fn value(seed: u64, i: u64) -> u64 {
    let mut z = seed.wrapping_add(i.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The values of `range` from `seed`.
pub fn values(range: Range<u64>, seed: u64) -> Vec<u64> {
    range.map(|i| value(seed, i)).collect()
}

/// The `tasks` ranges `0..len` splits into, in order: `len / tasks` indices each, and one more for
/// each of the first `len % tasks`.
pub fn split(len: u64, tasks: u64) -> Vec<Range<usize>> {
    let (len, tasks) = (len as usize, tasks.max(1) as usize);
    let (base, extra) = (len / tasks, len % tasks);
    let mut start = 0;
    (0..tasks)
        .map(|i| {
            let end = start + base + usize::from(i < extra);
            let range = start..end;
            start = end;
            range
        })
        .collect()
}

/// How many unit cells `(x, y)` of the grid `rows × 0..radius` have their centre inside the
/// circle of `radius` around the origin. The centre of cell `(x, y)` is `(x + ½, y + ½)`, so it
/// is inside when `(2x + 1)² + (2y + 1)² < 4 · radius²`. Over `0..radius`, the cells of a quarter
/// of the circle: about `π/4 · radius²`.
///
/// Counting the corners `(x, y)` instead would count the whole column `x = 0`, on the axis: the
/// estimate of π would be off in the third decimal for a radius of 2048.
pub fn cells_inside(rows: Range<u64>, radius: u64) -> u64 {
    let bound = 4 * radius * radius;
    rows.map(|x| {
        let dx = (2 * x + 1) * (2 * x + 1);
        (0..radius).filter(|y| dx + (2 * y + 1) * (2 * y + 1) < bound).count() as u64
    })
    .sum()
}

/// How many of `values` fall in each bin of a histogram, by their top 4 bits.
pub fn bin_counts(values: impl IntoIterator<Item = u64>) -> [u64; BINS] {
    let mut counts = [0u64; BINS];
    for v in values {
        counts[(v >> 60) as usize] += 1;
    }
    counts
}

/// How many of the values of `range` from `seed` fall in each bin, by their top 4 bits.
pub fn histogram(range: Range<u64>, seed: u64) -> [u64; BINS] {
    bin_counts(range.map(|i| value(seed, i)))
}

/// Where bucket `k` of `buckets` starts. The buckets split the `u64` values into ranges of equal
/// width: bucket `k` holds the values from `k · 2^64 / buckets` up to, and not including,
/// `(k + 1) · 2^64 / buckets`. Bucket `buckets`, one past the last, would start at 2^64, past
/// every value: hence the `u128`.
pub fn bucket_start(k: u64, buckets: u64) -> u128 {
    (u128::from(k) << 64) / u128::from(buckets)
}

/// Where the values of bucket `k` of `buckets` lie in the sorted `run`: two binary searches.
pub fn bucket_range(run: &[u64], k: u64, buckets: u64) -> Range<usize> {
    let first_from = |bound: u128| run.partition_point(|&v| u128::from(v) < bound);
    first_from(bucket_start(k, buckets))..first_from(bucket_start(k + 1, buckets))
}

/// A checksum of a sequence of values, sensitive to their order.
pub fn checksum(values: impl IntoIterator<Item = u64>) -> u64 {
    values.into_iter().fold(0u64, |sum, v| sum.rotate_left(5) ^ v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_covers_every_index_once() {
        for (len, tasks) in [(10, 3), (3, 8), (0, 4), (1 << 10, 8)] {
            let ranges = split(len, tasks);
            assert_eq!(ranges.len(), tasks as usize);
            assert_eq!(ranges.first().unwrap().start, 0);
            assert_eq!(ranges.last().unwrap().end, len as usize);
            assert!(ranges.windows(2).all(|w| w[0].end == w[1].start));
        }
    }

    #[test]
    fn cells_inside_estimate_pi() {
        assert_eq!(cells_inside(0..2, 2), 3);
        let radius = 2048;
        let estimate = 4.0 * cells_inside(0..radius, radius) as f64 / (radius * radius) as f64;
        assert!((estimate - core::f64::consts::PI).abs() < 1e-6, "{estimate}");
    }

    #[test]
    fn buckets_split_a_sorted_run_in_order() {
        let mut run = values(0..1000, 3);
        // Both ends of the values, and a value either side of a bucket's start
        run.extend([0, 0, u64::MAX, u64::MAX, 1 << 63, (1 << 63) - 1]);
        run.sort_unstable();
        for buckets in [1, 2, 3, 8, 13] {
            let ranges: Vec<_> = (0..buckets).map(|k| bucket_range(&run, k, buckets)).collect();
            assert_eq!(ranges.first().unwrap().start, 0);
            assert_eq!(ranges.last().unwrap().end, run.len());
            assert!(ranges.windows(2).all(|w| w[0].end == w[1].start));
            for (k, range) in (0..buckets).zip(&ranges) {
                let (start, end) = (bucket_start(k, buckets), bucket_start(k + 1, buckets));
                assert!(run[range.clone()].iter().all(|&v| (start..end).contains(&u128::from(v))));
            }
        }
    }
}
