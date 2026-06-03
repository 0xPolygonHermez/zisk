//! Plan-time and witness-time builder traits.
//!
//! * [`ComponentPlanBuilder`] — static (associated functions). Used by
//!   the executor's count + plan phase; needs no constructed SM.
//! * [`ComponentBuilder`] — instance-level. Used at witness time when
//!   `build_instance` / `configure_instances` need the constructed SM.

use crate::{BusDeviceMetrics, Instance, InstanceCtx, Plan, Planner};
use fields::PrimeField64;
use proofman_common::ProofCtx;

/// Static (no-`self`) plan-time builders.
pub trait ComponentPlanBuilder<F: PrimeField64> {
    /// Concrete counter type — kept concrete so planners can
    /// `downcast_ref::<...>()` it via `Metrics::as_any()`.
    type Counter: BusDeviceMetrics + 'static;

    /// Builds the per-execution bus counter. `is_asm_emulator` is
    /// uniform; implementations that don't care just ignore it.
    fn counter(is_asm_emulator: bool) -> Self::Counter;

    /// Builds the planner. `is_asm_emulator` is uniform; only `Mem`
    /// branches on it today (selecting `DummyMemPlanner`).
    fn planner(is_asm_emulator: bool) -> Box<dyn Planner>;
}

/// Instance-level (witness-time) builders.
pub trait ComponentBuilder<F: PrimeField64>: Send + Sync {
    /// Prepares and configures instances using the provided plans before their creation.
    ///
    /// # Arguments
    /// * `pctx` - A reference to the proof context, providing shared resources for configuration.
    /// * `plannings` - A collection of plans used to configure each instance appropriately.
    #[allow(unused_variables)]
    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Plan]) {}

    /// Builds an instance with the provided context.
    ///
    /// # Arguments
    /// * `ictx` - The instance context used to create the instance.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>>;
}
