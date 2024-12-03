use std::sync::Arc;

use crate::{
    BinaryBasicSM, BinaryBasicTableSM, BinaryCounter, BinaryExtensionSM, BinaryExtensionTableSM,
    BinaryInstance, BinaryPlanner,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use sm_common::{ComponentProvider, Instance, InstanceExpanderCtx, Metrics, Planner};
use zisk_core::ZiskRequiredOperation;
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
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,
}

impl<F: PrimeField> BinarySM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let binary_basic_table_sm =
            BinaryBasicTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS);
        let binary_basic_sm = BinaryBasicSM::new(
            wcm.clone(),
            binary_basic_table_sm,
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
            binary_extension_table_sm,
            ZISK_AIRGROUP_ID,
            BINARY_EXTENSION_AIR_IDS,
        );

        let binary_sm = Self { wcm: wcm.clone(), binary_basic_sm, binary_extension_sm };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone(), None, None);

        binary_sm
    }

    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredOperation>,
        is_extension: bool,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        if !is_extension {
            self.binary_basic_sm.prove_instance(operations, prover_buffer, offset);
        } else {
            self.binary_extension_sm.prove_instance(operations, prover_buffer, offset);
        }
    }
}

impl<F: PrimeField> ComponentProvider<F> for BinarySM<F> {
    fn get_counter(&self) -> Box<dyn Metrics> {
        Box::new(BinaryCounter::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::new(self.wcm.clone()))
    }

    fn get_instance(self: Arc<Self>, iectx: InstanceExpanderCtx<F>) -> Box<dyn Instance> {
        Box::new(BinaryInstance::new(self.clone(), self.wcm.clone(), iectx))
    }
}

impl<F: PrimeField> WitnessComponent<F> for BinarySM<F> {}
