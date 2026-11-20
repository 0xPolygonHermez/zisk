//! With the `sp` feature the wasm frame lives on the ZisK `sp` register: frame slots are
//! `[sp + off]` operands, and nothing reads `REG_FP` (x2) as a frame base.
#![cfg(feature = "sp")]

use zisk_core::{ROM_ADDR, SRC_MEM, SRC_REG, SRC_SP, STORE_MEM, ZISKLIB_ROM_ADDR};

const FIB: &[u8] = include_bytes!("../../../emulator/tests/data/wasm-fibonacci.wasm");

#[test]
fn frame_slots_are_sp_relative() {
    let rom = zisk_transpiler_wasm::wasm2rom(FIB).expect("transpile");
    let insts: Vec<_> = rom.insts.values().map(|b| &b.i).collect();
    let sp_operands = insts
        .iter()
        .filter(|i| {
            (i.a_src == SRC_MEM && i.a_use_sp_imm1 != 0)
                || (i.b_src == SRC_MEM && i.b_use_sp_imm1 != 0)
                || (i.store == STORE_MEM && i.store_use_sp)
        })
        .count();
    assert!(sp_operands > 100, "only {sp_operands} sp-relative operands");
    assert!(insts.iter().any(|i| i.set_sp), "the entry never sets sp");
    assert!(insts.iter().any(|i| i.inc_sp < 0), "no call moves sp to the callee's frame");
    assert!(insts.iter().any(|i| i.a_src == SRC_SP), "no stack check reads sp");
    // x2 is only written for the ZisK library routines, never read as a frame base by the
    // wasm code. The ZisK and float libraries, linked above `ZISKLIB_ROM_ADDR`, run on x2 as
    // their own stack pointer.
    let wasm_code = ROM_ADDR..ZISKLIB_ROM_ADDR;
    let fp_reads = insts
        .iter()
        .filter(|i| wasm_code.contains(&i.paddr) && i.a_src == SRC_REG && i.a_offset_imm0 == 2);
    assert_eq!(fp_reads.count(), 0, "x2 is still read as the frame pointer");
}
