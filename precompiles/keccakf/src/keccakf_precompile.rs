use tiny_keccak::keccakf;
use zisk_common::{MemPrecompileOps, PrecompiledEmulationMode, ZiskPrecompile};

use zisk_core::REG_A0;

pub struct KeccakPrecompileOp;

impl ZiskPrecompile for KeccakPrecompileOp {
    fn execute(
        &self,
        _a: u64,
        _b: u64,
        emulation_mode: PrecompiledEmulationMode,
        mut mem_ops: MemPrecompileOps,
    ) -> (u64, bool, Vec<u64>) {
        // Get address from register a0 = x10
        let address = (mem_ops.read_reg_fn)(REG_A0);
        assert!(address & 0x7 == 0, "opc_keccak() found address not aligned to 8 bytes");

        // Allocate room for 25 u64 = 128 bytes = 1600 bits
        const WORDS: usize = 25;
        let mut data = [0u64; WORDS];

        // Get input data from memory or from the precompiled context
        let mut input_data = Vec::new();
        match emulation_mode {
            PrecompiledEmulationMode::None | PrecompiledEmulationMode::GenerateMemReads => {
                for (i, d) in data.iter_mut().enumerate() {
                    *d = (mem_ops.read_mem_fn)(
                        address + (8 * i as u64),
                        emulation_mode == PrecompiledEmulationMode::GenerateMemReads,
                    );
                }
            }
            PrecompiledEmulationMode::ConsumeMemReads => {
                for d in data.iter_mut() {
                    *d = (mem_ops.get_mem_read).as_mut().unwrap()();
                    input_data.push(*d);
                }
            }
        }

        // Call keccakf
        keccakf(&mut data);

        // Write data to the memory address
        for (i, d) in data.iter().enumerate() {
            (mem_ops.write_mem_fn)(address + (8 * i as u64), *d);
        }

        (0, false, input_data)
    }
}
