/// Round constants
pub(crate) const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808A,
    0x8000000080008000,
    0x000000000000808B,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008A,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000A,
    0x000000008000808B,
    0x800000000000008B,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800A,
    0x800000008000000A,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

pub(crate) const RC_BITS: [[bool; 64]; 24] = [
    bits_from_u64(RC[0]),
    bits_from_u64(RC[1]),
    bits_from_u64(RC[2]),
    bits_from_u64(RC[3]),
    bits_from_u64(RC[4]),
    bits_from_u64(RC[5]),
    bits_from_u64(RC[6]),
    bits_from_u64(RC[7]),
    bits_from_u64(RC[8]),
    bits_from_u64(RC[9]),
    bits_from_u64(RC[10]),
    bits_from_u64(RC[11]),
    bits_from_u64(RC[12]),
    bits_from_u64(RC[13]),
    bits_from_u64(RC[14]),
    bits_from_u64(RC[15]),
    bits_from_u64(RC[16]),
    bits_from_u64(RC[17]),
    bits_from_u64(RC[18]),
    bits_from_u64(RC[19]),
    bits_from_u64(RC[20]),
    bits_from_u64(RC[21]),
    bits_from_u64(RC[22]),
    bits_from_u64(RC[23]),
];

pub(crate) const RHO: [usize; 24] =
    [1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44];

pub(crate) const PI: [(usize, usize); 24] = [
    (0, 2),
    (2, 1),
    (1, 2),
    (2, 3),
    (3, 3),
    (3, 0),
    (0, 1),
    (1, 3),
    (3, 1),
    (1, 4),
    (4, 4),
    (4, 0),
    (0, 3),
    (3, 4),
    (4, 3),
    (3, 2),
    (2, 2),
    (2, 0),
    (0, 4),
    (4, 2),
    (2, 4),
    (4, 1),
    (1, 1),
    (1, 0),
];

const fn bits_from_u64(value: u64) -> [bool; 64] {
    let mut bits = [false; 64];
    let mut i = 0;
    while i < 64 {
        bits[i] = (value >> i) & 1 == 1;
        i += 1;
    }
    bits
}
