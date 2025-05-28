use crate::syscalls::sha256f::{syscall_sha256_f, SyscallSha256Params};

#[inline]
pub fn sha256f_compress(state: &mut [u32; 8], blocks: &[[u8; 64]]) {
    let mut state_64 = [0u64; 4];

    // Convert the 32-bit state slice to a 64-bit state slice
    for (i, chunk) in state.chunks_exact(2).enumerate().take(4) {
        let [high, low]: [u32; 2] = chunk.try_into().unwrap();
        let high_bytes = high.to_be_bytes();
        let low_bytes = low.to_be_bytes();
        state_64[i] = u64::from_be_bytes([
            high_bytes[0],
            high_bytes[1],
            high_bytes[2],
            high_bytes[3],
            low_bytes[0],
            low_bytes[1],
            low_bytes[2],
            low_bytes[3],
        ]);
    }

    let mut input_u64 = [0u64; 8];

    for block in blocks {
        // Convert the byte block slice to a 64-bit input slice
        for (i, chunk) in block.chunks_exact(8).enumerate() {
            input_u64[i] = u64::from_be_bytes(chunk.try_into().unwrap());
        }

        let mut sha256_params = SyscallSha256Params { state: &mut state_64, input: &input_u64 };
        syscall_sha256_f(&mut sha256_params);
    }

    // Convert the 64-bit state slice back to 32-bit state slice
    for (i, word) in state_64.iter().enumerate() {
        let bytes = word.to_be_bytes();
        state[2 * i] = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        state[2 * i + 1] = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    }
}
