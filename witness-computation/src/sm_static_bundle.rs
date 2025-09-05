use std::{hash::Hash, sync::Arc};

use data_bus::{DataBus, DataBusTrait};
use executor::SMBundle;
use fields::PrimeField64;
use precomp_arith_eq::ArithEqManager;
use precomp_keccakf::KeccakfManager;
use precomp_sha256f::Sha256fManager;
use proofman_common::ProofCtx;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use zisk_pil::ZISK_AIRGROUP_ID;
use std::collections::HashMap;
use zisk_common::{
    BusDevice, BusDeviceMetrics, ChunkId, ComponentBuilder, Instance, InstanceCtx, Plan,
};

use executor::NestedDeviceMetricsList;

use crate::StaticDataBus;

const NUM_SM: usize = 8;
const NUM_SM_WITHOUT_MAIN: usize = NUM_SM - 1;

const _MAIN_SM_ID: usize = 0;
const MEM_SM_ID: usize = 1;
const ROM_SM_ID: usize = 2;
const BINARY_SM_ID: usize = 3;
const ARITH_SM_ID: usize = 4;
const KECCAK_SM_ID: usize = 5;
const SHA256_SM_ID: usize = 6;
const ARITH_EQ_SM_ID: usize = NUM_SM - 1;

pub enum StateMachines<F: PrimeField64> {
    RomSM(Arc<RomSM>),
    MemSM(Arc<Mem<F>>),
    BinarySM(Arc<BinarySM<F>>),
    ArithSM(Arc<ArithSM<F>>),
    KeccakfManager(Arc<KeccakfManager<F>>),
    Sha256fManager(Arc<Sha256fManager<F>>),
    ArithEqManager(Arc<ArithEqManager<F>>),
    Custom(Arc<dyn zisk_common::ComponentBuilder<F>>),
}

impl<F: PrimeField64> StateMachines<F> {
    fn build_planner(&self) -> Box<dyn zisk_common::Planner> {
        match self {
            StateMachines::RomSM(sm) => <RomSM as ComponentBuilder<F>>::build_planner(sm),
            StateMachines::MemSM(sm) => (**sm).build_planner(),
            StateMachines::BinarySM(sm) => (**sm).build_planner(),
            StateMachines::ArithSM(sm) => (**sm).build_planner(),
            StateMachines::KeccakfManager(sm) => (**sm).build_planner(),
            StateMachines::Sha256fManager(sm) => (**sm).build_planner(),
            StateMachines::ArithEqManager(sm) => (**sm).build_planner(),
            StateMachines::Custom(sm) => sm.build_planner(),
        }
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plans: &[Plan]) {
        match self {
            StateMachines::RomSM(sm) => <RomSM as ComponentBuilder<F>>::configure_instances(sm, pctx, plans),
            StateMachines::MemSM(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::BinarySM(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithSM(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::KeccakfManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::Sha256fManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithEqManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::Custom(sm) => sm.configure_instances(pctx, plans),
        }
    }
}

pub struct StaticSMBundle<F: PrimeField64> {
    process_only_operation_bus: bool,
    sm: HashMap<(usize, usize), StateMachines<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(process_only_operation_bus: bool, sm: Vec<(usize, usize, StateMachines<F>)>) -> Self {
        Self { process_only_operation_bus, sm:HashMap::from_iter(sm.into_iter().map(|(airgroup_id, air_id, sm)| ((airgroup_id, air_id), sm))) }
    }
}

impl<F: PrimeField64> SMBundle<F> for StaticSMBundle<F> {
    fn plan_sec(&self, vec_counters: NestedDeviceMetricsList) -> Vec<Vec<Plan>> {
        assert_eq!(vec_counters.len(), NUM_SM_WITHOUT_MAIN);

        let mut plans = Vec::new();
        let mut it = vec_counters.into_iter();
        for (_, sm) in self.sm.iter() {
            plans.push(sm.build_planner().plan(it.next().unwrap()));
        }

        plans
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Vec<Plan>]) {
        for (sm, plans) in self.sm.iter().zip(plannings.iter()) {
            sm.configure_instances(pctx, plans);
        }
    }

    fn build_instance(&self, idx: usize, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        let airgroup_id = ictx.plan.airgroup_id;
        let air_id = ictx.plan.air_id;

        if airgroup_id != ZISK_AIRGROUP_ID {
            panic!("Unsupported AIR group ID: {}", airgroup_id);
        }

        match air_id {
            MEM_SM_ID[0] => {

            },

        }




        assert!(idx < NUM_SM_WITHOUT_MAIN);

        match idx + 1 {
            MEM_SM_ID => self.mem_sm.build_instance(ictx),
            ROM_SM_ID => self.rom_sm.build_instance(ictx),
            BINARY_SM_ID => self.binary_sm.build_instance(ictx),
            ARITH_SM_ID => self.arith_sm.build_instance(ictx),
            KECCAK_SM_ID => self.keccakf_sm.build_instance(ictx),
            SHA256_SM_ID => self.sha256f_sm.build_instance(ictx),
            ARITH_EQ_SM_ID => self.arith_eq_sm.build_instance(ictx),
            _ => unreachable!(),
        }
    }

    fn build_data_bus_counters(
        &self,
    ) -> impl DataBusTrait<u64, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static {
        StaticDataBus::new(
            self.process_only_operation_bus,
            self.mem_sm.build_mem_counter(),
            self.binary_sm.build_binary_counter(),
            self.arith_sm.build_arith_counter(),
            self.keccakf_sm.build_keccakf_counter(),
            self.sha256f_sm.build_sha256f_counter(),
            self.arith_eq_sm.build_arith_eq_counter(),
        )
    }

    fn main_counter_idx(&self) -> Option<usize> {
        Some(0)
    }

    fn build_data_bus_collectors(
        &self,
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
        chunks_to_execute: Vec<Vec<usize>>,
    ) -> Vec<Option<DataBus<u64, Box<dyn BusDevice<u64>>>>> {
        chunks_to_execute
            .iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                if global_idxs.is_empty() {
                    return None;
                }

                let mut data_bus = DataBus::new();

                let mut used = false;
                for global_idx in global_idxs {
                    let secn_instance = secn_instances.get(global_idx).unwrap();
                    if let Some(bus_device) =
                        secn_instance.build_inputs_collector(ChunkId(chunk_id))
                    {
                        data_bus.connect_device(Some(*global_idx), Some(bus_device));

                        used = true;
                    }
                }

                if used {
                    macro_rules! add_generator {
                        ($field:ident, $type:ty) => {
                            if let Some(inputs_generator) =
                                <$type as ComponentBuilder<F>>::build_inputs_generator(
                                    &*self.$field,
                                )
                            {
                                data_bus.connect_device(None, Some(inputs_generator));
                            }
                        };
                    }

                    add_generator!(mem_sm, Mem<F>);
                    add_generator!(rom_sm, RomSM);
                    add_generator!(binary_sm, BinarySM<F>);
                    add_generator!(arith_sm, ArithSM<F>);
                    add_generator!(keccakf_sm, KeccakfManager<F>);
                    add_generator!(sha256f_sm, Sha256fManager<F>);
                    add_generator!(arith_eq_sm, ArithEqManager<F>);

                    Some(data_bus)
                } else {
                    None
                }
            })
            .collect()
    }
}
