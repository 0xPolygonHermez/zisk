//! Constants for the [Secp256r1](https://csrc.nist.gov/pubs/sp/800/186/final) elliptic curve

/// B parameter of the curve E: y² = x³ + a·x + b
pub const E_A: [u64; 4] =
    [0xFFFF_FFFF_FFFF_FFFC, 0x0000_0000_FFFF_FFFF, 0x0000_0000_0000_0000, 0xFFFF_FFFF_0000_0001];
pub const E_B: [u64; 4] =
    [0x3BCE_3C3E_27D2_604B, 0x651D_06B0_CC53_B0F6, 0xB3EB_BD55_7698_86BC, 0x5AC6_35D8_AA3A_93E7];

/// Secp256r1 base field size
pub const P: [u64; 4] =
    [0xFFFF_FFFF_FFFF_FFFF, 0x0000_0000_FFFF_FFFF, 0x0000_0000_0000_0000, 0xFFFF_FFFF_0000_0001];
pub const P_MINUS_ONE: [u64; 4] = [P[0] - 1, P[1], P[2], P[3]];

/// Secp256r1 scalar field size
pub const N: [u64; 4] =
    [0xF3B9_CAC2_FC63_2551, 0xBCE6_FAAD_A717_9E84, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_0000_0000];
pub const N_MINUS_ONE: [u64; 4] = [N[0] - 1, N[1], N[2], N[3]];

/// Secp256r1 group identity point
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

/// Secp256r1 group of points generator
pub const G_X: [u64; 4] =
    [0xF4A1_3945_D898_C296, 0x7703_7D81_2DEB_33A0, 0xF8BC_E6E5_63A4_40F2, 0x6B17_D1F2_E12C_4247];
pub const G_Y: [u64; 4] =
    [0xCBB6_4068_37BF_51F5, 0x2BCE_3357_6B31_5ECE, 0x8EE7_EB4A_7C0F_9E16, 0x4FE3_42E2_FE1A_7F9B];
