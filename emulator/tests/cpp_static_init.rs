//! C++ static-initialization regression test for the program-segment based ELF
//! interpreter.
//!
//! A C++ translation unit with static initializers is the one common case that
//! puts *executable* code, an `INIT_ARRAY` section and relocated data pointers
//! into the ROM/RAM `PT_LOAD` segments at once. This test pins down that
//! `collect_elf_payload_from_bytes` maps those segments the way the runtime
//! needs, and that a program relying on them actually produces the right result
//! end to end.
//!
//! The guest is committed at `elf-regressions/prebuilt-elfs/cpp_static_init.elf`
//! so this test needs no C++ cross toolchain. Sources and the regeneration
//! script live in `elf-regressions/cpp_static_init/` — after changing them, run
//! `elf-regressions/cpp_static_init/build.sh` and update `EXPECTED` below.

use zisk_transpiler_common::elf_extraction::{collect_elf_payload_from_bytes, validate_entry_point};
use zisk_core::mem::{RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_SIZE};
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

const ELF_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../elf-regressions/prebuilt-elfs/cpp_static_init.elf");

/// Output words the guest writes, in order. See `main.cpp` for what each one
/// proves; the leading word is the count written by the guest itself.
const EXPECTED: &[u32] = &[
    0x0000_0010, // 16 words follow
    0xa000_0001, // init_priority(101) ctor
    0xa000_0002, // init_priority(102) ctor — priorities kept in order
    0xc000_0011, // unprioritised global ctor, after the prioritised ones
    0xe000_0000, // ctor that writes pointers into another global
    0xf000_030e, // ctor summing a const table read from ROM
    0x0000_0001, // main reached
    0x0000_0011, // g_counter.v — ctor really ran
    0x0000_444c, // dynamically initialised global
    0x0000_de81, // virtual dispatch through the ROM vtable
    0x0000_0003, // node list linked by a ctor
    0xc000_0022, // function-local static constructed on first use (.bss guard)
    0x0000_0022, // ...and its value
    0x0000_030e, // const-table sum visible in main
    0x0000_ffff, // main done
    0xd000_0022, // destructors, reverse registration order
    0xd000_0011,
];

/// The whole ELF must be accepted and land in the windows the runtime expects:
/// code and `.init_array` in ROM, writable data in RAM.
#[test]
fn payload_places_init_array_in_rom() {
    let elf = std::fs::read(ELF_PATH).expect("committed cpp_static_init.elf");
    let payload = collect_elf_payload_from_bytes(&elf).expect("C++ ELF must be accepted");
    validate_entry_point(&payload).expect("entry point must be inside an exec segment");

    let in_rom =
        |addr: u64, len: usize| addr >= ROM_ADDR && addr + len as u64 <= ROM_ADDR + ROM_SIZE;
    let in_ram =
        |addr: u64, len: usize| addr >= RAM_ADDR && addr + len as u64 <= RAM_ADDR + RAM_SIZE;

    assert_eq!(payload.exec.len(), 1, "expected a single code segment");
    for s in &payload.exec {
        assert!(in_rom(s.addr, s.data.len()), "code segment 0x{:x} must be in ROM", s.addr);
        assert_eq!(s.data.len() % 2, 0, "code segment must hold whole instruction units");
    }

    // `.rodata` + `.init_array` share one read-only segment; the constructor
    // pointers are loaded from ROM at run time, so that segment must be there.
    assert!(!payload.ro.is_empty(), "C++ static init needs a read-only ROM segment");
    for s in &payload.ro {
        assert!(in_rom(s.addr, s.data.len()), "RO segment 0x{:x} must be in ROM", s.addr);
    }

    // `.bss` holds the local-static guard variable. Its segment is not
    // file-backed at all (`p_filesz == 0`), so the interpreter must materialize
    // it from the `p_memsz` zero-fill tail — i.e. an all-zero writable section.
    assert!(
        payload.rw.iter().any(|s| !s.data.is_empty() && s.data.iter().all(|&b| b == 0)),
        "expected a zero-filled writable region (.bss) materialized from p_memsz"
    );
    for s in &payload.rw {
        assert!(in_ram(s.addr, s.data.len()), "RW segment 0x{:x} must be in RAM", s.addr);
    }
}

/// End-to-end: constructors run before `main`, in priority order, and
/// destructors run after it.
#[test]
fn constructors_and_destructors_run() {
    let options = EmuOptions { elf: Some(ELF_PATH.to_string()), ..Default::default() };
    let output = ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(zisk_common::EmuTrace)>>)
        .expect("emulation must succeed");

    let words: Vec<u32> =
        output.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();

    assert!(words.len() >= EXPECTED.len(), "output too short: {} words", words.len());
    assert_eq!(
        &words[..EXPECTED.len()],
        EXPECTED,
        "static initialization trace changed; see elf-regressions/cpp_static_init/main.cpp"
    );
    assert!(
        words[EXPECTED.len()..].iter().all(|&w| w == 0),
        "guest wrote more output words than expected"
    );
}
