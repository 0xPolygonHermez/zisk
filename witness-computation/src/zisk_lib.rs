//! The `WitnessLib` library defines the core witness computation framework,
//! integrating the ZisK execution environment with state machines and witness components.
//!
//! This module leverages `WitnessLibrary` to orchestrate the setup of state machines,
//! program conversion, and execution pipelines to generate required witnesses.

#[cfg(feature = "dev")]
use executor::DynSMBundle;
#[cfg(not(feature = "dev"))]
use executor::StaticSMBundle;
#[cfg(not(feature = "unit"))]
use executor::ZiskExecutor;
#[cfg(feature = "unit")]
use executor::ZiskExecutorTest;
use fields::{Goldilocks, PrimeField64};
use pil_std_lib::Std;
#[cfg(not(feature = "dev"))]
use precomp_arith_eq::ArithEqManager;
#[cfg(not(feature = "dev"))]
use precomp_keccakf::KeccakfManager;
#[cfg(not(feature = "dev"))]
use precomp_sha256f::Sha256fManager;
#[cfg(all(not(feature = "dev"), not(feature = "unit")))]
use proofman::register_std;
#[cfg(all(not(feature = "dev"), feature = "unit"))]
use proofman::register_std_dev;
#[cfg(not(feature = "dev"))]
use sm_arith::ArithSM;
#[cfg(not(feature = "dev"))]
use sm_binary::BinarySM;
#[cfg(not(feature = "unit"))]
use sm_main::MainSM;
#[cfg(not(feature = "dev"))]
use sm_mem::Mem;
#[cfg(all(not(feature = "dev"), not(feature = "unit")))]
use sm_rom::RomSM;
use std::{any::Any, path::PathBuf, sync::Arc};
use witness::{WitnessLibrary, WitnessManager};
use zisk_core::Riscv2zisk;
#[cfg(not(feature = "unit"))]
use zisk_core::ZiskRom;

const DEFAULT_CHUNK_SIZE_BITS: u64 = 18;

#[cfg(not(feature = "dev"))]
type Bundle<F> = StaticSMBundle<F>;

#[cfg(feature = "dev")]
type Bundle<F> = DynSMBundle<F>;

