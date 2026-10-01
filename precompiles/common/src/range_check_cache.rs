//! Per-batch lookup multiplicities, counted locally and flushed once per table.
//!
//! Shared by `ArithEq` and `ArithEq384`: both register the same three ranges (16-bit chunks, a
//! 22-bit quotient range and a signed carry range) and both look up the same `ArithEqLtTable`, so
//! one layout serves them rather than each carrying its own copy.
//!
//! # Why
//!
//! `expand_data_on_trace` used to reach `std.range_check_one` for every column of every row -- about
//! 208 calls per operation -- plus `std.inc_virtual_row_one` once per row for the LT table, and
//! every one of those ends in an atomic `fetch_add` into a single shared multiplicity table. With every thread hitting the same buckets, that contention (not the field
//! arithmetic) is what the fill spends itself on: the executor accounts for roughly a quarter of the
//! per-operation cost, the rest is those atomics and the trace writes.
//!
//! A batch counts into its own array instead. The arrays are summed once the batches are done and
//! handed to `std` with one call per range.
//!
//! # Layout
//!
//! The three ranges sit back to back in one allocation with compile-time bases, so an increment is a
//! single indexed add: no range id to look up, no dispatch through `std`, no atomic.
//!
//! | range    | values                | buckets |
//! |----------|-----------------------|---------|
//! | `q_hsc`  | `[0, 2^22 - 1]`       | 2^22    |
//! | `chunk`  | `[0, 2^16 - 1]`       | 2^16    |
//! | `carry`  | `[-(2^22 - 1), 2^22]` | 2^23    |
//! | `lt`     | table rows            | 655_360 |
//! | `dual`   | `DualByte` rows       | 2^16    |
//!
//! `carry` is signed, so it is shifted by [`CARRY_BIAS`] on the way in and the flush tells `std`
//! which value the slice starts at.
//!
//! `lt` is not a range but the `ArithEqLtTable` virtual table, whose rows
//! `ArithEqLtTableSM::calculate_table_row` already returns as a flat index. It rides along in the
//! same allocation because it shares the batch's lifetime and adds 2.5 MiB to 48.25.
//!
//! `dual` is the `DualByte` virtual table (`range_dual_byte` in PIL): the big-endian airs range-check
//! their six memory operands as byte pairs instead of 16-bit chunks. The PIL looks the pair up high
//! byte first, so the row of a chunk is the chunk value itself, and [`MultiplicityCache::dual_chunk`]
//! is [`MultiplicityCache::chunk`] counting into this region instead. Little-endian airs never
//! touch it, and the flush skips it when nothing landed there.
//!
//! # Why `u32` counters
//!
//! 12_648_448 buckets at 4 bytes is 48.25 MiB per batch — half what `u64` would cost, and the
//! headroom is ample. The busiest range is `chunk`: an instance holds at most
//! `NUM_ROWS / ARITH_EQ_ROWS_BY_OP` operations (262_144 for the tallest air) and each contributes
//! about 157 `chunk` values, so even if every one of those 41M values landed in the *same* bucket it
//! would still sit two orders of magnitude below `u32::MAX`. [`MultiplicityCache::add`] saturates rather
//! than wrapping, so a future air that broke that bound would clamp instead of silently corrupting
//! a multiplicity.

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;

/// Buckets of the `q_hsc` range, `[0, 2^22 - 1]`.
const Q_HSC_LEN: usize = 1 << 22;
/// Buckets of the `chunk` range, `[0, 2^16 - 1]`.
const CHUNK_LEN: usize = 1 << 16;
/// Buckets of the `carry` range, `[-(2^22 - 1), 2^22]`.
const CARRY_LEN: usize = 1 << 23;
/// Rows of the `ArithEqLtTable`: `2^18` middle clocks + `2^18` first + `2^17` last, matching
/// `ARITH_EQ_LT_TABLE_SIZE` in `arith_eq_lt_table.pil`.
const LT_LEN: usize = (1 << 18) + (1 << 18) + (1 << 17);

