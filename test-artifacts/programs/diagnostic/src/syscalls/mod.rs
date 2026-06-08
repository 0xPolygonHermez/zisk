mod arith256;
mod arith384;
mod blake2;
mod bls12_381;
mod bn254;
mod keccakf;
mod poseidon1;
mod poseidon2;
mod secp256k1;
mod secp256r1;
mod sha256f;

pub fn diagnostic_syscalls() {
    arith256::diagnostic_arith256();
    arith384::diagnostic_arith384();
    blake2::diagnostic_blake2();
    bls12_381::diagnostic_bls12_381();
    bn254::diagnostic_bn254();
    keccakf::diagnostic_keccakf();
    poseidon2::diagnostic_poseidon2();
    secp256k1::diagnostic_secp256k1();
    secp256r1::diagnostic_secp256r1();
    sha256f::diagnostic_sha256f();

    println!("All system call diagnostics passed!");
}
