use precompiles_common::{PrecompileCall, PrecompileCode};

use tiny_keccak::keccakf;

use crate::KeccakfSM;

pub const KECCAK_OPCODE: u32 = 0x010101;

impl PrecompileCall for KeccakfSM {
    fn execute(&self, opcode: PrecompileCode, a: u64, b: u64) -> Option<(u64, bool)> {
        unimplemented!();

        // // Get address from register a0 = x10
        // let address = ctx.mem.read(REG_A0, 8);

        // // Allocate room for 25 u64 = 128 bytes = 1600 bits
        // const WORDS: usize = 25;
        // let mut data = [0u64; WORDS];

        // // Read them from the address
        // for (i, d) in data.iter_mut().enumerate() {
        //     *d = ctx.mem.read(address + (8 * i as u64), 8);
        // }

        // // Call keccakf
        // keccakf(&mut data);

        // // Write them from the address
        // for (i, d) in data.iter().enumerate() {
        //     ctx.mem.write(address + (8 * i as u64), *d, 8);
        // }
    }
}
