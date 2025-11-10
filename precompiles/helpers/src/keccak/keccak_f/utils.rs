use super::KeccakStateBits;

pub const fn bits_from_u64(value: u64) -> [bool; 64] {
    let mut bits = [false; 64];
    let mut i = 0;
    while i < 64 {
        bits[i] = (value >> i) & 1 == 1;
        i += 1;
    }
    bits
}

/// Convert from linear [u64; 25] to 5x5x64 bit array
pub fn state_from_linear(linear: &[u64; 25]) -> KeccakStateBits {
    let mut state = [[[0u64; 64]; 5]; 5];
    for x in 0..5 {
        for y in 0..5 {
            let word = linear[x + y * 5];
            for z in 0..64 {
                state[x][y][z] = (word >> z) & 1;
            }
        }
    }
    state
}

/// Convert from 5x5x64 bit array to linear [u64; 25]
pub fn state_to_linear(state: &KeccakStateBits) -> [u64; 25] {
    let mut linear = [0u64; 25];
    for x in 0..5 {
        for y in 0..5 {
            let mut word = 0u64;
            for z in 0..64 {
                if state[x][y][z] == 1 {
                    word |= 1u64 << z;
                }
            }
            linear[x + y * 5] = word;
        }
    }
    linear
}
