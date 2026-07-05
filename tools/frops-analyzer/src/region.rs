//! Box-shaped FROPS regions and candidate generation.
//!
//! Every proposed predicate is a (possibly strided) box: `a in {a_lo + i*a_stride : 0 <= i <
//! a_count}` and `b in [b_lo, b_lo + b_count)`. This single shape subsumes all the cheap, CPU-bound
//! templates we care about:
//!   * low rectangle  -> a_lo = 0, a_stride = 1, b_lo = 0   (`a < MA && b < MB`)
//!   * address range  -> a_lo > 0, a finite               (`lo <= a < hi`)
//!   * strided range  -> a_stride > 1                       (`lo <= a < hi && (a & (s-1)) == r`)
//!   * high mask      -> a span reaches u64::MAX            (`a >= MASK`)
//!   * b-constant     -> b_count == 1                       (`b == k`)
//!
//! Membership is a few integer comparisons (plus one mask for strided boxes) and the row offset is a
//! closed-form linear expression, so both generated functions stay branch-light and memory-free.

use crate::ingest::{OpAgg, HIGH_FROM, PAGE_SHIFT};

/// Absolute floor on hits for a mid-region b-cluster box. Boxes below `max(this, total/20000)` are
/// pruned so the membership test does not accumulate a long tail of barely-useful comparisons.
const MIN_BOX_HITS: u64 = 5000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegionKind {
    LowRect,
    MidBox,
    HighBox,
}

impl RegionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RegionKind::LowRect => "low_rect",
            RegionKind::MidBox => "mid_box",
            RegionKind::HighBox => "high_box",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Region {
    pub a_lo: u64,
    /// Number of distinct `a` values (already accounts for the stride).
    pub a_count: u64,
    /// Spacing between consecutive `a` values (1 for dense boxes; a power of two for strided ones).
    pub a_stride: u64,
    pub b_lo: u64,
    pub b_count: u64,
    pub kind: RegionKind,
}

impl Region {
    /// Number of table rows this box materialises.
    pub fn rows(&self) -> u64 {
        let r = (self.a_count as u128) * (self.b_count as u128);
        r.min(u64::MAX as u128) as u64
    }

    /// Width of the box on `a` (last value + stride), as u128 to detect a wrap to u64::MAX+1.
    fn a_span(&self) -> u128 {
        self.a_count as u128 * self.a_stride as u128
    }

    /// True when the box extends up to u64::MAX on `a` (high-mask shape).
    pub fn a_to_max(&self) -> bool {
        self.a_lo as u128 + self.a_span() == 1u128 << 64
    }
    pub fn b_to_max(&self) -> bool {
        self.b_lo.wrapping_add(self.b_count) == 0
    }

    /// Reference membership implementation mirrored by the generated code; exercised by the unit
    /// tests below.
    #[allow(dead_code)]
    pub fn contains(&self, a: u64, b: u64) -> bool {
        let a_in_range =
            a >= self.a_lo && (self.a_to_max() || (a as u128) < self.a_lo as u128 + self.a_span());
        let a_aligned = self.a_stride == 1 || (a - self.a_lo) % self.a_stride == 0;
        let b_ok = b >= self.b_lo && (self.b_to_max() || b < self.b_lo + self.b_count);
        a_in_range && a >= self.a_lo && a_aligned && b_ok
    }

    /// Relative offset of `(a, b)` inside this box (row-major over `b`). Caller must ensure
    /// `contains(a, b)`.
    #[allow(dead_code)]
    pub fn offset(&self, a: u64, b: u64) -> u64 {
        ((a - self.a_lo) / self.a_stride) * self.b_count + (b - self.b_lo)
    }

    /// Exclusive upper bound on `a` (only valid when `!a_to_max()`).
    fn a_hi(&self) -> u64 {
        self.a_lo + self.a_count * self.a_stride
    }

