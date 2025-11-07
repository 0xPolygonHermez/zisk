const RC: [u64; 24] = [
    1u64,
    0x8082u64,
    0x800000000000808au64,
    0x8000000080008000u64,
    0x808bu64,
    0x80000001u64,
    0x8000000080008081u64,
    0x8000000000008009u64,
    0x8au64,
    0x88u64,
    0x80008009u64,
    0x8000000au64,
    0x8000808bu64,
    0x800000000000008bu64,
    0x8000000000008089u64,
    0x8000000000008003u64,
    0x8000000000008002u64,
    0x8000000000000080u64,
    0x800au64,
    0x800000008000000au64,
    0x8000000080008081u64,
    0x8000000000008080u64,
    0x80000001u64,
    0x8000000080008008u64,
];

const RHO: [u32; 24] =
    [1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44];

const PI: [usize; 24] =
    [10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1];

/// Super fast implementation of Keccak-f[1600] round function,
/// inspired by https://github.com/debris/tiny-keccak/blob/master/src/lib.rs
pub fn keccak_f_round(state: &mut [u64; 25], round: usize) {
    let mut tmp: [u64; 5] = [0; 5];

    // θ (Theta) step - Column parity computation and mixing
    // Compute column parities
    for x in 0..5 {
        for y_count in 0..5 {
            let y = y_count * 5;
            tmp[x] ^= state[x + y];
        }
    }

    // Apply theta transformation
    for x in 0..5 {
        for y_count in 0..5 {
            let y = y_count * 5;
            state[y + x] ^= tmp[(x + 4) % 5] ^ tmp[(x + 1) % 5].rotate_left(1);
        }
    }

    // ρ (Rho) and π (Pi) steps combined - Rotation and permutation
    let mut last = state[1];
    for x in 0..24 {
        tmp[0] = state[PI[x]];
        state[PI[x]] = last.rotate_left(RHO[x]);
        last = tmp[0];
    }

    // χ (Chi) step - Nonlinear transformation
    for y_step in 0..5 {
        let y = y_step * 5;

        // Store the row
        for x in 0..5 {
            tmp[x] = state[y + x];
        }

        // Apply chi transformation
        for x in 0..5 {
            state[y + x] = tmp[x] ^ ((!tmp[(x + 1) % 5]) & (tmp[(x + 2) % 5]));
        }
    }

    // ι (Iota) step - Add round constant
    state[0] ^= RC[round];
}
