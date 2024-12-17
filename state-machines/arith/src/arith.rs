use std::sync::Arc;

use p3_field::PrimeField;
use sm_common::{
    instance, table_instance, ComponentProvider, Instance, InstanceExpanderCtx, Metrics, Planner,
    RegularCounter,
};
use zisk_pil::{ArithRangeTableTrace, ArithTableTrace, ArithTrace};

use crate::{ArithFullSM, ArithPlanner, ArithRangeTableSM, ArithTableSM};

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

impl<F: PrimeField> ComponentProvider<F> for ArithSM {
    fn get_counter(&self) -> Box<dyn Metrics> {
        Box::new(RegularCounter::new(zisk_core::ZiskOperationType::Arith))
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(ArithPlanner::<F>::default())
    }

    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn Instance<F>> {
        match iectx.plan.air_id {
            id if id == ArithTrace::<usize>::AIR_ID => {
                instance!(
                    ArithFullInstance,
                    ArithFullSM,
                    ArithTrace::<usize>::NUM_ROWS,
                    zisk_core::ZiskOperationType::Arith
                );
                Box::new(ArithFullInstance::new(self.arith_full_sm.clone(), iectx))
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
}
