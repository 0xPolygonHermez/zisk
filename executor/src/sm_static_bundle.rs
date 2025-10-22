use std::sync::Arc;

use crate::NestedDeviceMetricsList;
use crate::StaticDataBusCollect;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use precomp_arith_eq::{ArithEqInstance, ArithEqManager};
use precomp_arith_eq_384::ArithEq384Instance;
use precomp_arith_eq_384::ArithEq384Manager;
use precomp_big_int::{Add256Instance, Add256Manager};
use precomp_keccakf::{KeccakfInstance, KeccakfManager};
use precomp_sha256f::{Sha256fInstance, Sha256fManager};
use proofman_common::ProofCtx;
use sm_arith::{ArithFullInstance, ArithSM};
use sm_binary::{BinaryAddInstance, BinaryBasicInstance, BinaryExtensionInstance, BinarySM};
use sm_mem::{
    Mem, MemAlignByteInstance, MemAlignInstance, MemAlignReadByteInstance,
    MemAlignWriteByteInstance, MemModuleInstance,
};
use sm_rom::{RomInstance, RomSM};
use std::collections::HashMap;
use zisk_common::{BusDeviceMetrics, ChunkId, ComponentBuilder, Instance, InstanceCtx, Plan};
use zisk_pil::ADD_256_AIR_IDS;
use zisk_pil::{
    ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS, BINARY_AIR_IDS,
    BINARY_EXTENSION_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS,
    MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS, MEM_ALIGN_WRITE_BYTE_AIR_IDS, ROM_AIR_IDS,
    ROM_DATA_AIR_IDS, SHA_256_F_AIR_IDS, ZISK_AIRGROUP_ID,
};

use crate::StaticDataBus;
use rayon::prelude::*;

type SMAirType = Vec<(usize, usize)>;
pub type SMType<F> = (SMAirType, StateMachines<F>);

pub enum StateMachines<F: PrimeField64> {
    RomSM(Arc<RomSM>),
    MemSM(Arc<Mem<F>>),
    BinarySM(Arc<BinarySM<F>>),
    ArithSM(Arc<ArithSM<F>>),
    KeccakfManager(Arc<KeccakfManager<F>>),
    Sha256fManager(Arc<Sha256fManager<F>>),
    ArithEqManager(Arc<ArithEqManager<F>>),
    ArithEq384Manager(Arc<ArithEq384Manager<F>>),
    Add256Manager(Arc<Add256Manager<F>>),
}

impl<F: PrimeField64> StateMachines<F> {
    pub fn type_id(&self) -> usize {
        match self {
            StateMachines::RomSM(_) => 0,
            StateMachines::MemSM(_) => 1,
            StateMachines::BinarySM(_) => 2,
            StateMachines::ArithSM(_) => 3,
            StateMachines::KeccakfManager(_) => 4,
            StateMachines::Sha256fManager(_) => 5,
            StateMachines::ArithEqManager(_) => 6,
            StateMachines::ArithEq384Manager(_) => 7,
            StateMachines::Add256Manager(_) => 8,
        }
    }

