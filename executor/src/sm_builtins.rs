//! Built-in state machines — the executor's hand-written counterparts
//! to the macro-generated precompiles in `sm_precompiles.rs`.
//!
//! Exposes `BuiltinSMs<F>` (enum + dispatch), the bus-side wrapper
//! structs `BuiltinCounters` / `BuiltinCollectors`, and the air-id
//! mappings used by `StaticSMBundle::new`.

use std::sync::Arc;

use anyhow::Result;
use fields::PrimeField64;
use mem_common::MemCounters;
use pil_std_lib::Std;
use precomp_dma::{
    Dma64AlignedCollector, Dma64AlignedInstance, DmaCollector, DmaCounterInputGen, DmaInstance,
    DmaManager, DmaPrePostCollector, DmaPrePostInstance, DmaUnalignedCollector,
    DmaUnalignedInstance,
};
use proofman_common::ProofCtx;
use sm_arith::{ArithCounterInputGen, ArithFullInstance, ArithInstanceCollector, ArithSM};
use sm_binary::{
    BinaryAddCollector, BinaryAddInstance, BinaryBasicCollector, BinaryBasicInstance,
    BinaryCounter, BinaryExtensionCollector, BinaryExtensionInstance, BinarySM,
};
use sm_mem::{
    Mem, MemAlignByteInstance, MemAlignCollector, MemAlignInstance, MemAlignReadByteInstance,
    MemAlignWriteByteInstance, MemModuleCollector, MemModuleInstance,
};
use sm_rom::{RomCollector, RomInstance, RomSM};
use zisk_common::{ChunkId, ComponentBuilder, Instance, InstanceCtx, Plan, Planner};

use crate::{StateMachines, StaticSMBundle};
use zisk_pil::{
    ARITH_AIR_IDS, BINARY_ADD_AIR_IDS, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS,
    DMA_64_ALIGNED_AIR_IDS, DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS,
    DMA_64_ALIGNED_MEM_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS,
    DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS, DMA_PRE_POST_AIR_IDS,
    DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS, DMA_UNALIGNED_AIR_IDS,
    INPUT_DATA_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS,
    MEM_ALIGN_READ_BYTE_AIR_IDS, MEM_ALIGN_WRITE_BYTE_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

/// `(airgroup_id, air_id)` pairs owned by one SM. Each bundle entry
/// is keyed by one of these.
pub type SMAirType = Vec<(usize, usize)>;

/// Built-in state machines.
pub enum BuiltinSMs<F: PrimeField64> {
    /// Rom state machine
    RomSM(Arc<RomSM>),
    /// Memory-related state machines.
    MemSM(Arc<Mem<F>>),
    /// Binary operation state machines.
    BinarySM(Arc<BinarySM<F>>),
    /// Arithmetic operation state machines.
    ArithSM(Arc<ArithSM<F>>),
    /// DMA-related state machines.
    DmaManager(Arc<DmaManager<F>>),
}

impl<F: PrimeField64> BuiltinSMs<F> {
    /// Constructs every built-in SM paired with its AIR-id coverage,
    /// ready to be wrapped in `StateMachines::Builtin` and pushed into
    /// the bundle.
    pub(crate) fn all(std: Arc<Std<F>>, is_asm_emulator: bool) -> Vec<(SMAirType, Self)> {
        vec![
            (rom_air_ids(), Self::RomSM(RomSM::new(is_asm_emulator))),
            (mem_air_ids(), Self::MemSM(Mem::new(std.clone()))),
            (binary_air_ids(), Self::BinarySM(BinarySM::new(std.clone()))),
            (arith_air_ids(), Self::ArithSM(ArithSM::new(std.clone()))),
            (dma_air_ids(), Self::DmaManager(DmaManager::new(std))),
        ]
    }

    pub(crate) fn build_planner(&self, is_asm_emulator: bool) -> Box<dyn Planner> {
        match self {
            Self::RomSM(sm) => <RomSM as ComponentBuilder<F>>::build_planner(sm),
            Self::MemSM(sm) => {
                if is_asm_emulator {
                    (**sm).build_dummy_planner()
                } else {
                    (**sm).build_planner()
                }
            }
            Self::BinarySM(sm) => (**sm).build_planner(),
            Self::ArithSM(sm) => (**sm).build_planner(),
            Self::DmaManager(sm) => (**sm).build_planner(),
        }
    }

    pub(crate) fn configure_instances(&self, pctx: &ProofCtx<F>, plans: &[Plan]) {
        match self {
            Self::RomSM(sm) => <RomSM as ComponentBuilder<F>>::configure_instances(sm, pctx, plans),
            Self::MemSM(sm) => (**sm).configure_instances(pctx, plans),
            Self::BinarySM(sm) => (**sm).configure_instances(pctx, plans),
            Self::ArithSM(sm) => (**sm).configure_instances(pctx, plans),
            Self::DmaManager(sm) => (**sm).configure_instances(pctx, plans),
        }
    }

    pub(crate) fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match self {
            Self::RomSM(sm) => <RomSM as ComponentBuilder<F>>::build_instance(sm, ictx),
            Self::MemSM(sm) => (**sm).build_instance(ictx),
            Self::BinarySM(sm) => (**sm).build_instance(ictx),
            Self::ArithSM(sm) => (**sm).build_instance(ictx),
            Self::DmaManager(sm) => (**sm).build_instance(ictx),
        }
    }
}

