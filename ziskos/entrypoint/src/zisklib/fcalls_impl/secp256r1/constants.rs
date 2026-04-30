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

/// Curve parameter $a = -3 \bmod p$.
pub const E_A: [u64; 4] =
    [0xFFFF_FFFF_FFFF_FFFC, 0x0000_0000_FFFF_FFFF, 0x0000_0000_0000_0000, 0xFFFF_FFFF_0000_0001];

/// The point at infinity, represented as all zeros.
pub const IDENTITY: [u64; 8] = [0u64; 8];

/// secp256r1 generator point $G$ in affine coordinates `(x, y)`.
pub const G: [u64; 8] = [
    0xF4A1_3945_D898_C296,
    0x7703_7D81_2DEB_33A0,
    0xF8BC_E6E5_63A4_40F2,
    0x6B17_D1F2_E12C_4247,
    0xCBB6_4068_37BF_51F5,
    0x2BCE_3357_6B31_5ECE,
    0x8EE7_EB4A_7C0F_9E16,
    0x4FE3_42E2_FE1A_7F9B,
];