#[allow(dead_code)]
pub struct WitnessLib<F: PrimeField64> {
    elf_path: Option<PathBuf>,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    sha256f_script_path: PathBuf,
    #[cfg(not(feature = "unit"))]
    executor: Option<Arc<ZiskExecutor<F, Bundle<F>>>>,
    #[cfg(feature = "unit")]
    executor: Option<Arc<ZiskExecutorTest<F>>>,
    chunk_size: u64,
    world_rank: i32,
    local_rank: i32,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    #[cfg(feature = "dev")]
    #[allow(clippy::type_complexity)]
    register_state_machines_fn: fn(
        Arc<WitnessManager<F>>,
        Arc<Std<F>>,
        Arc<ZiskRom>,
        Option<PathBuf>,
        PathBuf,
    ) -> (Bundle<F>, bool),
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn init_library(
    verbose_mode: proofman_common::VerboseMode,
    elf_path: Option<PathBuf>,
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    sha256f_script_path: PathBuf,
    chunk_size_bits: Option<u64>,
    world_rank: Option<i32>,
    local_rank: Option<i32>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    #[cfg(feature = "dev")] register_state_machines_fn: fn(
        Arc<WitnessManager<Goldilocks>>,
        Arc<Std<Goldilocks>>,
        Arc<ZiskRom>,
        Option<PathBuf>,
        PathBuf,
    ) -> (Bundle<Goldilocks>, bool),
) -> Result<Box<dyn witness::WitnessLibrary<Goldilocks>>, Box<dyn std::error::Error>> {
    proofman_common::initialize_logger(verbose_mode, world_rank);
    let chunk_size = 1 << chunk_size_bits.unwrap_or(DEFAULT_CHUNK_SIZE_BITS);

    let result = Box::new(WitnessLib {
        elf_path,
        asm_path,
        asm_rom_path,
        sha256f_script_path,
        executor: None,
        chunk_size,
        world_rank: world_rank.unwrap_or(0),
        local_rank: local_rank.unwrap_or(0),
        base_port,
        unlock_mapped_memory,
        #[cfg(feature = "dev")]
        register_state_machines_fn,
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
        #[cfg(not(feature = "unit"))]
        {
            // Step 1: Create an instance of the RISCV -> ZisK program converter
            let rv2zk = Riscv2zisk::new(self.elf_path.as_ref().unwrap().display().to_string());

            // Step 2: Convert program to ROM
            let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
            let zisk_rom = Arc::new(zisk_rom);

            let std = Std::new(wcm.get_pctx(), wcm.get_sctx());

            #[cfg(not(feature = "dev"))]
            let (bundle, add_main_sm) = register_state_machines(
                wcm.clone(),
                std.clone(),
                zisk_rom.clone(),
                self.asm_path.clone(),
                self.sha256f_script_path.clone(),
            );

            #[cfg(feature = "dev")]
            let (bundle, add_main_sm) = (self.register_state_machines_fn)(
                wcm.clone(),
                std.clone(),
                self.asm_path.clone(),
                self.sha256f_script_path.clone(),
            );

            let main_sm = match add_main_sm {
                true => Some(MainSM::new(std.clone())),
                false => None,
            };

            // Create the executor and register the secondary state machines
            let executor: ZiskExecutor<F, Bundle<F>> = ZiskExecutor::new(
                self.elf_path.as_ref().unwrap().to_path_buf(),
                self.asm_path.clone(),
                self.asm_rom_path.clone(),
                zisk_rom,
                bundle,
                main_sm,
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

        #[cfg(feature = "unit")]
        {
            let zisk_rom = if let Some(elf_path) = &self.elf_path {
                // Step 1: Create an instance of the RISCV -> ZisK program converter
                let rv2zk = Riscv2zisk::new(elf_path.display().to_string());

                // Step 2: Convert program to ROM
                let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
                Arc::new(Some(zisk_rom))
            } else {
                Arc::new(None)
            };

            let std = Std::new(wcm.get_pctx(), wcm.get_sctx());

            let (bundle, _) = register_state_machines(
                wcm.clone(),
                std.clone(),
                self.asm_path.clone(),
                self.sha256f_script_path.clone(),
            );

            let executor = Arc::new(ZiskExecutorTest::new(bundle, zisk_rom));
            wcm.register_component(executor.clone());
            self.executor = Some(executor);
        }
    }

    /// Returns the execution result of the witness computation.
    ///
    /// # Returns
    /// * `u16` - The execution result code.
    #[cfg(not(feature = "unit"))]
    fn get_execution_result(&self) -> Option<Box<dyn std::any::Any>> {
        match &self.executor {
            None => Some(Box::new(0u64) as Box<dyn Any>),
            Some(executor) => Some(Box::new(executor.get_execution_result()) as Box<dyn Any>),
        }
    }

    #[cfg(feature = "unit")]
    fn get_execution_result(&self) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(0u64) as Box<dyn Any>)
    }
}

#[cfg(all(not(feature = "dev"), not(feature = "unit")))]
pub fn register_state_machines<F: PrimeField64>(
    wcm: Arc<WitnessManager<F>>,
    std: Arc<Std<F>>,
    zisk_rom: Arc<ZiskRom>,
    asm_path: Option<PathBuf>,
    sha256f_script_path: PathBuf,
) -> (Bundle<F>, bool) {
    register_std(&wcm, &std);

    // Step 3: Initialize the secondary state machines
    #[cfg(not(feature = "unit"))]
    let rom_sm = RomSM::new(zisk_rom.clone(), None);

    let binary_sm = BinarySM::new(std.clone());
    let arith_sm = ArithSM::new();
    let mem_sm = Mem::new(std.clone());

    // Step 4: Initialize the precompiles state machines
    let keccakf_sm = KeccakfManager::new(wcm.get_sctx());
    let sha256f_sm = Sha256fManager::new(wcm.get_sctx(), sha256f_script_path.clone());
    let arith_eq_sm = ArithEqManager::new(std.clone());

    let sm_bundle = StaticSMBundle::new(
        asm_path.is_some(),
        mem_sm.clone(),
        rom_sm.clone(),
        binary_sm.clone(),
        arith_sm.clone(),
        // The precompiles state machines
        keccakf_sm.clone(),
        sha256f_sm.clone(),
        arith_eq_sm.clone(),
    );

    (sm_bundle, true)
}

#[cfg(all(not(feature = "dev"), feature = "unit"))]
pub fn register_state_machines<F: PrimeField64>(
    wcm: Arc<WitnessManager<F>>,
    std: Arc<Std<F>>,
    asm_path: Option<PathBuf>,
    sha256f_script_path: PathBuf,
) -> (Bundle<F>, bool) {
    register_std_dev(&wcm, &std, false, false, false);

    // Step 3: Initialize the secondary state machines
    let binary_sm = BinarySM::new(std.clone());
    let arith_sm = ArithSM::new();
    let mem_sm = Mem::new(std.clone());

    // Step 4: Initialize the precompiles state machines
    let keccakf_sm = KeccakfManager::new(wcm.get_sctx());
    let sha256f_sm = Sha256fManager::new(wcm.get_sctx(), sha256f_script_path.clone());
    let arith_eq_sm = ArithEqManager::new(std.clone());

    let sm_bundle = StaticSMBundle::new(
        asm_path.is_some(),
        mem_sm.clone(),
        binary_sm.clone(),
        arith_sm.clone(),
        // The precompiles state machines
        keccakf_sm.clone(),
        sha256f_sm.clone(),
        arith_eq_sm.clone(),
    );

    (sm_bundle, true)
}
