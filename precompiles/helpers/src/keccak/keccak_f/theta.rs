/// Theta step of Keccak-f permutation
/// Computes column parity and adds it to adjacent columns
pub fn keccak_f_theta(state: &mut [u64; 25]) {
    // Compute column parities C[x] = A[x,0] ⊕ A[x,1] ⊕ A[x,2] ⊕ A[x,3] ⊕ A[x,4]
    let mut c = [0u64; 5];
    for x in 0..5 {
        c[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
    }

    // Compute D[x] = C[(x+4) mod 5] ⊕ ROT(C[(x+1) mod 5], 1)
    let mut d = [0u64; 5];
    for x in 0..5 {
        d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
    }

    // Apply D[x] to all lanes in column x: A[x,y] = A[x,y] ⊕ D[x]
    for x in 0..5 {
        for y in 0..5 {
            state[5 * y + x] ^= d[x];
        }
    }
}
