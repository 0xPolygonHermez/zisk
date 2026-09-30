//! Precompiles never raise the register flag, so proof generation requires their
//! `jmp_offset1` to be 0 — except the ops with a static argument (the DMA
//! "extended" ops and `blake2b`, whose round index travels there). The assembler
//! must not default it to `INST_SIZE` (the fall-through value used for regular ops).

use zisk_core::zisk_ops::ZiskOp;
use ziskasm::{assemble_files, collect_zisk_files};

#[test]
fn precompiles_have_zero_jmp_offset1_unless_static_arg() {
    let files = collect_zisk_files("programs/diagnostic").expect("collect diagnostic files");
    let rom = assemble_files(&files).expect("assemble diagnostic");

    // The ops with a static argument legitimately carry it in jmp_offset1.
    let static_arg_ops: Vec<u8> = ["dma_xmemcpy", "dma_xmemcmp", "dma_xmemset", "blake2b"]
        .iter()
        .map(|n| ZiskOp::try_from_name(n).unwrap().code())
        .collect();

    let mut checked = 0;
    for (addr, zib) in &rom.insts {
        let i = &zib.i;
        if i.is_precompiled && !static_arg_ops.contains(&i.op) {
            assert_eq!(
                i.jmp_offset1, 0,
                "precompile `{}` at 0x{addr:x} has jmp_offset1={} (must be 0 for proving)",
                i.op_str, i.jmp_offset1
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "expected the diagnostic to contain precompiles");
}
