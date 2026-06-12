use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use fields::Goldilocks;
use precomp_arith_eq::ArithEqSm;
use precomp_arith_eq_384::ArithEq384Sm;
use precomp_big_int::Add256Sm;
use precomp_blake2::Blake2Sm;
use precomp_dma::{
    Dma64AlignedInputCpySm, Dma64AlignedMemCpySm, Dma64AlignedMemSetSm, Dma64AlignedMemSm,
    Dma64AlignedSm, DmaInputCpySm, DmaMemCpySm, DmaPrePostInputCpySm, DmaPrePostMemCpySm,
    DmaPrePostSm, DmaSm, DmaUnalignedSm,
};
use precomp_keccakf::KeccakfSm;
use precomp_poseidon2::Poseidon2Sm;
use precomp_sha256f::Sha256fSm;
use sm_arith::ArithSm;
use sm_binary::{BinaryAddSm, BinaryExtensionSm, BinarySm};
use sm_mem::{InputDataSm, MemAlignSm, MemSm, RomDataSm};
use zisk_common::{DynTraceOverride, DynUnitTestSm};
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS,
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BLAKE_2_BR_AIR_IDS, DMA_64_ALIGNED_AIR_IDS,
    DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS, DMA_64_ALIGNED_MEM_CPY_AIR_IDS,
    DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS, DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS,
    DMA_PRE_POST_AIR_IDS, DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS,
    DMA_UNALIGNED_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS,
    POSEIDON_2_AIR_IDS, ROM_DATA_AIR_IDS, SHA_256_F_AIR_IDS,
};

use crate::{BuiltinSMs, Precompiles, StateMachines, StaticSMBundle};

/// Declares the single list of SM markers registered with the unit-test
/// framework and expands it into both trait-object registries (every
/// `unit_test_sm!` marker implements both `DynUnitTestSm` and
/// `DynTraceOverride`, so the two lists are always identical).
macro_rules! registry {
    ($($sm:expr),* $(,)?) => {
        /// All SMs registered with the unit-test framework. Order doesn't
        /// matter; the executor looks SMs up by AIR id or name.
        pub const REGISTRY: &[&'static dyn DynUnitTestSm<Goldilocks>] = &[$(&$sm),*];

        /// Raw trace-authoring override builders, one per SM (see
        /// [`crate::unit_test_trace_override`]).
        pub const OVERRIDE_REGISTRY: &[&'static dyn DynTraceOverride<Goldilocks>] = &[$(&$sm),*];
    };
}

registry![
    BinarySm,
    BinaryAddSm,
    BinaryExtensionSm,
    ArithSm,
    KeccakfSm,
    Sha256fSm,
    Poseidon2Sm,
    Blake2Sm,
    ArithEqSm,
    ArithEq384Sm,
    Add256Sm,
    MemSm,
    RomDataSm,
    InputDataSm,
    MemAlignSm,
    DmaSm,
    DmaMemCpySm,
    DmaInputCpySm,
    DmaPrePostSm,
    DmaPrePostMemCpySm,
    DmaPrePostInputCpySm,
    Dma64AlignedSm,
    Dma64AlignedMemCpySm,
    Dma64AlignedInputCpySm,
    Dma64AlignedMemSetSm,
    Dma64AlignedMemSm,
    DmaUnalignedSm,
];

/// Look up an SM in the registry by AIR id.
pub fn lookup_by_air_id(air_id: usize) -> Option<&'static dyn DynUnitTestSm<Goldilocks>> {
    REGISTRY.iter().copied().find(|s| s.air_id() == air_id)
}

/// Look up an SM in the registry by name.
pub fn lookup_by_name(name: &str) -> Option<&'static dyn DynUnitTestSm<Goldilocks>> {
    REGISTRY.iter().copied().find(|s| s.name() == name)
}

/// Look up a trace-override builder by AIR id. `None` means the SM has no
/// override support, so the executor takes the normal `compute_witness` path.
pub fn lookup_override_by_air_id(
    air_id: usize,
) -> Option<&'static dyn DynTraceOverride<Goldilocks>> {
    OVERRIDE_REGISTRY.iter().copied().find(|s| s.air_id() == air_id)
}

