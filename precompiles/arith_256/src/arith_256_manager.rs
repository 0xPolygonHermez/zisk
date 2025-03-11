use std::sync::Arc;

use data_bus::{BusDevice, PayloadType};
use p3_field::{PrimeField, PrimeField64};

use sm_common::{
    BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner,
};
use zisk_core::ZiskOperationType;
use zisk_pil::Arith256Trace;

use crate::{Arith256CounterInputGen, Arith256Instance, Arith256Planner, Arith256SM};

/// The `Arith256Manager` struct represents the Arith256 manager,
/// which is responsible for managing the Arith256 state machine.
#[allow(dead_code)]
pub struct Arith256Manager {
    /// Arith256 state machine
    arith256_sm: Arc<Arith256SM>,
}

impl Arith256Manager {
    /// Creates a new instance of `Arith256Manager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Arith256Manager`.
    pub fn new<F: PrimeField>() -> Arc<Self> {
        let arith256_sm = Arith256SM::new();

        Arc::new(Self { arith256_sm })
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Arith256Manager {
    /// Builds and returns a new counter for monitoring arith256 operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for arith256 operations.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(Arith256CounterInputGen::new(BusDeviceMode::Counter))
    }

    /// Builds a planner to plan arith256-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of arith256s that a single arith256 instance can handle
        let num_available_arith256s = self.arith256_sm.num_available_arith256s;

        Box::new(Arith256Planner::new().add_instance(InstanceInfo::new(
            Arith256Trace::<usize>::AIRGROUP_ID,
            Arith256Trace::<usize>::AIR_ID,
            num_available_arith256s,
            ZiskOperationType::Arith256,
        )))
    }

    /// Builds an inputs data collector for arith256 operations.
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
            id if id == Arith256Trace::<usize>::AIR_ID => {
                Box::new(Arith256Instance::new(self.arith256_sm.clone(), ictx))
            }
            _ => {
                panic!("Arith256Builder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(Arith256CounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
