use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{eq, lt},
};

use super::constants::R;

/// Reduces a 256-bit value modulo the BN254 scalar field order R
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