/// Rows of the `DualByte` virtual table: one per `(high byte, low byte)` pair, i.e. one per chunk value.
const DUAL_LEN: usize = 1 << 16;

/// Added to a carry value to index its bucket: the range starts at `-(2^22 - 1)`.
const CARRY_BIAS: i64 = (1 << 22) - 1;

const Q_HSC_BASE: usize = 0;
const CHUNK_BASE: usize = Q_HSC_BASE + Q_HSC_LEN;
const CARRY_BASE: usize = CHUNK_BASE + CHUNK_LEN;
const LT_BASE: usize = CARRY_BASE + CARRY_LEN;
const DUAL_BASE: usize = LT_BASE + LT_LEN;

/// One allocation for the three ranges, the LT table and the DualByte table.
const CACHE_LEN: usize = DUAL_BASE + DUAL_LEN;

/// Bytes one batch's cache occupies. Reported by the caller's timing so the cost of `n` batches is
/// visible rather than inferred.
pub const CACHE_BYTES: usize = CACHE_LEN * std::mem::size_of::<u32>();

pub struct MultiplicityCache {
    counts: Vec<u32>,
}

impl Default for MultiplicityCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiplicityCache {
    /// A zeroed cache. `vec![0; _]` goes through `alloc_zeroed`, so the pages arrive zeroed from the
    /// kernel and are faulted in as the batch touches them rather than written up front.
    pub fn new() -> Self {
        Self { counts: vec![0u32; CACHE_LEN] }
    }

    /// Counts `value` in the `q_hsc` range and hands it back, so a call site reads as one
    /// expression: `to_field(cache.q_hsc(v))`.
    #[inline(always)]
    pub fn q_hsc(&mut self, value: i64) -> i64 {
        debug_assert!(
            (0..Q_HSC_LEN as i64).contains(&value),
            "q_hsc value {value} outside [0, 2^22)"
        );
        self.counts[Q_HSC_BASE + value as usize] += 1;
        value
    }

    #[inline(always)]
    pub fn chunk(&mut self, value: i64) -> i64 {
        debug_assert!(
            (0..CHUNK_LEN as i64).contains(&value),
            "chunk value {value} outside [0, 2^16)"
        );
        self.counts[CHUNK_BASE + value as usize] += 1;
        value
    }

    /// Counts a 16-bit chunk of a big-endian memory operand: the same value as [`Self::chunk`], but
    /// looked up in the `DualByte` table (as its two bytes, high byte first) rather than the 16-bit
    /// range. `big_endian` selects between the two so the call site reads the same in both modes.
    #[inline(always)]
    pub fn mem_chunk(&mut self, value: i64, big_endian: bool) -> i64 {
        if big_endian {
            self.dual_chunk(value)
        } else {
            self.chunk(value)
        }
    }

    #[inline(always)]
    pub fn dual_chunk(&mut self, value: i64) -> i64 {
        debug_assert!(
            (0..DUAL_LEN as i64).contains(&value),
            "dual chunk value {value} outside [0, 2^16)"
        );
        self.counts[DUAL_BASE + value as usize] += 1;
        value
    }

    #[inline(always)]
    pub fn carry(&mut self, value: i64) -> i64 {
        debug_assert!(
            (-CARRY_BIAS..=CARRY_BIAS + 1).contains(&value),
            "carry value {value} outside [-(2^22-1), 2^22]"
        );
        self.counts[CARRY_BASE + (value + CARRY_BIAS) as usize] += 1;
        value
    }

    /// Counts a row of the `ArithEqLtTable`. The row comes from
    /// `ArithEqLtTableSM::calculate_table_row`, which already panics on a combination the table does
    /// not hold, so the only thing left to guard is the table's own bound.
    #[inline(always)]
    pub fn lt_row(&mut self, row: usize) {
        debug_assert!(row < LT_LEN, "lt row {row} outside the table's {LT_LEN} rows");
        self.counts[LT_BASE + row] += 1;
    }

