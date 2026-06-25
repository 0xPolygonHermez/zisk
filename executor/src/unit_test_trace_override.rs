//! Raw trace authoring for the unit-test executor.
//!
//! Where [`crate::unit_test_hooks`] patches individual rows *after* the SM's
//! `compute_witness` runs, a trace override *replaces* `compute_witness`
//! entirely for one AIR id: the registered closure receives the freshly
//! allocated typed trace plus the shared [`Std`] handle and writes the whole
//! trace itself. No inputs are involved — the executor plans one max-size
//! instance for the AIR from its fixed shape.

use std::any::Any;
use std::collections::HashMap;

use pil_std_lib::Std;
use zisk_common::{TraceOverrideFn, UnitTestSm};

use crate::unit_test_targets::lookup_override_by_air_id;

/// One registered override: the user's typed closure, erased to `dyn Any`,
/// plus how many instances of the AIR it authors. The matching
/// [`zisk_common::DynTraceOverride`] builder (looked up by AIR id) downcasts
/// it back to the concrete [`TraceOverrideFn`] and runs it once per instance.
pub(crate) struct OverrideSlot {
    pub(crate) override_fn: Box<dyn Any + Send + Sync>,
    pub(crate) instances: usize,
}

/// A bag of per-SM raw trace-authoring overrides, keyed by AIR id. Register
/// with [`TraceOverrideBag::register`]. When an override is registered for an
/// AIR id, the executor plans one instance for it (no inputs needed), skips
/// that SM's `compute_witness`, and builds the `AirInstance` from the closure
/// instead.
#[derive(Default)]
pub struct TraceOverrideBag {
    pub(crate) overrides: HashMap<usize /* air_id */, OverrideSlot>,
}

impl TraceOverrideBag {
    /// Create an empty override bag (no overrides registered).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a raw trace-authoring override for SM `S`, authoring
    /// `instances` instances of its AIR. The closure receives the per-AIR
    /// instance index, the freshly allocated zeroed typed trace
    /// (`&mut S::Trace`), and the shared [`Std`] handle, for emitting the
    /// range-check / lookup multiplicities a valid trace requires.
    ///
    /// Panics if `S` has no `DynTraceOverride` impl registered in the targets
    /// registry — a closure with no matching builder could never be
    /// dispatched. A later `register::<S>` replaces the earlier override
    /// (unlike hooks, overrides do not stack — there is one trace author).
    pub fn register<S>(
        &mut self,
        instances: usize,
        override_fn: impl Fn(
                usize,
                &mut S::Trace,
                &Std<fields::Goldilocks>,
            ) -> proofman_common::ProofmanResult<()>
            + Send
            + Sync
            + 'static,
    ) where
        S: UnitTestSm<fields::Goldilocks>,
    {
        let air_id = S::air_id();

        assert!(
            lookup_override_by_air_id(air_id).is_some(),
            "TraceOverrideBag::register: no DynTraceOverride registered for air_id {air_id} \
             (SM {}). Add `unit_test_sm!` for this SM and register it in \
             unit_test_targets.",
            S::name(),
        );
        assert!(instances > 0, "TraceOverrideBag::register: instances must be > 0");

        let boxed: TraceOverrideFn<fields::Goldilocks, S::Trace> = Box::new(override_fn);
        self.overrides.insert(air_id, OverrideSlot { override_fn: Box::new(boxed), instances });
    }

    /// True if no overrides are registered. Used by the executor to take the
    /// normal `compute_witness` path cleanly when no override is requested.
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }
}