    /// Human- and Rust-readable membership predicate.
    pub fn predicate(&self) -> String {
        let mut parts = Vec::new();
        // a axis (with optional stride mask).
        if self.a_stride > 1 {
            if self.a_lo != 0 {
                parts.push(format!("a >= {}", fmt_val(self.a_lo)));
            }
            if !self.a_to_max() {
                parts.push(format!("a < {}", fmt_val(self.a_hi())));
            }
            let mask = self.a_stride - 1;
            let rem = self.a_lo & mask;
            parts.push(format!("(a & {}) == {}", fmt_val(mask), fmt_val(rem)));
        } else if let Some(p) = axis_predicate("a", self.a_lo, self.a_count, self.a_to_max()) {
            parts.push(p);
        }
        // b axis.
        if let Some(p) = axis_predicate("b", self.b_lo, self.b_count, self.b_to_max()) {
            parts.push(p);
        }
        if parts.is_empty() {
            "true".to_string()
        } else {
            parts.join(" && ")
        }
    }
}

/// Renders the predicate for a single (unstrided) coordinate.
fn axis_predicate(var: &str, lo: u64, count: u64, to_max: bool) -> Option<String> {
    if count == 1 {
        return Some(format!("{var} == {}", fmt_val(lo)));
    }
    if lo == 0 {
        if to_max {
            return None;
        }
        return Some(format!("{var} < {}", fmt_val(count)));
    }
    if to_max {
        return Some(format!("{var} >= {}", fmt_val(lo)));
    }
    let hi = lo + count;
    Some(format!("{var} >= {} && {var} < {}", fmt_val(lo), fmt_val(hi)))
}

/// Small values in decimal, large values in hex (addresses / masks read better that way).
pub fn fmt_val(v: u64) -> String {
    if v < 65536 {
        format!("{v}")
    } else {
        format!("{v:#X}")
    }
}

/// A candidate region together with the number of observed occurrences it covers.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub region: Region,
    pub hits: u64,
}

/// Generates, per template (low / mid / high), a *Pareto frontier* of candidate boxes (increasing
/// rows, increasing hits). Each inner vector is one mutually-exclusive group: the optimizer picks at
/// most one box from it, choosing the size that best fits the budget. Templates address disjoint `a`
/// ranges, so candidates from different groups never overlap.
pub fn candidate_groups(agg: &OpAgg, low_cap: u64) -> Vec<Vec<Candidate>> {
    let mut groups = Vec::new();
    let f = low_rect_frontier(agg, low_cap);
    if !f.is_empty() {
        groups.push(f);
    }
    for f in mid_box_groups(agg) {
        if !f.is_empty() {
            groups.push(f);
        }
    }
    let f = high_box_frontier(agg);
    if !f.is_empty() {
        groups.push(f);
    }
    groups
}

/// Pareto frontier over (rows ascending, hits): keep a box only if it covers strictly more hits than
/// any cheaper (or equal-row) box. Result is sorted by rows ascending with strictly increasing hits.
fn pareto(mut pts: Vec<Candidate>) -> Vec<Candidate> {
    pts.sort_by(|x, y| x.region.rows().cmp(&y.region.rows()).then(y.hits.cmp(&x.hits)));
    let mut out: Vec<Candidate> = Vec::new();
    let mut best_hits = 0u64;
    for c in pts {
        if c.hits > best_hits {
            best_hits = c.hits;
            out.push(c);
        }
    }
    out
}

/// Evenly-spaced subset of `0..n` of size at most `cap`, always including the last index. Keeps the
/// frontier search bounded when `low_cap` is large.
fn subsample(n: usize, cap: usize) -> Vec<usize> {
    if n <= cap {
        return (0..n).collect();
    }
    let mut idx: Vec<usize> = (0..cap).map(|k| k * (n - 1) / (cap - 1)).collect();
    idx.dedup();
    idx
}

/// Largest power-of-two stride `s in {8,4,2}` such that every mid-region address agrees on the low
/// `log2(s)` bits, with the common remainder. Returns `(1, 0)` when alignment is mixed.
fn detect_stride(and: u64, or: u64) -> (u64, u64) {
    for k in (1..=3).rev() {
        let mask = (1u64 << k) - 1;
        if (and & mask) == (or & mask) {
            return (1 << k, and & mask);
        }
    }
    (1, 0)
}

