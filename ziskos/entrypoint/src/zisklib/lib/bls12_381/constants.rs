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
