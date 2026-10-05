//! A failed guest execution must end as failed, so that it is never proven
//! (zkvm-standards, Termination Semantics). Each guest in
//! `elf-regressions/failed_execution/` fails one way: a trap (`unimp`, `ebreak`,
//! `c.ebreak`), a write to a read-only CSR, or a nonzero exit code; `exit_ok` is the
//! control, which reads read-only CSRs and exits 0.
//!
//! Both emulator entry points must reject a failed execution: `emulate` (ziskemu,
//! `cargo-zisk execute`) and `compute_minimal_traces`, the first phase of proving, so
//! the prover stops before generating any proof. The ELFs are committed; their
//! sources and `build.sh` are next to them.

use zisk_common::EmuTrace;
use zisk_transpiler_common::elf2rom::elf2rom;
use ziskemu::{EmuOptions, Emulator, FailureReason, ZiskEmulator, ZiskEmulatorErr};

fn elf_path(name: &str) -> String {
    format!("{}/../elf-regressions/failed_execution/{name}.elf", env!("CARGO_MANIFEST_DIR"))
}

fn emulate(name: &str) -> Result<Vec<u8>, ZiskEmulatorErr> {
    let options = EmuOptions { elf: Some(elf_path(name)), ..Default::default() };
    ZiskEmulator.emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
}

fn minimal_traces(name: &str) -> Result<Vec<EmuTrace>, ZiskEmulatorErr> {
    let elf = std::fs::read(elf_path(name)).expect("committed ELF");
    let rom = elf2rom(&elf).expect("ELF must transpile");
    let options = EmuOptions { chunk_size: Some(1 << 18), ..Default::default() };
    ZiskEmulator::compute_minimal_traces(&rom, &[], &options, 2)
}

/// Both entry points must reject `name` for `reason`, returning (the failing pc of
/// emulate, that of minimal traces).
fn assert_fails(name: &str, reason: FailureReason) -> (u64, u64) {
    let emulate_pc = match emulate(name) {
        Err(ZiskEmulatorErr::ExecutionFailed { reason: r, pc, .. }) if r == reason => pc,
        other => panic!("{name}: emulate must fail with {reason:?}, got {other:?}"),
    };
    let traces_pc = match minimal_traces(name) {
        Err(ZiskEmulatorErr::ExecutionFailed { reason: r, pc, .. }) if r == reason => pc,
        other => panic!(
            "{name}: minimal traces must fail with {reason:?}, got {:?}",
            other.map(|t| t.len())
        ),
    };
    (emulate_pc, traces_pc)
}

/// A trap must be reported at the trapping instruction.
fn assert_fails_at(name: &str, failing_pc: u64) {
    assert_eq!(assert_fails(name, FailureReason::Trap), (failing_pc, failing_pc), "{name}");
}

#[test]
fn trap_fails() {
    // unimp is the first instruction
    assert_fails_at("trap_unimp", 0x8000_0000);
}

#[test]
fn ebreak_fails() {
    assert_fails_at("trap_ebreak", 0x8000_0000);
}

#[test]
fn compressed_ebreak_fails() {
    assert_fails_at("trap_c_ebreak", 0x8000_0000);
}

#[test]
fn write_to_read_only_csr_fails() {
    // li t0, 1; csrrs x0, mvendorid, t0
    assert_fails_at("write_ro_csr", 0x8000_0004);
}

#[test]
fn nonzero_exit_code_fails() {
    // exit(42): the error carries the code, and the exit code is checked in the BIOS
    // exit handler, not in the guest.
    let (pc, traces_pc) = assert_fails("exit_code", FailureReason::ExitCode(42));
    assert!(pc < 0x8000_0000, "exit_code: must fail in the BIOS, got pc={pc:#x}");
    assert_eq!(pc, traces_pc);
}

#[test]
fn successful_execution_is_accepted() {
    emulate("exit_ok").expect("exit_ok must succeed");
    minimal_traces("exit_ok").expect("exit_ok minimal traces must succeed");
}
