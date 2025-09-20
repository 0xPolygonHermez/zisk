/// Base field size
pub const P: [u64; 6] = [
    0xB9FEFFFFFFFFAAAB,
    0x1EABFFFEB153FFFF,
    0x6730D2A0F6B0F624,
    0x64774B84F38512BF,
    0x4B1BA7B6434BACD7,
    0x1A0111EA397FE69A,
];

/// Base field size minus one
pub const P_MINUS_ONE: [u64; 6] = [P[0] - 1, P[1], P[2], P[3], P[4], P[5]];

/// Scalar field size
pub const R: [u64; 4] =
    [0xFFFFFFFF00000001, 0x53BDA402FFFE5BFE, 0x3339D80809A1D805, 0x73EDA753299D7D48];

/// Scalar field size minus one
pub const R_MINUS_ONE: [u64; 4] = [R[0] - 1, R[1], R[2], R[3]];

/// A known non-quadratic residue in Fp
pub const NQR: [u64; 6] = [2, 0, 0, 0, 0, 0];

/// R = 2³⁸⁴ mod p
#[allow(dead_code)]
pub const R_MONT: [u64; 6] = [
    0x7609_0000_0002_fffd,
    0xebf4_000b_c40c_0002,
    0x5f48_9857_53c7_58ba,
    0x77ce_5853_7052_5745,
    0x5c07_1a97_a256_ec6d,
    0x15f6_5ec3_fa80_e493,
];

/// R² = 2⁷⁶⁸ mod p
pub const R2_MONT: [u64; 6] = [
    0xf4df_1f34_1c34_1746,
    0x0a76_e6a6_09d1_04f1,
    0x8de5_476c_4c95_b6d5,
    0x67eb_88a9_939d_83c0,
    0x9a79_3e85_b519_952d,
    0x1198_8fe5_92ca_e3aa,
];

/// R⁻¹ = 2⁻³⁸⁴ mod p
#[allow(dead_code)]
pub const R_INV_MONT: [u64; 6] = [
    0xf4d3_8259_380b_4820,
    0x7fe1_1274_d898_fafb,
    0x343e_a979_1495_6dc8,
    0x1797_ab14_58a8_8de9,
    0xed5e_6427_3c4f_538b,
    0x14fe_c701_e8fb_0ce9,
];
