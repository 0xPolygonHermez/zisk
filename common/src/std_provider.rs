//! A mockable seam over the write surface of [`pil_std_lib::Std`].
//!
//! State machines, precompiles, and input collectors only need to *emit* into
//! `Std` — bumping range-check and virtual-table multiplicities. Depending on
//! these traits instead of the concrete `Std` lets those components be
//! constructed, unit-tested, and benchmarked without a `ProofCtx`/`SetupCtx`-backed
//! `Std`. In production they are instantiated with the real `Std<F>`, which
//! monomorphizes to the same code as before.
//!
//! The surface is split into two independent sibling traits, each a 1:1 mirror
//! of the corresponding API section of `Std`:
//! - [`VirtualTableAccumulator`] — the "Virtual Table API",
//! - [`RangeCheckAccumulator`] — the "Range Check API".
//!
//! [`StdProvider`] is the combinator supertrait of both; it is blanket-implemented
//! so any type implementing both siblings qualifies (and `Std<F>` does). Components
//! bound their range-checker on [`StdProvider`]; the two sibling traits exist to
//! document the two API sections and to be the units `StdProvider` composes.

use fields::PrimeField64;
use pil_std_lib::{RCMultiplicity, RCValue, Std};
use proofman_common::ProofmanResult;
use std::sync::Mutex;

/// The combinator over both accumulation surfaces of [`pil_std_lib::Std`].
///
/// Blanket-implemented for every type that implements both sibling traits, so
/// `Std<F>` (and the test doubles below) satisfy it automatically. This is the
/// bound components use; the siblings exist to document the two API sections.
pub trait StdProvider: VirtualTableAccumulator + RangeCheckAccumulator {}
impl<T: VirtualTableAccumulator + RangeCheckAccumulator> StdProvider for T {}

/// The virtual-table accumulation surface of [`pil_std_lib::Std`].
///
/// A 1:1 mirror of `Std`'s "Virtual Table API". The method signatures match
/// `Std`'s exactly, so call sites are unchanged when a component is made generic
/// over this trait. In production it is instantiated with the real `Std<F>`
/// (which monomorphizes to the same code as before); tests and benchmarks can
/// supply a lightweight mock and avoid a `ProofCtx`/`SetupCtx`-backed `Std`.
///
/// `'static` is part of the trait so the many witness-gen types that erase a
/// sink into `Box<dyn Any>`/`Box<dyn Instance>` don't each repeat `+ 'static`.
/// All implementors (`Std<F>`, [`NoopStdProvider`], [`RecordingStdProvider`]) are `'static`.
pub trait VirtualTableAccumulator: Send + Sync + 'static {
    /// See [`pil_std_lib::Std::get_virtual_table_id`].
    fn get_virtual_table_id(&self, id: usize) -> ProofmanResult<usize>;

    /// See [`pil_std_lib::Std::inc_virtual_row`].
    fn inc_virtual_row<M: RCMultiplicity>(&self, id: usize, row: M, mul: M);

    /// See [`pil_std_lib::Std::inc_virtual_row_one`].
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, id: usize, row: M);

    /// See [`pil_std_lib::Std::inc_virtual_row_batch`].
    fn inc_virtual_row_batch<M: RCMultiplicity>(&self, id: usize, rows: &[M], muls: &[M]);

    /// See [`pil_std_lib::Std::inc_virtual_row_batch_one`].
    fn inc_virtual_row_batch_one<M: RCMultiplicity>(&self, id: usize, rows: &[M]);

    /// See [`pil_std_lib::Std::inc_virtual_rows_same_mul`].
    fn inc_virtual_rows_same_mul<M: RCMultiplicity>(&self, id: usize, rows: &[M], mul: M);

    /// See [`pil_std_lib::Std::inc_virtual_rows_ranged`].
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<u64>, muls: &[M]);
}

