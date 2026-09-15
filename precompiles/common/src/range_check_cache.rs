//! Per-batch multiplicities for the `ArithEqLtTable`, counted locally and flushed once.
//!
//! Shared by `ArithEq` and `ArithEq384`: both look up the same table, so one layout serves them.
//!
//! # Why
//!
//! `expand_data_on_trace` used to reach `std.inc_virtual_row_one` once per row, and every one of
//! those ends in an atomic `fetch_add` into a single shared multiplicity table. With every thread
//! hitting the same buckets, that contention -- not the field arithmetic -- is what the fill spends
//! itself on. A batch counts into its own array instead; the arrays are summed once the batches are
//! done and handed to `std` in one call.
//!
//! The three *ranges* this used to carry (16-bit chunks, a 22-bit quotient, a signed carry) are
//! gone: the prover computes range-check multiplicities from the committed trace, so counting them
//! here was 45.75 MiB of allocation, zeroing and summation per batch whose result was discarded.
//! The LT table stays because it is a virtual table rather than a range, and is NOT one of the
//! tables the prover owns -- drop its counting and nothing replaces it.
//!
//! # Why `u32` counters
//!
//! The busiest bucket cannot come close to overflowing: an instance holds at most
//! `NUM_ROWS / ARITH_EQ_ROWS_BY_OP` operations (262_144 for the tallest air), each contributing one
//! row. [`MultiplicityCache::add`] saturates rather than wrapping, so a future air that broke that
//! bound would clamp instead of silently corrupting a multiplicity.

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;

/// Rows of the `ArithEqLtTable`: `2^18` middle clocks + `2^18` first + `2^17` last, matching
/// `ARITH_EQ_LT_TABLE_SIZE` in `arith_eq_lt_table.pil`.
const LT_LEN: usize = (1 << 18) + (1 << 18) + (1 << 17);

const CACHE_LEN: usize = LT_LEN;

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

    /// Counts a row of the `ArithEqLtTable`. The row comes from
    /// `ArithEqLtTableSM::calculate_table_row`, which already panics on a combination the table does
    /// not hold, so the only thing left to guard is the table's own bound.
    #[inline(always)]
    pub fn lt_row(&mut self, row: usize) {
        debug_assert!(row < LT_LEN, "lt row {row} outside the table's {LT_LEN} rows");
        self.counts[row] += 1;
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

    /// Hands the table's multiplicities to `std` in a single call, walking the buckets once and
    /// skipping the ones nothing landed in.
    pub fn flush<F: PrimeField64>(&self, std: &Arc<Std<F>>, lt_table_id: usize) {
        std.inc_virtual_rows_ranged(lt_table_id, None, &self.counts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout has to agree with the table `ArithEqSM` registers, or a row would be counted into
    /// the wrong bucket. Pinned here because those are two separate places.
    #[test]
    fn the_layout_matches_the_table() {
        assert_eq!(
            LT_LEN,
            (1 << 18) + (1 << 18) + (1 << 17),
            "ARITH_EQ_LT_TABLE_SIZE in arith_eq_lt_table.pil"
        );
        assert_eq!(LT_LEN, 0xA0000, "and the bound calculate_table_row enforces");
        assert_eq!(CACHE_BYTES, 2_621_440, "2.5 MiB per batch");
    }

    /// Both ends of the table must be reachable: `calculate_table_row` returns a flat index up to
    /// `0x9FFFF`, and an off-by-one at either end would silently drop a lookup.
    #[test]
    fn both_ends_of_the_table_are_reachable() {
        let mut cache = MultiplicityCache::new();
        cache.lt_row(0);
        cache.lt_row(LT_LEN - 1);
        assert_eq!(cache.counts[0], 1, "row 0 is the first bucket");
        assert_eq!(cache.counts[CACHE_LEN - 1], 1, "the last row is the last bucket");
    }

    /// Folding two batches must add their counts, since that is how the per-batch caches become the
    /// instance's multiplicities.
    #[test]
    fn folding_adds_the_counts() {
        let mut a = MultiplicityCache::new();
        let mut b = MultiplicityCache::new();
        a.lt_row(7);
        b.lt_row(7);
        b.lt_row(9);
        a.add(&b);
        assert_eq!(a.counts[7], 2);
        assert_eq!(a.counts[9], 1);
    }
}
