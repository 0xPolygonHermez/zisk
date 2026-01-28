//! The `ArithSM` module implements the Arithmetic State Machine,
//! coordinating sub-state machines to handle various arithmetic operations seamlessly.
//!
//! Key components of this module include:
//! - The `ArithSM` struct, encapsulating the full, table, and range table state machines.
//! - `ComponentBuilder` trait implementations for creating counters, planners, input collectors,
//!   and input generators specific to arithmetic computations.

use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithTrace;

use crate::{ArithCounterInputGen, ArithFullInstance, ArithFullSM, ArithPlanner};

/// The `ArithSM` struct represents the Arithmetic State Machine, which
/// is a proxy machine to manage state machines involved in arithmetic operations.
pub struct ArithSM<F: PrimeField64> {
    /// Arith Full state machine
    arith_full_sm: Arc<ArithFullSM<F>>,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> ArithSM<F> {
    /// Creates a new instance of the `ArithSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithSM` containing initialized sub-state machines.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let arith_full_sm = ArithFullSM::new(std.clone());

        Arc::new(Self { arith_full_sm, std })
    }

    pub fn build_arith_counter(&self) -> ArithCounterInputGen {
        ArithCounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_arith_input_generator(&self) -> ArithCounterInputGen {
        ArithCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for ArithSM<F> {
    /// Builds a planner to plan arithmetic-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `ArithPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(ArithPlanner::new().add_instance(InstanceInfo::new(
            ArithTrace::<F>::AIRGROUP_ID,
            ArithTrace::<F>::AIR_ID,
            ArithTrace::<F>::NUM_ROWS,
            ZiskOperationType::Arith,
        )))
    }

    /// Builds an instance of the Arithmetic state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of `StdInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            ArithTrace::<F>::AIR_ID => {
                Box::new(ArithFullInstance::new(self.arith_full_sm.clone(), ictx, self.std.clone()))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
