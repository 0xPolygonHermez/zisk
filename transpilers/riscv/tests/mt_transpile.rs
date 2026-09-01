//! Lowering of the `mt` DMA patterns and of the `execute_advice` / temporal-reference primitives
//! that feed them, checked on hand-assembled RISC-V.

use zisk_core::{
    ZiskInstBuilder, ZiskRom, EXTRA_PARAMS_ADDR, EXTRA_PARAMS_TEMPORAL_REF_ADDR, ROM_ADDR,
    ROM_ENTRY, SRC_IMM, SRC_REG, STORE_MEM, STORE_NONE, STORE_REG,
};
use zisk_definitions::{
    EXECUTE_ADVICE_MARKER_ID, SYSCALL_DMA_MTCMP_ID, SYSCALL_DMA_MTCPY_ID, SYSCALL_TEMPORAL_REF_ID,
    TEMPORAL_REF_REQUEST_TAG,
};
use zisk_riscv::add_zisk_code;

fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xFFF) << 20) | (rs1 << 15) | (rd << 7) | 0x13
}

fn add(rd: u32, rs1: u32, rs2: u32) -> u32 {
    (rs2 << 20) | (rs1 << 15) | (rd << 7) | 0x33
}

fn csrrs(rd: u32, csr: u16, rs1: u32) -> u32 {
    ((csr as u32) << 20) | (rs1 << 15) | (0b010 << 12) | (rd << 7) | 0x73
}

/// `nop`, used as filler so the patterns always have the instructions they look ahead to
fn nop() -> u32 {
    addi(0, 0, 0)
}

/// Transpiles `words` placed at [`ROM_ADDR`] and returns the resulting ROM
fn transpile(words: &[u32]) -> ZiskRom {
    let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    add_zisk_code(&mut rom, ROM_ADDR, &bytes, (0, 0, 0, 0));
    rom
}

/// The instruction the ROM holds at `addr`
fn inst_at(rom: &ZiskRom, addr: u64) -> &ZiskInstBuilder {
    rom.insts.get(&addr).unwrap_or_else(|| {
        panic!(
            "no instruction at 0x{addr:08X}; ROM holds [{}]",
            rom.insts.keys().map(|k| format!("0x{k:08X}")).collect::<Vec<_>>().join(", ")
        )
    })
}

/// The chain of instructions a single RISC-V instruction at `addr` expanded into: the one at `addr`
/// itself followed by the internal ones it links to
fn chain(rom: &ZiskRom, addr: u64) -> Vec<&ZiskInstBuilder> {
    let mut chain = vec![inst_at(rom, addr)];
    while let Some(next) = chain.last().unwrap().i.next_internal_inst {
        chain.push(inst_at(rom, next));
    }
    chain
}

#[test]
fn temporal_ref_request_becomes_a_tagged_flag_storing_step_into_rd() {
    // csrrs a0, TEMPORAL_REF, x0
    let rom = transpile(&[csrrs(10, SYSCALL_TEMPORAL_REF_ID, 0), nop()]);

    let inst = &inst_at(&rom, ROM_ADDR).i;
    assert_eq!(inst.op_str, "flag");
    assert_eq!(inst.b_src, SRC_IMM);
    assert_eq!(inst.b_offset_imm0, TEMPORAL_REF_REQUEST_TAG);
    assert_eq!(inst.b_use_sp_imm1, 0);
    assert_eq!(inst.store, STORE_REG);
    assert_eq!(inst.store_offset, 10);
    assert!(!inst.store_pc);
    // flag sets the flag, so both jump offsets must agree for execution to just carry on
    assert_eq!(inst.jmp_offset1, 4);
    assert_eq!(inst.jmp_offset2, 4);
}

