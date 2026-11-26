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

mod rom_optimizer {
    use std::collections::BTreeMap;
    use zisk_core::{zisk_ops::ZiskOp, ZiskInstBuilder};
    use zisk_riscv::{optimize_rom, RomOptPasses};

    /// A pure instruction at `pc` on two immediate zeros that falls through to pc + 4.
    fn inst(pc: u64, op: &str) -> ZiskInstBuilder {
        let mut b = ZiskInstBuilder::default();
        b.i.paddr = pc;
        b.i.index = pc;
        b.src_a("imm", 0, false);
        b.src_b("imm", 0, false);
        b.op(op).unwrap();
        b.j(4, 4);
        b
    }
    /// An end that reads `reg`.
    fn end_reading(pc: u64, reg: u64) -> ZiskInstBuilder {
        let mut b = inst(pc, "copyb");
        b.src_b("reg", reg, false);
        b.i.end = true;
        b
    }

    /// The ROM optimizer, which runs on the wasm output, keeps every sp update: it neither
    /// threads through a jump that moves sp, nor folds an instruction that moves sp into its
    /// successor (which would also read the successor's `[sp + off]` with the old sp).
    #[test]
    fn rom_optimizer_keeps_sp_updates() {
        // 100: eq -> 104; 104: flag jump to 200 with incsp(16); 200: end.
        let mut br = inst(100, "eq");
        br.src_a("reg", 5, false);
        let mut j = inst(104, "flag");
        j.j(96, 96);
        j.inc_sp(16);
        let mut insts = BTreeMap::from([(100, br), (104, j), (200, end_reading(200, 5))]);
        optimize_rom(&mut insts, RomOptPasses::default());
        assert_eq!(insts[&100].i.jmp_offset2, 4, "threaded through a jump that moves sp");

        // 100: copyb 7 -> r5, incsp(-8); 104: add_w(r5, [sp - 8]) -> r10; 108: end reading r10.
        let mut li = inst(100, "copyb");
        li.src_b("imm", 7, false);
        li.store("reg", 5, false, false);
        li.inc_sp(-8);
        let mut add = inst(104, "add_w");
        add.src_a("reg", 5, false);
        add.src_b("mem", (-8i64) as u64, true);
        add.store("reg", 10, false, false);
        let mut insts = BTreeMap::from([(100, li), (104, add), (108, end_reading(108, 10))]);
        let st = optimize_rom(&mut insts, RomOptPasses::default());
        assert_eq!(st.constant_folding, 0, "folded an instruction that moves sp");
        let x = &insts[&100].i;
        assert_eq!((x.op, x.inc_sp), (ZiskOp::COPYB, -8));
    }
}
