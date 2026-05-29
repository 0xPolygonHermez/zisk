/// Secp256k1 base field size
#[allow(dead_code)]
pub const P: [u64; 4] =
    [0xFFFFFFFEFFFFFC2F, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF];

/// Secp256k1 scalar field size
#[allow(dead_code)]
pub const N: [u64; 4] =
    [0xBFD25E8CD0364141, 0xBAAEDCE6AF48A03B, 0xFFFFFFFFFFFFFFFE, 0xFFFFFFFFFFFFFFFF];

/// Secp256k1 group identity point
pub const IDENTITY_X: [u64; 4] = [0; 4];
pub const IDENTITY_Y: [u64; 4] = [0; 4];
pub const IDENTITY: [u64; 8] = [
    IDENTITY_X[0],
    IDENTITY_X[1],
    IDENTITY_X[2],
    IDENTITY_X[3],
    IDENTITY_Y[0],
    IDENTITY_Y[1],
    IDENTITY_Y[2],
    IDENTITY_Y[3],
];

/// Secp256k1 group of points generator
pub const G_X: [u64; 4] =
    [0x59F2815B16F81798, 0x029BFCDB2DCE28D9, 0x55A06295CE870B07, 0x79BE667EF9DCBBAC];
pub const G_Y: [u64; 4] =
    [0x9C47D08FFB10D4B8, 0xFD17B448A6855419, 0x5DA4FBFC0E1108A8, 0x483ADA7726A3C465];
pub const G: [u64; 8] = [G_X[0], G_X[1], G_X[2], G_X[3], G_Y[0], G_Y[1], G_Y[2], G_Y[3]];
pub const G_NEG: [u64; 8] = [
    G_X[0],
    G_X[1],
    G_X[2],
    G_X[3],
    0x63b82f6f04ef2777,
    0x2e84bb7597aabe6,
    0xa25b0403f1eef757,
    0xb7c52588d95c3b9a,
];
