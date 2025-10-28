use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{BusDevice, PayloadType};

use zisk_common::{
    BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner,
};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithEq384Trace;

use crate::{ArithEq384CounterInputGen, ArithEq384Instance, ArithEq384Planner, ArithEq384SM};

/// The `Arith256Manager` struct represents the ArithEq384 manager,
/// which is responsible for managing the ArithEq384 state machine.
#[allow(dead_code)]
pub struct ArithEq384Manager<F: PrimeField64> {
    /// ArithEq384 state machine
    arith_eq_384_sm: Arc<ArithEq384SM<F>>,
}

impl<F: PrimeField64> ArithEq384Manager<F> {
    /// Creates a new instance of `ArithEq384Manager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithEq384Manager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let arith_eq_384_sm = ArithEq384SM::new(std);

        Arc::new(Self { arith_eq_384_sm })
    }

    pub fn build_arith_eq_384_counter(&self) -> ArithEq384CounterInputGen {
        ArithEq384CounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_arith_eq_384_input_generator(&self) -> ArithEq384CounterInputGen {
        ArithEq384CounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for ArithEq384Manager<F> {
    /// Builds and returns a new counter for monitoring arith256 operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for arith256 operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(ArithEq384CounterInputGen::new(BusDeviceMode::Counter)))
    }

    /// Builds a planner to plan arith256-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of arith256s that a single arith256 instance can handle
        let num_available_ops = self.arith_eq_384_sm.num_available_ops;

        Box::new(ArithEq384Planner::new().add_instance(InstanceInfo::new(
            ArithEq384Trace::<F>::AIRGROUP_ID,
            ArithEq384Trace::<F>::AIR_ID,
            num_available_ops,
            ZiskOperationType::ArithEq384,
        )))
    }

    /// Builds an inputs data collector for arith_eq_384 operations.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///   configurations.
    ///
    /// # Returns
    /// A boxed implementation of `BusDeviceInstance` specific to the requested `air_id` instance.
    ///
    /// # Panics
    /// Panics if the provided `air_id` is not supported.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            id if id == ArithEq384Trace::<F>::AIR_ID => {
                Box::new(ArithEq384Instance::new(self.arith_eq_384_sm.clone(), ictx))
            }
            _ => {
                panic!(
                    "ArithEq384Builder::get_instance() Unsupported air_id: {:?}",
                    ictx.plan.air_id
                )
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(ArithEq384CounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
