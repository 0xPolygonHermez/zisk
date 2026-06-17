mod arith256;
mod arith256_be;
mod arith384;
mod arith384_be;
mod blake2;
mod bls12_381;
mod bls12_381_be;
mod bn254;
mod bn254_be;
mod keccakf;
mod poseidon1;
mod poseidon2;
mod secp256k1;
mod secp256k1_be;
mod secp256r1;
mod secp256r1_be;
mod sha256f;

pub fn diagnostic_syscalls() {
    arith256::diagnostic_arith256();
    arith256_be::diagnostic_arith256_be();
    arith384::diagnostic_arith384();
    arith384_be::diagnostic_arith384_be();
    blake2::diagnostic_blake2();
    bls12_381::diagnostic_bls12_381();
    bls12_381_be::diagnostic_bls12_381_be();
    bn254::diagnostic_bn254();
    bn254_be::diagnostic_bn254_be();
    keccakf::diagnostic_keccakf();
    poseidon1::diagnostic_poseidon1();
    poseidon2::diagnostic_poseidon2();
    secp256k1::diagnostic_secp256k1();
    secp256k1_be::diagnostic_secp256k1_be();
    secp256r1::diagnostic_secp256r1();
    secp256r1_be::diagnostic_secp256r1_be();
    sha256f::diagnostic_sha256f();

    println!("All system call diagnostics passed!");
}
