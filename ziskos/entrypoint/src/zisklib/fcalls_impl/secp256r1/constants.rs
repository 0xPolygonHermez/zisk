use lazy_static::lazy_static;
use num_bigint::BigUint;

lazy_static! {
    /// secp256r1 base field prime.
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"ffffffff00000001000000000000000000000000ffffffffffffffffffffffff",
        16
    )
    .unwrap();
    /// secp256r1 group order.
    pub static ref N: BigUint = BigUint::parse_bytes(
        b"ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551",
        16
    )
    .unwrap();
}
