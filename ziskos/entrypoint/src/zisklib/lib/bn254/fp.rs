use crate::arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams};

use super::constants::P;

pub fn add_fp_bn254(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

pub fn mul_fp_bn254(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

pub fn square_fp_bn254(x: &[u64; 4]) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}
