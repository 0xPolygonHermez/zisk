//! The `WitnessLib` library defines the core witness computation framework,
//! integrating the ZisK execution environment with state machines and witness components.
//!
//! This module leverages `WitnessLibrary` to orchestrate the setup of state machines,
//! program conversion, and execution pipelines to generate required witnesses.

use executor::{StateMachines, StaticSMBundle, ZiskExecutor};
use fields::{Goldilocks, PrimeField64};
use pil_std_lib::Std;
use precomp_arith_eq::ArithEqManager;
use precomp_arith_eq_384::ArithEq384Manager;
use precomp_big_int::Add256Manager;
use precomp_keccakf::KeccakfManager;
use precomp_sha256f::Sha256fManager;
use proofman::register_std;
use proofman_common::PackedInfo;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use witness::{WitnessLibrary, WitnessManager};
use zisk_common::{ExecutorStats, ZiskExecutionResult, ZiskLib, ZiskWitnessLibrary};
use zisk_core::{Riscv2zisk, CHUNK_SIZE};
#[cfg(feature = "packed")]
use zisk_pil::PACKED_INFO;
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS,
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS,
    MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS,
    MEM_ALIGN_WRITE_BYTE_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS, SHA_256_F_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

pub struct WitnessLib<F: PrimeField64> {
    elf_path: PathBuf,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    executor: Option<Arc<ZiskExecutor<F>>>,
    chunk_size: u64,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    shared_tables: bool,
    verbose_mode: proofman_common::VerboseMode,
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
fn init_library(
    verbose_mode: proofman_common::VerboseMode,
    elf_path: PathBuf,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    shared_tables: bool,
) -> Result<Box<dyn ZiskLib<Goldilocks>>, Box<dyn std::error::Error>> {
    let chunk_size = CHUNK_SIZE;

    let result = Box::new(WitnessLib {
        elf_path,
        asm_path,
        asm_rom_path,
        executor: None,
        chunk_size,
        base_port,
        unlock_mapped_memory,
        shared_tables,
        verbose_mode,
    });

    Ok(result)
}

impl<F: PrimeField64> WitnessLibrary<F> for WitnessLib<F> {
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
    fn register_witness(&mut self, wcm: &WitnessManager<F>) {
        let world_rank = wcm.get_world_rank();
        let local_rank = wcm.get_local_rank();

        proofman_common::initialize_logger(self.verbose_mode, Some(world_rank));

        // Step 1: Create an instance of the RISCV -> ZisK program converter
        let rv2zk = Riscv2zisk::new(self.elf_path.display().to_string());

        // Step 2: Convert program to ROM
        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
        let zisk_rom = Arc::new(zisk_rom);

        // Step 3: Initialize the secondary state machines
        let std = Std::new(wcm.get_pctx(), wcm.get_sctx(), self.shared_tables);
        register_std(wcm, &std);

        let rom_sm = RomSM::new(zisk_rom.clone(), self.asm_rom_path.clone());
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new(std.clone());
        let mem_sm = Mem::new(std.clone());
        // Step 4: Initialize the precompiles state machines
        let keccakf_sm = KeccakfManager::new(wcm.get_sctx(), std.clone());
        let sha256f_sm = Sha256fManager::new(std.clone());
        let arith_eq_sm = ArithEqManager::new(std.clone());
        let arith_eq_384_sm = ArithEq384Manager::new(std.clone());
        let add256_sm = Add256Manager::new(std.clone());

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

        let sm_bundle = StaticSMBundle::new(
            self.asm_path.is_some(),
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
            ],
        );

        // Step 5: Create the executor and register the secondary state machines
        let executor: ZiskExecutor<F> = ZiskExecutor::new(
            self.elf_path.clone(),
            self.asm_path.clone(),
            self.asm_rom_path.clone(),
            zisk_rom,
            std,
            sm_bundle,
            Some(rom_sm.clone()),
            self.chunk_size,
            world_rank,
            local_rank,
            self.base_port,
            self.unlock_mapped_memory,
        );

        let executor = Arc::new(executor);

        // Step 7: Register the executor as a component in the Witness Manager
        wcm.register_component(executor.clone());

        self.executor = Some(executor);
    }

    fn get_packed_info(&self) -> HashMap<(usize, usize), PackedInfo> {
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
}

impl ZiskWitnessLibrary<Goldilocks> for WitnessLib<Goldilocks> {
    /// Returns the execution result of the witness computation.
    ///
    /// # Returns
    /// * `u16` - The execution result code.
    fn get_execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)> {
        self.executor.as_ref().map(|executor| executor.get_execution_result())
    }
}

impl ZiskLib<Goldilocks> for WitnessLib<Goldilocks> {}
