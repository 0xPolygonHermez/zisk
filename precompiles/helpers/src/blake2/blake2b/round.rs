/// Message word permutation schedule
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
const R1: u32 = 32;
const R2: u32 = 24;
const R3: u32 = 16;
const R4: u32 = 63;

/// BLAKE2b round function
pub fn blake2b_round(v: &mut [u64; 16], m: &[u64; 16], round: u32) {
    // Message word selection permutation for this round
    let s = &SIGMA[(round % 10) as usize];

    // Column step
    g(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
    g(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
    g(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
    g(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);

    // Diagonal step
    g(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
    g(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
    g(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
    g(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
}

/// G mixing function
///
/// The G function mixes two input words `x` and `y` from the message block into the state.
/// It operates on 4 state words: v[a], v[b], v[c], v[d]
#[allow(clippy::too_many_arguments)]
fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    let mut va = v[a];
    let mut vb = v[b];
    let mut vc = v[c];
    let mut vd = v[d];

    va = va.wrapping_add(vb).wrapping_add(x);
    vd = (vd ^ va).rotate_right(R1);
    vc = vc.wrapping_add(vd);
    vb = (vb ^ vc).rotate_right(R2);

    va = va.wrapping_add(vb).wrapping_add(y);
    vd = (vd ^ va).rotate_right(R3);
    vc = vc.wrapping_add(vd);
    vb = (vb ^ vc).rotate_right(R4);

    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}