#[test]
fn execute_advice_pattern_with_an_immediate_count() {
    //  addi x0, x0, ID
    //  addi x0, a1, 64
    //  addi x0, x0, ID
    let rom = transpile(&[
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        addi(0, 11, 64),
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        nop(),
    ]);

    let inst = &inst_at(&rom, ROM_ADDR).i;
    assert_eq!(inst.op_str, "execute_advice");
    assert_eq!(inst.a_src, SRC_REG);
    assert_eq!(inst.a_offset_imm0, 11);
    assert_eq!(inst.b_src, SRC_IMM);
    assert_eq!(inst.b_offset_imm0, 64);
    assert_eq!(inst.store, STORE_NONE);
    // jumps over the two instructions the pattern swallowed
    assert_eq!(inst.jmp_offset2, 12);

    // the swallowed instructions are still transpiled, they are just never reached
    assert_eq!(inst_at(&rom, ROM_ADDR + 4).i.op_str, "flag");
    assert_eq!(inst_at(&rom, ROM_ADDR + 8).i.op_str, "flag");
}

#[test]
fn execute_advice_pattern_with_a_register_count() {
    //  addi x0, x0, ID
    //  add  x0, a1, a2
    //  addi x0, x0, ID
    let rom = transpile(&[
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        add(0, 11, 12),
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        nop(),
    ]);

    let inst = &inst_at(&rom, ROM_ADDR).i;
    assert_eq!(inst.op_str, "execute_advice");
    assert_eq!(inst.a_src, SRC_REG);
    assert_eq!(inst.a_offset_imm0, 11);
    assert_eq!(inst.b_src, SRC_REG);
    assert_eq!(inst.b_offset_imm0, 12);
    assert_eq!(inst.jmp_offset2, 12);
}

#[test]
fn the_closing_marker_does_not_start_a_second_pattern() {
    // Two markers in a row followed by an addi would match again if the transpiler did not
    // remember how far the first pattern reached.
    let rom = transpile(&[
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        addi(0, 11, 64),
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        addi(0, 12, 32),
        addi(0, 0, EXECUTE_ADVICE_MARKER_ID),
        nop(),
    ]);

    assert_eq!(inst_at(&rom, ROM_ADDR).i.op_str, "execute_advice");
    // The closing marker of the first pattern stays a plain flag...
    assert_eq!(inst_at(&rom, ROM_ADDR + 8).i.op_str, "flag");
    // ...and the second pattern, which genuinely starts at +12, is not one: its leading marker is
    // at +16, so nothing matches there either.
    assert_eq!(inst_at(&rom, ROM_ADDR + 12).i.op_str, "flag");
    assert_eq!(inst_at(&rom, ROM_ADDR + 16).i.op_str, "flag");
}

#[test]
fn an_ordinary_hint_addi_is_untouched() {
    let rom = transpile(&[addi(0, 11, 64), nop(), nop(), nop()]);
    assert_eq!(inst_at(&rom, ROM_ADDR).i.op_str, "flag");
}

#[test]
fn mtcpy_pattern_stages_count_and_temporal_ref_before_the_operation() {
    //  csrs  MTCPY, a1            (a1 = src)
    //  add   x0, a2, a3           (a2 = dst, a3 = count)
    //  add   x0, a4, x0           (a4 = temporal reference)
    let rom =
        transpile(&[csrrs(0, SYSCALL_DMA_MTCPY_ID, 11), add(0, 12, 13), add(0, 14, 0), nop()]);

    let chain = chain(&rom, ROM_ADDR);
    assert_eq!(chain.len(), 3);

    // 1/3: count -> EXTRA_PARAMS
    let stage = &chain[0].i;
    assert_eq!(stage.op_str, "copyb");
    assert_eq!(stage.b_src, SRC_REG);
    assert_eq!(stage.b_offset_imm0, 13);
    assert_eq!(stage.store, STORE_MEM);
    assert_eq!(stage.store_offset, EXTRA_PARAMS_ADDR as i64);

    // 2/3: temporal reference -> EXTRA_PARAMS + 8
    let stage = &chain[1].i;
    assert_eq!(stage.op_str, "copyb");
    assert_eq!(stage.b_src, SRC_REG);
    assert_eq!(stage.b_offset_imm0, 14);
    assert_eq!(stage.store, STORE_MEM);
    assert_eq!(stage.store_offset, EXTRA_PARAMS_TEMPORAL_REF_ADDR as i64);

    // 3/3: the operation itself, a = dst and b = src
    let op = &chain[2].i;
    assert_eq!(op.op_str, "dma_mtcpy");
    assert_eq!(op.a_src, SRC_REG);
    assert_eq!(op.a_offset_imm0, 12);
    assert_eq!(op.b_src, SRC_REG);
    assert_eq!(op.b_offset_imm0, 11);
    assert_eq!(op.store, STORE_NONE);
    // the extended argument is unused when the count comes from a register
    assert_eq!(op.jmp_offset1, 0);
    // execution resumes past the two instructions the pattern swallowed
    assert_eq!(chain[2].i.paddr as i64 + op.jmp_offset2, (ROM_ADDR + 12) as i64);
}

