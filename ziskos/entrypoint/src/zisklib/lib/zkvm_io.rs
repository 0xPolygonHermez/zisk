//! Standard zkVM IO functions implementing the C interface from zkvm_io.h.
//!
//! ZisK stdin is stored as length-prefixed records. The standard IO interface is
//! exposed as the first logical input record and is idempotent. Guests should use
//! either this standard IO interface or ZisK's streaming input APIs for a given
//! input, not both: standard reads do not advance ZisK's streaming input cursor.

use core::ptr::{self, addr_of_mut};

/// SHA-256 initial hash values (FIPS 180-4).
const SHA256_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

#[cfg(not(zisk_guest))]
static STANDARD_INPUT: std::sync::Mutex<Option<&'static [u8]>> = std::sync::Mutex::new(None);

/// Streaming SHA-256 state for the standard public-output commitment.
///
/// `write_output` absorbs its bytes into this hasher online — one 64-byte block at a time,
/// via the SHA-256 compression precompile on the guest — so an unbounded amount of output
/// can be committed without buffering it. `flush_output` applies the final padding after
/// `main` returns and writes the 32-byte digest into the first 8 public-output slots at
/// `OUTPUT_ADDR`. The verifier recomputes `sha256(public_output)` and checks it against
/// those 8 slots.
///
/// `#[repr(C, align(8))]` with `block` first guarantees the 8-byte alignment the compression
/// precompile requires for its `[u64; 8]` view of the block and `[u64; 4]` view of the state.
#[repr(C, align(8))]
struct OutputHasher {
    block: [u8; 64],
    state: [u32; 8],
    block_len: usize,
    total_len: u64,
}

static mut OUTPUT_HASHER: OutputHasher =
    OutputHasher { block: [0u8; 64], state: SHA256_INIT, block_len: 0, total_len: 0 };

/// Set once `write_output` is used. A guest that relies on ZisK's native public-values
/// mechanism (and never calls `write_output`) is therefore not clobbered by the digest at
/// flush time.
static mut OUTPUT_ENGAGED: bool = false;

/// # Safety
///
/// `buf_ptr` and `buf_size` must be valid writable pointers.
///
/// This function is idempotent and does not advance ZisK's streaming input
/// cursor. Mixing it with streaming reads may expose the first input record more
/// than once.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_read_input")]
pub unsafe extern "C" fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) {
    #[cfg(zisk_guest)]
    {
        let (data_ptr, len) = zkvm_standard_input();
        ptr::write(buf_ptr, data_ptr);
        ptr::write(buf_size, len);
    }
    #[cfg(not(zisk_guest))]
    {
        let mut input = STANDARD_INPUT.lock().unwrap();
        if input.is_none() {
            let saved_pos = unsafe { crate::INPUT_POS };
            unsafe { crate::INPUT_POS = crate::INPUT_INITIAL_OFFSET };
            let data: &'static [u8] = Box::leak(crate::read_input().into_boxed_slice());
            unsafe { crate::INPUT_POS = saved_pos };
            *input = Some(data);
        }
        let data = input.expect("standard input initialized");
        ptr::write(buf_ptr, if data.is_empty() { ptr::null() } else { data.as_ptr() });
        ptr::write(buf_size, data.len());
    }
}

#[cfg(zisk_guest)]
fn zkvm_standard_input() -> (*const u8, usize) {
    static mut INPUT_PTR: *const u8 = ptr::null();
    static mut INPUT_LEN: usize = 0;
    static mut INPUT_READY: bool = false;

    unsafe {
        if !INPUT_READY {
            let addr = (crate::ziskos_definitions::ziskos_config::INPUT_ADDR as usize)
                + crate::INPUT_INITIAL_OFFSET;

            crate::zisklib::fcall_input_ready(&((addr + 7) as u64));
            let len = {
                let bytes = core::slice::from_raw_parts(addr as *const u8, 8);
                u64::from_le_bytes(bytes.try_into().unwrap()) as usize
            };

            let data_addr = addr + 8;
            if len > 0 {
                let last_byte_addr = data_addr + len - 1;
                crate::zisklib::fcall_input_ready(&(last_byte_addr as u64));
                INPUT_PTR = data_addr as *const u8;
            } else {
                INPUT_PTR = ptr::null();
            }
            INPUT_LEN = len;
            INPUT_READY = true;
        }

        (INPUT_PTR, INPUT_LEN)
    }
}

/// # Safety
///
/// If `size > 0`, `output` must point to at least `size` readable bytes.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_write_output")]
pub unsafe extern "C" fn write_output(output: *const u8, size: usize) {
    // Engaging the standard output interface — even with an empty buffer — commits to a
    // digest of the concatenated output at flush time.
    *addr_of_mut!(OUTPUT_ENGAGED) = true;

    if size == 0 {
        return;
    }

    // Transport (unconstrained): stream the plaintext chunk to the host so the proof can carry
    // it alongside the digest. Soundness comes from the digest below, not this call.
    crate::zisklib::fcall_public_output(output, size);

    // Binding (constrained): absorb into the streaming SHA-256 committed at OUTPUT_ADDR.
    absorb(core::slice::from_raw_parts(output, size));
}

