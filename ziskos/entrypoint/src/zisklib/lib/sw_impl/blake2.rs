//! Blake2b F compression function software fallback (non-hints, non-zkVM only).
//!
//! The `algo` module below is copied verbatim from revm-precompile-32.1.0/src/blake2.rs:68-184.
//! The only omitted part is the AVX2 fast path (conditional on `target_feature = "avx2"`),
//! which is irrelevant in a pure-software fallback context.
//!
//! The public `compress` wrapper at the bottom adapts the `rounds: usize` signature from revm
//! to the `rounds: u32` type used by zkvm_accelerators.rs.

// ============================================================
// Copied verbatim from revm-precompile-32.1.0/src/blake2.rs:68-184
// (`mod algo` block, minus the AVX2 branch inside `compress`)
// ============================================================

/// Blake2 algorithm
// revm-precompile-32.1.0/src/blake2.rs:68
mod algo {
    /// SIGMA from spec: <https://datatracker.ietf.org/doc/html/rfc7693#section-2.7>
    // revm-precompile-32.1.0/src/blake2.rs:70-81
    pub const SIGMA: [[usize; 16]; 10] = [
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

    /// got IV from: <https://en.wikipedia.org/wiki/BLAKE_(hash_function)>
    // revm-precompile-32.1.0/src/blake2.rs:83-93
    pub const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    #[inline(always)]
    #[allow(clippy::many_single_char_names)]
    /// G function: <https://tools.ietf.org/html/rfc7693#section-3.1>
    // revm-precompile-32.1.0/src/blake2.rs:95-118
    fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
        let mut va = v[a];
        let mut vb = v[b];
        let mut vc = v[c];
        let mut vd = v[d];

        va = va.wrapping_add(vb).wrapping_add(x);
        vd = (vd ^ va).rotate_right(32);
        vc = vc.wrapping_add(vd);
        vb = (vb ^ vc).rotate_right(24);

        va = va.wrapping_add(vb).wrapping_add(y);
        vd = (vd ^ va).rotate_right(16);
        vc = vc.wrapping_add(vd);
        vb = (vb ^ vc).rotate_right(63);

        v[a] = va;
        v[b] = vb;
        v[c] = vc;
        v[d] = vd;
    }

    /// Compression function F takes as an argument the state vector "h",
    /// message block vector "m" (last block is padded with zeros to full
    /// block size, if required), 2w-bit offset counter "t", and final block
    /// indicator flag "f".  Local vector v[0..15] is used in processing.  F
    /// returns a new state vector.  The number of rounds, "r", is 12 for
    /// BLAKE2b and 10 for BLAKE2s.  Rounds are numbered from 0 to r - 1.
    // revm-precompile-32.1.0/src/blake2.rs:126-166 (AVX2 branch omitted)
    #[allow(clippy::many_single_char_names)]
    pub fn compress(rounds: usize, h: &mut [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) {
        // if avx2 is not available, use the fallback portable implementation

        let mut v = [0u64; 16];
        v[..h.len()].copy_from_slice(h); // First half from state.
        v[h.len()..].copy_from_slice(&IV); // Second half from IV.

        v[12] ^= t[0];
        v[13] ^= t[1];

        if f {
            v[14] = !v[14] // Invert all bits if the last-block-flag is set.
        }
        for i in 0..rounds {
            round(&mut v, &m, i);
        }

        for i in 0..8 {
            h[i] ^= v[i] ^ v[i + 8];
        }
    }

    // revm-precompile-32.1.0/src/blake2.rs:168-183
    #[inline(always)]
    fn round(v: &mut [u64; 16], m: &[u64; 16], r: usize) {
        // Message word selection permutation for this round.
        let s = &SIGMA[r % 10];
        // g1
        g(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        g(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        g(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        g(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);

        // g2
        g(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        g(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        g(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        g(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }
}

// ============================================================
// Public wrapper adapting the API used by zkvm_accelerators.rs
//
// revm's algo::compress takes `rounds: usize`; zkvm_accelerators.rs passes `rounds: u32`.
// ============================================================

pub fn compress(rounds: u32, h: &mut [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) {
    algo::compress(rounds as usize, h, m, t, f)
}