#[test]
fn xmtcpy_pattern_carries_the_count_in_the_extended_argument() {
    //  csrs  MTCPY, a1
    //  addi  x0, a2, 128
    //  add   x0, a4, x0
    let rom =
        transpile(&[csrrs(0, SYSCALL_DMA_MTCPY_ID, 11), addi(0, 12, 128), add(0, 14, 0), nop()]);

    let chain = chain(&rom, ROM_ADDR);
    assert_eq!(chain.len(), 2);

    // With an immediate count there is nothing to stage but the temporal reference
    let stage = &chain[0].i;
    assert_eq!(stage.op_str, "copyb");
    assert_eq!(stage.b_offset_imm0, 14);
    assert_eq!(stage.store_offset, EXTRA_PARAMS_TEMPORAL_REF_ADDR as i64);

    let op = &chain[1].i;
    assert_eq!(op.op_str, "dma_xmtcpy");
    assert_eq!(op.a_offset_imm0, 12);
    assert_eq!(op.b_offset_imm0, 11);
    assert_eq!(op.jmp_offset1, 128);
    assert_eq!(chain[1].i.paddr as i64 + op.jmp_offset2, (ROM_ADDR + 12) as i64);
}

#[test]
fn mtcmp_pattern_stores_its_result_in_rd() {
    //  csrrs a0, MTCMP, a1
    //  add   x0, a2, a3
    //  add   x0, a4, x0
    let rom =
        transpile(&[csrrs(10, SYSCALL_DMA_MTCMP_ID, 11), add(0, 12, 13), add(0, 14, 0), nop()]);

    let chain = chain(&rom, ROM_ADDR);
    assert_eq!(chain.len(), 3);
    let op = &chain[2].i;
    assert_eq!(op.op_str, "dma_mtcmp");
    assert_eq!(op.store, STORE_REG);
    assert_eq!(op.store_offset, 10);
}

#[test]
fn xmtcmp_pattern_stores_its_result_in_rd() {
    let rom =
        transpile(&[csrrs(10, SYSCALL_DMA_MTCMP_ID, 11), addi(0, 12, 64), add(0, 14, 0), nop()]);

    let chain = chain(&rom, ROM_ADDR);
    assert_eq!(chain.len(), 2);
    let op = &chain[1].i;
    assert_eq!(op.op_str, "dma_xmtcmp");
    assert_eq!(op.jmp_offset1, 64);
    assert_eq!(op.store, STORE_REG);
    assert_eq!(op.store_offset, 10);
}

#[test]
#[should_panic(expected = "must be used as mtcpy/mtcmp")]
fn mtcpy_without_the_temporal_ref_instruction_is_rejected() {
    // The `mem` pattern (no third instruction carrying the temporal reference) is not valid here
    transpile(&[csrrs(0, SYSCALL_DMA_MTCPY_ID, 11), add(0, 12, 13), nop(), nop()]);
}
