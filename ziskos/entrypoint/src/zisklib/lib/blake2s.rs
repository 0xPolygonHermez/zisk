//! BLAKE2s hash function.

use crate::syscalls::{syscall_blake2s_round, SyscallBlake2sRoundParams};

/// BLAKE2s initialization vectors
const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

/// BLAKE2s compression function F as defined in RFC 7693.
///
/// Updates the hash state `h` in place by mixing the message block `m`
/// over `rounds` iterations using counter `t` and finalization flag `f`.
///
/// The round syscall takes its state and message as 32-bit values held one per
/// 64-bit slot with a zero high half, so the widening and narrowing happen here
/// rather than in the caller.
pub fn blake2s_compress(
    rounds: u32,
    h: &mut [u32; 8],
    m: &[u32; 16],
    t: &[u32; 2],
    f: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut v = [0u64; 16];

    for i in 0..8 {
        v[i] = h[i] as u64;
    }
    for i in 0..4 {
        v[8 + i] = IV[i] as u64;
    }
    v[12] = (t[0] ^ IV[4]) as u64;
    v[13] = (t[1] ^ IV[5]) as u64;
    v[14] = (IV[6] ^ if f { u32::MAX } else { 0 }) as u64;
    v[15] = IV[7] as u64;

    let mut msg = [0u64; 16];
    for i in 0..16 {
        msg[i] = m[i] as u64;
    }

    for r in 0..rounds {
        let mut params =
            SyscallBlake2sRoundParams { index: (r % 10) as u64, state: &mut v, input: &msg };
        assert!(syscall_blake2s_round(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        ));
    }

    for i in 0..8 {
        h[i] ^= (v[i] as u32) ^ (v[i + 8] as u32);
    }
}

/// BLAKE2s-256 over a byte slice, unkeyed.
///
/// Implements the parameter block for a 32-byte digest, no key, fanout 1,
/// depth 1 — the configuration ZKsync OS hashes its state tree with.
pub fn blake2s256(input: &[u8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u8; 32] {
    let mut h = IV;
    h[0] ^= 0x0101_0020;

    let mut processed: u64 = 0;
    let mut chunks = input.chunks(64).peekable();

    // An empty input still runs one padded final block.
    if chunks.peek().is_none() {
        let m = [0u32; 16];
        blake2s_compress(
            10,
            &mut h,
            &m,
            &[0, 0],
            true,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    while let Some(chunk) = chunks.next() {
        let last = chunks.peek().is_none();
        processed += chunk.len() as u64;

        let mut block = [0u8; 64];
        block[..chunk.len()].copy_from_slice(chunk);
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }

        let t = [processed as u32, (processed >> 32) as u32];
        blake2s_compress(
            10,
            &mut h,
            &m,
            &t,
            last,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    let mut out = [0u8; 32];
    for (i, w) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    out
}
