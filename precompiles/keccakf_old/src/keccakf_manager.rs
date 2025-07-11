use std::sync::Arc;

use fields::PrimeField64;
use zisk_common::{BusDevice, PayloadType, OPERATION_BUS_ID};

use zisk_common::{
    table_instance_array, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx,
    InstanceInfo, Planner, TableInfo,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{KeccakfTableTrace, KeccakfTrace};

use crate::{KeccakfCounterInputGen, KeccakfInstance, KeccakfPlanner, KeccakfSM, KeccakfTableSM};

/// The `KeccakfManager` struct represents the Keccakf manager,
/// which is responsible for managing the Keccakf state machine and its table state machine.
#[allow(dead_code)]
pub struct KeccakfManager {
    /// Keccakf state machine
    keccakf_sm: Arc<KeccakfSM>,

    /// Keccakf table state machine
    keccakf_table_sm: Arc<KeccakfTableSM>,
}

impl KeccakfManager {
    /// Creates a new instance of `KeccakfManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `KeccakfManager`.
    pub fn new<F: PrimeField64>() -> Arc<Self> {
        let keccakf_table_sm = KeccakfTableSM::new::<F>();
        let keccakf_sm = KeccakfSM::new(keccakf_table_sm.clone());

        Arc::new(Self { keccakf_sm, keccakf_table_sm })
    }

    pub fn build_keccakf_counter(&self) -> KeccakfCounterInputGen {
        KeccakfCounterInputGen::new(BusDeviceMode::Counter)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for KeccakfManager {
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

        Box::new(
            KeccakfPlanner::new()
                .add_instance(InstanceInfo::new(
                    KeccakfTrace::<usize>::AIRGROUP_ID,
                    KeccakfTrace::<usize>::AIR_ID,
                    num_available_keccakfs,
                    ZiskOperationType::Keccak,
                ))
                .add_table_instance(TableInfo::new(
                    KeccakfTableTrace::<usize>::AIRGROUP_ID,
                    KeccakfTableTrace::<usize>::AIR_ID,
                )),
        )
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
            id if id == KeccakfTrace::<usize>::AIR_ID => {
                Box::new(KeccakfInstance::new(self.keccakf_sm.clone(), ictx))
            }
            id if id == KeccakfTableTrace::<usize>::AIR_ID => {
                table_instance_array!(KeccakfTableInstance, KeccakfTableSM, KeccakfTableTrace);
                Box::new(KeccakfTableInstance::new(
                    self.keccakf_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
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
