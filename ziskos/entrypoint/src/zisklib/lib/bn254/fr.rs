use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{eq, lt},
};

use super::constants::R;

pub fn reduce_fr_bn254(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    if lt(x, &R) {
        return *x;
    }

    // x·1 + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Convert big-endian bytes to little-endian u64 limbs for a scalar (32 bytes -> [u64; 4])
pub fn scalar_bytes_be_to_u64_le_bn254(bytes: &[u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];

    for i in 0..4 {
        for j in 0..8 {
            result[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    result
}
