//! A mockable seam over the slice of [`pil_std_lib::Std`] used by input collectors.
//!
//! Input collectors only need to resolve a virtual-table id and bump a single
//! row's multiplicity. Depending on this trait instead of the concrete `Std`
//! lets collectors be constructed, unit-tested, and benchmarked without a
//! `ProofCtx`/`SetupCtx`-backed `Std`. In production the trait is instantiated
//! with the real `Std<F>`, which monomorphizes to the same code as before.

use fields::PrimeField64;
use pil_std_lib::{RCMultiplicity, RCValue, Std};
use proofman_common::ProofmanResult;
use std::sync::Mutex;

/// The virtual-table accumulation surface that input collectors depend on.
///
/// This is the minimal subset of [`pil_std_lib::Std`] reached from a
/// collector's `process_data`: resolving a table id (at construction) and
/// incrementing a single row (on the FROPS path).
///
/// `'static` is part of the trait so the many witness-gen types that erase a
/// sink into `Box<dyn Any>`/`Box<dyn Instance>` don't each repeat `+ 'static`.
/// Both implementors (`Std<F>` and `NoopRangeChecker`) are `'static`.
pub trait VirtualTableSink: Send + Sync + 'static {
    /// Resolves the runtime virtual-table id for the given logical table id.
    fn virtual_table_id(&self, table_id: usize) -> usize;

    /// Increments the multiplicity of a single virtual-table row by one.
    fn inc_row_one(&self, table_id: usize, row: usize);
}

impl<F: PrimeField64> VirtualTableSink for Std<F> {
    #[inline(always)]
    fn virtual_table_id(&self, table_id: usize) -> usize {
        self.get_virtual_table_id(table_id).expect("Failed to get virtual table ID")
    }

    #[inline(always)]
    fn inc_row_one(&self, table_id: usize, row: usize) {
        self.inc_virtual_row_one(table_id, row);
    }
}

/// The range-check and virtual-table surface of [`pil_std_lib::Std`] used by
/// state-machine and precompile witness generation.
///
/// The method signatures mirror `Std`'s exactly, so witness-generation call
/// sites are unchanged when a state machine is made generic over this trait.
/// In production it is instantiated with the real `Std<F>` (which monomorphizes
/// to the same code as before); tests and benchmarks can supply a lightweight
/// mock and avoid a `ProofCtx`/`SetupCtx`-backed `Std`.
///
/// This trait is not object-safe (its methods are generic); it is always used
/// via static dispatch (`<RC: RangeChecker>`), never `dyn RangeChecker`.
///
/// It extends [`VirtualTableSink`] so a single `RC: RangeChecker` bound can serve
/// both witness generation (this trait) and the collectors an instance builds
/// (which only need [`VirtualTableSink`]).
pub trait RangeChecker: VirtualTableSink {
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

    /// See [`pil_std_lib::Std::range_check_ranged`].
    fn range_check_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<i64>, muls: &[M]);

    /// See [`pil_std_lib::Std::get_virtual_table_id`].
    fn get_virtual_table_id(&self, id: usize) -> ProofmanResult<usize>;

    /// See [`pil_std_lib::Std::inc_virtual_row`].
    fn inc_virtual_row<M: RCMultiplicity>(&self, id: usize, row: M, mul: M);

    /// See [`pil_std_lib::Std::inc_virtual_row_one`].
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, id: usize, row: M);

    /// See [`pil_std_lib::Std::inc_virtual_rows_ranged`].
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<u64>, muls: &[M]);
}

impl<F: PrimeField64> RangeChecker for Std<F> {
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
    fn range_check_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<i64>, muls: &[M]) {
        Std::range_check_ranged(self, id, start, muls);
    }

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
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(
        &self,
        id: usize,
        start: Option<u64>,
        muls: &[M],
    ) {
        Std::inc_virtual_rows_ranged(self, id, start, muls);
    }
}

/// A no-op [`RangeChecker`]/[`VirtualTableSink`]. All range-check /
/// virtual-table writes are discarded.
///
/// Use it as the range-checker wherever range-checking genuinely does not
/// happen, so no `ProofCtx`/`SetupCtx`-backed `Std` is needed:
/// - unit tests and benches of state machines / precompiles,
/// - execute-only executors (no witness phase),
/// - as the type token for the RC-independent count/plan phase, where
///   `<SM<F, RC> as ComponentPlanBuilder>::counter/planner` only need *a*
///   `RangeChecker` to name the impl and never touch an instance.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopRangeChecker;

impl VirtualTableSink for NoopRangeChecker {
    fn virtual_table_id(&self, _table_id: usize) -> usize {
        0
    }
    fn inc_row_one(&self, _table_id: usize, _row: usize) {}
}