// Per-built-in AIR-id maps. Internal detail — kept next to the
// `BuiltinSMs` enum since they describe each variant's AIR coverage.

fn rom_air_ids() -> SMAirType {
    vec![(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])]
}

fn mem_air_ids() -> SMAirType {
    vec![
        (ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, INPUT_DATA_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_BYTE_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_WRITE_BYTE_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_READ_BYTE_AIR_IDS[0]),
    ]
}

fn binary_air_ids() -> SMAirType {
    vec![
        (ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, BINARY_ADD_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]),
    ]
}

fn arith_air_ids() -> SMAirType {
    vec![(ZISK_AIRGROUP_ID, ARITH_AIR_IDS[0])]
}

fn dma_air_ids() -> SMAirType {
    vec![
        (ZISK_AIRGROUP_ID, DMA_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_PRE_POST_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_64_ALIGNED_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_UNALIGNED_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_MEM_CPY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_INPUT_CPY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_PRE_POST_MEM_CPY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_PRE_POST_INPUT_CPY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_64_ALIGNED_MEM_CPY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_64_ALIGNED_MEM_SET_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_64_ALIGNED_INPUT_CPY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, DMA_64_ALIGNED_MEM_AIR_IDS[0]),
    ]
}

/// Counter-phase slots for the built-in SMs. Mirrors
/// `PrecompileCounters` on the precompile side; the two together
/// populate `StaticDataBus`.
pub struct BuiltinCounters {
    /// Memory-related counters.
    pub mem: (usize, Option<MemCounters>),
    /// Binary operation counters.
    pub binary: (usize, BinaryCounter),
    /// Arithmetic operation counters.
    pub arith: (usize, ArithCounterInputGen),
    /// DMA-related counters.
    pub dma: (usize, DmaCounterInputGen),
}

impl BuiltinCounters {
    /// Walks the bundle once, pulling each built-in SM's counter.
    /// `RomSM` has no bus-side counter and is skipped. The ASM/native
    /// switch comes from `bundle.is_asm()` — same value the bundle was
    /// constructed with, no longer threaded through this signature.
    pub(crate) fn from_bundle<F: PrimeField64>(bundle: &StaticSMBundle<F>) -> Result<Self> {
        let is_asm = bundle.is_asm();
        let mut mem = None;
        let mut binary = None;
        let mut arith = None;
        let mut dma = None;

        for (pos, (_, sm)) in bundle.entries().iter().enumerate() {
            if let StateMachines::Builtin(b) = sm {
                match b {
                    BuiltinSMs::MemSM(mem_sm) => {
                        let counter = if is_asm { None } else { Some(mem_sm.build_mem_counter()) };
                        mem = Some((pos, counter));
                    }
                    BuiltinSMs::BinarySM(binary_sm) => {
                        binary = Some((pos, binary_sm.build_binary_counter()));
                    }
                    BuiltinSMs::ArithSM(arith_sm) => {
                        arith = Some((pos, arith_sm.build_arith_counter()));
                    }
                    BuiltinSMs::DmaManager(dma_sm) => {
                        dma = Some((pos, dma_sm.build_dma_counter(is_asm)));
                    }
                    BuiltinSMs::RomSM(_) => {}
                }
            }
        }

        Ok(Self {
            mem: mem.ok_or_else(|| anyhow::anyhow!("Counter not found: Mem"))?,
            binary: binary.ok_or_else(|| anyhow::anyhow!("Counter not found: Binary"))?,
            arith: arith.ok_or_else(|| anyhow::anyhow!("Counter not found: Arith"))?,
            dma: dma.ok_or_else(|| anyhow::anyhow!("Counter not found: Dma"))?,
        })
    }
}

