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

/// Secp256k1 group identity point
pub const IDENTITY: [u64; 8] = [0; 8];
pub const IDENTITY_X: [u64; 4] = [0; 4];
pub const IDENTITY_Y: [u64; 4] = [0; 4];

/// Secp256k1 group of points generator
pub const G_X: [u64; 4] =
    [0x59F2815B16F81798, 0x029BFCDB2DCE28D9, 0x55A06295CE870B07, 0x79BE667EF9DCBBAC];
pub const G_Y: [u64; 4] =
    [0x9C47D08FFB10D4B8, 0xFD17B448A6855419, 0x5DA4FBFC0E1108A8, 0x483ADA7726A3C465];
pub const G_NEG_Y: [u64; 4] =
    [0x63B82F6F04EF2777, 0x02E84BB7597AABE6, 0xA25B0403F1EEF757, 0xB7C52588D95C3B9A];
pub const G: [u64; 8] = [G_X[0], G_X[1], G_X[2], G_X[3], G_Y[0], G_Y[1], G_Y[2], G_Y[3]];

/// GLV endomorphism constants for secp256k1.
///
/// The curve admits the endomorphism φ : (x, y) ↦ (β·x, y) where β is a primitive cube root of
/// unity in Fp. For any point P of order n, φ(P) = [λ]P where λ is a primitive cube root of
/// unity in Fn. A scalar k can then be split into (k₁, k₂) with |k₁|, |k₂| < 2¹²⁸ such that
/// k ≡ k₁ + k₂·λ (mod n), enabling a 2× speedup of scalar multiplication.
pub const BETA: [u64; 4] =
    [0xC1396C28719501EE, 0x9CF0497512F58995, 0x6E64479EAC3434E9, 0x7AE96A2B657C0710];
pub const LAMBDA: [u64; 4] =
    [0xDF02967C1B23BD72, 0x122E22EA20816678, 0xA5261C028812645A, 0x5363AD4CC05C30E0];

/// The point φ(G) = (β·Gx, Gy) = (G_PHI_X, G_PHI_Y)
pub const G_PHI_X: [u64; 4] =
    [0xA7BBA04400B88FCB, 0x872844067F15E98D, 0xAB0102B696902325, 0xBCACE2E99DA01887];
pub const G_PHI_Y: [u64; 4] = G_Y;
