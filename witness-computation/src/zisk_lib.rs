//! The `WitnessLib` library defines the core witness computation framework,
//! integrating the ZisK execution environment with state machines and witness components.
//!
//! This module leverages `WitnessLibrary` to orchestrate the setup of state machines,
//! program conversion, and execution pipelines to generate required witnesses.

use crate::StaticSMBundle;
use executor::{/*DynSMBundle,*/ ZiskExecutor};
use fields::{Goldilocks, PrimeField64};
use pil_std_lib::Std;
use precomp_arith_eq::ArithEqManager;
use precomp_keccakf::KeccakfManager;
use precomp_sha256f::Sha256fManager;
use proofman::register_std;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use std::{any::Any, path::PathBuf, sync::Arc};
use witness::{WitnessLibrary, WitnessManager};
use zisk_core::Riscv2zisk;

const DEFAULT_CHUNK_SIZE_BITS: u64 = 18;

pub struct WitnessLib<F: PrimeField64> {
    elf_path: PathBuf,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    executor: Option<Arc<ZiskExecutor<F, StaticSMBundle<F>>>>,
    chunk_size: u64,
    world_rank: i32,
    local_rank: i32,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
fn init_library(
    verbose_mode: proofman_common::VerboseMode,
    elf_path: PathBuf,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    chunk_size_bits: Option<u64>,
    world_rank: Option<i32>,
    local_rank: Option<i32>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
) -> Result<Box<dyn witness::WitnessLibrary<Goldilocks>>, Box<dyn std::error::Error>> {
    proofman_common::initialize_logger(verbose_mode, world_rank);
    let chunk_size = 1 << chunk_size_bits.unwrap_or(DEFAULT_CHUNK_SIZE_BITS);

    let result = Box::new(WitnessLib {
        elf_path,
        asm_path,
        asm_rom_path,
        executor: None,
        chunk_size,
        world_rank: world_rank.unwrap_or(0),
        local_rank: local_rank.unwrap_or(0),
        base_port,
        unlock_mapped_memory,
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
        // Step 1: Create an instance of the RISCV -> ZisK program converter
        let rv2zk = Riscv2zisk::new(self.elf_path.display().to_string());

        // Step 2: Convert program to ROM
        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
        let zisk_rom = Arc::new(zisk_rom);

        // Step 3: Initialize the secondary state machines
        let std = Std::new(wcm.get_pctx(), wcm.get_sctx());
        register_std(wcm, &std);

        let rom_sm = RomSM::new(zisk_rom.clone(), self.asm_rom_path.clone());
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new();
        let mem_sm = Mem::new(std.clone());

        // Step 4: Initialize the precompiles state machines
        let keccakf_sm = KeccakfManager::new(wcm.get_sctx());
        let sha256f_sm = Sha256fManager::new(std.clone());
        let arith_eq_sm = ArithEqManager::new(std.clone());

        // let sm_bundle = DynSMBundle::new(vec![
        //     mem_sm.clone(),
        //     rom_sm.clone(),
        //     binary_sm.clone(),
        //     arith_sm.clone(),
        //     keccakf_sm.clone(),
        //     sha256f_sm.clone(),
        //     arith_eq_sm.clone(),
        // ]);

        let sm_bundle = StaticSMBundle::new(
            self.asm_path.is_some(),
            mem_sm.clone(),
            rom_sm.clone(),
            binary_sm.clone(),
            arith_sm.clone(),
            // The precompiles state machines
            keccakf_sm.clone(),
            sha256f_sm.clone(),
            arith_eq_sm.clone(),
        );

        // Step 5: Create the executor and register the secondary state machines
        let executor: ZiskExecutor<F, StaticSMBundle<F>> = ZiskExecutor::new(
            self.elf_path.clone(),
            self.asm_path.clone(),
            self.asm_rom_path.clone(),
            zisk_rom,
            std,
            sm_bundle,
            Some(rom_sm.clone()),
            self.chunk_size,
            self.world_rank,
            self.local_rank,
            self.base_port,
            self.unlock_mapped_memory,
        );

        let executor = Arc::new(executor);

        // Step 7: Register the executor as a component in the Witness Manager
        wcm.register_component(executor.clone());

        self.executor = Some(executor);
    }

    /// Returns the execution result of the witness computation.
    ///
    /// # Returns
    /// * `u16` - The execution result code.
    fn get_execution_result(&self) -> Option<Box<dyn std::any::Any>> {
        match &self.executor {
            None => Some(Box::new(0u64) as Box<dyn Any>),
            Some(executor) => Some(Box::new(executor.get_execution_result()) as Box<dyn Any>),
        }
    }
}
