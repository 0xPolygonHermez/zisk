use crate::syscalls::sha256f::{syscall_sha256_f, SyscallSha256Params};

#[inline]
pub fn sha256f_compress(state: &mut [u32; 8], blocks: &[[u8; 64]]) {
    // Convert the 32-bit state slice to a 64-bit state slice
    let mut state_64 = convert_u32_to_u64(state);

    for block in blocks {
        // Convert the byte block slice to a 64-bit input slice
        let input_u64 = convert_bytes_to_u64(block);

        let mut sha256_params = SyscallSha256Params { state: &mut state_64, input: &input_u64 };
        syscall_sha256_f(&mut sha256_params);
    }

    // Convert the 64-bit state slice back to 32-bit state slice
    *state = convert_u64_to_u32(&state_64);
}

#[inline]
fn convert_u32_to_u64(words: &[u32; 8]) -> [u64; 4] {
    let mut out = [0u64; 4];
    for i in 0..4 {
        out[i] = ((words[2 * i] as u64) << 32) | (words[2 * i + 1] as u64);
    }
    out
}

#[inline]
fn convert_u64_to_u32(input: &[u64; 4]) -> [u32; 8] {
    let mut out = [0u32; 8];
    for (i, &word) in input.iter().enumerate() {
        out[2 * i] = (word >> 32) as u32;
        out[2 * i + 1] = (word & 0xFFFF_FFFF) as u32;
    }
    out
}

#[inline]
fn convert_bytes_to_u64(input: &[u8; 64]) -> [u64; 8] {
    let mut out = [0u64; 8];
    for (i, chunk) in input.chunks_exact(8).enumerate() {
        let mut word = 0u64;
        for (j, &byte) in chunk.iter().enumerate() {
            word |= (byte as u64) << (56 - j * 8);
        }
        out[i] = word;
    }
    out
}
