use std::sync::Arc;

use p3_field::PrimeField;
use proofman::WitnessManager;
use sm_common::{
    ComponentProvider, Instance, InstanceExpanderCtx, Metrics, Planner, RegularCounter,
};
use zisk_pil::{ARITH_AIR_IDS, ARITH_RANGE_TABLE_AIR_IDS, ARITH_TABLE_AIR_IDS};

use crate::{
    ArithFullInstance, ArithFullSM, ArithPlanner, ArithRangeTableInstance, ArithRangeTableSM,
    ArithTableInstance, ArithTableSM,
};

pub struct ArithSM<F> {
    wcm: Arc<WitnessManager<F>>,
    arith_full_sm: Arc<ArithFullSM>,
    arith_table_sm: Arc<ArithTableSM>,
    arith_range_table_sm: Arc<ArithRangeTableSM>,
}

impl<F: PrimeField> ArithSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let arith_table_sm = ArithTableSM::new::<F>();
        let arith_range_table_sm = ArithRangeTableSM::new::<F>();

        let arith_full_sm = ArithFullSM::new(arith_table_sm.clone(), arith_range_table_sm.clone());

        let arith_sm = Self { wcm, arith_full_sm, arith_table_sm, arith_range_table_sm };

        Arc::new(arith_sm)
    }
}

impl<F: PrimeField> ComponentProvider<F> for ArithSM<F> {
    fn get_counter(&self) -> Box<dyn Metrics> {
        Box::new(RegularCounter::new(zisk_core::ZiskOperationType::Arith))
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(ArithPlanner::<F>::new())
    }

    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn Instance<F>> {
        match iectx.plan.air_id {
            id if id == ARITH_AIR_IDS[0] => {
                Box::new(ArithFullInstance::new(self.arith_full_sm.clone(), iectx))
            }
            id if id == ARITH_TABLE_AIR_IDS[0] => Box::new(ArithTableInstance::new(
                self.wcm.clone(),
                self.arith_table_sm.clone(),
                iectx,
            )),
            id if id == ARITH_RANGE_TABLE_AIR_IDS[0] => Box::new(ArithRangeTableInstance::new(
                self.wcm.clone(),
                self.arith_range_table_sm.clone(),
                iectx,
            )),
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", iectx.plan.air_id),
        }
    }
}
