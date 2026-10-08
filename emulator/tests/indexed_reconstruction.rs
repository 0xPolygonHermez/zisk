//! CPU golden test for the indexed Main packing.
//!
//! Fills both the full packed row and the compact indexed row from `MAIN_LANES` different
//! instructions, one per lane, then asserts the reconstruction reproduces the full row lane by
//! lane: runtime columns from the compact row, instruction-derived columns from the entry that
//! lane's index selects.

use proofman_fields::Goldilocks;
use zisk_core::InstContext;
use zisk_pil::{MainTraceRowInstrTable, MainTraceRowPacked, MainTraceRowPackedIndexed, MAIN_LANES};
use zisk_transpiler_riscv::Riscv2zisk;
use ziskemu::{Emu, EmuRegTrace};

#[test]
fn indexed_reconstruction_matches_full_packing() {
    // Override with ZISK_TEST_ELF to exercise another program's instruction set.
    let elf_path = std::env::var("ZISK_TEST_ELF").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/benches/data/my.elf").to_string()
    });
    let elf = std::fs::read(&elf_path).expect("read test elf");
    let rom = Riscv2zisk::new(&elf).run().expect("build rom");

    // Instruction table, entry i at [i*wpe .. (i+1)*wpe], indexed by sorted_pc_list_index.
    let table = Emu::build_main_instr_table::<Goldilocks>(&rom);
    let wpe = MainTraceRowInstrTable::<Goldilocks>::PACKED_WORDS;
    assert_eq!(table.len(), rom.sorted_pc_list.len() * wpe);

    // MAIN_LANES consecutive instructions per row, so its lanes hold different pcs.
    for (row, pcs) in rom.sorted_pc_list.chunks(MAIN_LANES).enumerate() {
        let mut full = MainTraceRowPacked::<Goldilocks>::default();
        let mut compact = MainTraceRowPackedIndexed::<Goldilocks>::default();

        for (lane, &pc) in pcs.iter().enumerate() {
            let inst = &rom.insts[&pc].i;
            // Arbitrary but deterministic; need only match across the two fills.
            let seed = (row * MAIN_LANES + lane) as u64;
            let ctx = InstContext {
                a: 0x1122_3344_5566_7788u64 ^ seed.wrapping_mul(0x9E37_79B9),
                b: 0xAABB_CCDD_EEFF_0011u64.wrapping_add(seed << 7),
                c: 0x0F0F_0F0F_0F0F_0F0Fu64 ^ (seed << 11),
                flag: seed % 3 == 0,
                ..Default::default()
            };
            let mut reg = EmuRegTrace::new();
            reg.reg_prev_steps = [seed, seed.wrapping_mul(2), seed.wrapping_mul(3)];
            reg.store_reg_prev_value = 0xDEAD_BEEF_CAFE_BABEu64 ^ seed;

            Emu::build_full_trace_step::<_, Goldilocks>(&mut full, lane, inst, &ctx, &reg);
            Emu::build_full_trace_step::<_, Goldilocks>(&mut compact, lane, inst, &ctx, &reg);
        }

        for (lane, &pc) in pcs.iter().enumerate() {
            let inst = &rom.insts[&pc].i;
            let at = format!("pc {pc:#x} lane {lane}");

            // The lane's index must select this instruction's table entry.
            let idx = compact.get_index(lane) as usize;
            assert_eq!(idx, inst.sorted_pc_list_index, "index mismatch @{at}");
            let mut tbl = MainTraceRowInstrTable::<Goldilocks>::default();
            tbl.packed.copy_from_slice(&table[idx * wpe..(idx + 1) * wpe]);

            // Runtime columns: reconstructed from the compact row.
            for limb in 0..2 {
                assert_eq!(full.get_a(lane, limb), compact.get_a(lane, limb), "a @{at}");
                assert_eq!(full.get_b(lane, limb), compact.get_b(lane, limb), "b @{at}");
                assert_eq!(full.get_c(lane, limb), compact.get_c(lane, limb), "c @{at}");
                assert_eq!(
                    full.get_store_reg_prev_value(lane, limb),
                    compact.get_store_reg_prev_value(lane, limb),
                    "store_val @{at}"
                );
            }
            assert_eq!(full.get_flag(lane), compact.get_flag(lane), "flag @{at}");
            assert_eq!(full.get_addr1(lane), compact.get_addr1(lane), "addr1 @{at}");
            assert_eq!(
                full.get_a_reg_prev_mem_step(lane),
                compact.get_a_reg_prev_mem_step(lane),
                "a_prev @{at}"
            );
            assert_eq!(
                full.get_b_reg_prev_mem_step(lane),
                compact.get_b_reg_prev_mem_step(lane),
                "b_prev @{at}"
            );
            assert_eq!(
                full.get_store_reg_prev_mem_step(lane),
                compact.get_store_reg_prev_mem_step(lane),
                "store_prev @{at}"
            );

            // Instruction-derived columns: reconstructed from that lane's table entry.
            assert_eq!(full.get_pc(lane), tbl.get_pc(), "pc @{at}");
            assert_eq!(full.get_a_src_imm(lane), tbl.get_a_src_imm(), "a_src_imm @{at}");
            assert_eq!(full.get_a_src_mem(lane), tbl.get_a_src_mem(), "a_src_mem @{at}");
            assert_eq!(full.get_a_src_reg(lane), tbl.get_a_src_reg(), "a_src_reg @{at}");
            assert_eq!(full.get_a_offset_imm0(lane), tbl.get_a_offset_imm0(), "a_off @{at}");
            assert_eq!(full.get_a_imm1(lane), tbl.get_a_imm1(), "a_imm1 @{at}");
            assert_eq!(full.get_is_precompiled(lane), tbl.get_is_precompiled(), "precomp @{at}");
            assert_eq!(full.get_b_src_imm(lane), tbl.get_b_src_imm(), "b_src_imm @{at}");
            assert_eq!(full.get_b_src_mem(lane), tbl.get_b_src_mem(), "b_src_mem @{at}");
            assert_eq!(full.get_b_src_reg(lane), tbl.get_b_src_reg(), "b_src_reg @{at}");
            assert_eq!(full.get_b_src_ind(lane), tbl.get_b_src_ind(), "b_src_ind @{at}");
            assert_eq!(full.get_b_offset_imm0(lane), tbl.get_b_offset_imm0(), "b_off @{at}");
            assert_eq!(full.get_b_imm1(lane), tbl.get_b_imm1(), "b_imm1 @{at}");
            assert_eq!(full.get_ind_width(lane), tbl.get_ind_width(), "ind_width @{at}");
            assert_eq!(full.get_is_external_op(lane), tbl.get_is_external_op(), "ext_op @{at}");
            assert_eq!(full.get_op(lane), tbl.get_op(), "op @{at}");
            assert_eq!(full.get_store_pc(lane), tbl.get_store_pc(), "store_pc @{at}");
            assert_eq!(full.get_store_mem(lane), tbl.get_store_mem(), "store_mem @{at}");
            assert_eq!(full.get_store_ind(lane), tbl.get_store_ind(), "store_ind @{at}");
            assert_eq!(full.get_store_reg(lane), tbl.get_store_reg(), "store_reg @{at}");
            assert_eq!(full.get_store_offset(lane), tbl.get_store_offset(), "store_off @{at}");
            assert_eq!(full.get_set_pc(lane), tbl.get_set_pc(), "set_pc @{at}");
            assert_eq!(full.get_jmp_offset1(lane), tbl.get_jmp_offset1(), "jmp1 @{at}");
            assert_eq!(full.get_jmp_offset2(lane), tbl.get_jmp_offset2(), "jmp2 @{at}");
            assert_eq!(full.get_m32(lane), tbl.get_m32(), "m32 @{at}");
        }
    }

    println!("indexed reconstruction verified for {} instructions", rom.sorted_pc_list.len());
}