/// The range-check accumulation surface of [`pil_std_lib::Std`].
///
/// A 1:1 mirror of `Std`'s "Range Check API". The method signatures match
/// `Std`'s exactly, so call sites are unchanged when a component is made generic
/// over this trait. In production it is instantiated with the real `Std<F>`
/// (which monomorphizes to the same code as before); tests and benchmarks can
/// supply a lightweight mock and avoid a `ProofCtx`/`SetupCtx`-backed `Std`.
///
/// This trait is not object-safe (its methods are generic); it is always used
/// via static dispatch (`<RC: RangeCheckAccumulator>`), never `dyn`.
pub trait RangeCheckAccumulator: Send + Sync + 'static {
    /// See [`pil_std_lib::Std::get_range_id`].
    fn get_range_id<V: RCValue>(
        &self,
        min: V,
        max: V,
        predefined: Option<bool>,
    ) -> ProofmanResult<usize>;

    /// See [`pil_std_lib::Std::range_check`].
    fn range_check<V: RCValue, M: RCMultiplicity>(&self, id: usize, val: V, mul: M);

    /// See [`pil_std_lib::Std::range_check_one`].
    fn range_check_one<V: RCValue>(&self, id: usize, val: V);

    /// See [`pil_std_lib::Std::range_check_batch`].
    fn range_check_batch<V: RCValue, M: RCMultiplicity>(&self, id: usize, vals: &[V], muls: &[M]);

    /// See [`pil_std_lib::Std::range_check_batch_one`].
    fn range_check_batch_one<V: RCValue>(&self, id: usize, vals: &[V]);

    /// See [`pil_std_lib::Std::range_checks_same_mul`].
    fn range_checks_same_mul<V: RCValue, M: RCMultiplicity>(&self, id: usize, vals: &[V], mul: M);

    /// See [`pil_std_lib::Std::range_check_ranged`].
    fn range_check_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<i64>, muls: &[M]);
}

impl<F: PrimeField64> VirtualTableAccumulator for Std<F> {
    #[inline(always)]
    fn get_virtual_table_id(&self, id: usize) -> ProofmanResult<usize> {
        Std::get_virtual_table_id(self, id)
    }

    #[inline(always)]
    fn inc_virtual_row<M: RCMultiplicity>(&self, id: usize, row: M, mul: M) {
        Std::inc_virtual_row(self, id, row, mul);
    }

    #[inline(always)]
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, id: usize, row: M) {
        Std::inc_virtual_row_one(self, id, row);
    }

    #[inline(always)]
    fn inc_virtual_row_batch<M: RCMultiplicity>(&self, id: usize, rows: &[M], muls: &[M]) {
        Std::inc_virtual_row_batch(self, id, rows, muls);
    }

    #[inline(always)]
    fn inc_virtual_row_batch_one<M: RCMultiplicity>(&self, id: usize, rows: &[M]) {
        Std::inc_virtual_row_batch_one(self, id, rows);
    }

    #[inline(always)]
    fn inc_virtual_rows_same_mul<M: RCMultiplicity>(&self, id: usize, rows: &[M], mul: M) {
        Std::inc_virtual_rows_same_mul(self, id, rows, mul);
    }

    #[inline(always)]
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(
        &self,
        id: usize,
        start: Option<u64>,
        muls: &[M],
    ) {
        Std::inc_virtual_rows_ranged(self, id, start, muls);
    }
}

impl<F: PrimeField64> RangeCheckAccumulator for Std<F> {
    #[inline(always)]
    fn get_range_id<V: RCValue>(
        &self,
        min: V,
        max: V,
        predefined: Option<bool>,
    ) -> ProofmanResult<usize> {
        Std::get_range_id(self, min, max, predefined)
    }

    #[inline(always)]
    fn range_check<V: RCValue, M: RCMultiplicity>(&self, id: usize, val: V, mul: M) {
        Std::range_check(self, id, val, mul);
    }

    #[inline(always)]
    fn range_check_one<V: RCValue>(&self, id: usize, val: V) {
        Std::range_check_one(self, id, val);
    }

    #[inline(always)]
    fn range_check_batch<V: RCValue, M: RCMultiplicity>(&self, id: usize, vals: &[V], muls: &[M]) {
        Std::range_check_batch(self, id, vals, muls);
    }

    #[inline(always)]
    fn range_check_batch_one<V: RCValue>(&self, id: usize, vals: &[V]) {
        Std::range_check_batch_one(self, id, vals);
    }

    #[inline(always)]
    fn range_checks_same_mul<V: RCValue, M: RCMultiplicity>(&self, id: usize, vals: &[V], mul: M) {
        Std::range_checks_same_mul(self, id, vals, mul);
    }

