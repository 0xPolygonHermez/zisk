use crate::arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams};

const P: [u64; 4] =
    [0x3C208C16D87CFD47, 0x97816A916871CA8D, 0xB85045B68181585D, 0x30644E72E131A029];

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
