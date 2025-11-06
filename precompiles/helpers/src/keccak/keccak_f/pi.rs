/// Pi step of Keccak-f permutation  
/// Rearranges the positions of the lanes
pub fn keccak_f_pi(state: &mut [u64; 25]) {
    let mut new_state = [0u64; 25];

    // Apply the pi permutation: A'[y, (2x + 3y) mod 5] = A[x, y]
    // In linear indexing: new_index = ((2*x + 3*y) % 5) * 5 + y
    for y in 0..5 {
        for x in 0..5 {
            let old_index = 5 * y + x;
            let new_x = (2 * x + 3 * y) % 5;
            let new_y = y;
            let new_index = 5 * new_x + new_y;
            new_state[new_index] = state[old_index];
        }
    }

    *state = new_state;
}
