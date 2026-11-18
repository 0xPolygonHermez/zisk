//! Fork-join regions from C (`zkvm_parallel.h`): `zkvm_parallel` runs its tasks as if one after
//! another. The Rust emulator does run them one after another, and checks at every pass of the
//! driver the invariant a parallel emulation relies on (each pass starts on a chunk boundary, in
//! the state of the first pass but for the task index); a region that breaks it panics.
//!
//! The committed guests of `elf-regressions/zkvm_parallel/` (built from
//! `ziskasm/lang/c/example/`):
//! - `zkvm_parallel.elf` runs SAXPY over four tasks that own their slice of the output, and outputs
//!   the sum of the result and its own check against a sequential run (0 = they match);
//! - `zkvm_parallel_fcsr.elf` has every task report the FCSR it starts with and leave another one,
//!   and outputs those and the FCSR after the region.

use zisk_common::EmuTrace;
use zisk_transpiler_common::program2rom;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

/// The sum of y = 3*x + y over 4096 words, x[i] = 7i + 1 and y[i] = i
const SUM: u64 = 184_516_608;

fn elf_path() -> String {
    format!("{}/../elf-regressions/zkvm_parallel/zkvm_parallel.elf", env!("CARGO_MANIFEST_DIR"))
}

/// The FCSR guest, and the FCSR its caller leaves before the region
fn fcsr_elf_path() -> String {
    format!(
        "{}/../elf-regressions/zkvm_parallel/zkvm_parallel_fcsr.elf",
        env!("CARGO_MANIFEST_DIR")
    )
}
const REGION_FCSR: u64 = 0x60;

#[test]
fn the_region_gives_what_running_its_tasks_in_order_gives() {
    let options = EmuOptions { elf: Some(elf_path()), ..Default::default() };
    let output = ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
        .expect("the guest must run to completion");
    let word = |i: usize| u64::from_le_bytes(output[8 * i..8 * i + 8].try_into().unwrap());
    assert_eq!(word(0), SUM);
    assert_eq!(word(1), 0, "in-guest check: the region's result differs from the sequential one");
}

/// The minimal traces go through the driver's invariant checks at every pass: the region pads
/// each task to a chunk of its own, so there is at least a chunk per task
#[test]
fn minimal_traces_hold_the_driver_invariant() {
    let elf = std::fs::read(elf_path()).expect("committed ELF");
    let rom = program2rom(&elf).expect("ELF must transpile");
    let options = EmuOptions { chunk_size: Some(1 << 18), ..Default::default() };
    let traces = ZiskEmulator::compute_minimal_traces(&rom, &[], &options, 2)
        .expect("minimal traces must succeed");
    assert!(traces.len() >= 4, "{} chunks for a region of 4 tasks", traces.len());
}

/// Floating-point code rewrites the FCSR, which lives in memory no task owns: every task starts
/// with the FCSR the region started with, and the caller gets it back after the region
#[test]
fn every_task_starts_with_the_regions_fcsr() {
    let options = EmuOptions { elf: Some(fcsr_elf_path()), ..Default::default() };
    let output = ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
        .expect("the guest must run to completion");
    let words: Vec<u64> =
        output[..5 * 8].chunks(8).map(|w| u64::from_le_bytes(w.try_into().unwrap())).collect();
    assert_eq!(words, [REGION_FCSR; 5], "4 tasks' starting FCSR, then the FCSR after the region");
}
