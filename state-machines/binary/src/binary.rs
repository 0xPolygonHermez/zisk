use std::sync::Arc;

use crate::{
    BinaryBasicInstance, BinaryBasicSM, BinaryBasicTableSM, BinaryExtensionInstance,
    BinaryExtensionSM, BinaryExtensionTableSM,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{
    table_instance, BusDeviceInstance, BusDeviceMetrics, ComponentBuilder, InstanceCtx,
    InstanceInfo, Planner, RegularCounters, RegularPlanner, TableInfo,
};
use zisk_common::OPERATION_BUS_ID;
use zisk_core::ZiskOperationType;
use zisk_pil::{BinaryExtensionTableTrace, BinaryExtensionTrace, BinaryTableTrace, BinaryTrace};

/// Binary state machine
#[allow(dead_code)]
pub struct BinarySM<F: PrimeField> {
    /// Binary Basic state machine
    binary_basic_sm: Arc<BinaryBasicSM>,

    /// Binary Basic Table state machine
    binary_basic_table_sm: Arc<BinaryBasicTableSM>,

    /// Binary Extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Binary Extension Table state machine
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
}

impl<F: PrimeField> BinarySM<F> {
    /// Creates a new BinarySM instance
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let binary_basic_table_sm = BinaryBasicTableSM::new::<F>();
        let binary_basic_sm = BinaryBasicSM::new(binary_basic_table_sm.clone());

        let binary_extension_table_sm = BinaryExtensionTableSM::new::<F>();
        let binary_extension_sm = BinaryExtensionSM::new(std, binary_extension_table_sm.clone());

        Arc::new(Self {
            binary_basic_sm,
            binary_basic_table_sm,
            binary_extension_sm,
            binary_extension_table_sm,
        })
    }
}

impl<F: PrimeField> ComponentBuilder<F> for BinarySM<F> {
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(RegularCounters::new(
            OPERATION_BUS_ID,
            vec![ZiskOperationType::Binary, ZiskOperationType::BinaryE],
        ))
    }

    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(
            RegularPlanner::new()
                .add_instance(InstanceInfo::new(
                    BinaryTrace::<usize>::AIR_ID,
                    BinaryTrace::<usize>::AIRGROUP_ID,
                    BinaryTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::Binary,
                ))
                .add_instance(InstanceInfo::new(
                    BinaryExtensionTrace::<usize>::AIR_ID,
                    BinaryExtensionTrace::<usize>::AIRGROUP_ID,
                    BinaryExtensionTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::BinaryE,
                ))
                .add_table_instance(TableInfo::new(
                    BinaryTableTrace::<usize>::AIR_ID,
                    BinaryTableTrace::<usize>::AIRGROUP_ID,
                ))
                .add_table_instance(TableInfo::new(
                    BinaryExtensionTableTrace::<usize>::AIR_ID,
                    BinaryExtensionTableTrace::<usize>::AIRGROUP_ID,
                )),
        )
    }

    fn build_inputs_collector(&self, ictx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
        match ictx.plan.air_id {
            id if id == BinaryTrace::<usize>::AIR_ID => {
                Box::new(BinaryBasicInstance::new(self.binary_basic_sm.clone(), ictx))
            }
            id if id == BinaryExtensionTrace::<usize>::AIR_ID => {
                Box::new(BinaryExtensionInstance::new(self.binary_extension_sm.clone(), ictx))
            }
            id if id == BinaryTableTrace::<usize>::AIR_ID => {
                table_instance!(BinaryBasicTableInstance, BinaryBasicTableSM, BinaryTableTrace);
                Box::new(BinaryBasicTableInstance::new(self.binary_basic_table_sm.clone(), ictx))
            }
            id if id == BinaryExtensionTableTrace::<usize>::AIR_ID => {
                table_instance!(
                    BinaryExtensionTableInstance,
                    BinaryExtensionTableSM,
                    BinaryExtensionTableTrace
                );
                Box::new(BinaryExtensionTableInstance::new(
                    self.binary_extension_table_sm.clone(),
                    ictx,
                ))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
