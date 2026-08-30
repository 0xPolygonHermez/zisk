mod round;

pub use round::{blake2s_round, BLAKE2S_ROUNDS};

/// BLAKE2s initialization vectors.
///
/// The same fractional parts of the square roots of the first eight primes that
/// BLAKE2b uses, truncated to 32 bits.
const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

/// BLAKE2s compression function.
///
/// # Arguments
/// * `rounds` - Number of rounds (10 for BLAKE2s)
/// * `h` - The internal state (8 x 32-bit words), updated in place
/// * `m` - The message block (16 x 32-bit words)
/// * `t` - Offset counter (2 x 32-bit words: low, high)
/// * `f` - Final block flag
pub fn blake2s_compress(rounds: u32, h: &mut [u32; 8], m: &[u32; 16], t: &[u32; 2], f: bool) {
    let mut v = [0u32; 16];

    v[..8].copy_from_slice(h);
    v[8..16].copy_from_slice(&IV);

    v[12] ^= t[0];
    v[13] ^= t[1];

    if f {
        v[14] = !v[14];
    }

    for r in 0..rounds {
        blake2s_round(&mut v, m, r);
    }

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the compression function the way BLAKE2s-256 does for a single
    /// final block, and return the 32-byte digest.
    ///
    /// Parameter block: digest_length=32, key_length=0, fanout=1, depth=1, so
    /// h[0] is IV[0] ^ 0x01010020.
    fn blake2s_256(input: &[u8]) -> [u8; 32] {
        assert!(input.len() <= 64, "single-block helper");
        let mut h = IV;
        h[0] ^= 0x0101_0020;

        let mut block = [0u8; 64];
        block[..input.len()].copy_from_slice(input);
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }

        blake2s_compress(10, &mut h, &m, &[input.len() as u32, 0], true);

        let mut out = [0u8; 32];
        for (i, w) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }

    /// RFC 7693 appendix B: BLAKE2s-256 of "abc".
    #[test]
    fn test_blake2s_rfc7693_abc() {
        let expected: [u8; 32] = [
            0x50, 0x8C, 0x5E, 0x8C, 0x32, 0x7C, 0x14, 0xE2, 0xE1, 0xA7, 0x2B, 0xA3, 0x4E, 0xEB,
            0x45, 0x2F, 0x37, 0x45, 0x8B, 0x20, 0x9E, 0xD6, 0x3A, 0x29, 0x4D, 0x99, 0x9B, 0x4C,
            0x86, 0x67, 0x59, 0x82,
        ];
        assert_eq!(blake2s_256(b"abc"), expected);
    }

    /// BLAKE2s-256 of the empty string.
    #[test]
    fn test_blake2s_empty() {
        let expected: [u8; 32] = [
            0x69, 0x21, 0x7A, 0x30, 0x79, 0x90, 0x80, 0x94, 0xE1, 0x11, 0x21, 0xD0, 0x42, 0x35,
            0x4A, 0x7C, 0x1F, 0x55, 0xB6, 0x48, 0x2C, 0xA1, 0xA5, 0x1E, 0x1B, 0x25, 0x0D, 0xFD,
            0x1E, 0xD0, 0xEE, 0xF9,
        ];
        assert_eq!(blake2s_256(b""), expected);
    }

    /// Differential check against the RustCrypto `blake2` crate over random
    /// inputs — the same crate ZKsync OS hashes its state tree with, so this is
    /// the implementation the precompile must agree with bit for bit.
    #[test]
    fn test_blake2s_matches_rustcrypto() {
        use blake2::digest::{Digest, FixedOutput};
        use blake2::Blake2s256;

        // Deterministic pseudo-random inputs; no rand dependency.
        let mut seed: u64 = 0x243F_6A88_85A3_08D3;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        for len in 0..=64usize {
            let mut input = vec![0u8; len];
            for b in input.iter_mut() {
                *b = (next() & 0xFF) as u8;
            }

            let mut hasher = Blake2s256::new();
            hasher.update(&input);
            let reference: [u8; 32] = hasher.finalize_fixed().into();

            assert_eq!(
                blake2s_256(&input),
                reference,
                "mismatch at len {len} for input {input:02x?}"
            );
        }
    }
}
