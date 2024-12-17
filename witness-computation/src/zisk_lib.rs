use executor::ZiskExecutor;
use pil_std_lib::Std;
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_rom::RomSM;
use sm_std::StdSM;
use std::sync::Arc;
use zisk_core::Riscv2zisk;

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use witness::{witness_library, WitnessLibrary, WitnessManager};

witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField> WitnessLibrary<F> for WitnessLib {
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        // If rom_path has an .elf extension it must be converted to a ZisK ROM
        let zisk_rom = if wcm.get_rom_path().unwrap().extension().unwrap() == "elf" {
            // Create an instance of the RISCV -> ZisK program converter
            let rv2zk = Riscv2zisk::new(
                wcm.get_rom_path().unwrap().display().to_string(),
                String::new(),
                String::new(),
                String::new(),
            );

            // Convert program to ROM
            rv2zk.run().unwrap_or_else(|e| panic!("Application error: {}", e))
        } else {
            panic!("ROM file must be an ELF file");
        };
        let zisk_rom = Arc::new(zisk_rom);

        // Create the secondary state machines
        let std = Std::new(wcm.clone());

        let std_sm = StdSM::new(std.clone());
        let rom_sm = RomSM::new(zisk_rom.clone());
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new();

        let mut executor = ZiskExecutor::new(wcm.get_public_inputs_path().unwrap(), zisk_rom);
        executor.register_sm(std_sm);
        executor.register_sm(rom_sm);
        executor.register_sm(binary_sm);
        executor.register_sm(arith_sm);

        wcm.register_component(Arc::new(executor));
    }
}
