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
#[allow(clippy::needless_range_loop)]
pub fn keccakf_state_from_linear(linear: &[u64; 25]) -> KeccakStateBits {
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
#[allow(clippy::needless_range_loop)]
pub fn keccakf_state_to_linear(state: &KeccakStateBits) -> [u64; 25] {
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

#[allow(clippy::needless_range_loop)]
pub fn keccakf_state_to_linear_1d(state: &KeccakStateBits) -> [u64; 1600] {
    let mut linear_1d = [0u64; 1600];
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let idx = keccakf_bit_pos(x, y, z);
                linear_1d[idx] = state[x][y][z];
            }
        }
    }
    linear_1d
}

pub const fn keccakf_idx_pos(idx: usize) -> (usize, usize, usize) {
    debug_assert!(idx < 1600);

    let x = (idx / 64) % 5;
    let y = (idx / 320) % 5;
    let z = idx % 64;
    (x, y, z)
}

pub const fn keccakf_bit_pos(x: usize, y: usize, z: usize) -> usize {
    assert!(x < 5 && y < 5 && z < 64);

    64 * x + 320 * y + z
}
