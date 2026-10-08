//! Round-trip test: render the sample groups and assert every `#[constants]`/`#[emit]`
//! feature lands correctly in each target.

use zisk_definitions_generator::{meta, render, GenFile};

use super::{execution, memory, opcodes};

/// The demo groups, rendered by `sample_round_trips` to exercise every attribute.
const CONSTANT_SAMPLES: &[(&meta::GroupMeta, &[meta::Export])] = &[
    (&memory::GROUP, memory::EXPORTS),
    (&opcodes::GROUP, opcodes::EXPORTS),
    (&execution::GROUP, execution::EXPORTS),
];

fn contents<'a>(files: &'a [GenFile], name: &str) -> Option<&'a str> {
    files.iter().find(|f| f.name == name).map(|f| f.contents.as_str())
}

#[test]
fn sample_round_trips() {
    let files = render(CONSTANT_SAMPLES, "test").expect("render");

    let mem_h = contents(&files, "memory.gen.h").expect("memory.gen.h missing");
    let mem_pil = contents(&files, "memory.gen.pil").expect("memory.gen.pil missing");

    // `internal` is emitted as its own definition nowhere (it may still appear
    // inside a derived constant's provenance comment, which is intended).
    assert!(!mem_h.contains("#define STACK_SIZE"));
    assert!(!mem_pil.contains("const int STACK_SIZE"));

    // Derived value stamped as a literal, with its expression as provenance.
    assert!(mem_h.contains("#define SYS_ADDR"));
    assert!(mem_h.contains("(uint64_t)0xa0400000"));
    assert!(mem_h.contains("RAM_ADDR + STACK_SIZE"));

    // `skip(c)`: EXTRA_PARAMS_ADDR reaches PIL but not the C header.
    assert!(!mem_h.contains("#define EXTRA_PARAMS_ADDR"));
    assert!(mem_pil.contains("EXTRA_PARAMS_ADDR"));
    assert!(mem_pil.contains("0xA0400F00"));

    // asm: memory is `to(rust, c, pil, asm)`, so it also emits a GAS `.equ` include.
    let mem_inc = contents(&files, "memory.gen.inc").expect("memory.gen.inc missing");
    assert!(mem_inc.contains(".equ RAM_ADDR, 0xA0000000"));
    assert!(mem_inc.contains(".equ SYS_ADDR, 0xA0400000"));
    assert!(mem_inc.contains("# RAM_ADDR + STACK_SIZE")); // provenance comment

    // `skip(c)` only drops C: EXTRA_PARAMS_ADDR still inherits asm.
    assert!(mem_inc.contains(".equ EXTRA_PARAMS_ADDR, 0xA0400F00"));

    // Opcodes: PIL-only, with the `OP_` prefix.
    assert!(contents(&files, "opcodes.gen.h").is_none());
    let op_pil = contents(&files, "opcodes.gen.pil").expect("opcodes.gen.pil missing");
    assert!(op_pil.contains("const int OP_ADD = 0xA;"));
    assert!(op_pil.contains("const int OP_SUB = 0xB;"));

    // Execution: decimal radix override, and a 2^36 value in hex.
    let ex_pil = contents(&files, "execution.gen.pil").expect("execution.gen.pil missing");
    assert!(ex_pil.contains("= 36;"));
    assert!(ex_pil.contains("= 0x1000000000;"));
}

/// `usize`/`isize` are 64-bit for C, but the generated Rust keeps the pointer-sized
/// type so a length/index const stays usable as one.
#[test]
fn pointer_sized_types_survive_into_rust() {
    use zisk_definitions_macros::constants;

    #[constants(group = "sizes", to(rust, c))]
    mod sizes {
        pub const LEN: usize = 4;
        pub const OFF: isize = -2;
    }

    let files = render(&[(&sizes::GROUP, sizes::EXPORTS)], "test").expect("render");
    let rs = contents(&files, "sizes.rs").expect("sizes.rs missing");
    assert!(rs.contains("pub const LEN: usize = 0x4;"), "{rs}");
    assert!(rs.contains("pub const OFF: isize = -2;"), "{rs}");
    let h = contents(&files, "sizes.gen.h").expect("sizes.gen.h missing");
    assert!(h.contains("((uint64_t)0x4)") && h.contains("((int64_t)-2)"), "{h}");
}
