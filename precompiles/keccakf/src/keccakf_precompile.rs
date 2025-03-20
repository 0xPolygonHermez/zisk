use tiny_keccak::keccakf;
use zisk_common::{MemPrecompileOps, PrecompiledEmulationMode, ZiskPrecompile};

pub struct KeccakPrecompileOp;

impl ZiskPrecompile for KeccakPrecompileOp {
    fn execute(
        &self,
        a: u64,
        _b: u64,
        emulation_mode: PrecompiledEmulationMode,
        mut mem_ops: MemPrecompileOps,
    ) -> (u64, bool, Vec<u64>) {
        let address = a;
        assert!(address & 0x7 == 0, "opc_keccak() found address not aligned to 8 bytes");

        // Allocate room for 25 u64 = 128 bytes = 1600 bits
        const WORDS: usize = 25;
        let mut data = [0u64; WORDS];

        // Get input data from memory or from the precompiled context
        let mut io_data = Vec::new();
        for (i, d) in data.iter_mut().enumerate() {
            *d = if emulation_mode == PrecompiledEmulationMode::ConsumeMemReads {
                let d = mem_ops.consume_mread();
                io_data.push(d);
                d
            } else {
                mem_ops.read_mem(address + (8 * i as u64))
            };
        }

        // Call keccakf
        keccakf(&mut data);

        // Write data to the memory address
        for (i, d) in data.iter().enumerate() {
            mem_ops.write_mem(address + (8 * i as u64), *d);
            io_data.push(*d);
        }

        (0, false, io_data)
    }
}
