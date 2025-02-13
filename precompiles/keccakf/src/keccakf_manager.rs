use std::sync::Arc;

use data_bus::{BusDevice, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID};
use p3_field::{PrimeField, PrimeField64};

use sm_common::{
    table_instance, BusDeviceMetrics, ComponentBuilder, Instance, InstanceCtx, InstanceInfo,
    Planner, RegularPlanner, TableInfo,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{KeccakfTableTrace, KeccakfTrace};

use crate::{KeccakfCounter, KeccakfInputGenerator, KeccakfInstance, KeccakfSM, KeccakfTableSM};

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
    pub fn new<F: PrimeField>() -> Arc<Self> {
        let keccakf_table_sm = KeccakfTableSM::new::<F>();
        let keccakf_sm = KeccakfSM::new(keccakf_table_sm.clone());

        Arc::new(Self { keccakf_sm, keccakf_table_sm })
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for KeccakfManager {
    /// Builds and returns a new counter for monitoring keccakf operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for keccakf operations.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(KeccakfCounter::new(OPERATION_BUS_ID, vec![ZiskOperationType::Keccak]))
    }

    /// Builds a planner to plan keccakf-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // TODO: Read the slot size from file instead of hardcoding it.
        let slot_size = 155286;
        let num_available_slots = (KeccakfTrace::<usize>::NUM_ROWS - 1) / slot_size;
        let num_available_keccakfs = KeccakfSM::NUM_KECCAKF_PER_SLOT * num_available_slots;

        Box::new(
            RegularPlanner::new()
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
                Box::new(KeccakfInstance::new(self.keccakf_sm.clone(), ictx, OPERATION_BUS_ID))
            }
            id if id == KeccakfTableTrace::<usize>::AIR_ID => {
                table_instance!(KeccakfTableInstance, KeccakfTableSM, KeccakfTableTrace);
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
        Some(Box::new(KeccakfInputGenerator::new(MEM_BUS_ID)))
    }
}
