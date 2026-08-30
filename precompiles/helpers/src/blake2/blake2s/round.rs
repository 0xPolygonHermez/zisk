/// Message word permutation schedule.
///
/// Identical to BLAKE2b's: RFC 7693 uses one SIGMA table for both variants.
/// BLAKE2s runs 10 rounds, so it consumes every row exactly once.
/// Number of distinct message schedules. BLAKE2s runs exactly this many rounds,
/// and the AIR encodes `round_idx` as a one-hot over the same count, so this is
/// also the exclusive upper bound on a valid round index. Callers holding a
/// wider integer must check against it *before* narrowing.
pub const BLAKE2S_ROUNDS: usize = 10;

const SIGMA: [[usize; 16]; BLAKE2S_ROUNDS] = [
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

/// Rotation constants for the G function.
///
/// BLAKE2s rotates 32-bit words by 16/12/8/7, against BLAKE2b's 64-bit
/// 32/24/16/63. In the arithmetization only R1 and R3 are byte-aligned; R2 and
/// R4 decompose into a byte rotation plus a shift-and-carry step (see
/// `blake2sr.pil`).
const R1: u32 = 16;
const R2: u32 = 12;
const R3: u32 = 8;
const R4: u32 = 7;

/// BLAKE2s round function.
///
/// `round` must be in [0, 10). Callers reduce before this point, exactly as
/// `zisklib::blake2s_compress` does. Rejecting rather than reducing keeps this
/// agreeing with the state machine and the AIR, whose `round_idx` is a one-hot
/// over SIGMA_LENGTH selectors and so cannot represent an index >= 10; silently
/// reducing here would let a bad index execute and only fail later, during
/// witness generation.
pub fn blake2s_round(v: &mut [u32; 16], m: &[u32; 16], round: u32) {
    assert!(
        (round as usize) < BLAKE2S_ROUNDS,
        "blake2s round index {round} exceeds SIGMA ({BLAKE2S_ROUNDS}); reduce before calling"
    );

    // Message word selection permutation for this round
    let s = &SIGMA[round as usize];

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

/// G mixing function.
///
/// Mixes two message words `x` and `y` into the four state words indexed by
/// `a`, `b`, `c`, `d`. Structurally identical to BLAKE2b's G; only the word
/// width and the rotation amounts differ.
#[allow(clippy::too_many_arguments)]
fn g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, x: u32, y: u32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The AIR encodes `round_idx` as a one-hot over SIGMA_LENGTH selectors and
    /// its param port ties that to the index the guest wrote to memory, so an
    /// index >= 10 is unprovable. Reject rather than reduce, so the failure
    /// lands here instead of surfacing later as a constraint error.
    #[test]
    #[should_panic(expected = "exceeds SIGMA")]
    fn rejects_out_of_range_round_index() {
        blake2s_round(&mut [0u32; 16], &[0u32; 16], 10);
    }

    /// Control: the last valid index is accepted.
    #[test]
    fn accepts_the_last_valid_index() {
        blake2s_round(&mut [0u32; 16], &[0u32; 16], 9);
    }

    /// A u64 index whose low 32 bits look valid must not slip through. Callers
    /// hold a u64 and `as u32` would map 2^32 onto round 0, so they check the
    /// wide value first; this pins the narrowing hazard the bound exists for.
    #[test]
    fn narrowing_a_wide_index_would_alias_onto_a_valid_round() {
        let wide: u64 = 1 << 32;
        assert_eq!(wide as u32, 0, "the cast aliases onto round 0");
        assert!(wide >= BLAKE2S_ROUNDS as u64, "so callers must check before casting");
    }
}
