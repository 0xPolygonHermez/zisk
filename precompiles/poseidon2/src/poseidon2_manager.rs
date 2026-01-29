use std::sync::Arc;

use fields::PrimeField64;
use zisk_common::{BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner};
use zisk_core::ZiskOperationType;
use zisk_pil::Poseidon2Trace;

use crate::{Poseidon2CounterInputGen, Poseidon2Instance, Poseidon2Planner, Poseidon2SM};

/// The `Poseidon2Manager` struct represents the Poseidon2 manager,
/// which is responsible for managing the Poseidon2 state machine and its table state machine.
#[allow(dead_code)]
pub struct Poseidon2Manager<F: PrimeField64> {
    /// Poseidon2 state machine
    poseidon2_sm: Arc<Poseidon2SM<F>>,
}

impl<F: PrimeField64> Poseidon2Manager<F> {
    /// Creates a new instance of `Poseidon2Manager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Poseidon2Manager`.
    pub fn new() -> Arc<Self> {
        let poseidon2_sm = Poseidon2SM::new();

        Arc::new(Self { poseidon2_sm })
    }

    pub fn build_poseidon2_counter(&self, asm_execution: bool) -> Poseidon2CounterInputGen {
        match asm_execution {
            true => Poseidon2CounterInputGen::new(BusDeviceMode::CounterAsm),
            false => Poseidon2CounterInputGen::new(BusDeviceMode::Counter),
        }
    }

    pub fn build_poseidon2_input_generator(&self) -> Poseidon2CounterInputGen {
        Poseidon2CounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Poseidon2Manager<F> {
    /// Builds a planner to plan poseidon2-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of poseidon2s that a single poseidon2 instance can handle
        let num_available_poseidon2s = self.poseidon2_sm.num_available_poseidon2s;

        Box::new(Poseidon2Planner::new().add_instance(InstanceInfo::new(
            Poseidon2Trace::<F>::AIRGROUP_ID,
            Poseidon2Trace::<F>::AIR_ID,
            num_available_poseidon2s,
            ZiskOperationType::Poseidon2,
        )))
    }

    /// Builds an inputs data collector for poseidon2 operations.
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
            id if id == Poseidon2Trace::<F>::AIR_ID => {
                Box::new(Poseidon2Instance::new(self.poseidon2_sm.clone(), ictx))
            }
            _ => {
                panic!(
                    "Poseidon2Builder::get_instance() Unsupported air_id: {:?}",
                    ictx.plan.air_id
                )
            }
        }
    }
}
