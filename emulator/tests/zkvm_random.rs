//! Host randomness (zkvm-standards, `zkvm_random.h`): `zkvm_random_u64` returns the
//! SHAKE256 output stream of the 256-bit seed the host provides with each execution,
//! which the ZisK library routine reads through the FCALL_RANDOM_SEED_ID fcall.
//!
//! The committed guest `elf-regressions/zkvm_random/zkvm_random.elf` (built from
//! `ziskasm/lang/c/example/zkvm_random_guest.c`) draws 500 values and outputs the first
//! 20, the XOR of all of them and its own checks (0 = all distinct, every bit varies).

use zisk_common::EmuTrace;
use zisk_core::{AsmGenerationMethod, ZiskRom2Asm};
use zisk_transpiler_common::{program2rom, program2romfile, HINTS_WITH_RANDOM_ERROR};
use ziskemu::{parse_random_seed, EmuOptions, Emulator, ZiskEmulator};

/// The 32 seed bytes 00 01 .. 1f: all distinct, so a byte or word order slip shows
const SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

/// SHAKE256 of SEED's bytes as little-endian u64 words (Python hashlib.shake_256): the
/// first 20 (word 17 starts the second squeezed block)
const STREAM: [u64; 20] = [
    0x0280ce40887cf069,
    0x5b3d2c883909b34d,
    0x13451ee3b3989cbc,
    0xd3cd03459bcad2eb,
    0x73712c454207c9c9,
    0x4ee16391c45aa7d4,
    0x72b23570ef24cce0,
    0x3f339b09f17a9ad1,
    0x785b5f9bd6657461,
    0xc9ec1c1b4a4e91ae,
    0x3fae301879d5f621,
    0x88b292029bee4b91,
    0x5f91bec4abec7c33,
    0x32066fff7b605314,
    0x6e4553ab8e3e7fca,
    0xdce01fd60a3047ba,
    0x3cba2be4176cf0eb,
    0x4a1a5f667155f0dc,
    0x5f8ca3b2e05f1b11,
    0xafe3d608d0866665,
];

/// ... and the XOR of its first 500 words
const STREAM_XOR: u64 = 0xf775d39d7e0fd4e5;

fn elf_path() -> String {
    format!("{}/../elf-regressions/zkvm_random/zkvm_random.elf", env!("CARGO_MANIFEST_DIR"))
}

fn seed() -> [u64; 4] {
    parse_random_seed(SEED).unwrap()
}

/// Runs the guest with `random_seed`, returning its 22 output words
fn emulate(random_seed: Option<[u64; 4]>) -> Vec<u64> {
    let options = EmuOptions { elf: Some(elf_path()), random_seed, ..Default::default() };
    let output = ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
        .expect("the guest must run to completion");
    output[..22 * 8].chunks(8).map(|w| u64::from_le_bytes(w.try_into().unwrap())).collect()
}

#[test]
fn returns_the_shake256_stream_of_the_seed() {
    let output = emulate(Some(seed()));
    assert_eq!(output[..20], STREAM);
    assert_eq!(output[20], STREAM_XOR);
    assert_eq!(output[21], 0, "in-guest checks: values not distinct (1) or a bit never varies (2)");
}

#[test]
fn another_seed_gives_another_stream() {
    let mut other = seed();
    other[3] ^= 1 << 63;
    let output = emulate(Some(other));
    assert_ne!(output[..20], STREAM);
    assert_eq!(output[21], 0, "in-guest checks: values not distinct (1) or a bit never varies (2)");
}

#[test]
fn minimal_traces_read_the_seed() {
    let elf = std::fs::read(elf_path()).expect("committed ELF");
    let rom = program2rom(&elf).expect("ELF must transpile");
    let options =
        EmuOptions { chunk_size: Some(1 << 18), random_seed: Some(seed()), ..Default::default() };
    let traces = ZiskEmulator::compute_minimal_traces(&rom, &[], &options, 2)
        .expect("minimal traces must succeed");
    assert!(!traces.is_empty());
}

#[test]
fn the_seed_is_read_as_its_bytes_in_order() {
    assert_eq!(
        seed(),
        [0x0706050403020100, 0x0f0e0d0c0b0a0908, 0x1716151413121110, 0x1f1e1d1c1b1a1918]
    );
    assert_eq!(parse_random_seed(&format!("0x{SEED}")), Ok(seed()));
    assert!(parse_random_seed(&SEED[2..]).is_err(), "31 bytes");
    assert!(parse_random_seed(&format!("{SEED}00")).is_err(), "33 bytes");
    assert!(parse_random_seed(&SEED.replace('a', "g")).is_err(), "not hex");
}

#[test]
#[should_panic(expected = "no random seed was provided")]
fn a_missing_seed_fails() {
    emulate(None);
}

/// The seed is not part of the precompile hints: when the ASM emulator consumes
/// precompile results, every other fcall reads its result from them, but the seed's
/// still calls the emulator, which provides it itself
#[test]
fn the_seed_never_comes_from_the_precompile_results() {
    let elf = std::fs::read(elf_path()).expect("committed ELF");
    let rom = program2rom(&elf).expect("ELF must transpile");
    let fcall_calls = |precompile_results| {
        let mut asm = String::new();
        ZiskRom2Asm::save_to_asm(
            &rom,
            &mut asm,
            AsmGenerationMethod::AsmMinimalTraces,
            false,
            false,
            precompile_results,
        );
        asm.matches("call _opcode_fcall").count()
    };
    // The ROM holds the whole ZisK library, so there are other fcalls besides the seed's
    assert!(fcall_calls(false) > 1);
    assert_eq!(fcall_calls(true), 1, "only the seed's fcall calls the emulator");
}

/// A guest that uses zkvm_random_u64 cannot run with hints: the native run that produces
/// them draws other random values, so its precompile calls may differ from the prover's.
/// The hints setup (the precompile-results assembly) must reject it
#[test]
fn the_hints_setup_rejects_a_guest_that_uses_it() {
    let asm_file = std::env::temp_dir().join(format!("zkvm_random_{}.asm", std::process::id()));
    let setup = |elf: &str, hints| {
        let elf = std::fs::read(elf).expect("committed ELF");
        let result = program2romfile(
            &elf,
            &asm_file,
            AsmGenerationMethod::AsmMinimalTraces,
            false,
            false,
            hints,
        )
        .map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&asm_file);
        result
    };
    assert_eq!(setup(&elf_path(), true), Err(HINTS_WITH_RANDOM_ERROR.to_string()));
    assert_eq!(setup(&elf_path(), false), Ok(()));
    // A guest that does not use it still gets the hints setup
    let other =
        format!("{}/../elf-regressions/failed_execution/exit_ok.elf", env!("CARGO_MANIFEST_DIR"));
    assert_eq!(setup(&other, true), Ok(()));
}
