//! Built-in state machines: `BuiltinSMs<F>` enum + witness-time dispatch
//! (`configure_instances`, `build_instance`) and plan-time static
//! dispatch (`planner_for_position`).

use fields::PrimeField64;
use pil_std_lib::Std;
use precomp_dma::DmaManager;
use proofman_common::ProofCtx;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use std::borrow::Cow;
use std::sync::Arc;

use zisk_common::{ComponentBuilder, ComponentPlanBuilder, Instance, InstanceCtx, Plan, Planner};
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

// Per-built-in AIR-id maps.
const ROM_AIR_IDS_MAP: &[(usize, usize)] = &[(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])];

const MEM_AIR_IDS_MAP: &[(usize, usize)] = &[
    (ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, INPUT_DATA_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, MEM_ALIGN_BYTE_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, MEM_ALIGN_WRITE_BYTE_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, MEM_ALIGN_READ_BYTE_AIR_IDS[0]),
];

const BINARY_AIR_IDS_MAP: &[(usize, usize)] = &[
    (ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, BINARY_ADD_AIR_IDS[0]),
    (ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]),
];

const ARITH_AIR_IDS_MAP: &[(usize, usize)] = &[(ZISK_AIRGROUP_ID, ARITH_AIR_IDS[0])];

const DMA_AIR_IDS_MAP: &[(usize, usize)] = &[
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
];

/// Tuple of built-in SMs and their AIR-id coverage.
pub type SMAirType = Cow<'static, [(usize, usize)]>;

/// Bundle positions for each built-in (matches the order in [`BuiltinSMs::all`]).
pub const ROM_POSITION: usize = 0;
pub const MEM_POSITION: usize = 1;
pub const BINARY_POSITION: usize = 2;
pub const ARITH_POSITION: usize = 3;
pub const DMA_POSITION: usize = 4;

/// Number of built-in SMs registered before any precompile.
pub const BUILTIN_COUNT: usize = 5;

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
    /// Constructs every built-in SM paired with its AIR-id coverage.
    pub(crate) fn all(std: Arc<Std<F>>) -> Vec<(SMAirType, Self)> {
        vec![
            (Cow::Borrowed(ROM_AIR_IDS_MAP), Self::RomSM(RomSM::new::<F>())),
            (Cow::Borrowed(MEM_AIR_IDS_MAP), Self::MemSM(Mem::new(std.clone()))),
            (Cow::Borrowed(BINARY_AIR_IDS_MAP), Self::BinarySM(BinarySM::new(std.clone()))),
            (Cow::Borrowed(ARITH_AIR_IDS_MAP), Self::ArithSM(ArithSM::new(std.clone()))),
            (Cow::Borrowed(DMA_AIR_IDS_MAP), Self::DmaManager(DmaManager::new(std))),
        ]
    }

    /// Static planner dispatch by bundle position — no SM instance needed.
    pub(crate) fn planner_for_position(position: usize, is_asm_emulator: bool) -> Box<dyn Planner> {
        match position {
            ROM_POSITION => unreachable!(
                "ROM planning goes through RomPlanner::plan_for_chunks, not the Planner trait"
            ),
            MEM_POSITION => <Mem<F> as ComponentPlanBuilder<F>>::planner(is_asm_emulator),
            BINARY_POSITION => <BinarySM<F> as ComponentPlanBuilder<F>>::planner(is_asm_emulator),
            ARITH_POSITION => <ArithSM<F> as ComponentPlanBuilder<F>>::planner(is_asm_emulator),
            DMA_POSITION => <DmaManager<F> as ComponentPlanBuilder<F>>::planner(is_asm_emulator),
            _ => panic!("planner_for_position: invalid builtin position {position}"),
        }
    }

    /// Configures the instances of this built-in for the given plans.
    pub(crate) fn configure_instances(&self, pctx: &ProofCtx<F>, plans: &[Plan]) {
        match self {
            Self::RomSM(sm) => <RomSM as ComponentBuilder<F>>::configure_instances(sm, pctx, plans),
            Self::MemSM(sm) => (**sm).configure_instances(pctx, plans),
            Self::BinarySM(sm) => (**sm).configure_instances(pctx, plans),
            Self::ArithSM(sm) => (**sm).configure_instances(pctx, plans),
            Self::DmaManager(sm) => (**sm).configure_instances(pctx, plans),
        }
    }

    /// Builds an instance of this built-in for the given instance context.
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
