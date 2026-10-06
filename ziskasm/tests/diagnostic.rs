//! Runs the ZisK diagnostic program (`ziskasm/programs/diagnostic`) and checks it
//! reports success. The diagnostic writes `output[0] = 0` when every test passes,
//! or the first failing sub-check's error code (`0xFFCC`) otherwise.

use std::path::PathBuf;

use zisk_common::EmuTrace;
use ziskasm::{assemble_files, collect_zisk_files};
use ziskemu::{EmuOptions, ZiskEmulator};

fn diag_dir() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "programs", "diagnostic"].iter().collect()
}

#[test]
fn diagnostic_reports_success() {
    // Assemble every .zisk file in the diagnostic directory (like `ziskemu -z`).
    let files = collect_zisk_files(diag_dir().to_str().unwrap()).expect("collect diagnostic files");
    let rom = assemble_files(&files).expect("assembly should succeed");

    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");

    let code = u64::from_le_bytes(out[0..8].try_into().unwrap());
    assert_eq!(code, 0, "diagnostic reported failure code 0x{code:04x}");
}
