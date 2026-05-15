//! Zisklib guest-emulation tests. Lives in `tests/` (not `src/`) so it is not
//! pulled into the `ziskos-hints` crate via the `src/core` symlink; that would
//! create a dev-dependency cycle (test-artifacts -> zisk-sdk -> ... ->
//! precompiles-hints -> ziskos-hints) that duplicates `ziskos-hints`'s
//! `#[no_mangle]` C symbols at link time.

use serial_test::serial;
use test_artifacts::{
    ELF_BLS12_381, ELF_BN254, ELF_MODEXP, ELF_SECP256K1, ELF_SECP256R1, ELF_UINT256,
};
use zisk_common::io::ZiskStdin;

// Tests share a global lock (#[serial]) because each `run_emulation`
// allocates several GB; running them in parallel exceeds RAM.

#[test]
#[serial]
fn execute_bls12_381() {
    ELF_BLS12_381
        .run_emulation(ZiskStdin::new(), None)
        .expect("bls12_381 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_bn254() {
    ELF_BN254.run_emulation(ZiskStdin::new(), None).expect("bn254 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_modexp() {
    ELF_MODEXP
        .run_emulation(ZiskStdin::new(), None)
        .expect("modexp zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_secp256k1() {
    ELF_SECP256K1
        .run_emulation(ZiskStdin::new(), None)
        .expect("secp256k1 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_secp256r1() {
    ELF_SECP256R1
        .run_emulation(ZiskStdin::new(), None)
        .expect("secp256r1 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_uint256() {
    ELF_UINT256
        .run_emulation(ZiskStdin::new(), None)
        .expect("uint256 zisklib guest emulation failed");
}
