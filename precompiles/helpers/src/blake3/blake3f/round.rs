/// Message word permutation schedule
const SIGMA: [[usize; 16]; 7] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8],
    [3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1],
    [10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6],
    [12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4],
    [9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7],
    [11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13],
];

/// BLAKE3 round function
pub(crate) fn blake3_round(v: &mut [u32; 16], m: &[u32; 16], round: usize) {
    let s = SIGMA[round];

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
fn g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, x: u32, y: u32) {
    let mut va = v[a];
    let mut vb = v[b];
    let mut vc = v[c];
    let mut vd = v[d];

    va = va.wrapping_add(vb).wrapping_add(x);
    vd = (vd ^ va).rotate_right(16);
    vc = vc.wrapping_add(vd);
    vb = (vb ^ vc).rotate_right(12);

    va = va.wrapping_add(vb).wrapping_add(y);
    vd = (vd ^ va).rotate_right(8);
    vc = vc.wrapping_add(vd);
    vb = (vb ^ vc).rotate_right(7);

    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}