impl RangeChecker for NoopRangeChecker {
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
    fn range_check_ranged<M: RCMultiplicity>(&self, _id: usize, _start: Option<i64>, _muls: &[M]) {}
    fn get_virtual_table_id(&self, _id: usize) -> ProofmanResult<usize> {
        Ok(0)
    }
    fn inc_virtual_row<M: RCMultiplicity>(&self, _id: usize, _row: M, _mul: M) {}
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, _id: usize, _row: M) {}
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(
        &self,
        _id: usize,
        _start: Option<u64>,
        _muls: &[M],
    ) {
    }
}

/// A recording [`RangeChecker`]/[`VirtualTableSink`] for tests.
///
/// Unlike [`NoopRangeChecker`] (which discards every write), this captures what
/// a collector or witness-generator emits, so a test can assert *what* was
/// recorded:
/// - virtual-table row bumps from [`VirtualTableSink::inc_row_one`] (the FROPS
///   path collectors take) — see [`Self::recorded_rows`];
/// - range checks from `range_check` / `range_check_one` / `range_check_ranged`
///   — see [`Self::range_checks`] (the `RCValue` is recorded as `i64`, the
///   multiplicity as `u64`);
/// - virtual-row increments from `inc_virtual_row` / `inc_virtual_row_one` /
///   `inc_virtual_rows_ranged` — see [`Self::virtual_rows`].
///
/// `virtual_table_id` / `get_virtual_table_id` echo the logical id so recorded
/// entries correlate with the table the caller resolved.
#[derive(Debug, Default)]
pub struct RecordingRangeChecker {
    rows: Mutex<Vec<(usize, usize)>>,
    range_checks: Mutex<Vec<(usize, i64, u64)>>,
    virtual_rows: Mutex<Vec<(usize, u64, u64)>>,
}

impl RecordingRangeChecker {
    /// `(table_id, row)` for every `inc_row_one` call, in call order.
    pub fn recorded_rows(&self) -> Vec<(usize, usize)> {
        self.rows.lock().unwrap().clone()
    }

    /// Number of `inc_row_one` calls recorded.
    pub fn row_count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }

    /// `(range_id, value, multiplicity)` for every `range_check*` call, in order.
    pub fn range_checks(&self) -> Vec<(usize, i64, u64)> {
        self.range_checks.lock().unwrap().clone()
    }

    /// `(table_id, row, multiplicity)` for every `inc_virtual_row*` call, in order.
    pub fn virtual_rows(&self) -> Vec<(usize, u64, u64)> {
        self.virtual_rows.lock().unwrap().clone()
    }
}

impl VirtualTableSink for RecordingRangeChecker {
    fn virtual_table_id(&self, table_id: usize) -> usize {
        table_id
    }
    fn inc_row_one(&self, table_id: usize, row: usize) {
        self.rows.lock().unwrap().push((table_id, row));
    }
}

impl RangeChecker for RecordingRangeChecker {
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
    fn range_check_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<i64>, muls: &[M]) {
        let base = start.unwrap_or(0);
        let mut guard = self.range_checks.lock().unwrap();
        for (i, m) in muls.iter().enumerate() {
            guard.push((id, base + i as i64, (*m).to_u64()));
        }
    }
    fn get_virtual_table_id(&self, id: usize) -> ProofmanResult<usize> {
        Ok(id)
    }
    fn inc_virtual_row<M: RCMultiplicity>(&self, id: usize, row: M, mul: M) {
        self.virtual_rows.lock().unwrap().push((id, row.to_u64(), mul.to_u64()));
    }
    fn inc_virtual_row_one<M: RCMultiplicity>(&self, id: usize, row: M) {
        self.virtual_rows.lock().unwrap().push((id, row.to_u64(), 1));
    }
    fn inc_virtual_rows_ranged<M: RCMultiplicity>(&self, id: usize, start: Option<u64>, muls: &[M]) {
        let base = start.unwrap_or(0);
        let mut guard = self.virtual_rows.lock().unwrap();
        for (i, m) in muls.iter().enumerate() {
            guard.push((id, base + i as u64, (*m).to_u64()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_range_checker_captures_each_surface() {
        let rc = RecordingRangeChecker::default();

        // VirtualTableSink::inc_row_one — the FROPS path collectors take.
        rc.inc_row_one(5010, 7);
        rc.inc_row_one(5010, 9);
        assert_eq!(rc.recorded_rows(), vec![(5010, 7), (5010, 9)]);
        assert_eq!(rc.row_count(), 2);

        // RangeChecker range checks — value recorded as i64, multiplicity as u64.
        rc.range_check_one(3, 42u64);
        rc.range_check(3, -1i64, 4u64);
        rc.range_check_ranged(7, Some(10), &[2u32, 5u32]);
        assert_eq!(
            rc.range_checks(),
            vec![(3, 42, 1), (3, -1, 4), (7, 10, 2), (7, 11, 5)],
        );

        // RangeChecker virtual-row increments.
        rc.inc_virtual_row_one(8, 100u32);
        rc.inc_virtual_row(8, 101u64, 2u64);
        assert_eq!(rc.virtual_rows(), vec![(8, 100, 1), (8, 101, 2)]);
    }
}
