//! The `WitnessLib` library defines the core witness computation framework,
//! integrating the ZisK execution environment with state machines and witness components.
//!
//! program conversion, and execution pipelines to generate required witnesses.

use asm_runner::{HintsFile, HintsShmem};
use executor::{
    EmulatorAsm, EmulatorKind, EmulatorRust, StateMachines, StaticSMBundle, ZiskExecutor,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use precomp_arith_eq::ArithEqManager;
use precomp_arith_eq_384::ArithEq384Manager;
use precomp_big_int::Add256Manager;
use precomp_dma::DmaManager;
use precomp_keccakf::KeccakfManager;
use precomp_poseidon2::Poseidon2Manager;
use precomp_sha256f::Sha256fManager;
use precompiles_hints::HintsProcessor;
use proofman::register_std;
use proofman_common::{PackedInfo, ProofmanResult};
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tracing::debug;
use witness::WitnessManager;
use zisk_common::{
    io::{ZiskStdin, ZiskStream},
    ExecutorStatsHandle, ZiskExecutionResult,
};
use zisk_core::{Riscv2zisk, CHUNK_SIZE};
#[cfg(feature = "packed")]
use zisk_pil::PACKED_INFO;
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS,
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, DMA_64_ALIGNED_AIR_IDS, DMA_AIR_IDS,
    DMA_PRE_POST_AIR_IDS, DMA_UNALIGNED_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS,
    MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS,
    MEM_ALIGN_WRITE_BYTE_AIR_IDS, POSEIDON_2_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
    SHA_256_F_AIR_IDS, ZISK_AIRGROUP_ID,
};

use anyhow::Result;

pub struct WitnessLib<F: PrimeField64> {
    asm_mt_path: Option<PathBuf>,
    asm_rh_path: Option<PathBuf>,
    executor: Option<Arc<ZiskExecutor<F>>>,
    chunk_size: u64,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    shared_tables: bool,
    verbose_mode: proofman_common::VerboseMode,
    with_hints: bool,
}

pub fn get_packed_info() -> HashMap<(usize, usize), PackedInfo> {
    let mut _packed_info = HashMap::new();
    #[cfg(feature = "packed")]
    {
        for packed_info in PACKED_INFO.iter() {
            _packed_info.insert(
                (packed_info.0, packed_info.1),
                PackedInfo::new(
                    packed_info.2.is_packed,
                    packed_info.2.num_packed_words,
                    packed_info.2.unpack_info.to_vec(),
                ),
            );
        }
    }
    _packed_info
}

impl<F: PrimeField64> WitnessLib<F> {
    pub fn new(
        verbose_mode: proofman_common::VerboseMode,
        asm_mt_path: Option<PathBuf>,
        asm_rh_path: Option<PathBuf>,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        shared_tables: bool,
        with_hints: bool,
    ) -> Self {
        Self {
            asm_mt_path,
            asm_rh_path,
            executor: None,
            chunk_size: CHUNK_SIZE,
            base_port,
            unlock_mapped_memory,
            shared_tables,
            verbose_mode,
            with_hints,
        }
    }

    /// Registers the witness components and initializes the execution pipeline.
    ///
    /// # Arguments
    /// * `wcm` - An `Arc`-wrapped `WitnessManager` instance that orchestrates witness generation.
    ///
    /// This method performs the following steps:
    /// 1. Converts a RISC-V program to the ZisK ROM format using `Riscv2zisk`.
    /// 2. Initializes core and secondary state machines for witness generation.
    /// 3. Registers the state machines with the `ZiskExecutor`.
    /// 4. Registers the `ZiskExecutor` as a component in the `WitnessManager`.
    ///
    /// # Panics
    /// Panics if the `Riscv2zisk` conversion fails or if required paths cannot be resolved.
    pub fn register_witness(&mut self, elf: &[u8], wcm: &WitnessManager<F>) -> ProofmanResult<()> {
        assert_eq!(self.asm_mt_path.is_some(), self.asm_rh_path.is_some());

        let world_rank = wcm.get_world_rank();
        let local_rank = wcm.get_local_rank();

        proofman_common::initialize_logger(self.verbose_mode, Some(world_rank));

        // Step 1: Create an instance of the RISCV -> ZisK program converter
        let rv2zk = Riscv2zisk::new(elf);

        // Step 2: Convert program to ROM
        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
        let zisk_rom = Arc::new(zisk_rom);

        // Step 3: Initialize the secondary state machines
        let std = Std::new(wcm.get_pctx(), wcm.get_sctx(), self.shared_tables)?;
        register_std(wcm, &std);

        let rom_sm = RomSM::new(zisk_rom.clone(), self.asm_rh_path.clone());
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new(std.clone());
        let mem_sm = Mem::new(std.clone());
        // Step 4: Initialize the precompiles state machines
        let keccakf_sm = KeccakfManager::new(std.clone());
        let sha256f_sm = Sha256fManager::new(std.clone());
        let poseidon2_sm = Poseidon2Manager::new();
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
        ];

