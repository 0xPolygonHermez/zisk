//! A failed guest execution must end as failed, so that it is never proven
//! (zkvm-standards, Termination Semantics). Each guest in
//! `elf-regressions/failed_execution/` fails one way: a trap (`unimp`), a write to a
//! read-only CSR, or a nonzero exit code; `exit_ok` is the control, which reads
//! read-only CSRs and exits 0.
//!
//! Both emulator entry points must reject a failed execution: `emulate` (ziskemu,
//! `cargo-zisk execute`) and `compute_minimal_traces`, the first phase of proving, so
//! the prover stops before generating any proof. The ELFs are committed; their
//! sources and `build.sh` are next to them.

use zisk_common::EmuTrace;
use zisk_transpiler_common::elf2rom::elf2rom;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator, ZiskEmulatorErr};

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

/// The failure must be reported at the failing instruction.
fn assert_fails_at(name: &str, failing_pc: u64) {
    match emulate(name) {
        Err(ZiskEmulatorErr::ExecutionFailed { pc, .. }) => {
            assert_eq!(pc, failing_pc, "{name}: emulate failed at the wrong pc")
        }
        other => panic!("{name}: emulate must fail, got {other:?}"),
    }
    match minimal_traces(name) {
        Err(ZiskEmulatorErr::ExecutionFailed { pc, .. }) => {
            assert_eq!(pc, failing_pc, "{name}: minimal traces failed at the wrong pc")
        }
        other => panic!("{name}: minimal traces must fail, got {:?}", other.map(|t| t.len())),
    }
}

// The reported pc is the one after the halting instruction.

#[test]
fn trap_fails() {
    assert_fails_at("trap_unimp", 0x8000_0004);
}

#[test]
fn write_to_read_only_csr_fails() {
    assert_fails_at("write_ro_csr", 0x8000_0008);
}

#[test]
fn nonzero_exit_code_fails() {
    // The exit code is checked in the BIOS exit handler, not in the guest.
    match emulate("exit_code") {
        Err(ZiskEmulatorErr::ExecutionFailed { pc, .. }) => {
            assert!(pc < 0x8000_0000, "exit_code: must fail in the BIOS, got pc={pc:#x}")
        }
        other => panic!("exit_code: emulate must fail, got {other:?}"),
    }
    assert!(
        matches!(minimal_traces("exit_code"), Err(ZiskEmulatorErr::ExecutionFailed { .. })),
        "exit_code: minimal traces must fail"
    );
}

#[test]
fn successful_execution_is_accepted() {
    emulate("exit_ok").expect("exit_ok must succeed");
    minimal_traces("exit_ok").expect("exit_ok minimal traces must succeed");
}