    fn build_planner(&self, process_only_operation_bus: bool) -> Box<dyn zisk_common::Planner> {
        match self {
            StateMachines::RomSM(sm) => <RomSM as ComponentBuilder<F>>::build_planner(sm),
            StateMachines::MemSM(sm) => {
                if process_only_operation_bus {
                    (**sm).build_dummy_planner()
                } else {
                    (**sm).build_planner()
                }
            }
            StateMachines::BinarySM(sm) => (**sm).build_planner(),
            StateMachines::ArithSM(sm) => (**sm).build_planner(),
            StateMachines::KeccakfManager(sm) => (**sm).build_planner(),
            StateMachines::Sha256fManager(sm) => (**sm).build_planner(),
            StateMachines::ArithEqManager(sm) => (**sm).build_planner(),
            StateMachines::ArithEq384Manager(sm) => (**sm).build_planner(),
            StateMachines::Add256Manager(sm) => (**sm).build_planner(),
        }
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plans: &[Plan]) {
        match self {
            StateMachines::RomSM(sm) => {
                <RomSM as ComponentBuilder<F>>::configure_instances(sm, pctx, plans)
            }
            StateMachines::MemSM(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::BinarySM(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithSM(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::KeccakfManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::Sha256fManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithEqManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithEq384Manager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::Add256Manager(sm) => (**sm).configure_instances(pctx, plans),
        }
    }

    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match self {
            StateMachines::RomSM(sm) => <RomSM as ComponentBuilder<F>>::build_instance(sm, ictx),
            StateMachines::MemSM(sm) => (**sm).build_instance(ictx),
            StateMachines::BinarySM(sm) => (**sm).build_instance(ictx),
            StateMachines::ArithSM(sm) => (**sm).build_instance(ictx),
            StateMachines::KeccakfManager(sm) => (**sm).build_instance(ictx),
            StateMachines::Sha256fManager(sm) => (**sm).build_instance(ictx),
            StateMachines::ArithEqManager(sm) => (**sm).build_instance(ictx),
            StateMachines::ArithEq384Manager(sm) => (**sm).build_instance(ictx),
            StateMachines::Add256Manager(sm) => (**sm).build_instance(ictx),
        }
    }
}

pub struct StaticSMBundle<F: PrimeField64> {
    process_only_operation_bus: bool,
    sm: HashMap<usize, SMType<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(process_only_operation_bus: bool, sm: Vec<(SMAirType, StateMachines<F>)>) -> Self {
        Self {
            process_only_operation_bus,
            sm: HashMap::from_iter(
                sm.into_iter().map(|(air_ids, sm)| (sm.type_id(), (air_ids, sm))),
            ),
        }
    }

    pub fn get_mem_sm_id(&self) -> usize {
        1
    }

    pub fn plan_sec(
        &self,
        vec_counters: &mut NestedDeviceMetricsList,
    ) -> HashMap<usize, Vec<Plan>> {
        let mut plans = HashMap::new();

        // Iterate over vec_counters hashmap
        for (id, (_, sm)) in self.sm.iter() {
            if let Some(counters) = vec_counters.remove(id) {
                plans.insert(*id, sm.build_planner(self.process_only_operation_bus).plan(counters));
            }
        }

        plans
    }

    pub fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &HashMap<usize, Vec<Plan>>) {
        for (id, (_, sm)) in self.sm.iter() {
            if let Some(plans) = plannings.get(id) {
                sm.configure_instances(pctx, plans);
            }
        }
    }

    pub fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        let airgroup_id = ictx.plan.airgroup_id;
        let air_id = ictx.plan.air_id;

        if airgroup_id != ZISK_AIRGROUP_ID {
            panic!("Unsupported AIR group ID: {}", airgroup_id);
        }

        let (_, sm) = self
            .sm
            .values()
            .find(|(air_ids, _)| air_ids.contains(&(airgroup_id, air_id)))
            .unwrap_or_else(|| {
                panic!("State machine not found for pair ({}, {})", airgroup_id, air_id)
            });

        sm.build_instance(ictx)
    }

    pub fn build_data_bus_counters(
        &self,
    ) -> impl DataBusTrait<u64, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static {
        // Extract counters from each state machine type
        let mut mem_counter = None;
        let mut binary_counter = None;
        let mut arith_counter = None;
        let mut keccakf_counter = None;
        let mut sha256f_counter = None;
        let mut arith_eq_counter = None;
        let mut arith_eq_384_counter = None;
        let mut add256_counter = None;

        for (_, sm) in self.sm.values() {
            match sm {
                StateMachines::MemSM(mem_sm) => {
                    if !self.process_only_operation_bus {
                        mem_counter = Some((sm.type_id(), Some(mem_sm.build_mem_counter())));
                    } else {
                        mem_counter = Some((sm.type_id(), None));
                    }
                }
                StateMachines::BinarySM(binary_sm) => {
                    binary_counter = Some((sm.type_id(), binary_sm.build_binary_counter()));
                }
                StateMachines::ArithSM(arith_sm) => {
                    arith_counter = Some((sm.type_id(), arith_sm.build_arith_counter()));
                }
                StateMachines::KeccakfManager(keccak_sm) => {
                    keccakf_counter = Some((sm.type_id(), keccak_sm.build_keccakf_counter()));
                }
                StateMachines::Sha256fManager(sha256_sm) => {
                    sha256f_counter = Some((sm.type_id(), sha256_sm.build_sha256f_counter()));
                }
                StateMachines::ArithEqManager(arith_eq_sm) => {
                    arith_eq_counter = Some((sm.type_id(), arith_eq_sm.build_arith_eq_counter()));
                }
                StateMachines::ArithEq384Manager(arith_eq_384_sm) => {
                    arith_eq_384_counter =
                        Some((sm.type_id(), arith_eq_384_sm.build_arith_eq_384_counter()));
                }
                StateMachines::Add256Manager(add256_sm) => {
                    add256_counter = Some((sm.type_id(), add256_sm.build_add256_counter()));
                }
                StateMachines::RomSM(_) => {}
            }
        }

        StaticDataBus::new(
            self.process_only_operation_bus,
            mem_counter.expect("Mem counter not found"),
            binary_counter.expect("Binary counter not found"),
            arith_counter.expect("Arith counter not found"),
            keccakf_counter.expect("Keccakf counter not found"),
            sha256f_counter.expect("Sha256f counter not found"),
            arith_eq_counter.expect("ArithEq counter not found"),
            arith_eq_384_counter.expect("ArithEq384 counter not found"),
            add256_counter.expect("Add256 counter not found"),
            Some(0),
        )
    }

