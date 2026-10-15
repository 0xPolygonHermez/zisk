mod bigint;
mod bls12_381;
mod bn254;
mod ed25519;
mod fcall_limits;
mod msb;
mod secp256k1;
mod secp256r1;
mod uint256;

pub fn diagnostic_fcalls() {
    bigint::diagnostic_bigint();
    bls12_381::diagnostic_bls12_381();
    bn254::diagnostic_bn254();
    ed25519::diagnostic_ed25519();
    fcall_limits::diagnostic_fcall_limits();
    msb::diagnostic_msb();
    secp256k1::diagnostic_secp256k1();
    secp256r1::diagnostic_secp256r1();
    uint256::diagnostic_uint256();

    println!("All free-input call diagnostics passed!");
}

// TODO: Add more tests
