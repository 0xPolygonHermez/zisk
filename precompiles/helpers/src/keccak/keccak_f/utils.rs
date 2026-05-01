use super::KeccakState;

/// Convert from linear [u64; 25] to 5x5x64 bit array
#[allow(clippy::needless_range_loop)]
pub fn keccakf_state_from_linear(linear: &[u64; 25]) -> KeccakState {
    let mut state = [[[0u8; 64]; 5]; 5];
    for x in 0..5 {
        for y in 0..5 {
            let word = linear[x + y * 5];
            for z in 0..64 {
                state[x][y][z] = ((word >> z) & 1) as u8;
            }
        }
    }
    state
}

#[allow(clippy::needless_range_loop)]
pub fn keccakf_state_flatten(state: &KeccakState) -> [u8; 1600] {
    let mut linear_1d = [0u8; 1600];
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

pub const fn keccakf_bit_pos(x: usize, y: usize, z: usize) -> usize {
    assert!(x < 5 && y < 5 && z < 64);

    64 * x + 320 * y + z
}
