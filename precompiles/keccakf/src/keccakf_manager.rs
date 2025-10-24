use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::SetupCtx;
use zisk_common::{BusDevice, PayloadType};

use zisk_common::{
    BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner,
};
use zisk_core::ZiskOperationType;
use zisk_pil::KeccakfTrace;

use crate::{KeccakfCounterInputGen, KeccakfInstance, KeccakfPlanner, KeccakfSM};

/// The `KeccakfManager` struct represents the Keccakf manager,
/// which is responsible for managing the Keccakf state machine and its table state machine.
#[allow(dead_code)]
pub struct KeccakfManager<F: PrimeField64> {
    /// Keccakf state machine
    keccakf_sm: Arc<KeccakfSM<F>>,
}

impl<F: PrimeField64> KeccakfManager<F> {
    /// Creates a new instance of `KeccakfManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `KeccakfManager`.
    pub fn new(sctx: Arc<SetupCtx<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let keccakf_sm = KeccakfSM::new(sctx, std);

        Arc::new(Self { keccakf_sm })
    }

    pub fn build_keccakf_counter(&self) -> KeccakfCounterInputGen {
        KeccakfCounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_keccakf_input_generator(&self) -> KeccakfCounterInputGen {
        KeccakfCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for KeccakfManager<F> {
    /// Builds and returns a new counter for monitoring keccakf operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for keccakf operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(KeccakfCounterInputGen::new(BusDeviceMode::Counter)))
    }

    /// Builds a planner to plan keccakf-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of keccakfs that a single keccakf instance can handle
        let num_available_keccakfs = self.keccakf_sm.num_available_keccakfs;

        Box::new(KeccakfPlanner::new().add_instance(InstanceInfo::new(
            KeccakfTrace::<F>::AIRGROUP_ID,
            KeccakfTrace::<F>::AIR_ID,
            num_available_keccakfs,
            ZiskOperationType::Keccak,
        )))
    }

    /// Builds an inputs data collector for keccakf operations.
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
            id if id == KeccakfTrace::<F>::AIR_ID => {
                Box::new(KeccakfInstance::new(self.keccakf_sm.clone(), ictx))
            }
            _ => {
                panic!("KeccakfBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(KeccakfCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
