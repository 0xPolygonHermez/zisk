//! [`InstanceFactory`] — pure factory for main and secondary instances.
//!
//! Extracted from [`crate::InstanceRegistry`] so the *construction*
//! responsibility (build a `MainInstance<F>` or `Box<dyn Instance<F>>`
//! from a `(global_id, Plan)`) is named separately from the *lifecycle*
//! responsibility (insert into the executor's instance maps,
//! `set_witness_ready` on the proof context, configure checkpoints,
//! etc.).
//!
//! The factory is stateless apart from the `Arc<StaticSMBundle<F>>`
//! that backs it. Every call is referentially transparent: given the
//! same `(plan, global_id)`, the constructed instance is determined
//! by the bundle's static SM registry. No state lookup, no
//! `swap_remove`, no `&mut` of caller state.
//!
//! See `.claude/executor_refactor_plan.md` step 3.1 for context.

use std::sync::Arc;

use crate::error::ExecutorResult;
use fields::PrimeField64;
use sm_main::MainInstance;
use zisk_common::{Instance, InstanceCtx, Plan};

use crate::StaticSMBundle;

/// Builds [`MainInstance<F>`] and [`Instance<F>`] trait objects from
/// plans + global ids. Holds an `Arc<StaticSMBundle<F>>` for the
/// underlying construction calls.
pub struct InstanceFactory<F: PrimeField64> {
    sm_bundle: Arc<StaticSMBundle<F>>,
}

impl<F: PrimeField64> InstanceFactory<F> {
    /// Wraps the shared SM bundle.
    pub fn new(sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self { sm_bundle }
    }

    /// Build the main-SM instance for `(plan, global_id)`.
    ///
    /// Main instances always succeed — `MainInstance::new` takes the
    /// plan + the shared `Std` reference and returns an instance
    /// directly.
    pub fn new_main(&self, plan: Plan, global_id: usize) -> MainInstance<F> {
        MainInstance::new(InstanceCtx::new(global_id, plan), self.sm_bundle.get_std())
    }

    /// Build the secondary-SM instance for `(plan, global_id)`.
    ///
    /// Routes through [`StaticSMBundle::build_instance`], which
    /// dispatches on the plan's `(airgroup_id, air_id)` to the
    /// matching built-in or precompile builder. Returns the boxed
    /// trait object.
    pub fn new_secn(&self, plan: Plan, global_id: usize) -> ExecutorResult<Box<dyn Instance<F>>> {
        self.sm_bundle.build_instance(InstanceCtx::new(global_id, plan))
    }

    /// Returns a reference to the wrapped SM bundle. Provided so
    /// `InstanceRegistry` (and other consumers that hold an
    /// `InstanceFactory`) can still expose `sm_bundle()` without
    /// duplicating the `Arc`.
    pub fn sm_bundle(&self) -> &StaticSMBundle<F> {
        &self.sm_bundle
    }
}

// Note: unit-testing `new_main` / `new_secn` requires constructing a
// real `Arc<StaticSMBundle<F>>`, which in turn requires
// `Std::new(ProofCtx, SetupCtx, ...)`. That setup is integration-test
// territory; the behavior under unit test would be a one-line
// delegate to `StaticSMBundle::build_instance`, which is itself
// exercised by the existing integration suite. We document the seam
// and rely on integration coverage rather than mocking a complex
// `Std` dependency.
