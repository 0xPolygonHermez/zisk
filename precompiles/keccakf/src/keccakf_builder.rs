use std::sync::Arc;

use data_bus::OPERATION_BUS_ID;
use p3_field::{PrimeField, PrimeField64};

use sm_common::{
    table_instance, BusDeviceInstance, BusDeviceMetrics, ComponentBuilder, InstanceCtx,
    InstanceInfo, Planner, RegularCounters, RegularPlanner, TableInfo,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{KeccakfTableTrace, KeccakfTrace};

use crate::{KeccakfInstance, KeccakfSM, KeccakfTableSM};

/// The `KeccakfBuilder` struct represents the Keccakf State Machine builder.
#[allow(dead_code)]
pub struct KeccakfBuilder {
    /// Keccakf state machine
    keccakf_sm: Arc<KeccakfSM>,

    /// Keccakf table state machine
    keccakf_table_sm: Arc<KeccakfTableSM>,
}

impl KeccakfBuilder {
    /// Creates a new instance of the `KeccakfBuilder` state machine.
    ///
    /// # Arguments
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `KeccakfBuilder`.
    pub fn new<F: PrimeField>() -> Arc<Self> {
        let keccakf_table_sm = KeccakfTableSM::new::<F>();
        let keccakf_sm = KeccakfSM::new(keccakf_table_sm.clone());

        Arc::new(Self { keccakf_sm, keccakf_table_sm })
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for KeccakfBuilder {
    /// Builds and returns a new counter for monitoring keccakf operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for keccakf operations.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(RegularCounters::new(OPERATION_BUS_ID, vec![ZiskOperationType::Keccak]))
    }

    /// Builds a planner to plan keccakf-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(
            RegularPlanner::new()
                .add_instance(InstanceInfo::new(
                    KeccakfTrace::<usize>::AIR_ID,
                    KeccakfTrace::<usize>::AIRGROUP_ID,
                    KeccakfTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::Keccak,
                ))
                .add_table_instance(TableInfo::new(
                    KeccakfTableTrace::<usize>::AIR_ID,
                    KeccakfTableTrace::<usize>::AIRGROUP_ID,
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
    fn build_inputs_collector(&self, ictx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
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
}