    /// The `q` columns are checked against `q_hsc` on the last clock and against `chunk` on the
    /// others. Kept here so the branch is one predictable compare next to the increment rather than
    /// a range id chosen a scope away.
    #[inline(always)]
    pub fn q_column(&mut self, value: i64, last_clock: bool) -> i64 {
        if last_clock {
            self.q_hsc(value)
        } else {
            self.chunk(value)
        }
    }

    /// Folds `other` into `self`, in parallel over the buckets.
    ///
    /// Saturating rather than wrapping: see the note on `u32` in the module docs. The bound has two
    /// orders of magnitude of headroom, so this cannot trigger for any air in the current PIL, and
    /// clamping is the safer failure if one ever grew past it.
    pub fn add(&mut self, other: &MultiplicityCache) {
        self.counts
            .par_iter_mut()
            .zip(other.counts.par_iter())
            .for_each(|(dst, src)| *dst = dst.saturating_add(*src));
    }

    /// Hands each range's multiplicities to `std` in a single call.
    ///
    /// One call per range, not one per value: `range_check_ranged` takes the whole slice and walks
    /// it once, skipping the buckets nothing landed in. The `DualByte` table is only handed over
    /// when a big-endian air counted into it, so a little-endian witness costs nothing extra.
    pub fn flush<F: PrimeField64>(
        &self,
        std: &Arc<Std<F>>,
        q_hsc_range_id: usize,
        chunk_range_id: usize,
        carry_range_id: usize,
        lt_table_id: usize,
        dual_byte_table_id: usize,
    ) {
        std.range_check_ranged(q_hsc_range_id, None, &self.counts[Q_HSC_BASE..CHUNK_BASE]);
        std.range_check_ranged(chunk_range_id, None, &self.counts[CHUNK_BASE..CARRY_BASE]);
        std.range_check_ranged(carry_range_id, None, &self.counts[CARRY_BASE..LT_BASE]);
        std.inc_virtual_rows_ranged(lt_table_id, None, &self.counts[LT_BASE..DUAL_BASE]);
        let dual = &self.counts[DUAL_BASE..];
        if dual.iter().any(|&c| c != 0) {
            std.inc_virtual_rows_ranged(dual_byte_table_id, None, dual);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout constants have to agree with the ranges `ArithEqSM::new` registers, or a value
    /// would be counted into the wrong bucket. Pinned here because those are two separate places.
    #[test]
    fn the_layout_matches_the_registered_ranges() {
        assert_eq!(Q_HSC_LEN, (1 << 22), "q_hsc is [0, 2^22 - 1]");
        assert_eq!(CHUNK_LEN, 0xFFFF + 1, "chunk is [0, 0xFFFF]");
        assert_eq!(CARRY_LEN, ((1 << 22) + CARRY_BIAS + 1) as usize, "carry is [-(2^22-1), 2^22]");
        assert_eq!(
            LT_LEN,
            (1 << 18) + (1 << 18) + (1 << 17),
            "ARITH_EQ_LT_TABLE_SIZE in arith_eq_lt_table.pil"
        );
        assert_eq!(LT_LEN, 0xA0000, "and the bound calculate_table_row enforces");
        assert_eq!(DUAL_LEN, 0xFFFF + 1, "DualByte has one row per (high, low) byte pair");
        assert_eq!(
            CACHE_BYTES, 53_477_376,
            "51 MiB per batch: 48.25 of ranges + 2.5 of LT + 0.25 of DualByte"
        );
    }

    /// The LT rows must land in their own region: `calculate_table_row` returns a flat index up to
    /// `0x9FFFF`, and the first and last of those have to stay clear of the carry range below.
    #[test]
    fn lt_rows_count_into_their_own_region() {
        let mut cache = MultiplicityCache::new();
        cache.lt_row(0);
        cache.lt_row(LT_LEN - 1);
        assert_eq!(cache.counts[LT_BASE], 1, "row 0 is the region's first bucket");
        assert_eq!(cache.counts[DUAL_BASE - 1], 1, "the last row is the region's last bucket");
        assert_eq!(cache.counts[LT_BASE - 1], 0, "the carry range above it is untouched");
        assert_eq!(cache.counts[DUAL_BASE], 0, "the DualByte table below it is untouched");
        assert_eq!(cache.counts.iter().filter(|&&c| c != 0).count(), 2);
    }

    /// A big-endian chunk lands in the DualByte region, at the row equal to its value, and never in
    /// the 16-bit chunk range; a little-endian chunk does the opposite.
    #[test]
    fn mem_chunk_picks_the_table_by_endianness() {
        let mut cache = MultiplicityCache::new();
        assert_eq!(cache.mem_chunk(0xABCD, true), 0xABCD);
        assert_eq!(cache.mem_chunk(0xABCD, false), 0xABCD);
        cache.dual_chunk(0);
        cache.dual_chunk(0xFFFF);
        assert_eq!(cache.counts[DUAL_BASE + 0xABCD], 1, "big-endian: DualByte row = chunk value");
        assert_eq!(cache.counts[CHUNK_BASE + 0xABCD], 1, "little-endian: 16-bit range bucket");
        assert_eq!(cache.counts[DUAL_BASE], 1, "row 0 is the region's first bucket");
        assert_eq!(cache.counts[CACHE_LEN - 1], 1, "row 0xFFFF is the last bucket of the cache");
        assert_eq!(cache.counts.iter().filter(|&&c| c != 0).count(), 4, "no bucket counted twice");
    }

    /// Every range must land in its own region, and the extremes of each must be in bounds: an
    /// off-by-one in the bias would corrupt a neighbouring range's multiplicities.
    #[test]
    fn each_range_counts_into_its_own_region() {
        let mut cache = MultiplicityCache::new();
        cache.q_hsc(0);
        cache.q_hsc(Q_HSC_LEN as i64 - 1);
        cache.chunk(0);
        cache.chunk(0xFFFF);
        cache.carry(-CARRY_BIAS);
        cache.carry(0);
        cache.carry(CARRY_BIAS + 1);

        assert_eq!(cache.counts[Q_HSC_BASE], 1);
        assert_eq!(cache.counts[CHUNK_BASE - 1], 1);
        assert_eq!(cache.counts[CHUNK_BASE], 1);
        assert_eq!(cache.counts[CARRY_BASE - 1], 1);
        assert_eq!(cache.counts[CARRY_BASE], 1, "the lowest carry is the region's first bucket");
        assert_eq!(cache.counts[CARRY_BASE + CARRY_BIAS as usize], 1, "carry 0 sits at the bias");
        assert_eq!(cache.counts[LT_BASE - 1], 1, "the highest carry closes the carry region");
        assert_eq!(cache.counts.iter().filter(|&&c| c != 0).count(), 7, "no bucket counted twice");
    }

    #[test]
    fn q_column_picks_the_range_by_clock() {
        let mut cache = MultiplicityCache::new();
        cache.q_column(5, true);
        cache.q_column(5, false);
        assert_eq!(cache.counts[Q_HSC_BASE + 5], 1, "last clock counts into q_hsc");
        assert_eq!(cache.counts[CHUNK_BASE + 5], 1, "the others count into chunk");
    }

    #[test]
    fn add_sums_the_buckets() {
        let mut a = MultiplicityCache::new();
        let mut b = MultiplicityCache::new();
        a.chunk(7);
        a.chunk(7);
        b.chunk(7);
        b.carry(-1);
        a.add(&b);
        assert_eq!(a.counts[CHUNK_BASE + 7], 3);
        assert_eq!(a.counts[CARRY_BASE + (CARRY_BIAS - 1) as usize], 1);
    }

    /// The counters must not wrap: a wrapped multiplicity is a silently wrong witness, whereas a
    /// clamped one is at least a constraint failure.
    #[test]
    fn add_saturates_instead_of_wrapping() {
        let mut a = MultiplicityCache::new();
        let mut b = MultiplicityCache::new();
        a.counts[CHUNK_BASE] = u32::MAX - 1;
        b.counts[CHUNK_BASE] = 5;
        a.add(&b);
        assert_eq!(a.counts[CHUNK_BASE], u32::MAX);
    }
}
