//! The `WitnessLib` library defines the core witness computation framework,
//! integrating the ZisK execution environment with state machines and witness components.
//!
//! This module leverages `WitnessLibrary` to orchestrate the setup of state machines,
//! program conversion, and execution pipelines to generate required witnesses.

use executor::ZiskExecutor;
use pil_std_lib::Std;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_mem::Mem;
use sm_rom::RomSM;
use sm_std::StdSM;
use std::sync::Arc;
use zisk_core::Riscv2zisk;

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use witness::{witness_library, WitnessLibrary, WitnessManager};

// Macro invocation to generate the core `WitnessLibrary` implementation for the ZisK system.
witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField> WitnessLibrary<F> for WitnessLib {
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
        let rv2zk = Riscv2zisk::new(
            wcm.get_rom_path().unwrap().display().to_string(),
            String::new(),
            String::new(),
            String::new(),
        );

        // Step 2: Convert program to ROM
        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {}", e));
        let zisk_rom = Arc::new(zisk_rom);

        // Step 3: Initialize the secondary state machines
        let std = Std::new(wcm.clone());
        let std_sm = StdSM::new(std.clone());
        let rom_sm = RomSM::new(zisk_rom.clone());
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new();
        let mem_sm = Mem::new(std.clone());

        // Step 4: Create the executor and register the secondary state machines
        let mut executor = ZiskExecutor::new(wcm.get_input_data_path().unwrap(), zisk_rom);
        executor.register_sm(std_sm);
        executor.register_sm(mem_sm);
        executor.register_sm(rom_sm);
        executor.register_sm(binary_sm);
        executor.register_sm(arith_sm);

        // Step 5: Register the executor as a component in the Witness Manager
        wcm.register_component(Arc::new(executor));
    }
}
