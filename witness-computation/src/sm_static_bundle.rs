use std::sync::Arc;

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
const ARITH_EQ_SM_ID: usize = 7;

pub struct StaticSMBundle<F: PrimeField64> {
    process_only_operation_bus: bool,
    mem_sm: Arc<Mem<F>>,
    rom_sm: Arc<RomSM>,
    binary_sm: Arc<BinarySM<F>>,
    arith_sm: Arc<ArithSM>,
    keccakf_sm: Arc<KeccakfManager<F>>,
    sha256f_sm: Arc<Sha256fManager<F>>,
    arith_eq_sm: Arc<ArithEqManager<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        process_only_operation_bus: bool,
        mem_sm: Arc<Mem<F>>,
        rom_sm: Arc<RomSM>,
        binary_sm: Arc<BinarySM<F>>,
        arith_sm: Arc<ArithSM>,
        keccakf_sm: Arc<KeccakfManager<F>>,
        sha256f_sm: Arc<Sha256fManager<F>>,
        arith_eq_sm: Arc<ArithEqManager<F>>,
    ) -> Self {
        Self {
            process_only_operation_bus,
            // main_sm,
            mem_sm,
            rom_sm,
            binary_sm,
            arith_sm,
            keccakf_sm,
            sha256f_sm,
            arith_eq_sm,
        }
    }
}

impl<F: PrimeField64> SMBundle<F> for StaticSMBundle<F> {
    fn plan_sec(&self, vec_counters: NestedDeviceMetricsList) -> Vec<Vec<Plan>> {
        assert_eq!(vec_counters.len(), NUM_SM_WITHOUT_MAIN);

        let mut it = vec_counters.into_iter();

        vec![
            self.mem_sm.build_planner().plan(it.next().unwrap()),
            <RomSM as ComponentBuilder<F>>::build_planner(&*self.rom_sm).plan(it.next().unwrap()),
            self.binary_sm.build_planner().plan(it.next().unwrap()),
            <ArithSM as ComponentBuilder<F>>::build_planner(&*self.arith_sm)
                .plan(it.next().unwrap()),
            self.keccakf_sm.build_planner().plan(it.next().unwrap()),
            self.sha256f_sm.build_planner().plan(it.next().unwrap()),
            self.arith_eq_sm.build_planner().plan(it.next().unwrap()),
        ]
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Vec<Plan>]) {
        self.mem_sm.configure_instances(pctx, &plannings[MEM_SM_ID - 1]);
        self.rom_sm.configure_instances(pctx, &plannings[ROM_SM_ID - 1]);
        self.binary_sm.configure_instances(pctx, &plannings[BINARY_SM_ID - 1]);
        self.arith_sm.configure_instances(pctx, &plannings[ARITH_SM_ID - 1]);
        self.keccakf_sm.configure_instances(pctx, &plannings[KECCAK_SM_ID - 1]);
        self.sha256f_sm.configure_instances(pctx, &plannings[SHA256_SM_ID - 1]);
        self.arith_eq_sm.configure_instances(pctx, &plannings[ARITH_EQ_SM_ID - 1]);
    }

    fn build_instance(&self, idx: usize, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
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
                    add_generator!(arith_sm, ArithSM);
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
