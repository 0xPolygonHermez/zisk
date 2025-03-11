use precompiles_common::{PrecompileCall, PrecompileCode};

use crate::Arith256SM;

use zisk_core::{zisk_ops::ZiskOp, InstContext};

impl PrecompileCall for Arith256SM {
    fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)> {
        if opcode.value() != ZiskOp::Arith256 as u16 {
            panic!("Invalid opcode for Arith256");
        }

        // TODO: memory access

        let address = ctx.b;

        // Allocate room for 30 u64 worst case
        const WORDS: usize = 6 + 6 * 4;
        let mut data = [0u64; WORDS];

        // Read data from memory
        for (i, d) in data.iter_mut().enumerate() {
            *d = ctx.mem.read(address + (8 * i as u64), 8);
        }

        // Call arith256
        // arith256(&mut data);

        // Write the modified data back to memory at the same address
        for (i, d) in data.iter().enumerate() {
            ctx.mem.write(address + (8 * i as u64), *d, 8);
        }

        Some((0, false))
    }
}
