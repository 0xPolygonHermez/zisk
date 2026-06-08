use serial_test::serial;
use test_artifacts::{
    ELF_BIGINT, ELF_BLS12_381, ELF_BN254, ELF_SECP256K1, ELF_SECP256R1, ELF_UINT256,
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
fn execute_bigint() {
    ELF_BIGINT
        .run_emulation(ZiskStdin::new(), None)
        .expect("bigint zisklib guest emulation failed");
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
