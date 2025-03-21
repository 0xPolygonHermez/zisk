use precompiles_common::{PrecompileCall, PrecompileCode};

use tiny_keccak::keccakf;

use crate::KeccakfSM;

use zisk_core::{zisk_ops::ZiskOp, InstContext};

impl PrecompileCall for KeccakfSM {
    fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)> {
        if opcode.value() != ZiskOp::Keccak as u16 {
            panic!("Invalid opcode for Keccakf");
        }

        let address = ctx.b;

        // Allocate room for 25 u64 = 128 bytes = 1600 bits
        const WORDS: usize = 25;
        let mut data = [0u64; WORDS];

        // Read data from memory
        for (i, d) in data.iter_mut().enumerate() {
            *d = ctx.mem.read(address + (8 * i as u64), 8);
        }

        // Call keccakf
        keccakf(&mut data);

        // Write the modified data back to memory at the same address
        for (i, d) in data.iter().enumerate() {
            ctx.mem.write(address + (8 * i as u64), *d, 8);
        }

        Some((0, false))
    }
}
