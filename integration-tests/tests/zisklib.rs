use serial_test::serial;
use zisk_sdk::{run, ZiskStdin};
use zisk_test_artifacts::{
    ELF_BIGINT, ELF_BLS12_381, ELF_BN254, ELF_SECP256K1, ELF_SECP256R1, ELF_UINT256,
    ELF_UNALIGNED_OUT,
};

// Tests share a global lock (#[serial]) because each emulation run
// allocates several GB; running them in parallel exceeds RAM.

#[test]
#[serial]
fn execute_bls12_381() {
    run(&ELF_BLS12_381, ZiskStdin::new(), None).expect("bls12_381 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_bn254() {
    run(&ELF_BN254, ZiskStdin::new(), None).expect("bn254 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_bigint() {
    run(&ELF_BIGINT, ZiskStdin::new(), None).expect("bigint zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_secp256k1() {
    run(&ELF_SECP256K1, ZiskStdin::new(), None).expect("secp256k1 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_secp256r1() {
    run(&ELF_SECP256R1, ZiskStdin::new(), None).expect("secp256r1 zisklib guest emulation failed");
}

#[test]
#[serial]
fn execute_uint256() {
    run(&ELF_UINT256, ZiskStdin::new(), None).expect("uint256 zisklib guest emulation failed");
}

/// Every zisklib C-ABI function with an output pointer, and the raw-pointer
/// `ziskos_jump_dest!` forms, with the output at byte offsets 0..7: the guest
/// panics if any offset writes other bytes than an aligned output.
#[test]
#[serial]
fn execute_unaligned_out() {
    let stdin = ZiskStdin::new();
    stdin.write(&0u64); // first case
    stdin.write(&u64::MAX); // last case: all of them
    run(&ELF_UNALIGNED_OUT, stdin, None).expect("unaligned_out zisklib guest emulation failed");
}
