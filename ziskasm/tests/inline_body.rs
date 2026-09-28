//! `ZiskLibrary::inline_body`: the control-flow graph of a routine that the
//! transpiler expands at an inline zkvmcall.

use zisk_core::{ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR};
use ziskasm::{assemble_library, parser, ZiskLibrary};

fn library(src: &str) -> ZiskLibrary {
    let program = parser::parse_program(src, "lib").expect("parse");
    assemble_library(&program, ZISKLIB_ROM_ADDR, ZISKLIB_RAM_ADDR).expect("assemble")
}

/// Branches are followed (into another routine's code too), `jump`s and `ret`s
/// are dropped with their predecessors pointed at their targets (`None` = the end
/// of the call site), and the entry comes first.
#[test]
fn branches_become_a_graph_without_jumps_or_rets() {
    let lib = library(
        "\
shared:
\tcopyb(0, 7) -> [OUT]
\tret
f:
\tcopyb(r10, 8[a + 0]) -> r32
\teq(r32, 0), j(zero)
\tjump(shared)
zero:
\tcopyb(r11, 0) -> 8[a + 0]
\tret
u64 OUT = 0
",
    );
    let body = lib.inline_body("f", 2).expect("inline body");
    assert_eq!(body.insts.len(), 4, "the load, the test, and one store per path");
    assert!(body.insts.iter().all(|i| !i.set_pc), "no jump or ret is kept");
    // 0 = the load (entry), then address order: 1 = shared's store, 2 = the test,
    // 3 = zero's store.
    assert_eq!(body.insts[0].op_str, "copyb");
    assert_eq!(body.next, vec![[Some(2), Some(2)], [None, None], [Some(3), Some(1)], [None, None]]);
}

#[test]
fn rejects_what_would_clobber_the_caller() {
    let lib = library(
        "\
reads_r12:
\tcopyb(0, r12) -> r32
\tret
writes_r5:
\tcopyb(0, r10) -> r5
\tret
calls:
\tcall reads_r12
\tret
starts_with_c:
\tcopyb(0, c) -> r32
\tret
jumps_to_c:
\tjump(uses_c)
uses_c:
\tcopyb(0, c) -> r32
\tret
",
    );
    for (name, msg) in [
        ("reads_r12", "reads r12"),
        ("writes_r5", "writes a register outside r32..r39"),
        ("calls", "a call"),
        ("starts_with_c", "starts by reading c"),
        ("jumps_to_c", "a jump target reads c"),
    ] {
        let err = lib.inline_body(name, 2).err().unwrap_or_else(|| panic!("{name} accepted"));
        assert!(err.contains(msg), "{name}: {err}");
    }
}