    #[inline(always)]
    fn range_check_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<i64>, muls: &[M]) {
        Std::range_check_ranged(self, id, start, muls);
    }
}

/// A no-op [`StdProvider`]. All range-check / virtual-table writes are discarded.
///
/// Use it wherever accumulation genuinely does not happen, so no
/// `ProofCtx`/`SetupCtx`-backed `Std` is needed:
/// - unit tests and benches of state machines / precompiles,
/// - execute-only executors (no witness phase),
/// - as the type token for the RC-independent count/plan phase, where
///   `<SM<STD> as ComponentPlanBuilder>::counter/planner` only need *a*
///   provider to name the impl and never touch an instance.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopStdProvider;

impl VirtualTableAccumulator for NoopStdProvider {
    fn get_virtual_table_id(&self, _id: usize) -> ProofmanResult<usize> {
        Ok(0)
    }
    fn inc_virtual_row<M: RCMultiplicity>(&self, _id: usize, _row: M, _mul: M) {}
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, _id: usize, _row: M) {}
    fn inc_virtual_row_batch<M: RCMultiplicity>(&self, _id: usize, _rows: &[M], _muls: &[M]) {}
    fn inc_virtual_row_batch_one<M: RCMultiplicity>(&self, _id: usize, _rows: &[M]) {}
    fn inc_virtual_rows_same_mul<M: RCMultiplicity>(&self, _id: usize, _rows: &[M], _mul: M) {}
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(
        &self,
        _id: usize,
        _start: Option<u64>,
        _muls: &[M],
    ) {
    }
}

impl RangeCheckAccumulator for NoopStdProvider {
    fn get_range_id<V: RCValue>(
        &self,
        _min: V,
        _max: V,
        _predefined: Option<bool>,
    ) -> ProofmanResult<usize> {
        Ok(0)
    }
    fn range_check<V: RCValue, M: RCMultiplicity>(&self, _id: usize, _val: V, _mul: M) {}
    fn range_check_one<V: RCValue>(&self, _id: usize, _val: V) {}
    fn range_check_batch<V: RCValue, M: RCMultiplicity>(
        &self,
        _id: usize,
        _vals: &[V],
        _muls: &[M],
    ) {
    }
    fn range_check_batch_one<V: RCValue>(&self, _id: usize, _vals: &[V]) {}
    fn range_checks_same_mul<V: RCValue, M: RCMultiplicity>(
        &self,
        _id: usize,
        _vals: &[V],
        _mul: M,
    ) {
    }
    fn range_check_ranged<M: RCMultiplicity>(&self, _id: usize, _start: Option<i64>, _muls: &[M]) {}
}

/// A recording [`StdProvider`] for tests.
///
/// Unlike [`NoopStdProvider`] (which discards every write), this captures what a
/// collector or witness-generator emits, so a test can assert *what* was recorded:
/// - range checks from `range_check` / `range_check_one` / `range_check_ranged`
///   — see [`Self::range_checks`] (the `RCValue` is recorded as `i64`, the
///   multiplicity as `u64`);
/// - virtual-row increments from `inc_virtual_row` / `inc_virtual_row_one` /
///   `inc_virtual_rows_ranged` — see [`Self::virtual_rows`].
///
/// `get_virtual_table_id` echoes the logical id so recorded entries correlate
/// with the table the caller resolved.
#[derive(Debug, Default)]
pub struct RecordingStdProvider {
    range_checks: Mutex<Vec<(usize, i64, u64)>>,
    virtual_rows: Mutex<Vec<(usize, u64, u64)>>,
}

impl RecordingStdProvider {
    /// `(range_id, value, multiplicity)` for every `range_check*` call, in order.
    pub fn range_checks(&self) -> Vec<(usize, i64, u64)> {
        self.range_checks.lock().unwrap().clone()
    }

    /// `(table_id, row, multiplicity)` for every `inc_virtual_row*` call, in order.
    pub fn virtual_rows(&self) -> Vec<(usize, u64, u64)> {
        self.virtual_rows.lock().unwrap().clone()
    }
}