/// Absorb `data` into the streaming SHA-256 state, compressing each complete 64-byte block.
unsafe fn absorb(mut data: &[u8]) {
    let h = &mut *addr_of_mut!(OUTPUT_HASHER);
    h.total_len = h.total_len.wrapping_add(data.len() as u64);

    // Top up a partially-filled block first.
    if h.block_len != 0 {
        let take = core::cmp::min(64 - h.block_len, data.len());
        h.block[h.block_len..h.block_len + take].copy_from_slice(&data[..take]);
        h.block_len += take;
        data = &data[take..];
        if h.block_len == 64 {
            let block = h.block;
            compress(&mut h.state, &block);
            h.block_len = 0;
        }
    }

    // Compress complete blocks straight from the input.
    while data.len() >= 64 {
        let block: &[u8; 64] = data[..64].try_into().unwrap();
        compress(&mut h.state, block);
        data = &data[64..];
    }

    // Buffer the remainder.
    if !data.is_empty() {
        h.block[..data.len()].copy_from_slice(data);
        h.block_len = data.len();
    }
}

/// Apply SHA-256 final padding and publish the 32-byte digest into public-output slots 0..8.
///
/// Called once from `zkvm_deinit` after `main` returns; a no-op unless `write_output` was
/// used. Public slot `i` holds the i-th big-endian SHA-256 state word, i.e.
/// `slot[i] == u32::from_be_bytes(sha256(public_output)[4*i .. 4*i + 4])`.
pub(crate) unsafe fn flush_output() {
    if !*addr_of_mut!(OUTPUT_ENGAGED) {
        return;
    }

    let h = &mut *addr_of_mut!(OUTPUT_HASHER);
    let bit_len = h.total_len.wrapping_mul(8);
    let rem = h.block_len;

    // Append the 0x80 byte, then zero to the end of the block.
    h.block[rem] = 0x80;
    for b in h.block[rem + 1..].iter_mut() {
        *b = 0;
    }

    // If the 8-byte length doesn't fit in this block, compress it and start a fresh one.
    if rem + 9 > 64 {
        let block = h.block;
        compress(&mut h.state, &block);
        h.block = [0u8; 64];
    }

    // Append the 64-bit big-endian message bit length and compress the final block.
    h.block[56..64].copy_from_slice(&bit_len.to_be_bytes());
    let block = h.block;
    compress(&mut h.state, &block);

    // Publish the digest words into OUTPUT_ADDR slots 0..8.
    let state = h.state;
    for (i, &word) in state.iter().enumerate() {
        crate::set_output(i, word);
    }
}

/// Compress one 64-byte block into `state` using the SHA-256 compression precompile.
#[cfg(zisk_guest)]
unsafe fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    use crate::syscalls::{syscall_sha256_f, SyscallSha256Params};

    // The precompile reads the block as `[u64; 8]`, which requires 8-byte alignment;
    // input blocks taken straight from guest memory may be unaligned, so copy through
    // an aligned buffer.
    #[repr(align(8))]
    struct AlignedBlock([u8; 64]);
    let mut aligned = AlignedBlock([0u8; 64]);
    aligned.0.copy_from_slice(block);

    let input: &[u64; 8] = &*(aligned.0.as_ptr() as *const [u64; 8]);
    let state_64: &mut [u64; 4] = &mut *(state.as_mut_ptr() as *mut [u64; 4]);
    let mut params = SyscallSha256Params { state: state_64, input };
    syscall_sha256_f(&mut params);
}

/// Software SHA-256 compression for native (non-guest) builds — same digest, no precompile.
#[cfg(not(zisk_guest))]
unsafe fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    use sha2::compress256;
    use sha2::digest::generic_array::{typenum::U64, GenericArray};
    let blocks: &[GenericArray<u8, U64>; 1] =
        &*(block.as_ptr() as *const [GenericArray<u8, U64>; 1]);
    compress256(state, blocks);
}

#[cfg(not(zisk_guest))]
pub(crate) fn reset() {
    // Leaked slices are intentionally not freed; each reset() will re-read
    // fresh input on the next call, leaking one allocation per test run.
    *STANDARD_INPUT.lock().unwrap() = None;

    reset_output();
}

pub(crate) fn reset_output() {
    unsafe {
        let h = &mut *addr_of_mut!(OUTPUT_HASHER);
        h.block = [0u8; 64];
        h.state = SHA256_INIT;
        h.block_len = 0;
        h.total_len = 0;
        *addr_of_mut!(OUTPUT_ENGAGED) = false;
    }
}

#[cfg(not(feature = "hints"))]
#[allow(dead_code)]
mod _interface_type_checks {
    use super::*;
    use zkvm_interface as bindings;

    fn _check() {
        let _ = [bindings::read_input, super::read_input];
        let _ = [bindings::write_output, super::write_output];
    }
}
