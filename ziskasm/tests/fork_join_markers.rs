//! End-to-end test of the fork-join fcalls on the Rust emulator: the four markers (fork,
//! boundary, task end, join) do nothing and return nothing, and the padding fcall returns how many
//! steps are left in the chunk after it.

use zisk_common::EmuTrace;
use zisk_core::{par_pad_steps, CHUNK_SIZE};
use ziskasm::{assemble, parser::parse_program};
use ziskemu::{EmuOptions, ZiskEmulator};

/// Runs a hand-written `.zisk` program and returns its first `words` output words
fn run(src: &str, words: usize) -> Vec<u64> {
    let program = parse_program(src, "fork_join_markers").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");
    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");
    out[..words * 8].chunks(8).map(|w| u64::from_le_bytes(w.try_into().unwrap())).collect()
}

#[test]
fn pad_returns_the_steps_left_in_the_chunk() {
    // r5 = the step of the instruction right before the padding fcall
    let src = "\
define OUTPUT_ADDR 0xa0410000
define FREE_INPUT 0x10000000
main:
\tadd(step, 0) -> r5
\tfcall(31, 0) -> r14
\tcopyb(0, [FREE_INPUT]) -> r6
\tcopyb(0, OUTPUT_ADDR) -> r8
\tcopyb(r8, r5) -> 8[a + 0]
\tadd(r8, 8) -> r8
\tcopyb(r8, r6) -> 8[a + 0]
\tret
";
    let out = run(src, 2);
    let (step, pad) = (out[0] + 1, out[1]);
    assert_eq!(pad, par_pad_steps(step), "the padding fcall ran at step {step}");
    assert!(pad < CHUNK_SIZE);
    assert_eq!((step + 1 + pad) % CHUNK_SIZE, 0, "padding by it lands on a chunk boundary");
}

#[test]
fn a_marker_returns_nothing() {
    // The free input the padding fcall leaves must not leak through the fork after it. (A fork
    // takes the task table and the number of tasks; this region is never run.)
    let src = "\
define OUTPUT_ADDR 0xa0410000
define FREE_INPUT 0x10000000
main:
\tfcall(31, 0) -> r14
\tcopyb(0, [FREE_INPUT]) -> r5
\tfcall_param(1, 0)
\tfcall_param(1, 0)
\tfcall(28, 0) -> r14
\tcopyb(0, [FREE_INPUT]) -> r6
\tcopyb(0, OUTPUT_ADDR) -> r8
\tcopyb(r8, r5) -> 8[a + 0]
\tadd(r8, 8) -> r8
\tcopyb(r8, r6) -> 8[a + 0]
\tret
";
    let out = run(src, 2);
    assert_ne!(out[0], 0, "the padding fcall left a free input");
    assert_eq!(out[1], 0, "the fork returns nothing");
}

#[test]
#[should_panic(expected = "outside a region")]
fn a_pass_outside_a_region_is_rejected() {
    let src = "main:\n\tfcall(29, 0) -> r14\n\tret\n";
    run(src, 0);
}
