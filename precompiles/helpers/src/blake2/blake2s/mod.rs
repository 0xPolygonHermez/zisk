mod round;

use round::blake2s_round;

/// BLAKE2s initialization vectors
const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

/// Number of rounds of the BLAKE2s compression function
pub const BLAKE2S_ROUNDS: usize = 10;

/// BLAKE2s simplified compression function: the 10-round permutation of the working
/// vector `v` with the message block `m`, without the initialisation and the feed-forward
pub fn blake2s_f(v: &mut [u32; 16], m: &[u32; 16]) {
    for round in 0..BLAKE2S_ROUNDS {
        blake2s_round(v, m, round);
    }
}

/// BLAKE2s compression function F as defined in RFC 7693
///
/// # Arguments
/// * `h` - The internal state h (8 x 32-bit words), updated in place
/// * `m` - The message block m (16 x 32-bit words)
/// * `t` - Offset counter (2 x 32-bit words, little-endian halves of the 64-bit counter)
/// * `f` - Final block flag
pub fn blake2s_compress(h: &mut [u32; 8], m: &[u32; 16], t: &[u32; 2], f: bool) {
    let mut v = [0u32; 16];

    v[..8].copy_from_slice(h);
    v[8..16].copy_from_slice(&IV);

    v[12] ^= t[0];
    v[13] ^= t[1];

    if f {
        v[14] = !v[14];
    }

    blake2s_f(&mut v, m);

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BLAKE2s-256 of "abc" (RFC 7693, Appendix B), computed through the compression
    /// function: one final block, t = 3, parameter block 0x01010020 folded into h[0].
    #[test]
    fn test_blake2s_rfc7693_abc() {
        let mut h = IV;
        h[0] ^= 0x01010020;

        let mut m = [0u32; 16];
        m[0] = 0x00636261; // "abc", little-endian

        blake2s_compress(&mut h, &m, &[3, 0], true);

        let expected: [u32; 8] = [
            0x8c5e8c50, 0xe2147c32, 0xa32ba7e1, 0x2f45eb4e, 0x208b4537, 0x293ad69e, 0x4c9b994d,
            0x82596786,
        ];
        assert_eq!(
            h, expected,
            "Blake2s does not match:\n   exp: {expected:08x?},\n   got: {h:08x?}"
        );
    }

    /// The raw permutation output for the same block (no feed-forward), which is what the
    /// `blake2sf` precompile exposes.
    #[test]
    fn test_blake2s_f_abc_permutation() {
        let mut v = [
            0x6b08e647, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19, 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527c, 0x9b05688c,
            0xe07c2654, 0x5be0cd19,
        ];
        let mut m = [0u32; 16];
        m[0] = 0x00636261;

        blake2s_f(&mut v, &m);

        let expected: [u32; 16] = [
            0xd9c994aa, 0xcfec3aa6, 0x700d0ab2, 0x2c38670e, 0xaf6a1f66, 0x1d023ef3, 0x1d9ec27d,
            0x945357a5, 0x3e9ffebd, 0x969fe811, 0xef485e21, 0xa632797a, 0xdeef082e, 0xaf3d80e1,
            0x4e86829b, 0x4deafd3a,
        ];
        assert_eq!(v, expected);
    }
}
