//! Free-input calls from C (`zkvm_fcall.h`): `zkvm_fcall` passes the parameters and issues the
//! hint, and `zkvm_fcall_get` / `zkvm_fcall_read` read its results (`src/zkvm_fcall.s` of the
//! ziskasm C binding).
//!
//! The committed guest `elf-regressions/zkvm_fcall/zkvm_fcall.elf` (built from
//! `ziskasm/lang/c/example/zkvm_fcall_guest.c`) issues a few hints and outputs their results.
//! The same functions compiled for wasm are covered by `wasm_zkvm.rs`.

use zisk_common::EmuTrace;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

#[test]
fn hints_return_their_results_in_order() {
    let elf =
        format!("{}/../elf-regressions/zkvm_fcall/zkvm_fcall.elf", env!("CARGO_MANIFEST_DIR"));
    let options = EmuOptions { elf: Some(elf), ..Default::default() };
    let output = ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
        .expect("the guest must run to completion");
    let words: Vec<u64> =
        output[..23 * 8].chunks(8).map(|w| u64::from_le_bytes(w.try_into().unwrap())).collect();
    assert_eq!(words[0], 7, "ID 27, ID 0 and too many parameters each return ZKVM_EFAIL");
    // BN254_FP_INV(2) = (p + 1) / 2
    assert_eq!(
        words[1..5],
        [0x9e10460b6c3e7ea4, 0xcbc0b548b438e546, 0xdc2822db40c0ac2e, 0x183227397098d014]
    );
    // UINT256_DIV(2^200 + 1000, 7)
    assert_eq!(words[5..9], [0x49249249249249b3, 0x2492492492492492, 0x9249249249249249, 0x24]);
    assert_eq!(words[9..13], [3, 0, 0, 0]);
    // BIN_DECOMP(0b1011): 4 digits, most significant first
    assert_eq!(words[13..18], [4, 1, 0, 1, 1]);
    // SECP256K1_FP_SQRT(4, even): a square, with root 2
    assert_eq!(words[18..23], [1, 2, 0, 0, 0]);
}
