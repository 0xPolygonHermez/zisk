use std::sync::Arc;

use p3_field::PrimeField;
use sm_common::{
    table_instance, BusDeviceInstance, BusDeviceMetrics, ComponentBuilder, InstanceCtx,
    InstanceInfo, Planner, TableInfo,
};
use zisk_common::OPERATION_BUS_ID;
use zisk_core::ZiskOperationType;
use zisk_pil::{ArithRangeTableTrace, ArithTableTrace, ArithTrace};

use crate::{
    ArithCounter, ArithFullInstance, ArithFullSM, ArithInputGenerator, ArithPlanner,
    ArithRangeTableSM, ArithTableSM,
};

pub struct ArithSM {
    arith_full_sm: Arc<ArithFullSM>,
    arith_table_sm: Arc<ArithTableSM>,
    arith_range_table_sm: Arc<ArithRangeTableSM>,
}

impl ArithSM {
    pub fn new() -> Arc<Self> {
        let arith_table_sm = ArithTableSM::new();
        let arith_range_table_sm = ArithRangeTableSM::new();

        let arith_full_sm = ArithFullSM::new(arith_table_sm.clone(), arith_range_table_sm.clone());

        Arc::new(Self { arith_full_sm, arith_table_sm, arith_range_table_sm })
    }
}

impl<F: PrimeField> ComponentBuilder<F> for ArithSM {
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(ArithCounter::new(OPERATION_BUS_ID, vec![zisk_core::ZiskOperationType::Arith]))
    }

    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(
            ArithPlanner::new()
                .add_instance(InstanceInfo::new(
                    ArithTrace::<usize>::AIR_ID,
                    ArithTrace::<usize>::AIRGROUP_ID,
                    ArithTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::Arith,
                ))
                .add_table_instance(TableInfo::new(
                    ArithTableTrace::<usize>::AIR_ID,
                    ArithTableTrace::<usize>::AIRGROUP_ID,
                ))
                .add_table_instance(TableInfo::new(
                    ArithRangeTableTrace::<usize>::AIR_ID,
                    ArithRangeTableTrace::<usize>::AIRGROUP_ID,
                )),
        )
    }

    fn build_inputs_collector(&self, iectx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
        match iectx.plan.air_id {
            id if id == ArithTrace::<usize>::AIR_ID => {
                Box::new(ArithFullInstance::new(self.arith_full_sm.clone(), iectx))
                // instance!(
                //     ArithFullInstance,
                //     ArithFullSM,
                //     ArithTrace::<usize>::NUM_ROWS,
                //     zisk_core::ZiskOperationType::Arith
                // );
                // Box::new(ArithFullInstance::new(self.arith_full_sm.clone(), iectx))
            }
            id if id == ArithTableTrace::<usize>::AIR_ID => {
                table_instance!(ArithTableInstance, ArithTableSM, ArithTableTrace);
                Box::new(ArithTableInstance::new(self.arith_table_sm.clone(), iectx))
            }
            id if id == ArithRangeTableTrace::<usize>::AIR_ID => {
                table_instance!(ArithRangeTableInstance, ArithRangeTableSM, ArithRangeTableTrace);
                Box::new(ArithRangeTableInstance::new(self.arith_range_table_sm.clone(), iectx))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", iectx.plan.air_id),
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDeviceInstance<F>>> {
        Some(Box::new(ArithInputGenerator::default()))
    }
}
