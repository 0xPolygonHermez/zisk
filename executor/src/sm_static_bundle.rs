use std::sync::Arc;

use crate::{NestedDeviceMetricsList, StaticDataBusCollect};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use pil_std_lib::Std;
use precomp_arith_eq::{ArithEqInstance, ArithEqManager};
use precomp_arith_eq_384::ArithEq384Instance;
use precomp_arith_eq_384::ArithEq384Manager;
use precomp_big_int::{Add256Instance, Add256Manager};
use precomp_blake2::{Blake2Instance, Blake2Manager};
use precomp_dma::Dma64AlignedInstance;
use precomp_dma::DmaInstance;
use precomp_dma::DmaManager;
use precomp_dma::DmaPrePostInstance;
use precomp_dma::DmaUnalignedInstance;
use precomp_keccakf::{KeccakfInstance, KeccakfManager};
use precomp_poseidon2::{Poseidon2Instance, Poseidon2Manager};
use precomp_sha256f::{Sha256fInstance, Sha256fManager};
use proofman_common::ProofCtx;
use sm_arith::{ArithFullInstance, ArithSM};
use sm_binary::{BinaryAddInstance, BinaryBasicInstance, BinaryExtensionInstance, BinarySM};
use sm_mem::{
    Mem, MemAlignByteInstance, MemAlignInstance, MemAlignReadByteInstance,
    MemAlignWriteByteInstance, MemModuleInstance,
};
use sm_rom::{RomInstance, RomSM};
use std::collections::{BTreeMap, HashMap};
use zisk_common::RomHistogramData;
use zisk_common::{BusDeviceMetrics, ChunkId, ComponentBuilder, Instance, InstanceCtx, Plan};
use zisk_pil::ADD_256_AIR_IDS;
use zisk_pil::DMA_64_ALIGNED_AIR_IDS;
use zisk_pil::DMA_64_ALIGNED_INPUT_CPY_AIR_IDS;
use zisk_pil::DMA_64_ALIGNED_MEM_AIR_IDS;
use zisk_pil::DMA_64_ALIGNED_MEM_CPY_AIR_IDS;
use zisk_pil::DMA_64_ALIGNED_MEM_SET_AIR_IDS;
use zisk_pil::DMA_AIR_IDS;
use zisk_pil::DMA_INPUT_CPY_AIR_IDS;
use zisk_pil::DMA_MEM_CPY_AIR_IDS;
use zisk_pil::DMA_PRE_POST_AIR_IDS;
use zisk_pil::DMA_PRE_POST_INPUT_CPY_AIR_IDS;
use zisk_pil::DMA_PRE_POST_MEM_CPY_AIR_IDS;
use zisk_pil::DMA_UNALIGNED_AIR_IDS;
use zisk_pil::{
    ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS, BINARY_AIR_IDS,
    BINARY_EXTENSION_AIR_IDS, BLAKE_2_BR_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS,
    MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS,
    MEM_ALIGN_WRITE_BYTE_AIR_IDS, POSEIDON_2_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
    SHA_256_F_AIR_IDS, ZISK_AIRGROUP_ID,
};

use crate::{StaticDataBus, ZiskRom};
use rayon::prelude::*;

use anyhow::Result;

type SMAirType = Vec<(usize, usize)>;
pub type SMType<F> = (SMAirType, StateMachines<F>);

