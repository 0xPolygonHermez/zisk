use super::Blake2StateBits;

/// Message word permutation schedule
/// 10 different permutations, cycling for rounds >= 10
const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

/// Rotation constants for G function
const R1: usize = 32;
const R2: usize = 24;
const R3: usize = 16;
const R4: usize = 63;

/// BLAKE2b round function
pub(crate) fn blake2b_round(v: &mut Blake2StateBits, m: &[[u64; 64]; 16], round: u32) {
    // Message word selection permutation for this round
    let s = &SIGMA[(round % 10) as usize];

    // Column step
    g(v, 0, 4, 8, 12, &m[s[0]], &m[s[1]]);
    g(v, 1, 5, 9, 13, &m[s[2]], &m[s[3]]);
    g(v, 2, 6, 10, 14, &m[s[4]], &m[s[5]]);
    g(v, 3, 7, 11, 15, &m[s[6]], &m[s[7]]);

    // Diagonal step
    g(v, 0, 5, 10, 15, &m[s[8]], &m[s[9]]);
    g(v, 1, 6, 11, 12, &m[s[10]], &m[s[11]]);
    g(v, 2, 7, 8, 13, &m[s[12]], &m[s[13]]);
    g(v, 3, 4, 9, 14, &m[s[14]], &m[s[15]]);
}

/// G mixing function
///
/// The G function mixes two input words `x` and `y` from the message block into the state.
/// It operates on 4 state words: v[a], v[b], v[c], v[d]
#[allow(clippy::too_many_arguments)]
fn g(
    v: &mut Blake2StateBits,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    x: &[u64; 64],
    y: &[u64; 64],
) {
    let mut va = v[a];
    let mut vb = v[b];
    let mut vc = v[c];
    let mut vd = v[d];

    // v[a] := (v[a] + v[b] + x) mod 2^64
    add64_bits(&mut va, &vb);
    add64_bits(&mut va, x);

    // v[d] := (v[d] ^ v[a]) >>> R1
    xor64_bits(&mut vd, &va);
    rotr64_bits(&mut vd, R1);

    // v[c] := (v[c] + v[d]) mod 2^64
    add64_bits(&mut vc, &vd);

    // v[b] := (v[b] ^ v[c]) >>> R2
    xor64_bits(&mut vb, &vc);
    rotr64_bits(&mut vb, R2);

    // v[a] := (v[a] + v[b] + y) mod 2^64
    add64_bits(&mut va, &vb);
    add64_bits(&mut va, y);

    // v[d] := (v[d] ^ v[a]) >>> R3
    xor64_bits(&mut vd, &va);
    rotr64_bits(&mut vd, R3);

    // v[c] := (v[c] + v[d]) mod 2^64
    add64_bits(&mut vc, &vd);

    // v[b] := (v[b] ^ v[c]) >>> R4
    xor64_bits(&mut vb, &vc);
    rotr64_bits(&mut vb, R4);

    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}

fn add64_bits(a: &mut [u64; 64], b: &[u64; 64]) {
    let mut carry = 0u64;
    for z in 0..64 {
        let sum = a[z] + b[z] + carry;
        a[z] = sum % 2;
        carry = sum / 2;
    }
}

fn xor64_bits(a: &mut [u64; 64], b: &[u64; 64]) {
    for z in 0..64 {
        a[z] ^= b[z];
    }
}

fn rotr64_bits(a: &mut [u64; 64], n: usize) {
    let mut temp = [0u64; 64];
    for z in 0..64 {
        temp[z] = a[(z + n) % 64];
    }
    *a = temp;
}
