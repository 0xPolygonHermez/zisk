use crate::syscalls::{syscall_blake2b_round, SyscallBlake2bRoundParams};

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
    v[8..16].copy_from_slice(&IV);

    v[12] ^= t[0];
    v[13] ^= t[1];

    if f {
        v[14] = !v[14];
    }

    for r in 0..rounds {
        blake2b_round(
            &mut v,
            m,
            r,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

fn blake2b_round(
    v: &mut [u64; 16],
    m: &[u64; 16],
    round: u32,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut params = SyscallBlake2bRoundParams { index: (round % 10) as u64, state: v, input: m };
    syscall_blake2b_round(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// C-compatible wrapper for full Blake2b compression function
///
/// # Safety
/// - `state` must point to a writable buffer of at least 8 `u64`s
/// - `message` must point to at least 16 `u64`s
/// - `offset` must point to at least 2 `u64`s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_blake2b_compress_c")]
pub unsafe extern "C" fn blake2b_compress_c(
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