/// The descriptor the C++/CUDA unpack consumes, pinned against the generated row.
#[test]
fn indexed_descriptor_covers_every_column() {
    type Ix = MainTraceRowPackedIndexed<Goldilocks>;

    assert_eq!(Ix::LANES, MAIN_LANES);
    assert_eq!(Ix::INDEX_BITS, 32);
    assert_eq!(Ix::COL_SOURCE.len(), Ix::COL_LANE.len());
    // 25 of the 34 declared fields are instruction-derived, one column per lane each.
    assert_eq!(Ix::COL_SOURCE.iter().filter(|&&s| s == 1).count(), 25 * MAIN_LANES);
    // Every column names a lane the row carries, and runtime columns name lane 0.
    assert!(Ix::COL_LANE.iter().all(|&l| (l as usize) < MAIN_LANES));
    assert!(Ix::COL_SOURCE.iter().zip(Ix::COL_LANE).all(|(&s, l)| s == 1 || l == 0));
    // The compact row must be smaller than the full one.
    const { assert!(Ix::PACKED_WORDS < MainTraceRowPacked::<Goldilocks>::PACKED_WORDS) };

    // `get_packed_info` pairs this descriptor with the FULL row's `unpack_info`, which the
    // unpack indexes per column (`unpack_indexed_row.hpp`). The widths agree because both
    // packings declare the same columns -- so pin the counts, or a column added to `traces.rs`
    // and not here reads past the end of the array.
    let main_unpack_info = zisk_pil::PACKED_INFO
        .iter()
        .find(|p| p.0 == zisk_pil::MAIN_AIRGROUP_ID && p.1 == zisk_pil::MAIN_AIR_ID)
        .expect("Main is in PACKED_INFO")
        .2
        .unpack_info;
    assert_eq!(Ix::COL_SOURCE.len(), main_unpack_info.len());
}