pub enum StateMachines<F: PrimeField64> {
    RomSM(Arc<RomSM>),
    MemSM(Arc<Mem<F>>),
    BinarySM(Arc<BinarySM<F>>),
    ArithSM(Arc<ArithSM<F>>),
    KeccakfManager(Arc<KeccakfManager<F>>),
    Sha256fManager(Arc<Sha256fManager<F>>),
    Poseidon2Manager(Arc<Poseidon2Manager<F>>),
    Blake2Manager(Arc<Blake2Manager<F>>),
    ArithEqManager(Arc<ArithEqManager<F>>),
    ArithEq384Manager(Arc<ArithEq384Manager<F>>),
    Add256Manager(Arc<Add256Manager<F>>),
    DmaManager(Arc<DmaManager<F>>),
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
            StateMachines::Poseidon2Manager(_) => 6,
            StateMachines::Blake2Manager(_) => 7,
            StateMachines::ArithEqManager(_) => 8,
            StateMachines::ArithEq384Manager(_) => 9,
            StateMachines::Add256Manager(_) => 10,
            StateMachines::DmaManager(_) => 11,
        }
    }

    fn build_planner(&self, is_asm_emulator: bool) -> Box<dyn zisk_common::Planner> {
        match self {
            StateMachines::RomSM(sm) => <RomSM as ComponentBuilder<F>>::build_planner(sm),
            StateMachines::MemSM(sm) => {
                if is_asm_emulator {
                    (**sm).build_dummy_planner()
                } else {
                    (**sm).build_planner()
                }
            }
            StateMachines::BinarySM(sm) => (**sm).build_planner(),
            StateMachines::ArithSM(sm) => (**sm).build_planner(),
            StateMachines::KeccakfManager(sm) => (**sm).build_planner(),
            StateMachines::Sha256fManager(sm) => (**sm).build_planner(),
            StateMachines::Poseidon2Manager(sm) => (**sm).build_planner(),
            StateMachines::Blake2Manager(sm) => (**sm).build_planner(),
            StateMachines::ArithEqManager(sm) => (**sm).build_planner(),
            StateMachines::ArithEq384Manager(sm) => (**sm).build_planner(),
            StateMachines::Add256Manager(sm) => (**sm).build_planner(),
            StateMachines::DmaManager(sm) => (**sm).build_planner(),
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
            StateMachines::Poseidon2Manager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::Blake2Manager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithEqManager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::ArithEq384Manager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::Add256Manager(sm) => (**sm).configure_instances(pctx, plans),
            StateMachines::DmaManager(sm) => (**sm).configure_instances(pctx, plans),
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
            StateMachines::Poseidon2Manager(sm) => (**sm).build_instance(ictx),
            StateMachines::Blake2Manager(sm) => (**sm).build_instance(ictx),
            StateMachines::ArithEqManager(sm) => (**sm).build_instance(ictx),
            StateMachines::ArithEq384Manager(sm) => (**sm).build_instance(ictx),
            StateMachines::Add256Manager(sm) => (**sm).build_instance(ictx),
            StateMachines::DmaManager(sm) => (**sm).build_instance(ictx),
        }
    }
}

pub struct StaticSMBundle<F: PrimeField64> {
    sm: BTreeMap<usize, SMType<F>>,
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(std: Arc<Std<F>>, sm: Vec<(SMAirType, StateMachines<F>)>) -> Self {
        Self {
            sm: BTreeMap::from_iter(
                sm.into_iter().map(|(air_ids, sm)| (sm.type_id(), (air_ids, sm))),
            ),
            std,
        }
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> Result<()> {
        for (_, sm) in self.sm.values() {
            if let StateMachines::RomSM(rom_sm) = sm {
                rom_sm.set_rom(zisk_rom.clone())?;
            }
        }
        Ok(())
    }

    pub fn set_rh_data(&self, rh_data: RomHistogramData) -> Result<()> {
        for (_, sm) in self.sm.values() {
            if let StateMachines::RomSM(rom_sm) = sm {
                rom_sm.set_rh_data(rh_data)?;
                break;
            }
        }

        Ok(())
    }

    pub fn get_std(&self) -> Arc<Std<F>> {
        self.std.clone()
    }

    pub fn get_mem_sm_id(&self) -> usize {
        1
    }

    pub fn plan_sec(
        &self,
        vec_counters: &mut NestedDeviceMetricsList,
        is_asm_emulator: bool,
    ) -> BTreeMap<usize, Vec<Plan>> {
        let mut plans = BTreeMap::new();

        // Iterate over vec_counters BTreeMap
        for (id, (_, sm)) in self.sm.iter() {
            if let Some(counters) = vec_counters.remove(id) {
                plans.insert(*id, sm.build_planner(is_asm_emulator).plan(counters));
            }
        }

        plans
    }

    pub fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &BTreeMap<usize, Vec<Plan>>) {
        for (id, (_, sm)) in self.sm.iter() {
            if let Some(plans) = plannings.get(id) {
                sm.configure_instances(pctx, plans);
            }
        }
    }

