//! BLAKE2f final flag (EIP-152, `zkvm_blake2f` in zkvm_accelerators.h): only f = 0 or 1
//! is valid, and any other flag must return ZKVM_EFAIL and leave h untouched.
//!
//! The committed guest `elf-regressions/blake2f_flag/blake2f_flag.elf` (built from
//! `ziskasm/lang/c/example/blake2f_flag_guest.c`) calls the zkvmcall thunk, which the
//! transpiler turns into a jump to the ziskasm routine `ziskasm_zkvm_blake2f`. For each
//! case it outputs the returned status and whether h is still the initial state.

use zisk_common::EmuTrace;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

const EOK: u32 = 0;
const EFAIL: u32 = -1i32 as u32;

/// The guest's cases, in order: (rounds, f, expected status, h untouched)
const CASES: [(u32, u8, u32, bool); 6] = [
    (12, 0, EOK, false),
    (12, 1, EOK, false),
    (12, 2, EFAIL, true),
    (12, 0x80, EFAIL, true),
    (12, 0xff, EFAIL, true),
    (0, 2, EFAIL, true),
];

fn elf_path() -> String {
    format!("{}/../elf-regressions/blake2f_flag/blake2f_flag.elf", env!("CARGO_MANIFEST_DIR"))
}

/// Runs the guest, returning its output words
fn emulate() -> Vec<u32> {
    let options = EmuOptions { elf: Some(elf_path()), ..Default::default() };
    let output = ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
        .expect("the guest must run to completion");
    output.chunks(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect()
}

#[test]
fn an_invalid_final_flag_fails_and_leaves_h_untouched() {
    let output = emulate();
    for (i, &(rounds, f, status, untouched)) in CASES.iter().enumerate() {
        let case = format!("rounds = {rounds}, f = {f:#x}");
        assert_eq!(output[2 * i], status, "status, {case}");
        assert_eq!(output[2 * i + 1], untouched as u32, "h untouched, {case}");
    }
}