/// Build the AIR-id → erased-inner-SM map from a `StaticSMBundle`. Each
/// AIR id maps to its specific inner SM (the actual witness producer), not
/// to the orchestrator that bundled them at construction time. This is
/// the one place where "which inner SM does this AIR id correspond to"
/// lives — the per-SM `UnitTestSm` impls just declare `type Manager =
/// <inner SM>` and call `mgr.compute_witness(...)` directly.
///
/// Walks the bundle's registered state machines and, for each, inserts the
/// inner witness-producing SM under every AIR id it serves. Built-in
/// orchestrators (`BinarySM`, `Mem`, `DmaManager`, …) and the generated
/// precompile managers each expose typed accessors to their inner SMs.
pub fn build_manager_registry(
    bundle: &StaticSMBundle<Goldilocks>,
) -> HashMap<usize, Arc<dyn Any + Send + Sync>> {
    let mut map: HashMap<usize, Arc<dyn Any + Send + Sync>> = HashMap::new();

    /// Coerce `&Arc<T>` to `Arc<dyn Any + Send + Sync>` (the intermediate
    /// binding keeps `Arc::clone`'s generic from resolving to the trait object).
    fn erase<T: Any + Send + Sync + 'static>(arc: &Arc<T>) -> Arc<dyn Any + Send + Sync> {
        let cloned: Arc<T> = arc.clone();
        cloned
    }

    for (_, sm) in bundle.iter_sms() {
        match sm {
            StateMachines::Builtin(BuiltinSMs::BinarySM(b)) => {
                map.insert(BINARY_AIR_IDS[0], erase(b.binary_basic_sm()));
                map.insert(BINARY_ADD_AIR_IDS[0], erase(b.binary_add_sm()));
                map.insert(BINARY_EXTENSION_AIR_IDS[0], erase(b.binary_extension_sm()));
            }
            StateMachines::Builtin(BuiltinSMs::ArithSM(a)) => {
                map.insert(ARITH_AIR_IDS[0], erase(a.arith_full_sm()));
            }
            StateMachines::Builtin(BuiltinSMs::MemSM(m)) => {
                map.insert(MEM_AIR_IDS[0], erase(m.mem_sm()));
                map.insert(ROM_DATA_AIR_IDS[0], erase(m.rom_data_sm()));
                map.insert(INPUT_DATA_AIR_IDS[0], erase(m.input_data_sm()));
                map.insert(MEM_ALIGN_AIR_IDS[0], erase(m.mem_align_sm()));
            }
            StateMachines::Builtin(BuiltinSMs::DmaManager(d)) => {
                map.insert(DMA_AIR_IDS[0], erase(d.dma_sm()));
                map.insert(DMA_MEM_CPY_AIR_IDS[0], erase(d.dma_memcpy_sm()));
                map.insert(DMA_INPUT_CPY_AIR_IDS[0], erase(d.dma_inputcpy_sm()));
                map.insert(DMA_PRE_POST_AIR_IDS[0], erase(d.dma_pre_post_sm()));
                map.insert(DMA_PRE_POST_MEM_CPY_AIR_IDS[0], erase(d.dma_pre_post_memcpy_sm()));
                map.insert(DMA_PRE_POST_INPUT_CPY_AIR_IDS[0], erase(d.dma_pre_post_inputcpy_sm()));
                map.insert(DMA_64_ALIGNED_AIR_IDS[0], erase(d.dma_64_aligned_sm()));
                map.insert(DMA_64_ALIGNED_MEM_CPY_AIR_IDS[0], erase(d.dma_64_aligned_memcpy_sm()));
                map.insert(
                    DMA_64_ALIGNED_INPUT_CPY_AIR_IDS[0],
                    erase(d.dma_64_aligned_inputcpy_sm()),
                );
                map.insert(DMA_64_ALIGNED_MEM_SET_AIR_IDS[0], erase(d.dma_64_aligned_memset_sm()));
                map.insert(DMA_64_ALIGNED_MEM_AIR_IDS[0], erase(d.dma_64_aligned_mem_sm()));
                map.insert(DMA_UNALIGNED_AIR_IDS[0], erase(d.dma_unaligned_sm()));
            }
            StateMachines::Builtin(BuiltinSMs::RomSM(_)) => {}
            StateMachines::Precompile(Precompiles::Keccakf(k)) => {
                map.insert(KECCAKF_AIR_IDS[0], erase(k.keccakf_sm()));
            }
            StateMachines::Precompile(Precompiles::Sha256f(s)) => {
                map.insert(SHA_256_F_AIR_IDS[0], erase(s.sha256f_sm()));
            }
            StateMachines::Precompile(Precompiles::Poseidon2(p)) => {
                map.insert(POSEIDON_2_AIR_IDS[0], erase(p.poseidon2_sm()));
            }
            StateMachines::Precompile(Precompiles::Blake2(b)) => {
                map.insert(BLAKE_2_BR_AIR_IDS[0], erase(b.blake2_sm()));
            }
            StateMachines::Precompile(Precompiles::ArithEq(a)) => {
                map.insert(ARITH_EQ_AIR_IDS[0], erase(a.arith_eq_sm()));
            }
            StateMachines::Precompile(Precompiles::ArithEq384(a)) => {
                map.insert(ARITH_EQ_384_AIR_IDS[0], erase(a.arith_eq384_sm()));
            }
            StateMachines::Precompile(Precompiles::Add256(a)) => {
                map.insert(ADD_256_AIR_IDS[0], erase(a.add256_sm()));
            }
        }
    }

    map
}