    pub fn build_instance(&self, ictx: InstanceCtx) -> Result<Box<dyn Instance<F>>> {
        let airgroup_id = ictx.plan.airgroup_id;
        let air_id = ictx.plan.air_id;

        if airgroup_id != ZISK_AIRGROUP_ID {
            anyhow::bail!("State machine not found: airgroup_id={airgroup_id}, air_id={air_id}");
        }

        let (_, sm) = self
            .sm
            .values()
            .find(|(air_ids, _)| air_ids.contains(&(airgroup_id, air_id)))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "State machine not found: airgroup_id={airgroup_id}, air_id={air_id}"
                )
            })?;

        Ok(sm.build_instance(ictx))
    }

    pub fn build_data_bus_counters(
        &self,
        is_asm_emulator: bool,
    ) -> Result<impl DataBusTrait<u64, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static> {
        // Extract counters from each state machine type
        let mut mem_counter = None;
        let mut binary_counter = None;
        let mut arith_counter = None;
        let mut keccakf_counter = None;
        let mut sha256f_counter = None;
        let mut poseidon2_counter = None;
        let mut blake2_counter = None;
        let mut arith_eq_counter = None;
        let mut arith_eq_384_counter = None;
        let mut add256_counter = None;
        let mut dma_counter = None;

        for (_, sm) in self.sm.values() {
            match sm {
                StateMachines::MemSM(mem_sm) => {
                    if !is_asm_emulator {
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
                    keccakf_counter =
                        Some((sm.type_id(), keccak_sm.build_keccakf_counter(is_asm_emulator)));
                }
                StateMachines::Sha256fManager(sha256_sm) => {
                    sha256f_counter =
                        Some((sm.type_id(), sha256_sm.build_sha256f_counter(is_asm_emulator)));
                }
                StateMachines::Poseidon2Manager(poseidon2_sm) => {
                    poseidon2_counter =
                        Some((sm.type_id(), poseidon2_sm.build_poseidon2_counter(is_asm_emulator)));
                }
                StateMachines::Blake2Manager(blake2_sm) => {
                    blake2_counter =
                        Some((sm.type_id(), blake2_sm.build_blake2_counter(is_asm_emulator)));
                }
                StateMachines::ArithEqManager(arith_eq_sm) => {
                    arith_eq_counter =
                        Some((sm.type_id(), arith_eq_sm.build_arith_eq_counter(is_asm_emulator)));
                }
                StateMachines::ArithEq384Manager(arith_eq_384_sm) => {
                    arith_eq_384_counter = Some((
                        sm.type_id(),
                        arith_eq_384_sm.build_arith_eq_384_counter(is_asm_emulator),
                    ));
                }
                StateMachines::Add256Manager(add256_sm) => {
                    add256_counter =
                        Some((sm.type_id(), add256_sm.build_add256_counter(is_asm_emulator)));
                }
                StateMachines::DmaManager(dma_sm) => {
                    dma_counter = Some((sm.type_id(), dma_sm.build_dma_counter(is_asm_emulator)));
                }
                StateMachines::RomSM(_) => {}
            }
        }

        Ok(StaticDataBus::new(
            is_asm_emulator,
            mem_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Mem"))?,
            binary_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Binary"))?,
            arith_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Arith"))?,
            keccakf_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Keccakf"))?,
            sha256f_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Sha256f"))?,
            poseidon2_counter
                .ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Poseidon2"))?,
            blake2_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Blake2"))?,
            arith_eq_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "ArithEq"))?,
            arith_eq_384_counter
                .ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "ArithEq384"))?,
            add256_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Add256"))?,
            dma_counter.ok_or_else(|| anyhow::anyhow!("Counter not found: {}", "Dma"))?,
            Some(0),
        ))
    }

    #[allow(clippy::borrowed_box)]
    pub fn build_data_bus_collectors(
        &self,
        pctx: &ProofCtx<F>,
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
        chunks_to_execute: &[Vec<usize>],
    ) -> Result<Vec<Option<StaticDataBusCollect<u64, F>>>> {
        chunks_to_execute
            .par_iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                if global_idxs.is_empty() {
                    return Ok(None);
                }

                let mut binary_basic_collectors = Vec::new();
                let mut binary_add_collectors = Vec::new();
                let mut binary_extension_collectors = Vec::new();
                let mut mem_collectors = Vec::new();
                let mut mem_align_collectors = Vec::new();
                let mut arith_collectors = Vec::new();
                let mut keccakf_collectors = Vec::new();
                let mut sha256f_collectors = Vec::new();
                let mut poseidon2_collectors = Vec::new();
                let mut blake2_collectors = Vec::new();
                let mut arith_eq_collectors = Vec::new();
                let mut arith_eq_384_collectors = Vec::new();
                let mut add256_collectors = Vec::new();
                let mut rom_collectors = Vec::new();
                let mut dma_collectors = Vec::new();
                let mut dma_pre_post_collectors = Vec::new();
                let mut dma_64_aligned_collectors = Vec::new();
                let mut dma_unaligned_collectors = Vec::new();
                for global_idx in global_idxs {
                    let secn_instance = secn_instances.get(global_idx).ok_or_else(|| {
                        anyhow::anyhow!("Instance not found: global_id={}", global_idx)
                    })?;

                    let (_, air_id) = pctx
                        .dctx_get_instance_info(*global_idx)
                        .map_err(|e| anyhow::anyhow!("Execution failed: {e}"))?;
                    match air_id {
                        air_id if air_id == BINARY_AIR_IDS[0] => {
                            let binary_basic_instance = secn_instance
                                .as_any()
                                .downcast_ref::<BinaryBasicInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "BinaryBasicInstance"
                                    )
                                })?;
                            let binary_basic_collector = binary_basic_instance
                                .build_binary_basic_collector(ChunkId(chunk_id));
                            binary_basic_collectors.push((*global_idx, binary_basic_collector));
                        }
                        air_id if air_id == BINARY_ADD_AIR_IDS[0] => {
                            let binary_add_instance = secn_instance
                                .as_any()
                                .downcast_ref::<BinaryAddInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "BinaryAddInstance"
                                    )
                                })?;
                            let binary_add_collector =
                                binary_add_instance.build_binary_add_collector(ChunkId(chunk_id));
                            binary_add_collectors.push((*global_idx, binary_add_collector));
                        }
                        air_id if air_id == BINARY_EXTENSION_AIR_IDS[0] => {
                            let binary_extension_instance = secn_instance
                                .as_any()
                                .downcast_ref::<BinaryExtensionInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "BinaryExtensionInstance"
                                    )
                                })?;
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
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "MemModuleInstance"
                                    )
                                })?;
                            let mem_collector = mem_instance.build_mem_collector(ChunkId(chunk_id));
                            mem_collectors.push((*global_idx, mem_collector));
                        }
                        air_id if air_id == MEM_ALIGN_AIR_IDS[0] => {
                            let mem_align_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "MemAlignInstance"
                                    )
                                })?;
                            let mem_align_collector =
                                mem_align_instance.build_mem_align_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == MEM_ALIGN_BYTE_AIR_IDS[0] => {
                            let mem_align_byte_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignByteInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "MemAlignByteInstance"
                                    )
                                })?;
                            let mem_align_collector = mem_align_byte_instance
                                .build_mem_align_byte_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == MEM_ALIGN_READ_BYTE_AIR_IDS[0] => {
                            let mem_align_read_byte_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignReadByteInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "MemAlignReadByteInstance"
                                    )
                                })?;
                            let mem_align_collector = mem_align_read_byte_instance
                                .build_mem_align_read_byte_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == MEM_ALIGN_WRITE_BYTE_AIR_IDS[0] => {
                            let mem_align_write_byte_instance = secn_instance
                                .as_any()
                                .downcast_ref::<MemAlignWriteByteInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "MemAlignWriteByteInstance"
                                    )
                                })?;
                            let mem_align_collector = mem_align_write_byte_instance
                                .build_mem_align_write_byte_collector(ChunkId(chunk_id));
                            mem_align_collectors.push((*global_idx, mem_align_collector));
                        }
                        air_id if air_id == ARITH_AIR_IDS[0] => {
                            let arith_instance = secn_instance
                                .as_any()
                                .downcast_ref::<ArithFullInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "ArithFullInstance"
                                    )
                                })?;
                            let arith_collector =
                                arith_instance.build_arith_collector(ChunkId(chunk_id));
                            arith_collectors.push((*global_idx, arith_collector));
                        }
                        air_id if air_id == KECCAKF_AIR_IDS[0] => {
                            let keccakf_instance = secn_instance
                                .as_any()
                                .downcast_ref::<KeccakfInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "KeccakfInstance"
                                    )
                                })?;
                            let keccakf_collector =
                                keccakf_instance.build_keccakf_collector(ChunkId(chunk_id));
                            keccakf_collectors.push((*global_idx, keccakf_collector));
                        }
                        air_id if air_id == SHA_256_F_AIR_IDS[0] => {
                            let sha256f_instance = secn_instance
                                .as_any()
                                .downcast_ref::<Sha256fInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "Sha256fInstance"
                                    )
                                })?;
                            let sha256f_collector =
                                sha256f_instance.build_sha256f_collector(ChunkId(chunk_id));
                            sha256f_collectors.push((*global_idx, sha256f_collector));
                        }
                        air_id if air_id == POSEIDON_2_AIR_IDS[0] => {
                            let poseidon2_instance = secn_instance
                                .as_any()
                                .downcast_ref::<Poseidon2Instance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "Poseidon2Instance"
                                    )
                                })?;
                            let poseidon2_collector =
                                poseidon2_instance.build_poseidon2_collector(ChunkId(chunk_id));
                            poseidon2_collectors.push((*global_idx, poseidon2_collector));
                        }
                        air_id if air_id == BLAKE_2_BR_AIR_IDS[0] => {
                            let blake2_instance = secn_instance
                                .as_any()
                                .downcast_ref::<Blake2Instance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "Blake2Instance"
                                    )
                                })?;
                            let blake2_collector =
                                blake2_instance.build_blake2_collector(ChunkId(chunk_id));
                            blake2_collectors.push((*global_idx, blake2_collector));
                        }
                        air_id if air_id == ARITH_EQ_AIR_IDS[0] => {
                            let arith_eq_instance = secn_instance
                                .as_any()
                                .downcast_ref::<ArithEqInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "ArithEqInstance"
                                    )
                                })?;
                            let arith_eq_collector =
                                arith_eq_instance.build_arith_eq_collector(ChunkId(chunk_id));
                            arith_eq_collectors.push((*global_idx, arith_eq_collector));
                        }
                        air_id if air_id == ARITH_EQ_384_AIR_IDS[0] => {
                            let arith_eq_384_instance = secn_instance
                                .as_any()
                                .downcast_ref::<ArithEq384Instance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "ArithEq384Instance"
                                    )
                                })?;
                            let arith_eq_384_collector = arith_eq_384_instance
                                .build_arith_eq_384_collector(ChunkId(chunk_id));
                            arith_eq_384_collectors.push((*global_idx, arith_eq_384_collector));
                        }
                        air_id if air_id == ADD_256_AIR_IDS[0] => {
                            let add256_instance = secn_instance
                                .as_any()
                                .downcast_ref::<Add256Instance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "Add256Instance"
                                    )
                                })?;
                            let add256_collector =
                                add256_instance.build_add256_collector(ChunkId(chunk_id));
                            add256_collectors.push((*global_idx, add256_collector));
                        }
                        // DMA AIRS
                        air_id
                            if air_id == DMA_AIR_IDS[0]
                                || air_id == DMA_MEM_CPY_AIR_IDS[0]
                                || air_id == DMA_INPUT_CPY_AIR_IDS[0] =>
                        {
                            let dma_instance = secn_instance
                                .as_any()
                                .downcast_ref::<DmaInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Downcast failed: expected {}", "DmaInstance")
                                })?;
                            let dma_collector = dma_instance.build_dma_collector(ChunkId(chunk_id));
                            dma_collectors.push((*global_idx, dma_collector));
                        }
                        air_id
                            if air_id == DMA_PRE_POST_AIR_IDS[0]
                                || air_id == DMA_PRE_POST_MEM_CPY_AIR_IDS[0]
                                || air_id == DMA_PRE_POST_INPUT_CPY_AIR_IDS[0] =>
                        {
                            let dma_pre_post_instance = secn_instance
                                .as_any()
                                .downcast_ref::<DmaPrePostInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "DmaPrePostInstance"
                                    )
                                })?;
                            let dma_pre_post_collector =
                                dma_pre_post_instance.build_dma_collector(ChunkId(chunk_id));
                            dma_pre_post_collectors.push((*global_idx, dma_pre_post_collector));
                        }
                        air_id
                            if air_id == DMA_64_ALIGNED_AIR_IDS[0]
                                || air_id == DMA_64_ALIGNED_MEM_CPY_AIR_IDS[0]
                                || air_id == DMA_64_ALIGNED_INPUT_CPY_AIR_IDS[0]
                                || air_id == DMA_64_ALIGNED_MEM_SET_AIR_IDS[0]
                                || air_id == DMA_64_ALIGNED_MEM_AIR_IDS[0] =>
                        {
                            let dma_64_aligned_instance = secn_instance
                                .as_any()
                                .downcast_ref::<Dma64AlignedInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "Dma64AlignedInstance"
                                    )
                                })?;
                            let dma_64_aligned_collector =
                                dma_64_aligned_instance.build_dma_collector(ChunkId(chunk_id));
                            dma_64_aligned_collectors.push((*global_idx, dma_64_aligned_collector));
                        }
                        air_id if air_id == DMA_UNALIGNED_AIR_IDS[0] => {
                            let dma_unaligned_instance = secn_instance
                                .as_any()
                                .downcast_ref::<DmaUnalignedInstance<F>>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Downcast failed: expected {}",
                                        "DmaUnalignedInstance"
                                    )
                                })?;
                            let dma_unaligned_collector =
                                dma_unaligned_instance.build_dma_collector(ChunkId(chunk_id));
                            dma_unaligned_collectors.push((*global_idx, dma_unaligned_collector));
                        }
                        air_id if air_id == ROM_AIR_IDS[0] => {
                            let rom_instance = secn_instance
                                .as_any()
                                .downcast_ref::<RomInstance>()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Downcast failed: expected {}", "RomInstance")
                                })?;
                            let rom_collector = rom_instance.build_rom_collector(ChunkId(chunk_id));
                            if let Some(collector) = rom_collector {
                                rom_collectors.push((*global_idx, collector));
                            }
                        }
                        _ => {
                            anyhow::bail!(
                                "State machine not found: airgroup_id={}, air_id={air_id}",
                                ZISK_AIRGROUP_ID
                            );
                        }
                    }
                }

                let mut arith_eq_inputs_generator = None;
                let mut arith_eq_384_inputs_generator = None;
                let mut keccakf_inputs_generator = None;
                let mut sha256f_inputs_generator = None;
                let mut poseidon2_inputs_generator = None;
                let mut blake2_inputs_generator = None;
                let mut arith_inputs_generator = None;
                let mut add256_inputs_generator = None;
                let mut dma_inputs_generator = None;
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
                        StateMachines::Poseidon2Manager(poseidon2_sm) => {
                            poseidon2_inputs_generator =
                                Some(poseidon2_sm.build_poseidon2_input_generator());
                        }
                        StateMachines::Blake2Manager(blake2_sm) => {
                            blake2_inputs_generator =
                                Some(blake2_sm.build_blake2_input_generator());
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
                        StateMachines::DmaManager(dma_sm) => {
                            dma_inputs_generator = Some(dma_sm.build_dma_input_generator());
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
                    poseidon2_collectors,
                    blake2_collectors,
                    arith_eq_collectors,
                    arith_eq_384_collectors,
                    add256_collectors,
                    dma_collectors,
                    dma_pre_post_collectors,
                    dma_64_aligned_collectors,
                    dma_unaligned_collectors,
                    rom_collectors,
                    arith_eq_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "ArithEq input generator")
                    })?,
                    arith_eq_384_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "ArithEq384 input generator")
                    })?,
                    keccakf_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "KeccakF input generator")
                    })?,
                    sha256f_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "SHA256F input generator")
                    })?,
                    poseidon2_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "Poseidon2 input generator")
                    })?,
                    blake2_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "Blake2 input generator")
                    })?,
                    arith_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "Arith input generator")
                    })?,
                    add256_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "Add256 input generator")
                    })?,
                    dma_inputs_generator.ok_or_else(|| {
                        anyhow::anyhow!("Counter not found: {}", "Dma input generator")
                    })?,
                );

                Ok(Some(data_bus))
            })
            .collect()
    }
}
