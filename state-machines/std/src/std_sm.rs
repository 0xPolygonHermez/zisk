//! The `StdSM` module implements the PIL2 Standard Library State Machine,
//! managing interactions with the PIL2 standard library.
//!
//! Key components of this module include:
//! - The `StdSM` struct, which provides access to the PIL2 standard library.
//! - `ComponentBuilder` trait implementations for creating counters, planners, and input
//!   collectors.

use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{BusDeviceMetrics, ComponentBuilder, DummyCounter, Instance, InstanceCtx, Planner};

use crate::{StdInstance, StdPlanner};

/// The `StdSM` struct represents the PIL2 Standard Library State Machine,
/// enabling access to and management of the PIL2 standard library.
pub struct StdSM<F: PrimeField> {
    /// PIL2 standard library
    std: Arc<Std<F>>,
}

impl<F: PrimeField> StdSM<F> {
    /// Creates a new instance of the `StdSM` state machine.
    ///
    /// # Arguments
    /// * `std` - Reference to the PIL2 standard library.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `StdSM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std: std.clone() })
    }
}

impl<F: PrimeField> ComponentBuilder<F> for StdSM<F> {
    /// Builds a dummy counter for the standard library state machine.
    ///
    /// # Returns
    /// A boxed implementation of `DummyCounter`.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(DummyCounter::default())
    }

    /// Builds a planner for the PIL2 standard library operations.
    ///
    /// # Returns
    /// A boxed implementation of `StdPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(StdPlanner::new(self.std.clone()))
    }

    /// Builds an instance of the PIL2 standard library state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of `StdInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        Box::new(StdInstance::new(self.std.clone(), ictx))
    }
}