    #[allow(clippy::borrowed_box)]
    pub fn build_data_bus_collectors(
        &self,
        pctx: &ProofCtx<F>,
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
        chunks_to_execute: &[Vec<usize>],
    ) -> Vec<Option<StaticDataBusCollect<u64>>> {
        chunks_to_execute
            .par_iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                if global_idxs.is_empty() {
                    return None;
                }

                let mut binary_basic_collectors = Vec::new();
                let mut binary_add_collectors = Vec::new();
                let mut binary_extension_collectors = Vec::new();
                let mut mem_collectors = Vec::new();
                let mut mem_align_collectors = Vec::new();
                let mut arith_collectors = Vec::new();
                let mut keccakf_collectors = Vec::new();
                let mut sha256f_collectors = Vec::new();
                let mut arith_eq_collectors = Vec::new();
                let mut arith_eq_384_collectors = Vec::new();
                let mut add256_collectors = Vec::new();
                let mut rom_collectors = Vec::new();
                for global_idx in global_idxs {
                    let secn_instance = secn_instances.get(global_idx).unwrap();

                    let (_, air_id) = pctx.dctx_get_instance_info(*global_idx);
                    match air_id {
                        air_id if air_id == BINARY_AIR_IDS[0] => {
                            let binary_basic_instance = secn_instance
                                .as_any()
                                .downcast_ref::<BinaryBasicInstance<F>>()
                                .unwrap();
                            let binary_basic_collector = binary_basic_instance
                                .build_binary_basic_collector(ChunkId(chunk_id));
                            binary_basic_collectors.push((*global_idx, binary_basic_collector));
                        }
                        air_id if air_id == BINARY_ADD_AIR_IDS[0] => {
                            let binary_add_instance = secn_instance
                                .as_any()
                                .downcast_ref::<BinaryAddInstance<F>>()
                                .unwrap();
                            let binary_add_collector =
                                binary_add_instance.build_binary_add_collector(ChunkId(chunk_id));
                            binary_add_collectors.push((*global_idx, binary_add_collector));
                        }
                        air_id if air_id == BINARY_EXTENSION_AIR_IDS[0] => {
                            let binary_extension_instance = secn_instance
                                .as_any()
                                .downcast_ref::<BinaryExtensionInstance<F>>()
                                .unwrap();
                            let binary_extension_collector = binary_extension_instance
                                .build_binary_extension_collector(ChunkId(chunk_id));
                            binary_extension_collectors
                                .push((*global_idx, binary_extension_collector));
                        }
                        air_id
                            if air_id == MEM_AIR_IDS[0]
                                || air_id == INPUT_DATA_AIR_IDS[0]
                                || air_id == ROM_DATA_AIR_IDS[0] =>
                        {
                            let mem_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemModuleInstance<F>>()
                                .unwrap();
                            let mem_collector = mem_instance.build_mem_collector(ChunkId(chunk_id));
                            mem_collectors.push((*global_idx, mem_collector));
                        }
                        air_id if air_id == MEM_ALIGN_AIR_IDS[0] => {
                            let mem_align_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignInstance<F>>()
                                .unwrap();
                            let mem_align_collector =
                                mem_align_instance.build_mem_align_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == MEM_ALIGN_BYTE_AIR_IDS[0] => {
                            let mem_align_byte_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignByteInstance<F>>()
                                .unwrap();
                            let mem_align_collector = mem_align_byte_instance
                                .build_mem_align_byte_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == MEM_ALIGN_READ_BYTE_AIR_IDS[0] => {
                            let mem_align_read_byte_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignReadByteInstance<F>>()
                                .unwrap();
                            let mem_align_collector = mem_align_read_byte_instance
                                .build_mem_align_read_byte_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == MEM_ALIGN_WRITE_BYTE_AIR_IDS[0] => {
                            let mem_align_write_byte_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignWriteByteInstance<F>>()
                                .unwrap();
                            let mem_align_collector = mem_align_write_byte_instance
                                .build_mem_align_write_byte_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == ARITH_AIR_IDS[0] => {
                            let arith_instance = secn_instance
                                .as_any()
                                .downcast_ref::<ArithFullInstance<F>>()
                                .unwrap();
                            let arith_collector =
                                arith_instance.build_arith_collector(ChunkId(chunk_id));
                            arith_collectors.push((*global_idx, arith_collector));
                        }
                        air_id if air_id == KECCAKF_AIR_IDS[0] => {
                            let keccakf_instance = secn_instance
                                .as_any()
                                .downcast_ref::<KeccakfInstance<F>>()
                                .unwrap();
                            let keccakf_collector =
                                keccakf_instance.build_keccakf_collector(ChunkId(chunk_id));
                            keccakf_collectors.push((*global_idx, keccakf_collector));
                        }
                        air_id if air_id == SHA_256_F_AIR_IDS[0] => {
                            let sha256f_instance = secn_instance
                                .as_any()
                                .downcast_ref::<Sha256fInstance<F>>()
                                .unwrap();
                            let sha256f_collector =
                                sha256f_instance.build_sha256f_collector(ChunkId(chunk_id));
                            sha256f_collectors.push((*global_idx, sha256f_collector));
                        }
                        air_id if air_id == ARITH_EQ_AIR_IDS[0] => {
                            let arith_eq_instance = secn_instance
                                .as_any()
                                .downcast_ref::<ArithEqInstance<F>>()
                                .unwrap();
                            let arith_eq_collector =
                                arith_eq_instance.build_arith_eq_collector(ChunkId(chunk_id));
                            arith_eq_collectors.push((*global_idx, arith_eq_collector));
                        }
                        air_id if air_id == ARITH_EQ_384_AIR_IDS[0] => {
                            let arith_eq_384_instance = secn_instance
                                .as_any()
                                .downcast_ref::<ArithEq384Instance<F>>()
                                .unwrap();
                            let arith_eq_384_collector = arith_eq_384_instance
                                .build_arith_eq_384_collector(ChunkId(chunk_id));
                            arith_eq_384_collectors.push((*global_idx, arith_eq_384_collector));
                        }
                        air_id if air_id == ADD_256_AIR_IDS[0] => {
                            let add256_instance =
                                secn_instance.as_any().downcast_ref::<Add256Instance<F>>().unwrap();
                            let add256_collector =
                                add256_instance.build_add256_collector(ChunkId(chunk_id));
                            add256_collectors.push((*global_idx, add256_collector));
                        }
                        air_id if air_id == ROM_AIR_IDS[0] => {
                            let rom_instance =
                                secn_instance.as_any().downcast_ref::<RomInstance>().unwrap();
                            let rom_collector = rom_instance.build_rom_collector(ChunkId(chunk_id));
                            if let Some(collector) = rom_collector {
                                rom_collectors.push((*global_idx, collector));
                            }
                        }
                        _ => {
                            panic!("Unsupported AIR ID: {}", air_id);
                        }
                    }
                }

                let mut arith_eq_inputs_generator = None;
                let mut arith_eq_384_inputs_generator = None;
                let mut keccakf_inputs_generator = None;
                let mut sha256f_inputs_generator = None;
                let mut arith_inputs_generator = None;
                let mut add256_inputs_generator = None;
                for (_, sm) in self.sm.values() {
                    match sm {
                        StateMachines::ArithSM(arith_sm) => {
                            arith_inputs_generator = Some(arith_sm.build_arith_input_generator());
                        }
                        StateMachines::KeccakfManager(keccak_sm) => {
                            keccakf_inputs_generator =
                                Some(keccak_sm.build_keccakf_input_generator());
                        }
                        StateMachines::Sha256fManager(sha256_sm) => {
                            sha256f_inputs_generator =
                                Some(sha256_sm.build_sha256f_input_generator());
                        }
                        StateMachines::ArithEqManager(arith_eq_sm) => {
                            arith_eq_inputs_generator =
                                Some(arith_eq_sm.build_arith_eq_input_generator());
                        }
                        StateMachines::ArithEq384Manager(arith_eq_384_sm) => {
                            arith_eq_384_inputs_generator =
                                Some(arith_eq_384_sm.build_arith_eq_384_input_generator());
                        }
                        StateMachines::Add256Manager(add256_sm) => {
                            add256_inputs_generator =
                                Some(add256_sm.build_add256_input_generator());
                        }
                        _ => {}
                    }
                }

                let data_bus = StaticDataBusCollect::new(
                    mem_collectors,
                    mem_align_collectors,
                    binary_basic_collectors,
                    binary_add_collectors,
                    binary_extension_collectors,
                    arith_collectors,
                    keccakf_collectors,
                    sha256f_collectors,
                    arith_eq_collectors,
                    arith_eq_384_collectors,
                    add256_collectors,
                    rom_collectors,
                    arith_eq_inputs_generator.expect("ArithEq input generator not found"),
                    arith_eq_384_inputs_generator.expect("ArithEq384 input generator not found"),
                    keccakf_inputs_generator.expect("KeccakF input generator not found"),
                    sha256f_inputs_generator.expect("SHA256F input generator not found"),
                    arith_inputs_generator.expect("Arith input generator not found"),
                    add256_inputs_generator.expect("Add256 input generator not found"),
                );

                Some(data_bus)
            })
            .collect()
    }
}
