use super::KECCAK_F_RC;

/// Iota step of Keccak-f permutation
/// Adds round constant to A[0,0]
pub fn keccak_f_iota(state: &mut [u64; 25], round: usize) {
    // Add round constant to A[0,0] (position 0 in linear indexing)
    state[0] ^= KECCAK_F_RC[round];
}
