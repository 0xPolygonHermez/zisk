use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{BusDevice, PayloadType};

use zisk_common::{
    BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner,
};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithEqTrace;

use crate::{ArithEqCounterInputGen, ArithEqInstance, ArithEqPlanner, ArithEqSM};

/// The `Arith256Manager` struct represents the ArithEq manager,
/// which is responsible for managing the ArithEq state machine.
#[allow(dead_code)]
pub struct ArithEqManager<F: PrimeField64> {
    /// ArithEq state machine
    arith_eq_sm: Arc<ArithEqSM<F>>,
}

impl<F: PrimeField64> ArithEqManager<F> {
    /// Creates a new instance of `ArithEqManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithEqManager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let arith_eq_sm = ArithEqSM::new(std);

        Arc::new(Self { arith_eq_sm })
    }

    pub fn build_arith_eq_counter(&self) -> ArithEqCounterInputGen {
        ArithEqCounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_arith_eq_input_generator(&self) -> ArithEqCounterInputGen {
        ArithEqCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for ArithEqManager<F> {
    /// Builds and returns a new counter for monitoring arith256 operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for arith256 operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(ArithEqCounterInputGen::new(BusDeviceMode::Counter)))
    }

    /// Builds a planner to plan arith256-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of arith256s that a single arith256 instance can handle
        let num_available_ops = self.arith_eq_sm.num_available_ops;

        Box::new(ArithEqPlanner::new().add_instance(InstanceInfo::new(
            ArithEqTrace::<F>::AIRGROUP_ID,
            ArithEqTrace::<F>::AIR_ID,
            num_available_ops,
            ZiskOperationType::ArithEq,
        )))
    }

    /// Builds an inputs data collector for arith_eq operations.
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
            id if id == ArithEqTrace::<F>::AIR_ID => {
                Box::new(ArithEqInstance::new(self.arith_eq_sm.clone(), ictx))
            }
            _ => {
                panic!("ArithEqBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(ArithEqCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
