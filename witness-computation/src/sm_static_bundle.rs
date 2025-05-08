use std::sync::Arc;

use data_bus::{DataBus, DataBusTrait};
use executor::SMBundle;
use p3_field::PrimeField64;
use precomp_arith_eq::ArithEqManager;
use precomp_keccakf::KeccakfManager;
use proofman_common::ProofCtx;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use zisk_common::{
    BusDevice, BusDeviceMetrics, BusDeviceWrapper, ChunkId, ComponentBuilder, Instance,
    InstanceCtx, Plan, OPERATION_BUS_ID,
};

use executor::NestedDeviceMetricsList;

use crate::StaticDataBus;

const NUM_SM: usize = 7;
const NUM_SM_WITHOUT_MAIN: usize = NUM_SM - 1;

const _MAIN_SM_ID: usize = 0;
const MEM_SM_ID: usize = 1;
const ROM_SM_ID: usize = 2;
const BINARY_SM_ID: usize = 3;
const ARITH_SM_ID: usize = 4;
const KECCAK_SM_ID: usize = 5;
const ARITH_EQ_SM_ID: usize = 6;

pub struct StaticSMBundle<F: PrimeField64> {
    mem_sm: Arc<Mem<F>>,
    rom_sm: Arc<RomSM>,
    binary_sm: Arc<BinarySM<F>>,
    arith_sm: Arc<ArithSM>,
    keccak_sm: Arc<KeccakfManager>,
    arith_eq_sm: Arc<ArithEqManager<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    pub fn new(
        mem_sm: Arc<Mem<F>>,
        rom_sm: Arc<RomSM>,
        binary_sm: Arc<BinarySM<F>>,
        arith_sm: Arc<ArithSM>,
        keccakf_sm: Arc<KeccakfManager>,
        arith_eq_sm: Arc<ArithEqManager<F>>,
    ) -> Self {
        Self {
            // main_sm,
            mem_sm,
            rom_sm,
            binary_sm,
            arith_sm,
            keccak_sm: keccakf_sm,
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
            <KeccakfManager as ComponentBuilder<F>>::build_planner(&*self.keccak_sm)
                .plan(it.next().unwrap()),
            self.arith_eq_sm.build_planner().plan(it.next().unwrap()),
        ]
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Vec<Plan>]) {
        self.mem_sm.configure_instances(pctx, &plannings[MEM_SM_ID - 1]);
        self.rom_sm.configure_instances(pctx, &plannings[ROM_SM_ID - 1]);
        self.binary_sm.configure_instances(pctx, &plannings[BINARY_SM_ID - 1]);
        self.arith_sm.configure_instances(pctx, &plannings[ARITH_SM_ID - 1]);
        self.keccak_sm.configure_instances(pctx, &plannings[KECCAK_SM_ID - 1]);
        self.arith_eq_sm.configure_instances(pctx, &plannings[ARITH_EQ_SM_ID - 1]);
    }

    fn build_instance(&self, idx: usize, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        assert!(idx < NUM_SM_WITHOUT_MAIN);

        match idx + 1 {
            MEM_SM_ID => return self.mem_sm.build_instance(ictx),
            ROM_SM_ID => return self.rom_sm.build_instance(ictx),
            BINARY_SM_ID => return self.binary_sm.build_instance(ictx),
            ARITH_SM_ID => return self.arith_sm.build_instance(ictx),
            KECCAK_SM_ID => return self.keccak_sm.build_instance(ictx),
            ARITH_EQ_SM_ID => return self.arith_eq_sm.build_instance(ictx),
            _ => unreachable!(),
        }
    }

    fn get_data_bus_counters(
        &self,
    ) -> impl DataBusTrait<u64, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static {
        StaticDataBus::new(
            self.binary_sm.build_binary_counter(),
            self.arith_sm.build_arith_counter(),
            self.keccak_sm.build_keccakf_counter(),
            self.arith_eq_sm.build_arith_eq_counter(),
            self.mem_sm.build_mem_counter(),
        )
    }

    fn get_data_bus_collectors(
        &self,
        secn_instance: &mut Box<dyn Instance<F>>,
        chunks_to_execute: Vec<bool>,
    ) -> Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>> {
        chunks_to_execute
            .iter()
            .enumerate()
            .map(|(chunk_id, to_be_executed)| {
                if !to_be_executed {
                    return None;
                }

                let mut data_bus = DataBus::new();

                if let Some(bus_device) = secn_instance.build_inputs_collector(ChunkId(chunk_id)) {
                    let bus_device = BusDeviceWrapper::new(bus_device);
                    data_bus.connect_device(bus_device.bus_id(), bus_device);

                    macro_rules! add_generator {
                        ($field:ident, $type:ty) => {
                            if let Some(inputs_generator) =
                                <$type as ComponentBuilder<F>>::build_inputs_generator(
                                    &*self.$field,
                                )
                            {
                                data_bus.connect_device(
                                    vec![OPERATION_BUS_ID],
                                    BusDeviceWrapper::new(inputs_generator),
                                );
                            }
                        };
                    }

                    add_generator!(mem_sm, Mem<F>);
                    add_generator!(rom_sm, RomSM);
                    add_generator!(binary_sm, BinarySM<F>);
                    add_generator!(arith_sm, ArithSM);
                    add_generator!(keccak_sm, KeccakfManager);
                    add_generator!(arith_eq_sm, ArithEqManager<F>);

                    Some(data_bus)
                } else {
                    None
                }
            })
            .collect()
    }
}
