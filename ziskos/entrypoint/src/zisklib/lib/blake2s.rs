//! BLAKE2s hash function.

use crate::syscalls::{syscall_blake2sf, SyscallBlake2sfParams};

/// BLAKE2s initialization vectors
const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

/// BLAKE2s compression function F as defined in RFC 7693.
///
/// Updates the hash state `h` in place by mixing the message block `m`
/// using counter `t` (two little-endian 32-bit halves) and finalization flag `f`.
/// The 10 rounds run in a single `syscall_blake2sf` call.
pub fn blake2s_compress(
    h: &mut [u32; 8],
    m: &[u32; 16],
    t: &[u32; 2],
    f: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    // Initialize the local working vector
    let mut v = [0u32; 16];
    v[..8].copy_from_slice(h);
    v[8..12].copy_from_slice(&IV[..4]);
    v[12] = t[0] ^ IV[4];
    v[13] = t[1] ^ IV[5];
    v[14] = IV[6] ^ if f { u32::MAX } else { 0 };
    v[15] = IV[7];

    // Pack the u32 words into the u64 slots the syscall operates on (little-endian pairs)
    let mut state = [0u64; 8];
    let mut input = [0u64; 8];
    for i in 0..8 {
        state[i] = v[2 * i] as u64 | ((v[2 * i + 1] as u64) << 32);
        input[i] = m[2 * i] as u64 | ((m[2 * i + 1] as u64) << 32);
    }

    // Perform the cryptographic mixing
    let mut params = SyscallBlake2sfParams { state: &mut state, input: &input };
    syscall_blake2sf(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute the output state
    for i in 0..8 {
        v[2 * i] = state[i] as u32;
        v[2 * i + 1] = (state[i] >> 32) as u32;
    }
    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BLAKE2s-256("abc") from RFC 7693, Appendix B: a single final block with t = 3 and the
    /// parameter block 0x01010020 (digest length 32, fanout 1, depth 1) folded into h[0].
    #[test]
    fn test_blake2s_rfc7693_abc() {
        let mut h = IV;
        h[0] ^= 0x01010020;

        let mut m = [0u32; 16];
        m[0] = 0x00636261; // "abc", little-endian

        blake2s_compress(
            &mut h,
            &m,
            &[3, 0],
            true,
            #[cfg(feature = "hints")]
            &mut Vec::new(),
        );

        // 508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982
        let expected: [u32; 8] = [
            0x8c5e8c50, 0xe2147c32, 0xa32ba7e1, 0x2f45eb4e, 0x208b4537, 0x293ad69e, 0x4c9b994d,
            0x82596786,
        ];
        assert_eq!(
            h, expected,
            "blake2s does not match:\n   exp: {expected:08x?},\n   got: {h:08x?}"
        );
    }
}
