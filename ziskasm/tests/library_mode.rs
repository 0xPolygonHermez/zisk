//! Library mode: assemble `.zisk` as a set of callable functions at a fixed base
//! (no launcher / `_start` / BIOS), exporting the symbol table that resolves the
//! zkvmcalls of a RISC-V guest.

use zisk_core::{
    RAM_ADDR, ROM_ADDR, ROM_ADDR_MAX, ROM_SIZE, STACK_GUARD_ADDR, ZISKLIB_RAM_ADDR,
    ZISKLIB_ROM_ADDR,
};
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

/// A valid base is not enough: the code and the `const` data after it must also end
/// inside ROM, and the RW data inside RAM, without wrapping around.
#[test]
fn rejects_libraries_that_run_past_their_region() {
    let rom_end = ROM_ADDR + ROM_SIZE;
    let code = "zisklib_id:\n\tcopyb(0, r10) -> r10\n\tret\n";
    let with_const = format!("const u64 K[4] = 1, 2, 3, 4\n{code}");
    let with_rw = format!("u64 SCRATCH[2] = 0, 0\n{code}");
    let parse = |src: &str| parser::parse_program(src, "lib").expect("parse");
    let err =
        |src: &str, rom_base, ram_base| match assemble_library(&parse(src), rom_base, ram_base) {
            Ok(_) => panic!("rom_base 0x{rom_base:x} / ram_base 0x{ram_base:x} must be rejected"),
            Err(e) => e,
        };

    // Two instructions from the last ROM byte: the second is past the end.
    assert!(err(code, ROM_ADDR_MAX, ZISKLIB_RAM_ADDR).contains("past the end of ROM"));
    // The code fits exactly, but the const data after it does not.
    assert!(err(&with_const, rom_end - 8, ZISKLIB_RAM_ADDR).contains("past the end of ROM"));
    // RW data whose end would wrap around the address space.
    assert!(err(&with_rw, ZISKLIB_ROM_ADDR, u64::MAX - 8).contains("overflows"));

    // Code that ends exactly at the end of ROM is fine.
    assert!(assemble_library(&parse(code), rom_end - 8, ZISKLIB_RAM_ADDR).is_ok());
}
