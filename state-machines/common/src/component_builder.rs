//! The `ComponentBuilder` trait defines a blueprint for constructing various components
//! involved in managing and monitoring operations within a state machine or execution pipeline.
//!
//! This trait provides methods to create counters, planners, input collectors, and optional
//! input generators, enabling flexible and modular integration of components.

use crate::{BusDeviceInstance, BusDeviceMetrics, InstanceCtx, Plan, Planner};
use data_bus::{BusDevice, PayloadType};
use p3_field::PrimeField;
use proofman_common::ProofCtx;

/// The `ComponentBuilder` trait provides an interface for building components
/// such as counters, planners, input collectors, and optional input generators.
///
/// # Type Parameters
/// * `F` - A type that implements the `PrimeField` trait, representing the field over which
///   operations are performed.
pub trait ComponentBuilder<F: PrimeField>: Send + Sync {
    /// Builds and returns a bus device counter for monitoring metrics.
    ///
    /// # Returns
    /// A boxed implementation of `BusDeviceMetrics`, capable of tracking bus data.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics>;

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

    /// Builds an inputs bus device data collector for capturing and processing bus device inputs.
    ///
    /// # Arguments
    /// * `ictx` - The `InstanceCtx` associated with this collector, providing contextual
    ///   information about the instance's environment.
    ///
    /// # Returns
    /// A boxed implementation of `BusDeviceInstance` specific to the given context.
    fn build_inputs_collector(&self, ictx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>>;

    /// Optionally creates an input generator for producing inputs to be sent back to the bus.
    ///
    /// # Returns
    /// An `Option` containing a boxed implementation of `BusDeviceInstance`, or `None`
    /// if the component does not support input generation.
    ///
    /// # Default Implementation
    /// Returns `None` by default, indicating no input generator is provided.
    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        None
    }
}