/// Collector-phase slots for the built-in SMs. Mirrors
/// `PrecompileCollectors` on the precompile side; the two together
/// populate `StaticDataBusCollect`.
pub struct BuiltinCollectors<F: PrimeField64> {
    /// Memory-related collectors.
    pub mem: Vec<(usize, MemModuleCollector)>,
    /// Memory alignment-related collectors.
    pub mem_align: Vec<(usize, MemAlignCollector)>,
    /// Binary basic operation collectors.
    pub binary_basic: Vec<(usize, BinaryBasicCollector<F>)>,
    /// Binary add operation collectors.
    pub binary_add: Vec<(usize, BinaryAddCollector<F>)>,
    /// Binary extension operation collectors.
    pub binary_extension: Vec<(usize, BinaryExtensionCollector<F>)>,
    /// Arithmetic operation collectors.
    pub arith: Vec<(usize, ArithInstanceCollector<F>)>,
    /// ROM operation collectors.
    pub rom: Vec<(usize, RomCollector)>,
    /// DMA-related collectors.
    pub dma: Vec<(usize, DmaCollector)>,
    /// DMA pre/post operation collectors.
    pub dma_pre_post: Vec<(usize, DmaPrePostCollector)>,
    /// DMA 64-bit aligned operation collectors.
    pub dma_64_aligned: Vec<(usize, Dma64AlignedCollector)>,
    /// DMA unaligned operation collectors.
    pub dma_unaligned: Vec<(usize, DmaUnalignedCollector)>,
    /// Arithmetic input generator.
    pub arith_inputs_generator: ArithCounterInputGen,
    /// DMA input generator.
    pub dma_inputs_generator: DmaCounterInputGen,
}

