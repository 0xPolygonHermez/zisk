#![no_main]

ziskos::entrypoint!(main);

mod arith256;
mod arith384;
mod blake2;
mod bls12_381;
mod bn254;
mod fcall;
mod keccakf;
mod poseidon2;
mod riscv_c;
mod riscv_fd;
mod riscv_ima;
mod secp256k1;
mod secp256r1;
mod sha256f;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
fn main() {}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
fn main() {
    // Basic instructions
    riscv_c::diagnostic_riscv_c();
    riscv_fd::diagnostic_riscv_fd();
    riscv_ima::diagnostic_riscv_ima();
    //riscv_ima::diagnostic_riscv_ima_combinations();

    // Free-input calls
    fcall::diagnostic_fcall();

    // Precompiles
    arith256::test_arith256();
    arith384::test_arith384();
    blake2::test_blake2();
    bls12_381::test_bls12_381();
    bn254::test_bn254();
    keccakf::test_keccakf();
    poseidon2::test_poseidon2();
    secp256k1::test_secp256k1();
    secp256r1::test_secp256r1();
    sha256f::test_sha256f();
    println!("Success");
}
