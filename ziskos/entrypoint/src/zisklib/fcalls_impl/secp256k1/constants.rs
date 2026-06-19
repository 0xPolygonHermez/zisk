use lazy_static::lazy_static;
use num_bigint::BigUint;

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
        16
    )
    .unwrap();

    pub static ref P_HALF: BigUint = BigUint::parse_bytes(
        b"7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe17",
        16
    )
    .unwrap();

    pub static ref P_DIV_4: BigUint = BigUint::parse_bytes(
        b"3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffff0c",
        16
    )
    .unwrap();

    pub static ref NQR: BigUint = BigUint::from(3u64); // First non-quadratic residue in Fp

    pub static ref N: BigUint = BigUint::parse_bytes(
        b"fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
        16
    )
    .unwrap();
}
