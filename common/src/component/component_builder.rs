//! The `ComponentBuilder` trait defines a blueprint for constructing various components
//! involved in managing and monitoring operations within a state machine or execution pipeline.
//!
//! This trait provides methods to create counters, planners, input collectors, and optional
//! input generators, enabling flexible and modular integration of components.

use crate::{Instance, InstanceCtx, Plan, Planner};
use fields::PrimeField64;
use proofman_common::ProofCtx;

/// The `ComponentBuilder` trait provides an interface for building components
/// such as counters, planners, input collectors, and optional input generators.
///
/// # Type Parameters
/// * `F` - A type that implements the `PrimeField64` trait, representing the field over which
///   operations are performed.
pub trait ComponentBuilder<F: PrimeField64>: Send + Sync {
    /// Builds a planner for planning execution instances.
    ///
    /// # Returns
    /// A boxed implementation of `Planner`.
    fn build_planner(&self) -> Box<dyn Planner>;

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
