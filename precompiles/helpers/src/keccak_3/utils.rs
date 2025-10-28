/// Calculates the linear position of a bit in the Keccak state as per the specification.
///
/// The Keccak state is organized as a 3-dimensional array:
/// - x: 0..4 (5 lanes)
/// - y: 0..4 (5 lanes)
/// - z: 0..63 (64 bits per lane)
///
/// # Arguments
/// * `x` - Lane coordinate (0-4)
/// * `y` - Lane coordinate (0-4)
/// * `z` - Bit position within lane (0-63)
///
/// # Returns
/// Linear position in 0..1599 range
pub fn bit_position(x: usize, y: usize, z: usize) -> usize {
    assert!(x < 5, "x coordinate must be 0-4");
    assert!(y < 5, "y coordinate must be 0-4");
    assert!(z < 64, "z coordinate must be 0-63");

    64 * x + 320 * y + z
}

/// Convert state from lanes (u64) to bits
pub fn state_to_bits(state: &[u64; 25]) -> [u8; 1600] {
    let mut state_bits = [0u8; 1600];
    for x in 0..5 {
        for y in 0..5 {
            let lane = state[x + 5 * y];
            for z in 0..64 {
                state_bits[bit_position(x, y, z)] = ((lane >> z) & 1) as u8;
            }
        }
    }

    state_bits
}

/// Convert state from bits back to lanes (u64)
pub fn bits_to_state(state_bits: &[u8; 1600]) -> [u64; 25] {
    let mut state = [0u64; 25];
    for x in 0..5 {
        for y in 0..5 {
            let mut lane = 0u64;
            for z in 0..64 {
                if state_bits[bit_position(x, y, z)] != 0 {
                    lane |= 1u64 << z;
                }
            }
            state[x + 5 * y] = lane;
        }
    }

    state
}