/// Pareto frontier of low rectangles `[0, A) x [0, B)`, over observed coordinates via a compressed 2D
/// prefix sum. Corners are subsampled when there are many distinct values, to bound the search.
fn low_rect_frontier(agg: &OpAgg, low_cap: u64) -> Vec<Candidate> {
    if agg.low.is_empty() {
        return Vec::new();
    }
    let mut a_vals: Vec<u64> = Vec::new();
    let mut b_vals: Vec<u64> = Vec::new();
    {
        use std::collections::BTreeSet;
        let mut aset = BTreeSet::new();
        let mut bset = BTreeSet::new();
        for &key in agg.low.keys() {
            aset.insert(key / low_cap);
            bset.insert(key % low_cap);
        }
        a_vals.extend(aset);
        b_vals.extend(bset);
    }
    let na = a_vals.len();
    let nb = b_vals.len();
    let a_idx: std::collections::HashMap<u64, usize> =
        a_vals.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let b_idx: std::collections::HashMap<u64, usize> =
        b_vals.iter().enumerate().map(|(i, &v)| (v, i)).collect();

    let mut grid = vec![0u64; na * nb];
    for (&key, &cnt) in &agg.low {
        let i = a_idx[&(key / low_cap)];
        let j = b_idx[&(key % low_cap)];
        grid[i * nb + j] += cnt;
    }
    let mut cum = vec![0u64; na * nb];
    for i in 0..na {
        for j in 0..nb {
            let here = grid[i * nb + j];
            let up = if i > 0 { cum[(i - 1) * nb + j] } else { 0 };
            let left = if j > 0 { cum[i * nb + (j - 1)] } else { 0 };
            let diag = if i > 0 && j > 0 { cum[(i - 1) * nb + (j - 1)] } else { 0 };
            cum[i * nb + j] = here + up + left - diag;
        }
    }

    // Subsample corners to keep the candidate set bounded for large low_cap.
    const MAX_DIM: usize = 600;
    let ai = subsample(na, MAX_DIM);
    let bj = subsample(nb, MAX_DIM);
    let mut cands = Vec::with_capacity(ai.len() * bj.len());
    for &i in &ai {
        let a_hi = a_vals[i] + 1;
        for &j in &bj {
            let b_hi = b_vals[j] + 1;
            let hits = cum[i * nb + j];
            cands.push(Candidate {
                region: Region {
                    a_lo: 0,
                    a_count: a_hi,
                    a_stride: 1,
                    b_lo: 0,
                    b_count: b_hi,
                    kind: RegionKind::LowRect,
                },
                hits,
            });
        }
    }
    pareto(cands)
}

struct MidPageView {
    page: u64,
    max_b: u64,
    a_and: u64,
    a_or: u64,
}