        let sm_bundle = StaticSMBundle::new(
            self.asm_mt_path.is_some(),
            vec![
                (vec![(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])], StateMachines::RomSM(rom_sm.clone())),
                (mem_instances, StateMachines::MemSM(mem_sm.clone())),
                (binary_instances, StateMachines::BinarySM(binary_sm.clone())),
                (
                    vec![(ZISK_AIRGROUP_ID, ARITH_AIR_IDS[0])],
                    StateMachines::ArithSM(arith_sm.clone()),
                ),
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

        let is_asm_emulator = self.asm_mt_path.is_some();
        let emulator = if is_asm_emulator {
            debug!("Using ASM emulator");
            EmulatorKind::Asm(EmulatorAsm::new(
                zisk_rom.clone(),
                world_rank,
                local_rank,
                self.base_port,
                self.unlock_mapped_memory,
                self.chunk_size,
                Some(rom_sm.clone()),
            ))
        } else {
            debug!("Using Rust emulator");
            EmulatorKind::Rust(EmulatorRust::new(zisk_rom.clone(), self.chunk_size))
        };

        // Create hints pipeline with null hints stream initially.
        // Debug flag: true = HintsShmem (shared memory), false = HintsFile (file output)
        let hints_stream = if self.with_hints {
            const USE_SHARED_MEMORY_HINTS: bool = true;

            let hints_processor = if USE_SHARED_MEMORY_HINTS {
                let hints_shmem =
                    HintsShmem::new(self.base_port, local_rank, self.unlock_mapped_memory)
                        .expect("zisk_lib: Failed to create HintsShmem");

                HintsProcessor::builder(hints_shmem)
                    .enable_stats(self.verbose_mode != proofman_common::VerboseMode::Info)
                    .build()
                    .expect("zisk_lib: Failed to create PrecompileHintsProcessor")
            } else {
                let hints_file = HintsFile::new(format!("hints_results_{}.bin", local_rank))
                    .expect("zisk_lib: Failed to create HintsFile");

                HintsProcessor::builder(hints_file)
                    .enable_stats(self.verbose_mode != proofman_common::VerboseMode::Info)
                    .build()
                    .expect("zisk_lib: Failed to create PrecompileHintsProcessor")
            };

            Some(ZiskStream::new(hints_processor))
        } else {
            None
        };

        let executor = Arc::new(ZiskExecutor::new(
            zisk_rom,
            std,
            sm_bundle,
            self.chunk_size,
            emulator,
            hints_stream,
        ));

        // Step 7: Register the executor as a component in the Witness Manager
        wcm.register_component(executor.clone());

        self.executor = Some(executor);

        wcm.set_witness_initialized();

        Ok(())
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) {
        if let Some(executor) = &self.executor {
            executor.set_stdin(stdin);
        }
    }

    pub fn set_hints_stream(&self, hints_stream: zisk_common::io::StreamSource) -> Result<()> {
        if !self.with_hints {
            return Err(anyhow::anyhow!(
                "Hints stream cannot be set when WitnessLib is initialized without hints"
            ));
        }
        if let Some(executor) = &self.executor {
            executor.set_hints_stream_src(hints_stream)
        } else {
            Err(anyhow::anyhow!("Executor not initialized"))
        }
    }

    /// Returns the execution result of the witness computation.
    ///
    /// # Returns
    /// * `u16` - The execution result code.
    pub fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStatsHandle)> {
        self.executor.as_ref().map(|executor| executor.get_execution_result())
    }
}
