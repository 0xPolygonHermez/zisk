use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{
    BusDevice, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx,
    InstanceInfo, PayloadType, Planner,
};
use zisk_core::ZiskOperationType;
use zisk_pil::Sha256fTrace;

use crate::{Sha256fCounterInputGen, Sha256fInstance, Sha256fPlanner, Sha256fSM};

/// The `Sha256fManager` struct represents the Sha256f manager,
/// which is responsible for managing the Sha256f state machine and its table state machine.
#[allow(dead_code)]
pub struct Sha256fManager<F: PrimeField64> {
    /// Sha256f state machine
    sha256f_sm: Arc<Sha256fSM<F>>,
}

impl<F: PrimeField64> Sha256fManager<F> {
    /// Creates a new instance of `Sha256fManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Sha256fManager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let sha256f_sm = Sha256fSM::new(std);

        Arc::new(Self { sha256f_sm })
    }

    pub fn build_sha256f_counter(&self) -> Sha256fCounterInputGen {
        Sha256fCounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_sha256f_input_generator(&self) -> Sha256fCounterInputGen {
        Sha256fCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Sha256fManager<F> {
    /// Builds and returns a new counter for monitoring sha256f operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for sha256f operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(Sha256fCounterInputGen::new(BusDeviceMode::Counter)))
    }

    /// Builds a planner to plan sha256f-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of sha256fs that a single sha256f instance can handle
        let num_available_sha256fs = self.sha256f_sm.num_available_sha256fs;

        Box::new(Sha256fPlanner::new().add_instance(InstanceInfo::new(
            Sha256fTrace::<F>::AIRGROUP_ID,
            Sha256fTrace::<F>::AIR_ID,
            num_available_sha256fs,
            ZiskOperationType::Sha256,
        )))
    }

    /// Builds an inputs data collector for sha256f operations.
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
            id if id == Sha256fTrace::<F>::AIR_ID => {
                Box::new(Sha256fInstance::new(self.sha256f_sm.clone(), ictx))
            }
            _ => {
                panic!("Sha256fBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(Sha256fCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
