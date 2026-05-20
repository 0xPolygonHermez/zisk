use crunchy::unroll;

use super::{KeccakState, PI, RC_BITS, RHO};

// The maximum value that any expression during keccakf computation can get
// Operation summary:
//  - The θ.1 step has 4 add, this gives a number in the range <= 5
//  - The θ.2 step has 1 add, this gives a number in the range <= 10
//  - The θ.3 step has 1 add, this gives a number in the range <= 11
//  - The χ.1 step has 1 add and 1 prod, this gives a number in the range <= 132
//  - The χ.2 step has 1 add, this gives a number in the range <= 143
//  - The ι step has 1 add, this gives a number in the range <= 144
pub fn keccak_f_round(state: &mut KeccakState, round: usize) {
    let mut array = [[0u8; 64]; 5];

    // θ (Theta) step - Column parity computation and mixing

    // Step 1: Compute column parities
    unroll! {
        for x in 0..5 {
            for z in 0..64 {
                array[x][z] =
                    state[x][0][z] + state[x][1][z] + state[x][2][z] + state[x][3][z] + state[x][4][z];
            }
        }
    }

    // Step 2: Compute D[x, z]
    let mut d = [[0u8; 64]; 5];
    unroll! {
        for x in 0..5 {
            for z in 0..64 {
                d[x][z] = array[(x + 4) % 5][z] + array[(x + 1) % 5][(z + 63) % 64];
            }
        }
    }

    // Step 3: Apply theta transformation
    unroll! {
        for x in 0..5 {
            unroll! {
                for y in 0..5 {
                    for z in 0..64 {
                        state[x][y][z] += d[x][z];
                    }
                }
            }
        }
    }

    // ρ (Rho) step - Bitwise rotation
    // π (Pi) step - Lane permutation
    let mut last = state[1][0];
    for t in 0..24 {
        let (x, y) = PI[t];

        // save next lane
        let tmp = state[x][y];
        let rot = RHO[t];
        let shift = 64 - rot;

        // apply rotation
        for z in 0..64 {
            state[x][y][z] = last[(z + shift) & 63];
        }

        last = tmp;
    }

    // χ (Chi) step - Nonlinear transformation
    unroll! {
        for y in 0..5 {
            unroll! {
                for x in 0..5 {
                    for z in 0..64 {
                        array[x][z] = state[x][y][z];
                    }
                }
            }
            unroll! {
                for x in 0..5 {
                    for z in 0..64 {
                        state[x][y][z] += (1 + array[(x + 1) % 5][z]) * array[(x + 2) % 5][z];
                    }
                }
            }
        }
    }

    // ι (Iota) step - Add round constant
    for z in 0..64 {
        state[0][0][z] += RC_BITS[round][z] as u8;
    }
}
