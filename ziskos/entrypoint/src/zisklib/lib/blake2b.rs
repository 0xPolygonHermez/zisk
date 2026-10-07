//! BLAKE2b hash function.

use crate::syscalls::syscall_blake2b_round_const;

/// BLAKE2b initialization vectors
const IV: [u64; 8] = [
    0x6A09E667F3BCC908,
    0xBB67AE8584CAA73B,
    0x3C6EF372FE94F82B,
    0xA54FF53A5F1D36F1,
    0x510E527FADE682D1,
    0x9B05688C2B3E6C1F,
    0x1F83D9ABFB41BD6B,
    0x5BE0CD19137E2179,
];

/// BLAKE2b compression function F as defined in RFC 7693.
///
/// Updates the hash state `h` in place by mixing the message block `m`
/// over `rounds` iterations using counter `t` and finalization flag `f`.
pub fn blake2b_compress(
    rounds: u32,
    h: &mut [u64; 8],
    m: &[u64; 16],
    t: &[u64; 2],
    f: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut v = [0u64; 16];

    v[..8].copy_from_slice(h);
    v[8..12].copy_from_slice(&IV[..4]);
    v[12] = t[0] ^ IV[4];
    v[13] = t[1] ^ IV[5];
    v[14] = IV[6] ^ if f { u64::MAX } else { 0 };
    v[15] = IV[7];

    // The round index is a static immediate of the precompiled instruction, so every call site
    // names it at compile time: whole blocks of ten rounds are unrolled over the constant round
    // functions, and the (at most nine) rounds left over are selected with a chain of compares.
    // Dispatching a runtime `r % 10` through `syscall_blake2b_round` instead costs about nine
    // extra steps per round (measured on the `hashes` guest).
    macro_rules! round {
        ($i:literal) => {
            syscall_blake2b_round_const::<$i>(
                &mut v,
                m,
                #[cfg(feature = "hints")]
                hints,
            )
        };
    }
    macro_rules! tail_round {
        ($tail:ident, $i:literal) => {
            if $tail > $i {
                round!($i);
            }
        };
    }
    let mut r = 0;
    while rounds - r >= 10 {
        round!(0);
        round!(1);
        round!(2);
        round!(3);
        round!(4);
        round!(5);
        round!(6);
        round!(7);
        round!(8);
        round!(9);
        r += 10;
    }
    let tail = rounds - r;
    tail_round!(tail, 0);
    tail_round!(tail, 1);
    tail_round!(tail, 2);
    tail_round!(tail, 3);
    tail_round!(tail, 4);
    tail_round!(tail, 5);
    tail_round!(tail, 6);
    tail_round!(tail, 7);
    tail_round!(tail, 8);

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

/// C-compatible wrapper for full Blake2b compression function
///
/// # Safety
/// - `state` must point to a writable buffer of at least 8 `u64`s
/// - `message` must point to at least 16 `u64`s
/// - `offset` must point to at least 2 `u64`s
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn blake2b_compress_c(
    rounds: u32,
    state: *mut u64,
    message: *const u64,
    offset: *const u64,
    final_block: u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    // Parse state
    let state_slice = core::slice::from_raw_parts_mut(state, 8);
    let state_array: &mut [u64; 8] = &mut *(state_slice.as_mut_ptr() as *mut [u64; 8]);

    // Parse message
    let message_slice = core::slice::from_raw_parts(message, 16);
    let message_array: &[u64; 16] = &*(message_slice.as_ptr() as *const [u64; 16]);

    // Parse offset
    let offset_slice = core::slice::from_raw_parts(offset, 2);
    let offset_array: &[u64; 2] = &*(offset_slice.as_ptr() as *const [u64; 2]);

    blake2b_compress(
        rounds,
        state_array,
        message_array,
        offset_array,
        final_block != 0,
        #[cfg(feature = "hints")]
        hints,
    );
}
