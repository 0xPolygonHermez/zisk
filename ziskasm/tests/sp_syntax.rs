//! Parser checks around the `sp` register and indirections that hold with and without the
//! `sp` feature; the sp behaviour itself is tested in `sp.rs` (feature `sp`).

use ziskasm::parser::parse_program;

fn parse_err(line: &str) -> String {
    let src = format!("main:\n\t{line}\n\tret\n");
    parse_program(&src, "err").err().unwrap_or_else(|| panic!("`{line}` should not parse"))
}

fn parses(src: &str) -> bool {
    parse_program(src, "ok").is_ok()
}

#[test]
fn old_global_sp_modifier_is_rejected() {
    // sp is per operand (`[sp + N]`), never a modifier of the whole instruction.
    assert!(parse_err("add([0x100], 1) -> [0x108], sp").contains("no `sp` modifier"));
}

#[test]
fn sp_prefixed_symbols_are_not_sp() {
    // `spill` is a symbol, not the sp register.
    assert!(parses("define spill 0x100\nmain:\n\tcopyb(0, [spill]) -> r1\n\tret\n"));
}

#[test]
fn indirections_must_share_their_width() {
    // An instruction has a single ind_width, shared by the b and store indirections.
    assert!(parse_err("copyb(r1, 8[a + 0]) -> 4[a + 8]").contains("same width"));
    assert!(parses("main:\n\tcopyb(r1, 4[a + 0]) -> 4[a + 8]\n\tret\n"));
}

/// Without the feature nothing can change sp, so every sp construct is refused instead of
/// silently meaning sp = 0.
#[cfg(not(feature = "sp"))]
#[test]
fn sp_needs_the_feature() {
    for line in [
        "add(sp, 0) -> r1",
        "copyb(0, [sp + 8]) -> r1",
        "copyb(0, 1) -> [sp - 8]",
        "copyb(r1, 8[a + sp + 8]) -> r2",
        "copyb(r1, r2) -> 8[a + sp]",
        "copyb(0, 0x1000), setsp",
        "copyb(0, 0), incsp(8)",
    ] {
        assert!(parse_err(line).contains("`sp` feature"), "`{line}`");
    }
}