impl VirtualTableAccumulator for RecordingStdProvider {
    fn get_virtual_table_id(&self, id: usize) -> ProofmanResult<usize> {
        Ok(id)
    }
    fn inc_virtual_row<M: RCMultiplicity>(&self, id: usize, row: M, mul: M) {
        self.virtual_rows.lock().unwrap().push((id, row.to_u64(), mul.to_u64()));
    }
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, id: usize, row: M) {
        self.virtual_rows.lock().unwrap().push((id, row.to_u64(), 1));
    }
    fn inc_virtual_row_batch<M: RCMultiplicity>(&self, id: usize, rows: &[M], muls: &[M]) {
        let mut guard = self.virtual_rows.lock().unwrap();
        for (r, m) in rows.iter().zip(muls.iter()) {
            guard.push((id, (*r).to_u64(), (*m).to_u64()));
        }
    }
    fn inc_virtual_row_batch_one<M: RCMultiplicity>(&self, id: usize, rows: &[M]) {
        let mut guard = self.virtual_rows.lock().unwrap();
        for r in rows {
            guard.push((id, (*r).to_u64(), 1));
        }
    }
    fn inc_virtual_rows_same_mul<M: RCMultiplicity>(&self, id: usize, rows: &[M], mul: M) {
        let mut guard = self.virtual_rows.lock().unwrap();
        for r in rows {
            guard.push((id, (*r).to_u64(), mul.to_u64()));
        }
    }
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(
        &self,
        id: usize,
        start: Option<u64>,
        muls: &[M],
    ) {
        let base = start.unwrap_or(0);
        let mut guard = self.virtual_rows.lock().unwrap();
        for (i, m) in muls.iter().enumerate() {
            guard.push((id, base + i as u64, (*m).to_u64()));
        }
    }
}

impl RangeCheckAccumulator for RecordingStdProvider {
    fn get_range_id<V: RCValue>(
        &self,
        _min: V,
        _max: V,
        _predefined: Option<bool>,
    ) -> ProofmanResult<usize> {
        Ok(0)
    }
    fn range_check<V: RCValue, M: RCMultiplicity>(&self, id: usize, val: V, mul: M) {
        self.range_checks.lock().unwrap().push((id, val.to_i64(), mul.to_u64()));
    }
    fn range_check_one<V: RCValue>(&self, id: usize, val: V) {
        self.range_checks.lock().unwrap().push((id, val.to_i64(), 1));
    }
    fn range_check_batch<V: RCValue, M: RCMultiplicity>(&self, id: usize, vals: &[V], muls: &[M]) {
        let mut guard = self.range_checks.lock().unwrap();
        for (v, m) in vals.iter().zip(muls.iter()) {
            guard.push((id, (*v).to_i64(), (*m).to_u64()));
        }
    }
    fn range_check_batch_one<V: RCValue>(&self, id: usize, vals: &[V]) {
        let mut guard = self.range_checks.lock().unwrap();
        for v in vals {
            guard.push((id, (*v).to_i64(), 1));
        }
    }
    fn range_checks_same_mul<V: RCValue, M: RCMultiplicity>(&self, id: usize, vals: &[V], mul: M) {
        let mut guard = self.range_checks.lock().unwrap();
        for v in vals {
            guard.push((id, (*v).to_i64(), mul.to_u64()));
        }
    }
    fn range_check_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<i64>, muls: &[M]) {
        let base = start.unwrap_or(0);
        let mut guard = self.range_checks.lock().unwrap();
        for (i, m) in muls.iter().enumerate() {
            guard.push((id, base + i as i64, (*m).to_u64()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_std_provider_captures_each_surface() {
        let rc = RecordingStdProvider::default();

        // Virtual-table increments — the FROPS path collectors take.
        rc.inc_virtual_row_one(5010, 7usize);
        rc.inc_virtual_row(5010, 9u64, 2u64);
        assert_eq!(rc.virtual_rows(), vec![(5010, 7, 1), (5010, 9, 2)]);

        // Range checks — value recorded as i64, multiplicity as u64.
        rc.range_check_one(3, 42u64);
        rc.range_check(3, -1i64, 4u64);
        rc.range_check_ranged(7, Some(10), &[2u32, 5u32]);
        assert_eq!(rc.range_checks(), vec![(3, 42, 1), (3, -1, 4), (7, 10, 2), (7, 11, 5)]);
    }
}
