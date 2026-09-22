//! Library mode: assemble `.zisk` as a set of callable functions at a fixed base
//! (no launcher / `_start` / BIOS), exporting the symbol table for the RISC-V
//! symbol-redirect merge.

use zisk_core::{RAM_ADDR, ROM_ADDR, STACK_GUARD_ADDR, ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR};
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

/// The span below RAM_ADDR is the EF standard 6 stack guard: unmapped, so a stack
/// overflow traps. `assemble_library` takes its bases from the caller, so it must
/// refuse to place data there rather than silently defeat the guard.
#[test]
fn rejects_library_bases_outside_the_mapped_regions() {
    let src = "u64 SCRATCH[2] = 0, 0\nzisklib_id:\n\tcopyb(0, r10) -> r10\n\tret\n";
    let program = parser::parse_program(src, "lib").expect("parse");

    // RAM base inside the guard region, and below it.
    for bad in [STACK_GUARD_ADDR, RAM_ADDR - 8, ROM_ADDR] {
        // `expect_err` needs Ok: Debug, which ZiskLibrary is not.
        let err = match assemble_library(&program, ZISKLIB_ROM_ADDR, bad) {
            Ok(_) => panic!("ram_base 0x{bad:x} below RAM_ADDR must be rejected"),
            Err(e) => e,
        };
        assert!(err.contains("stack guard"), "ram_base 0x{bad:x}: {err}");
    }

    // ROM base outside the ROM window.
    for bad in [RAM_ADDR, ROM_ADDR - 8] {
        let err = match assemble_library(&program, bad, ZISKLIB_RAM_ADDR) {
            Ok(_) => panic!("rom_base 0x{bad:x} outside ROM must be rejected"),
            Err(e) => e,
        };
        assert!(err.contains("outside the ROM region"), "rom_base 0x{bad:x}: {err}");
    }

    // The real bases must still work -- the guard must not over-reject.
    assert!(
        assemble_library(&program, ZISKLIB_ROM_ADDR, ZISKLIB_RAM_ADDR).is_ok(),
        "the production bases must remain valid"
    );
}
