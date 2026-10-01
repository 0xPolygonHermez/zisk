//! Library mode: assemble `.zisk` as a set of callable functions at a fixed base
//! (no launcher / `_start` / BIOS), exporting the symbol table for the RISC-V
//! symbol-redirect merge.

use zisk_core::{ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR};
use ziskasm::{assemble_library, parser};

#[test]
fn assembles_functions_at_base_with_symbols() {
    // Two tiny functions plus a const and a rw variable.
    let src = "\
const u64 K = 0x1000
u64 SCRATCH[2] = 0, 0
zisklib_add:
\tadd(r10, r11) -> r10
\tret
zisklib_id:
\tcopyb(0, r10) -> r10
\tret
";
    let program = parser::parse_program(src, "lib").expect("parse");
    let lib = assemble_library(&program, ZISKLIB_ROM_ADDR, ZISKLIB_RAM_ADDR).expect("assemble");

    // Functions placed in file order at the ROM base.
    assert_eq!(lib.symbols["zisklib_add"], ZISKLIB_ROM_ADDR);
    assert_eq!(lib.symbols["zisklib_id"], ZISKLIB_ROM_ADDR + 8); // add(1) + ret(1) = 2 insts

    // const in ROM (after code, 32-aligned); rw variable at the RAM base.
    assert!(lib.symbols["K"] >= ZISKLIB_ROM_ADDR && lib.symbols["K"] < ZISKLIB_RAM_ADDR);
    assert_eq!(lib.symbols["SCRATCH"], ZISKLIB_RAM_ADDR);

    // 4 instructions total; every instruction address is inside the ROM region.
    assert_eq!(lib.insts.len(), 4);
    for &addr in lib.insts.keys() {
        assert!((ZISKLIB_ROM_ADDR..lib.symbols["K"]).contains(&addr));
    }

    // Data sections are present and 4-u64 aligned (provability constraint).
    for s in lib.ro_data.iter().chain(lib.rw_data.iter()) {
        assert_eq!(s.data.len() % 4, 0);
    }
}
