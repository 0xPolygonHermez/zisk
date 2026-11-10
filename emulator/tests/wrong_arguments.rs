//! `emulate` must reject option combinations that can never produce a complete execution
//! before it loads anything or runs a single step.

use zisk_common::EmuTrace;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator, ZiskEmulatorErr};

fn exit_ok_elf() -> String {
    format!("{}/../elf-regressions/failed_execution/exit_ok.elf", env!("CARGO_MANIFEST_DIR"))
}

fn emulate(options: EmuOptions) -> Result<Vec<u8>, ZiskEmulatorErr> {
    ZiskEmulator.emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
}

/// A zero step budget cannot even reach the `end` instruction, so it is a wrong argument
/// rather than an incomplete emulation; with statistics it used to print an empty report
/// (and overflow the top PC histogram in debug builds) before failing.
#[test]
fn zero_max_steps_is_a_wrong_argument() {
    let options = EmuOptions {
        elf: Some(exit_ok_elf()),
        max_steps: 0,
        stats: true,
        top_histogram: 1,
        ..Default::default()
    };
    match emulate(options) {
        Err(ZiskEmulatorErr::WrongArguments(e)) => assert!(e.description.contains("-n"), "{e:?}"),
        other => panic!("max_steps 0 must be rejected as a wrong argument, got {other:?}"),
    }
}

/// The control: the same guest with the default step budget runs to completion.
#[test]
fn default_max_steps_completes() {
    let options = EmuOptions { elf: Some(exit_ok_elf()), ..Default::default() };
    assert!(emulate(options).is_ok());
}
