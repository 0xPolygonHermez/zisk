/// Rho step of Keccak-f permutation
/// Rotates each lane by a specific offset
pub fn keccak_f_rho(state: &mut [u64; 25]) {
    // Rotation offsets for each position (x,y)
    // These are the official Keccak rotation offsets
    const RHO_OFFSETS: [u32; 25] = [
        0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56,
        14,
    ];

    // Apply rotation to each lane
    for i in 0..25 {
        state[i] = state[i].rotate_left(RHO_OFFSETS[i]);
    }
}
