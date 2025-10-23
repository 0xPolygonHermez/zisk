//! Constants for the [Secp256k1](https://en.bitcoin.it/wiki/Secp256k1) elliptic curve

/// B parameter of the curve E: y² = x³ + 7
pub const E_B: [u64; 4] = [0x7, 0, 0, 0];

/// Secp256k1 base field size
pub const P: [u64; 4] =
    [0xFFFFFFFEFFFFFC2F, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF];
pub const P_MINUS_ONE: [u64; 4] = [P[0] - 1, P[1], P[2], P[3]];

/// A known non-quadratic residue in Fp
pub const NQR: [u64; 4] = [3, 0, 0, 0];

/// Secp256k1 scalar field size
pub const N: [u64; 4] =
    [0xBFD25E8CD0364141, 0xBAAEDCE6AF48A03B, 0xFFFFFFFFFFFFFFFE, 0xFFFFFFFFFFFFFFFF];
pub const N_MINUS_ONE: [u64; 4] = [N[0] - 1, N[1], N[2], N[3]];

/// Secp256k1 group of points generator
pub const G_X: [u64; 4] =
    [0x59F2815B16F81798, 0x029BFCDB2DCE28D9, 0x55A06295CE870B07, 0x79BE667EF9DCBBAC];
pub const G_Y: [u64; 4] =
    [0x9C47D08FFB10D4B8, 0xFD17B448A6855419, 0x5DA4FBFC0E1108A8, 0x483ADA7726A3C465];
