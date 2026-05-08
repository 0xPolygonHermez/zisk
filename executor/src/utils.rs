use crate::{StateMachines, StaticSMBundle, ZiskExecutor};
use fields::PrimeField64;
use pil_std_lib::Std;
use precomp_arith_eq::ArithEqManager;
use precomp_arith_eq_384::ArithEq384Manager;
use precomp_big_int::Add256Manager;
use precomp_blake2::Blake2Manager;
use precomp_dma::DmaManager;
use precomp_keccakf::KeccakfManager;
use precomp_poseidon2::Poseidon2Manager;
use precomp_sha256f::Sha256fManager;
use proofman::register_std;
use proofman_common::PackedInfo;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use std::{collections::HashMap, sync::Arc};
use witness::WitnessManager;

use zisk_pil::PACKED_INFO;
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS,
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BLAKE_2_BR_AIR_IDS, DMA_64_ALIGNED_AIR_IDS,
    DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS, DMA_64_ALIGNED_MEM_CPY_AIR_IDS,
    DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS, DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS,
    DMA_PRE_POST_AIR_IDS, DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS,
    DMA_UNALIGNED_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS,
    MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS, MEM_ALIGN_WRITE_BYTE_AIR_IDS,
    POSEIDON_2_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS, SHA_256_F_AIR_IDS, ZISK_AIRGROUP_ID,
};

use anyhow::Result;

pub fn get_packed_info() -> HashMap<(usize, usize), PackedInfo> {
    let mut packed_info = HashMap::new();

    for pack_info in PACKED_INFO.iter() {
        packed_info.insert(
            (pack_info.0, pack_info.1),
            PackedInfo::new(
                pack_info.2.is_packed,
                pack_info.2.num_packed_words,
                pack_info.2.unpack_info.to_vec(),
            ),
        );
    }

    packed_info
}

/// Registers the witness components
///
/// # Arguments
/// * `wcm` - An `Arc`-wrapped `WitnessManager` instance that orchestrates witness generation.
///
/// This method performs the following steps:
/// 2. Initializes core and secondary state machines for witness generation.
/// 3. Registers the state machines with the `ZiskExecutor`.
/// 4. Registers the `ZiskExecutor` as a component in the `WitnessManager`.
pub fn initialize_executor<F: PrimeField64>(
    verbose_mode: proofman_common::VerboseMode,
    shared_tables: bool,
    is_asm_emulator: bool,
    wcm: &WitnessManager<F>,
) -> Result<Arc<ZiskExecutor<F>>> {
    let rank_info = wcm.get_rank_info();

    proofman_common::initialize_logger(verbose_mode, Some(&rank_info));

    // Step 3: Initialize the secondary state machines
    let std = Std::new(wcm.get_pctx(), wcm.get_sctx(), shared_tables)?;
    register_std(wcm, &std);

    let rom_sm = RomSM::new::<F>(is_asm_emulator);
    let binary_sm = BinarySM::new(std.clone());
    let arith_sm = ArithSM::new(std.clone());
    let mem_sm = Mem::new(std.clone());
    // Step 4: Initialize the precompiles state machines
    let keccakf_sm = KeccakfManager::new(std.clone());
    let sha256f_sm = Sha256fManager::new(std.clone());
    let poseidon2_sm = Poseidon2Manager::new();
    let blake2_sm = Blake2Manager::new(std.clone());
    let arith_eq_sm = ArithEqManager::new(std.clone());
    let arith_eq_384_sm = ArithEq384Manager::new(std.clone());
    let add256_sm = Add256Manager::new(std.clone());
    let dma_sm = DmaManager::new(std.clone());

    let mem_instances = vec![
        (ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, INPUT_DATA_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_BYTE_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_WRITE_BYTE_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, MEM_ALIGN_READ_BYTE_AIR_IDS[0]),
    ];

    let binary_instances = vec![
        (ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, BINARY_ADD_AIR_IDS[0]),
        (ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]),
    ];

    let dma_instances = vec![
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

    let sm_bundle = StaticSMBundle::new(
        std.clone(),
        vec![
            (vec![(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])], StateMachines::RomSM(rom_sm.clone())),
            (mem_instances, StateMachines::MemSM(mem_sm.clone())),
            (binary_instances, StateMachines::BinarySM(binary_sm.clone())),
            (vec![(ZISK_AIRGROUP_ID, ARITH_AIR_IDS[0])], StateMachines::ArithSM(arith_sm.clone())),
            // The precompiles state machines
            (
                vec![(ZISK_AIRGROUP_ID, KECCAKF_AIR_IDS[0])],
                StateMachines::KeccakfManager(keccakf_sm.clone()),
            ),
            (
                vec![(ZISK_AIRGROUP_ID, SHA_256_F_AIR_IDS[0])],
                StateMachines::Sha256fManager(sha256f_sm.clone()),
            ),
            (
                vec![(ZISK_AIRGROUP_ID, POSEIDON_2_AIR_IDS[0])],
                StateMachines::Poseidon2Manager(poseidon2_sm.clone()),
            ),
            (
                vec![(ZISK_AIRGROUP_ID, BLAKE_2_BR_AIR_IDS[0])],
                StateMachines::Blake2Manager(blake2_sm.clone()),
            ),
            (
                vec![(ZISK_AIRGROUP_ID, ARITH_EQ_AIR_IDS[0])],
                StateMachines::ArithEqManager(arith_eq_sm.clone()),
            ),
            (
                vec![(ZISK_AIRGROUP_ID, ARITH_EQ_384_AIR_IDS[0])],
                StateMachines::ArithEq384Manager(arith_eq_384_sm.clone()),
            ),
            (
                vec![(ZISK_AIRGROUP_ID, ADD_256_AIR_IDS[0])],
                StateMachines::Add256Manager(add256_sm.clone()),
            ),
            (dma_instances, StateMachines::DmaManager(dma_sm.clone())),
        ],
    );

    let executor = Arc::new(ZiskExecutor::new(sm_bundle));

    // Step 7: Register the executor as a component in the Witness Manager
    wcm.register_component(executor.clone());

    wcm.set_witness_initialized();

    Ok(executor)
}
