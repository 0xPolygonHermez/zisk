//! `end` stops only the emulators: the AIR never sees it, so in a proof an `end` row is an
//! ordinary instruction that continues to its jump targets. A failure exit must be a `halt`,
//! which the AIR cannot prove, and the assembler rejects `end` on any other operation.

use zisk_core::{ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR};
use ziskasm::{assemble_library, assemble_zisk_library, parser};

fn assemble(src: &str) -> Result<ziskasm::ZiskLibrary, String> {
    let program = parser::parse_program(src, "t").expect("parse");
    assemble_library(&program, ZISKLIB_ROM_ADDR, ZISKLIB_RAM_ADDR)
}

#[test]
fn end_is_rejected_on_any_operation_but_halt() {
    let err = assemble("f:\n\tcopyb(0, 0) -> r10, end\n").err().expect("copyb with end");
    assert!(err.contains("only `halt` may carry `end`"), "{err}");

    let lib = assemble("f:\n\thalt(0, 0), j(0, 0), end\n").expect("halt with end");
    let inst = &lib.insts[&ZISKLIB_ROM_ADDR].i;
    assert!(inst.end && inst.op_str == "halt");
}

#[test]
fn zisk_library_ends_only_on_halt() {
    let lib = assemble_zisk_library().expect("assemble the ZisK library");
    let ends = lib.insts.values().filter(|zib| zib.i.end).count();
    assert!(ends > 0, "expected the library's failure exits");
    for (addr, zib) in &lib.insts {
        assert!(!zib.i.end || zib.i.op_str == "halt", "`end` on `{}` at 0x{addr:x}", zib.i.op_str);
    }
}
