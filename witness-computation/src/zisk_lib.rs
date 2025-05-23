//! The `WitnessLib` library defines the core witness computation framework,
//! integrating the ZisK execution environment with state machines and witness components.
//!
//! This module leverages `WitnessLibrary` to orchestrate the setup of state machines,
//! program conversion, and execution pipelines to generate required witnesses.

use crate::StaticSMBundle;
use executor::{/*DynSMBundle,*/ ZiskExecutor};
use p3_field::PrimeField64;
use p3_goldilocks::Goldilocks;
use pil_std_lib::Std;
use precomp_arith_eq::ArithEqManager;
use precomp_keccakf::KeccakfManager;
use precomp_sha256f::Sha256fManager;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use std::{any::Any, path::PathBuf, sync::Arc};
use witness::{WitnessLibrary, WitnessManager};
use zisk_core::Riscv2zisk;

pub struct WitnessLib<F: PrimeField64> {
    elf_path: PathBuf,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    input_data_path: Option<PathBuf>,
    sha256f_script_path: PathBuf,
    executor: Option<Arc<ZiskExecutor<F, StaticSMBundle<F>>>>,
}

#[no_mangle]
fn init_library(
    verbose_mode: proofman_common::VerboseMode,
    elf_path: PathBuf,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    input_data_path: Option<PathBuf>,
    sha256f_script_path: PathBuf,
) -> Result<Box<dyn witness::WitnessLibrary<Goldilocks>>, Box<dyn std::error::Error>> {
    proofman_common::initialize_logger(verbose_mode);
    let result = Box::new(WitnessLib {
        elf_path,
        asm_path,
        asm_rom_path,
        input_data_path,
        sha256f_script_path,
        executor: None,
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
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        // Step 1: Create an instance of the RISCV -> ZisK program converter
        let rv2zk = Riscv2zisk::new(self.elf_path.display().to_string());

        // Step 2: Convert program to ROM
        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {}", e));
        let zisk_rom = Arc::new(zisk_rom);

        // Step 3: Initialize the secondary state machines
        let std = Std::new(wcm.clone());
        let rom_sm =
            RomSM::new(zisk_rom.clone(), self.asm_rom_path.clone(), self.input_data_path.clone());
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new();
        let mem_sm = Mem::new(std.clone());

        // Step 4: Initialize the precompiles state machines
        let keccakf_sm = KeccakfManager::new::<F>();
        let sha256f_sm = Sha256fManager::new::<F>(self.sha256f_script_path.clone());
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
            self.input_data_path.clone(),
            zisk_rom,
            std,
            sm_bundle,
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
