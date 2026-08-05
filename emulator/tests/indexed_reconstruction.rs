//! CPU golden test for the indexed Main packing.
//!
//! For a real program's ROM, fills BOTH the full packed Main row and the compact indexed
//! row from the same `(inst, inst_ctx, reg_trace)`, builds the instruction table, then
//! asserts the reconstruction — runtime columns from the compact row, instruction-derived
//! columns from the table entry selected by the row's index — reproduces the full row
//! exactly, column by column.
//!
//! This isolates the write-side seams (compact-row packing, `build_main_instr_table`
//! values, index selection, runtime/table partition) from the GPU. Combined with the
//! macro round-trip tests and the CUDA unpack golden test, it closes the correctness loop.

use fields::Goldilocks;
use riscv2zisk::Riscv2zisk;
use zisk_core::InstContext;
use zisk_pil::{MainTraceRowInstrTable, MainTraceRowPacked, MainTraceRowPackedIndexed};
use ziskemu::{Emu, EmuRegTrace};

#[test]
fn indexed_reconstruction_matches_full_packing() {
    // Defaults to the small committed ELF; override with ZISK_TEST_ELF to exercise a
    // specific program's instruction set (e.g. the reth guest).
    let elf_path = std::env::var("ZISK_TEST_ELF").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/benches/data/my.elf").to_string()
    });
    let elf = std::fs::read(&elf_path).expect("read test elf");
    let rom = Riscv2zisk::new(&elf).run().expect("build rom");

    // Instruction table, entry i at [i*wpe .. (i+1)*wpe], indexed by sorted_pc_list_index.
    let table = Emu::build_main_instr_table::<Goldilocks>(&rom);
    let wpe = MainTraceRowInstrTable::<Goldilocks>::PACKED_WORDS;
    assert_eq!(table.len(), rom.sorted_pc_list.len() * wpe);

    // The indexed descriptor is generated beside MainTraceRow (self-contained module).
    assert_eq!(MainTraceRowPackedIndexed::<Goldilocks>::COL_SOURCE.len(), 38);
    assert_eq!(MainTraceRowPackedIndexed::<Goldilocks>::INDEX_BITS, 32);
    assert_eq!(
        MainTraceRowPackedIndexed::<Goldilocks>::COL_SOURCE.iter().filter(|&&s| s == 1).count(),
        25,
    );

    for (i, &pc) in rom.sorted_pc_list.iter().enumerate() {
        let inst = &rom.insts[&pc].i;

        // Arbitrary but deterministic runtime state. Values need only be identical across
        // the two fills; they need not form a valid execution state.
        let mut ctx = InstContext::default();
        ctx.a = 0x1122_3344_5566_7788u64 ^ (i as u64).wrapping_mul(0x9E37_79B9);
        ctx.b = 0xAABB_CCDD_EEFF_0011u64.wrapping_add((i as u64) << 7);
        ctx.c = 0x0F0F_0F0F_0F0F_0F0Fu64 ^ ((i as u64) << 11);
        ctx.flag = i % 3 == 0;

        let mut reg = EmuRegTrace::new();
        reg.reg_prev_steps = [i as u64, (i as u64).wrapping_mul(2), (i as u64).wrapping_mul(3)];
        reg.store_reg_prev_value = 0xDEAD_BEEF_CAFE_BABEu64 ^ (i as u64);

        let mut full = MainTraceRowPacked::<Goldilocks>::default();
        Emu::build_full_trace_step::<_, Goldilocks>(&mut full, inst, &ctx, &reg);

        let mut compact = MainTraceRowPackedIndexed::<Goldilocks>::default();
        Emu::build_full_trace_step::<_, Goldilocks>(&mut compact, inst, &ctx, &reg);

        // The compact row's index must select this instruction's table entry.
        let idx = compact.get_index() as usize;
        assert_eq!(idx, inst.sorted_pc_list_index, "index mismatch @pc {pc:#x}");

        let mut tbl = MainTraceRowInstrTable::<Goldilocks>::default();
        tbl.packed.copy_from_slice(&table[idx * wpe..(idx + 1) * wpe]);

        // Runtime columns: reconstructed from the compact row.
        assert_eq!(full.get_all_a(), compact.get_all_a(), "a @pc {pc:#x}");
        assert_eq!(full.get_all_b(), compact.get_all_b(), "b @pc {pc:#x}");
        assert_eq!(full.get_all_c(), compact.get_all_c(), "c @pc {pc:#x}");
        assert_eq!(full.get_flag(), compact.get_flag(), "flag @pc {pc:#x}");
        assert_eq!(full.get_addr1(), compact.get_addr1(), "addr1 @pc {pc:#x}");
        assert_eq!(
            full.get_a_reg_prev_mem_step(),
            compact.get_a_reg_prev_mem_step(),
            "a_prev @pc {pc:#x}"
        );
        assert_eq!(
            full.get_b_reg_prev_mem_step(),
            compact.get_b_reg_prev_mem_step(),
            "b_prev @pc {pc:#x}"
        );
        assert_eq!(
            full.get_store_reg_prev_mem_step(),
            compact.get_store_reg_prev_mem_step(),
            "store_prev @pc {pc:#x}"
        );
        assert_eq!(
            full.get_all_store_reg_prev_value(),
            compact.get_all_store_reg_prev_value(),
            "store_val @pc {pc:#x}"
        );

        // Instruction-derived columns: reconstructed from the table entry.
        assert_eq!(full.get_pc(), tbl.get_pc(), "pc @pc {pc:#x}");
        assert_eq!(full.get_a_src_imm(), tbl.get_a_src_imm(), "a_src_imm @pc {pc:#x}");
        assert_eq!(full.get_a_src_mem(), tbl.get_a_src_mem(), "a_src_mem @pc {pc:#x}");
        assert_eq!(full.get_a_src_reg(), tbl.get_a_src_reg(), "a_src_reg @pc {pc:#x}");
        assert_eq!(full.get_a_offset_imm0(), tbl.get_a_offset_imm0(), "a_offset_imm0 @pc {pc:#x}");
        assert_eq!(full.get_a_imm1(), tbl.get_a_imm1(), "a_imm1 @pc {pc:#x}");
        assert_eq!(
            full.get_is_precompiled(),
            tbl.get_is_precompiled(),
            "is_precompiled @pc {pc:#x}"
        );
        assert_eq!(full.get_b_src_imm(), tbl.get_b_src_imm(), "b_src_imm @pc {pc:#x}");
        assert_eq!(full.get_b_src_mem(), tbl.get_b_src_mem(), "b_src_mem @pc {pc:#x}");
        assert_eq!(full.get_b_src_reg(), tbl.get_b_src_reg(), "b_src_reg @pc {pc:#x}");
        assert_eq!(full.get_b_src_ind(), tbl.get_b_src_ind(), "b_src_ind @pc {pc:#x}");
        assert_eq!(full.get_b_offset_imm0(), tbl.get_b_offset_imm0(), "b_offset_imm0 @pc {pc:#x}");
        assert_eq!(full.get_b_imm1(), tbl.get_b_imm1(), "b_imm1 @pc {pc:#x}");
        assert_eq!(full.get_ind_width(), tbl.get_ind_width(), "ind_width @pc {pc:#x}");
        assert_eq!(
            full.get_is_external_op(),
            tbl.get_is_external_op(),
            "is_external_op @pc {pc:#x}"
        );
        assert_eq!(full.get_op(), tbl.get_op(), "op @pc {pc:#x}");
        assert_eq!(full.get_store_pc(), tbl.get_store_pc(), "store_pc @pc {pc:#x}");
        assert_eq!(full.get_store_mem(), tbl.get_store_mem(), "store_mem @pc {pc:#x}");
        assert_eq!(full.get_store_ind(), tbl.get_store_ind(), "store_ind @pc {pc:#x}");
        assert_eq!(full.get_store_reg(), tbl.get_store_reg(), "store_reg @pc {pc:#x}");
        assert_eq!(full.get_store_offset(), tbl.get_store_offset(), "store_offset @pc {pc:#x}");
        assert_eq!(full.get_set_pc(), tbl.get_set_pc(), "set_pc @pc {pc:#x}");
        assert_eq!(full.get_jmp_offset1(), tbl.get_jmp_offset1(), "jmp_offset1 @pc {pc:#x}");
        assert_eq!(full.get_jmp_offset2(), tbl.get_jmp_offset2(), "jmp_offset2 @pc {pc:#x}");
        assert_eq!(full.get_m32(), tbl.get_m32(), "m32 @pc {pc:#x}");
    }

    println!("indexed reconstruction verified for {} instructions", rom.sorted_pc_list.len());
}
