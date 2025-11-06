/// Chi step of Keccak-f permutation
/// Applies nonlinear transformation row by row
pub fn keccak_f_chi(state: &mut [u64; 25]) {
    let mut new_state = [0u64; 25];

    // Apply chi transformation: A'[x,y] = A[x,y] ⊕ ((¬A[(x+1) mod 5, y]) ∧ A[(x+2) mod 5, y])
    for y in 0..5 {
        for x in 0..5 {
            let index = 5 * y + x;
            let x1_index = 5 * y + ((x + 1) % 5);
            let x2_index = 5 * y + ((x + 2) % 5);

            new_state[index] = state[index] ^ ((!state[x1_index]) & state[x2_index]);
        }
    }

    *state = new_state;
}