impl<F: PrimeField64> BuiltinCollectors<F> {
    /// Walks the bundle once to build the two built-in input
    /// generators (`Arith`, `Dma`). Collector vecs start empty and
    /// fill via `try_push_collector` as chunk dispatch proceeds.
    pub(crate) fn start_chunk(bundle: &StaticSMBundle<F>) -> Result<Self> {
        let mut arith_inputs_generator = None;
        let mut dma_inputs_generator = None;

        for (_, sm) in bundle.entries().iter() {
            if let StateMachines::Builtin(b) = sm {
                match b {
                    BuiltinSMs::ArithSM(arith_sm) => {
                        arith_inputs_generator = Some(arith_sm.build_arith_input_generator());
                    }
                    BuiltinSMs::DmaManager(dma_sm) => {
                        dma_inputs_generator = Some(dma_sm.build_dma_input_generator());
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            mem: Vec::new(),
            mem_align: Vec::new(),
            binary_basic: Vec::new(),
            binary_add: Vec::new(),
            binary_extension: Vec::new(),
            arith: Vec::new(),
            rom: Vec::new(),
            dma: Vec::new(),
            dma_pre_post: Vec::new(),
            dma_64_aligned: Vec::new(),
            dma_unaligned: Vec::new(),
            arith_inputs_generator: arith_inputs_generator
                .ok_or_else(|| anyhow::anyhow!("Counter not found: Arith input generator"))?,
            dma_inputs_generator: dma_inputs_generator
                .ok_or_else(|| anyhow::anyhow!("Counter not found: Dma input generator"))?,
        })
    }

    /// Per-chunk air-id dispatch. If `air_id` belongs to a built-in,
    /// downcasts `secn_instance`, builds the matching collector for
    /// this chunk, and pushes it onto the right vec. Returns:
    /// * `Ok(true)` — matched a built-in, pushed (or skipped, for
    ///   `RomSM` when its collector is `None`).
    /// * `Ok(false)` — `air_id` isn't a built-in's; caller tries the
    ///   precompile side next.
    /// * `Err(_)` — `air_id` matched but the downcast failed (bundle-
    ///   construction invariant violation).
    pub(crate) fn try_push_collector(
        &mut self,
        air_id: usize,
        secn_instance: &dyn Instance<F>,
        chunk_id: usize,
        global_idx: usize,
    ) -> Result<bool> {
        let chunk = ChunkId(chunk_id);
        match air_id {
            id if id == BINARY_AIR_IDS[0] => {
                let inst =
                    secn_instance.as_any().downcast_ref::<BinaryBasicInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected BinaryBasicInstance"),
                    )?;
                self.binary_basic.push((global_idx, inst.build_binary_basic_collector(chunk)));
                Ok(true)
            }
            id if id == BINARY_ADD_AIR_IDS[0] => {
                let inst =
                    secn_instance.as_any().downcast_ref::<BinaryAddInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected BinaryAddInstance"),
                    )?;
                self.binary_add.push((global_idx, inst.build_binary_add_collector(chunk)));
                Ok(true)
            }
            id if id == BINARY_EXTENSION_AIR_IDS[0] => {
                let inst = secn_instance
                    .as_any()
                    .downcast_ref::<BinaryExtensionInstance<F>>()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Downcast failed: expected BinaryExtensionInstance")
                    })?;
                self.binary_extension
                    .push((global_idx, inst.build_binary_extension_collector(chunk)));
                Ok(true)
            }
            id if id == MEM_AIR_IDS[0]
                || id == INPUT_DATA_AIR_IDS[0]
                || id == ROM_DATA_AIR_IDS[0] =>
            {
                let inst =
                    secn_instance.as_any().downcast_ref::<MemModuleInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected MemModuleInstance"),
                    )?;
                self.mem.push((global_idx, inst.build_mem_collector(chunk)));
                Ok(true)
            }
            id if id == MEM_ALIGN_AIR_IDS[0] => {
                let inst = secn_instance
                    .as_any()
                    .downcast_ref::<MemAlignInstance<F>>()
                    .ok_or_else(|| anyhow::anyhow!("Downcast failed: expected MemAlignInstance"))?;
                self.mem_align.push((global_idx, inst.build_mem_align_collector(chunk)));
                Ok(true)
            }
            id if id == MEM_ALIGN_BYTE_AIR_IDS[0] => {
                let inst =
                    secn_instance.as_any().downcast_ref::<MemAlignByteInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected MemAlignByteInstance"),
                    )?;
                self.mem_align.push((global_idx, inst.build_mem_align_byte_collector(chunk)));
                Ok(true)
            }
            id if id == MEM_ALIGN_READ_BYTE_AIR_IDS[0] => {
                let inst = secn_instance
                    .as_any()
                    .downcast_ref::<MemAlignReadByteInstance<F>>()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Downcast failed: expected MemAlignReadByteInstance")
                    })?;
                self.mem_align.push((global_idx, inst.build_mem_align_read_byte_collector(chunk)));
                Ok(true)
            }
            id if id == MEM_ALIGN_WRITE_BYTE_AIR_IDS[0] => {
                let inst = secn_instance
                    .as_any()
                    .downcast_ref::<MemAlignWriteByteInstance<F>>()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Downcast failed: expected MemAlignWriteByteInstance")
                    })?;
                self.mem_align.push((global_idx, inst.build_mem_align_write_byte_collector(chunk)));
                Ok(true)
            }
            id if id == ARITH_AIR_IDS[0] => {
                let inst =
                    secn_instance.as_any().downcast_ref::<ArithFullInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected ArithFullInstance"),
                    )?;
                self.arith.push((global_idx, inst.build_arith_collector(chunk)));
                Ok(true)
            }
            id if id == ROM_AIR_IDS[0] => {
                let inst = secn_instance
                    .as_any()
                    .downcast_ref::<RomInstance>()
                    .ok_or_else(|| anyhow::anyhow!("Downcast failed: expected RomInstance"))?;
                if let Some(collector) = inst.build_rom_collector(chunk) {
                    self.rom.push((global_idx, collector));
                }
                Ok(true)
            }
            id if id == DMA_AIR_IDS[0]
                || id == DMA_MEM_CPY_AIR_IDS[0]
                || id == DMA_INPUT_CPY_AIR_IDS[0] =>
            {
                let inst = secn_instance
                    .as_any()
                    .downcast_ref::<DmaInstance<F>>()
                    .ok_or_else(|| anyhow::anyhow!("Downcast failed: expected DmaInstance"))?;
                self.dma.push((global_idx, inst.build_dma_collector(chunk)));
                Ok(true)
            }
            id if id == DMA_PRE_POST_AIR_IDS[0]
                || id == DMA_PRE_POST_MEM_CPY_AIR_IDS[0]
                || id == DMA_PRE_POST_INPUT_CPY_AIR_IDS[0] =>
            {
                let inst =
                    secn_instance.as_any().downcast_ref::<DmaPrePostInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected DmaPrePostInstance"),
                    )?;
                self.dma_pre_post.push((global_idx, inst.build_dma_collector(chunk)));
                Ok(true)
            }
            id if id == DMA_64_ALIGNED_AIR_IDS[0]
                || id == DMA_64_ALIGNED_MEM_CPY_AIR_IDS[0]
                || id == DMA_64_ALIGNED_INPUT_CPY_AIR_IDS[0]
                || id == DMA_64_ALIGNED_MEM_SET_AIR_IDS[0]
                || id == DMA_64_ALIGNED_MEM_AIR_IDS[0] =>
            {
                let inst =
                    secn_instance.as_any().downcast_ref::<Dma64AlignedInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected Dma64AlignedInstance"),
                    )?;
                self.dma_64_aligned.push((global_idx, inst.build_dma_collector(chunk)));
                Ok(true)
            }
            id if id == DMA_UNALIGNED_AIR_IDS[0] => {
                let inst =
                    secn_instance.as_any().downcast_ref::<DmaUnalignedInstance<F>>().ok_or_else(
                        || anyhow::anyhow!("Downcast failed: expected DmaUnalignedInstance"),
                    )?;
                self.dma_unaligned.push((global_idx, inst.build_dma_collector(chunk)));
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
