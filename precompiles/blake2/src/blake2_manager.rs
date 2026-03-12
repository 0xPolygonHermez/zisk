use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner};
use zisk_core::ZiskOperationType;
use zisk_pil::Blake2brTrace;

use crate::{Blake2CounterInputGen, Blake2Instance, Blake2Planner, Blake2SM};

/// The `Blake2Manager` struct represents the Blake2 manager,
/// which is responsible for managing the Blake2 state machine and its table state machine.
#[allow(dead_code)]
pub struct Blake2Manager<F: PrimeField64> {
    /// Blake2 state machine
    blake2_sm: Arc<Blake2SM<F>>,
}

impl<F: PrimeField64> Blake2Manager<F> {
    /// Creates a new instance of `Blake2Manager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Blake2Manager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let blake2_sm = Blake2SM::new(std);

        Arc::new(Self { blake2_sm })
    }

    pub fn build_blake2_counter(&self, asm_execution: bool) -> Blake2CounterInputGen {
        match asm_execution {
            true => Blake2CounterInputGen::new(BusDeviceMode::CounterAsm),
            false => Blake2CounterInputGen::new(BusDeviceMode::Counter),
        }
    }

    pub fn build_blake2_input_generator(&self) -> Blake2CounterInputGen {
        Blake2CounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Blake2Manager<F> {
    /// Builds a planner to plan blake2-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of blake2s that a single blake2 instance can handle
        let num_available_blake2s = self.blake2_sm.num_available_blake2s;

        Box::new(Blake2Planner::new().add_instance(InstanceInfo::new(
            Blake2brTrace::<F>::AIRGROUP_ID,
            Blake2brTrace::<F>::AIR_ID,
            num_available_blake2s,
            ZiskOperationType::Blake2,
        )))
    }

    /// Builds an inputs data collector for blake2 operations.
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
            id if id == Blake2brTrace::<F>::AIR_ID => {
                Box::new(Blake2Instance::new(self.blake2_sm.clone(), ictx))
            }
            _ => {
                panic!("Blake2Manager::build_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }
}