/// Address-range groups from the mid-region histogram. Each contiguous run of pages is split into its
/// `b` clusters (the b-analog of the a-stride): a wide, sparse `b` range becomes several tight boxes
/// instead of one bloated `[0, max_b]` box. Each (run, b-cluster) is an independent group (disjoint in
/// `b`), and within it the run-prefixes form a Pareto frontier of `a` extents. Stride on `a` is still
/// detected per cluster.
fn mid_box_groups(agg: &OpAgg) -> Vec<Vec<Candidate>> {
    if agg.mid.is_empty() {
        return Vec::new();
    }
    let mut pages: Vec<MidPageView> = agg
        .mid
        .iter()
        .map(|(&p, v)| MidPageView { page: p, max_b: v.max_b, a_and: v.a_and, a_or: v.a_or })
        .collect();
    pages.sort_by_key(|x| x.page);

    // Group the joint (page, b) counts by page.
    let mut page_bs: std::collections::HashMap<u64, Vec<(u64, u64)>> =
        std::collections::HashMap::new();
    for (&(p, b), &c) in &agg.mid_pb {
        page_bs.entry(p).or_default().push((b, c));
    }

    let mut groups: Vec<Vec<Candidate>> = Vec::new();
    let mut start = 0usize;
    while start < pages.len() {
        let mut end = start;
        while end + 1 < pages.len() && pages[end + 1].page == pages[end].page + 1 {
            end += 1;
        }
        let page_lo = pages[start].page << PAGE_SHIFT;

        // b distribution over the whole run, then split into clusters.
        let mut brun: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
        for pg in &pages[start..=end] {
            if let Some(v) = page_bs.get(&pg.page) {
                for &(b, c) in v {
                    *brun.entry(b).or_default() += c;
                }
            }
        }
        let clusters =
            b_clusters(&brun, pages[start..=end].iter().map(|p| p.max_b).max().unwrap_or(0));

        // Prune the long tail of tiny boxes: each one would add a comparison to the (hot) membership
        // test for negligible coverage. Keep only clusters covering a meaningful number of hits.
        let floor = (agg.total / 20_000).max(MIN_BOX_HITS);

        for (b_lo, b_hi, b_total) in clusters {
            if b_total < floor {
                continue;
            }
            let mut opts: Vec<Candidate> = Vec::new();
            let mut and = u64::MAX;
            let mut or = 0u64;
            let mut hits = 0u64;
            for pg in &pages[start..=end] {
                if let Some(v) = page_bs.get(&pg.page) {
                    for &(b, c) in v {
                        if b >= b_lo && b < b_hi {
                            hits += c;
                        }
                    }
                }
                and &= pg.a_and;
                or |= pg.a_or;
                let (stride, rem) = detect_stride(and, or);
                let a_lo = page_lo + rem;
                let a_hi = (pg.page + 1) << PAGE_SHIFT;
                if a_hi > HIGH_FROM || a_hi <= a_lo {
                    break;
                }
                let a_count = (a_hi - a_lo).div_ceil(stride);
                opts.push(Candidate {
                    region: Region {
                        a_lo,
                        a_count,
                        a_stride: stride,
                        b_lo,
                        b_count: b_hi - b_lo,
                        kind: RegionKind::MidBox,
                    },
                    hits,
                });
            }
            groups.push(pareto(opts));
        }
        start = end + 1;
    }
    groups
}

/// Splits the observed `b` values into clusters: consecutive bands separated by a gap larger than
/// `B_MERGE_GAP` start a new cluster. Returns `(b_lo, b_hi, total_hits)`, capped to the most populated
/// ones. A fully dense range collapses to a single cluster (no change from the old single-box one).
fn b_clusters(
    brun: &std::collections::BTreeMap<u64, u64>,
    max_b_fallback: u64,
) -> Vec<(u64, u64, u64)> {
    const B_MERGE_GAP: u64 = 64;
    const MAX_CLUSTERS: usize = 16;
    if brun.is_empty() {
        // No b resolution (joint spilled): fall back to one wide box [0, max_b].
        return vec![(0, max_b_fallback + 1, u64::MAX)];
    }
    let mut runs: Vec<(u64, u64, u64)> = Vec::new(); // (b_lo, b_hi, total)
    let mut lo = None;
    let mut prev = 0u64;
    let mut tot = 0u64;
    for (&b, &c) in brun.iter() {
        match lo {
            None => {
                lo = Some(b);
                prev = b;
                tot = c;
            }
            Some(l) => {
                if b - prev <= B_MERGE_GAP {
                    prev = b;
                    tot += c;
                } else {
                    runs.push((l, prev + 1, tot));
                    lo = Some(b);
                    prev = b;
                    tot = c;
                }
            }
        }
    }
    if let Some(l) = lo {
        runs.push((l, prev + 1, tot));
    }
    runs.sort_by_key(|x| std::cmp::Reverse(x.2));
    runs.truncate(MAX_CLUSTERS);
    runs
}

