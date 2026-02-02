use lazy_static::lazy_static;
use num_bigint::BigUint;

lazy_static! {
    pub(crate) static ref P: BigUint = BigUint::parse_bytes(
        b"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        16
    )
    .unwrap();

    pub static ref P_HALF: BigUint = BigUint::parse_bytes(
        b"d0088f51cbff34d258dd3db21a5d66bb23ba5c279c2895fb39869507b587b120f55ffff58a9ffffdcff7fffffffd555",
        16
    )
    .unwrap();

    pub static ref P_DIV_4: BigUint = BigUint::parse_bytes(
        b"680447a8e5ff9a692c6e9ed90d2eb35d91dd2e13ce144afd9cc34a83dac3d8907aaffffac54ffffee7fbfffffffeaab",
        16
    )
    .unwrap();

    pub static ref P_MINUS_3_DIV_4: BigUint = BigUint::parse_bytes(
        b"680447A8E5FF9A692C6E9ED90D2EB35D91DD2E13CE144AFD9CC34A83DAC3D8907AAFFFFAC54FFFFEE7FBFFFFFFFEAAA",
        16
    )
    .unwrap();

    pub static ref P_MINUS_1_DIV_2: BigUint = BigUint::parse_bytes(
        b"D0088F51CBFF34D258DD3DB21A5D66BB23BA5C279C2895FB39869507B587B120F55FFFF58A9FFFFDCFF7FFFFFFFD555",
        16
    )
    .unwrap();

    pub static ref NQR_FP: BigUint = BigUint::from(2u64); // First non-quadratic residue in Fp
}

pub const ONE: [u64; 12] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub const P_MINUS_ONE: [u64; 12] = [
    0xB9FEFFFFFFFFAAAA,
    0x1EABFFFEB153FFFF,
    0x6730D2A0F6B0F624,
    0x64774B84F38512BF,
    0x4B1BA7B6434BACD7,
    0x1A0111EA397FE69A,
    0,
    0,
    0,
    0,
    0,
    0,
];

pub const I: [u64; 12] = [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]; // 0 + 1*u

pub const NQR_FP2: [u64; 12] = [1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]; // 1 + 1*u, a known non-quadratic residue in Fp2
