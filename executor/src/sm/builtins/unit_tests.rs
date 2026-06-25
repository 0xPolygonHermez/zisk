//! Built-in unit-test wiring — the single declarative source for which
//! inner SM each built-in AIR id maps to in the unit-test framework.
//!
//! Built-in orchestrators (`BinarySM`, `Mem`, `DmaManager`, …) bundle several
//! inner witness-producing SMs, each reached by a differently named accessor
//! (`binary_basic_sm`, `dma_64_aligned_memcpy_sm`, …). Unlike precompiles —
//! whose accessor/marker names follow the `zisk_precompile!` convention and so
//! are derived automatically in [`super::super::register_precompiles`] — these
//! names are irregular and must be declared explicitly.
//!
//! Adding a built-in unit-test target: ONE line in `builtin_unit_tests!`
//! below — the marker, its `*_AIR_IDS` const, and the accessor on the
//! orchestrator. The match arm for `build_manager_registry` and the two
//! global registry slices are all generated from it.
//!
//! NOTE: this list is intentionally separate from the `*_AIR_IDS_MAP`
//! dispatch slices in [`super::state_machines`]. Those carry every AIR id a
//! built-in serves for production dispatch — including AIR ids with no
//! unit-test marker (e.g. the `MEM_ALIGN_*_BYTE` AIRs) — and must not be
//! coupled to the unit-test surface.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use fields::Goldilocks;
use precomp_dma::{
    Dma64AlignedInputCpySm, Dma64AlignedMemCpySm, Dma64AlignedMemSetSm, Dma64AlignedMemSm,
    Dma64AlignedSm, DmaInputCpySm, DmaMemCpySm, DmaPrePostInputCpySm, DmaPrePostMemCpySm,
    DmaPrePostSm, DmaSm, DmaUnalignedSm,
};
use sm_arith::ArithSm;
use sm_binary::{BinaryAddSm, BinaryExtensionSm, BinarySm};
use sm_mem::{InputDataSm, MemAlignSm, MemSm, RomDataSm};
use zisk_common::{DynTraceOverride, DynUnitTestSm};
use zisk_pil::{
    ARITH_AIR_IDS, BINARY_ADD_AIR_IDS, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS,
    DMA_64_ALIGNED_AIR_IDS, DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS,
    DMA_64_ALIGNED_MEM_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS,
    DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS, DMA_PRE_POST_AIR_IDS,
    DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS, DMA_UNALIGNED_AIR_IDS,
    INPUT_DATA_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS, ROM_DATA_AIR_IDS,
};

use crate::unit_test_targets::erase;
use crate::BuiltinSMs;

/// Declarative map: per built-in orchestrator, the inner SMs it exposes to the
/// unit-test framework, each as `(marker, AIR_IDS const, accessor)`.
///
/// Emits:
/// * [`register_builtin_unit_tests`] — inserts each inner SM under its AIR id
///   into the `build_manager_registry` map (erased to `Arc<dyn Any>`).
/// * [`BUILTIN_UNIT_TEST_MARKERS`] / [`BUILTIN_UNIT_TEST_OVERRIDES`] — the
///   per-marker references spliced into the global unit-test registries.
macro_rules! builtin_unit_tests {
    (
        $(
            $bvariant:ident ( $bsm:ident ) => [
                $( ( $marker:ident, $air:expr, $accessor:ident ) ),* $(,)?
            ]
        ),* $(,)?
    ) => {
        /// Insert every built-in inner witness-producing SM under its AIR id.
        /// `bundle_sm` is one entry from `StaticSMBundle::iter_sms`; non-matching
        /// or marker-less built-ins (e.g. `RomSM`) contribute nothing.
        pub(crate) fn register_builtin_unit_tests(
            map: &mut HashMap<usize, Arc<dyn Any + Send + Sync>>,
            builtin: &BuiltinSMs<Goldilocks>,
        ) {
            match builtin {
                $(
                    BuiltinSMs::$bvariant($bsm) => {
                        $( map.insert($air[0], erase($bsm.$accessor())); )*
                    }
                )*
            }
        }

        /// Unit-test SM markers for every built-in inner SM, in declaration
        /// order. Spliced ahead of the precompile markers in `REGISTRY`.
        pub(crate) const BUILTIN_UNIT_TEST_MARKERS:
            &[&'static dyn DynUnitTestSm<Goldilocks>] =
            &[ $( $( &$marker, )* )* ];

        /// Raw trace-authoring overrides for every built-in inner SM, in
        /// declaration order. Same markers as [`BUILTIN_UNIT_TEST_MARKERS`].
        pub(crate) const BUILTIN_UNIT_TEST_OVERRIDES:
            &[&'static dyn DynTraceOverride<Goldilocks>] =
            &[ $( $( &$marker, )* )* ];
    };
}

builtin_unit_tests! {
    BinarySM(b) => [
        (BinarySm,          BINARY_AIR_IDS,           binary_basic_sm),
        (BinaryAddSm,       BINARY_ADD_AIR_IDS,       binary_add_sm),
        (BinaryExtensionSm, BINARY_EXTENSION_AIR_IDS, binary_extension_sm),
    ],
    ArithSM(a) => [
        (ArithSm, ARITH_AIR_IDS, arith_full_sm),
    ],
    MemSM(m) => [
        (MemSm,       MEM_AIR_IDS,        mem_sm),
        (RomDataSm,   ROM_DATA_AIR_IDS,   rom_data_sm),
        (InputDataSm, INPUT_DATA_AIR_IDS, input_data_sm),
        (MemAlignSm,  MEM_ALIGN_AIR_IDS,  mem_align_sm),
    ],
    DmaManager(d) => [
        (DmaSm,                   DMA_AIR_IDS,                    dma_sm),
        (DmaMemCpySm,             DMA_MEM_CPY_AIR_IDS,            dma_memcpy_sm),
        (DmaInputCpySm,           DMA_INPUT_CPY_AIR_IDS,          dma_inputcpy_sm),
        (DmaPrePostSm,            DMA_PRE_POST_AIR_IDS,           dma_pre_post_sm),
        (DmaPrePostMemCpySm,      DMA_PRE_POST_MEM_CPY_AIR_IDS,   dma_pre_post_memcpy_sm),
        (DmaPrePostInputCpySm,    DMA_PRE_POST_INPUT_CPY_AIR_IDS, dma_pre_post_inputcpy_sm),
        (Dma64AlignedSm,          DMA_64_ALIGNED_AIR_IDS,         dma_64_aligned_sm),
        (Dma64AlignedMemCpySm,    DMA_64_ALIGNED_MEM_CPY_AIR_IDS, dma_64_aligned_memcpy_sm),
        (Dma64AlignedInputCpySm,  DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, dma_64_aligned_inputcpy_sm),
        (Dma64AlignedMemSetSm,    DMA_64_ALIGNED_MEM_SET_AIR_IDS, dma_64_aligned_memset_sm),
        (Dma64AlignedMemSm,       DMA_64_ALIGNED_MEM_AIR_IDS,     dma_64_aligned_mem_sm),
        (DmaUnalignedSm,          DMA_UNALIGNED_AIR_IDS,          dma_unaligned_sm),
    ],
    // RomSM has no unit-test markers; the wildcard arm below covers it.
    RomSM(_unused) => [],
}