/// Frontier for the high-mask box covering `[a_min, u64::MAX]` x `[0, max_b]` (a single candidate).
fn high_box_frontier(agg: &OpAgg) -> Vec<Candidate> {
    if agg.high.is_empty() {
        return Vec::new();
    }
    let a_lo = agg.high_min_a;
    let a_count = u64::MAX - a_lo + 1;
    let b_count = agg.high_max_b + 1;
    let hits: u64 = agg.high.values().sum();
    vec![Candidate {
        region: Region { a_lo, a_count, a_stride: 1, b_lo: 0, b_count, kind: RegionKind::HighBox },
        hits,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(a_lo: u64, a_count: u64, b_lo: u64, b_count: u64) -> Region {
        Region { a_lo, a_count, a_stride: 1, b_lo, b_count, kind: RegionKind::LowRect }
    }

    #[test]
    fn low_rect_offset_and_predicate() {
        let r = rect(0, 386, 0, 386);
        assert_eq!(r.rows(), 386 * 386);
        assert_eq!(r.predicate(), "a < 386 && b < 386");
        assert!(r.contains(0, 0) && r.contains(385, 385));
        assert!(!r.contains(386, 0) && !r.contains(0, 386));
        assert_eq!(r.offset(0, 0), 0);
        assert_eq!(r.offset(1, 0), 386);
        assert_eq!(r.offset(2, 5), 2 * 386 + 5);
    }

    #[test]
    fn high_box_to_max() {
        let a_lo = 0xFFFF_FFFF_FFFF_F000;
        let r = Region {
            a_lo,
            a_count: u64::MAX - a_lo + 1,
            a_stride: 1,
            b_lo: 0,
            b_count: 65,
            kind: RegionKind::HighBox,
        };
        assert!(r.a_to_max());
        assert_eq!(r.predicate(), "a >= 0xFFFFFFFFFFFFF000 && b < 65");
        assert!(r.contains(u64::MAX, 64));
        assert!(!r.contains(u64::MAX, 65));
        assert!(!r.contains(a_lo - 1, 0));
        assert_eq!(r.offset(a_lo, 0), 0);
        assert_eq!(r.offset(a_lo + 1, 3), 65 + 3);
    }

    #[test]
    fn b_constant_box() {
        let r = Region {
            a_lo: 0,
            a_count: 1024,
            a_stride: 1,
            b_lo: 7,
            b_count: 1,
            kind: RegionKind::MidBox,
        };
        assert_eq!(r.predicate(), "a < 1024 && b == 7");
        assert!(r.contains(10, 7) && !r.contains(10, 8));
        assert_eq!(r.offset(10, 7), 10);
    }

    #[test]
    fn strided_box() {
        // 8-byte aligned addresses in [0xA0100000, 0xA0100000 + 8*4), b < 2.
        let a_lo = 0xA010_0000;
        let r =
            Region { a_lo, a_count: 4, a_stride: 8, b_lo: 0, b_count: 2, kind: RegionKind::MidBox };
        assert_eq!(r.rows(), 8);
        assert_eq!(r.predicate(), "a >= 0xA0100000 && a < 0xA0100020 && (a & 7) == 0 && b < 2");
        assert!(r.contains(a_lo, 0));
        assert!(r.contains(a_lo + 8, 1));
        assert!(!r.contains(a_lo + 1, 0)); // misaligned
        assert!(!r.contains(a_lo + 32, 0)); // out of range
        assert_eq!(r.offset(a_lo, 0), 0);
        assert_eq!(r.offset(a_lo + 8, 1), 1 * 2 + 1);
        assert_eq!(r.offset(a_lo + 24, 0), 3 * 2);
    }

    #[test]
    fn detect_stride_works() {
        // all 8-aligned -> stride 8, rem 0
        assert_eq!(detect_stride(0xA0100000, 0xA0100FF8), (8, 0));
        // mixed low bit -> stride 1
        assert_eq!(detect_stride(0xA0100000, 0xA0100FF9), (1, 0));
        // 4-aligned with remainder 0 (bit2 differs) -> stride 4
        assert_eq!(detect_stride(0x1000, 0x1004), (4, 0));
    }
}
