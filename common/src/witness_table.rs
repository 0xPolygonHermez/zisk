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
