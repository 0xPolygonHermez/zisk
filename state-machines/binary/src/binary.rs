use std::sync::Arc;

use crate::{
    BinaryBasicInstance, BinaryBasicSM, BinaryBasicTableInstance, BinaryBasicTableSM,
    BinaryCounter, BinaryExtensionInstance, BinaryExtensionSM, BinaryExtensionTableInstance,
    BinaryExtensionTableSM, BinaryPlanner,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use sm_common::{ComponentProvider, Instance, InstanceExpanderCtx, Metrics, Planner};
use zisk_pil::{
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BINARY_EXTENSION_TABLE_AIR_IDS, BINARY_TABLE_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

#[allow(dead_code)]
pub struct BinarySM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // Secondary State machines
    binary_basic_sm: Arc<BinaryBasicSM<F>>,
    binary_basic_table_sm: Arc<BinaryBasicTableSM<F>>,
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,
    binary_extension_table_sm: Arc<BinaryExtensionTableSM<F>>,
}

impl<F: PrimeField> BinarySM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let binary_basic_table_sm =
            BinaryBasicTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS);
        let binary_basic_sm = BinaryBasicSM::new(
            wcm.clone(),
            binary_basic_table_sm.clone(),
            ZISK_AIRGROUP_ID,
            BINARY_AIR_IDS,
        );

        let binary_extension_table_sm = BinaryExtensionTableSM::new(
            wcm.clone(),
            ZISK_AIRGROUP_ID,
            BINARY_EXTENSION_TABLE_AIR_IDS,
        );
        let binary_extension_sm = BinaryExtensionSM::new(
            wcm.clone(),
            std,
            binary_extension_table_sm.clone(),
            ZISK_AIRGROUP_ID,
            BINARY_EXTENSION_AIR_IDS,
        );

        let binary_sm = Self {
            wcm: wcm.clone(),
            binary_basic_sm,
            binary_basic_table_sm,
            binary_extension_sm,
            binary_extension_table_sm,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone(), None, None);

        binary_sm
    }
}

impl<F: PrimeField> ComponentProvider<F> for BinarySM<F> {
    fn get_counter(&self) -> Box<dyn Metrics> {
        Box::new(BinaryCounter::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::new(self.wcm.clone()))
    }

    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn Instance> {
        match iectx.plan.air_id {
            id if id == BINARY_AIR_IDS[0] => Box::new(BinaryBasicInstance::new(
                self.binary_basic_sm.clone(),
                self.wcm.clone(),
                iectx,
            )),
            id if id == BINARY_EXTENSION_AIR_IDS[0] => Box::new(BinaryExtensionInstance::new(
                self.binary_extension_sm.clone(),
                self.wcm.clone(),
                iectx,
            )),
            id if id == BINARY_TABLE_AIR_IDS[0] => Box::new(BinaryBasicTableInstance::new(
                self.wcm.clone(),
                self.binary_basic_table_sm.clone(),
                iectx,
            )),
            id if id == BINARY_EXTENSION_TABLE_AIR_IDS[0] => {
                Box::new(BinaryExtensionTableInstance::new(
                    self.wcm.clone(),
                    self.binary_extension_table_sm.clone(),
                    iectx,
                ))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", iectx.plan.air_id),
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for BinarySM<F> {}
