use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{fcall_secp256k1_fn_inv, lt},
};

use super::constants::{N, N_MINUS_ONE};

pub fn secp256k1_fn_reduce(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if lt(x, &N) {
        return *x;
    }

    // x·1 + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

pub fn secp256k1_fn_add(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

pub fn secp256k1_fn_neg(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &N_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

pub fn secp256k1_fn_sub(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &N_MINUS_ONE, c: x, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

pub fn secp256k1_fn_mul(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Inverts a non-zero element `x`
pub fn secp256k1_fn_inv(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // Hint the inverse
    let x_inv = fcall_secp256k1_fn_inv(
        x,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check that x·x_inv = 1 (N)
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &x_inv,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(*params.d, [0x1, 0x0, 0x0, 0x0]);

    x_inv
}

/// # Safety
/// - `x_ptr` must point to 4 u64s
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fn_reduce_c")]
pub unsafe extern "C" fn secp256k1_fn_reduce_c(
    x_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);

    if lt(x, &N) {
        *out_ptr.add(0) = x[0];
        *out_ptr.add(1) = x[1];
        *out_ptr.add(2) = x[2];
        *out_ptr.add(3) = x[3];
        return;
    }

    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *out_ptr.add(0) = params.d[0];
    *out_ptr.add(1) = params.d[1];
    *out_ptr.add(2) = params.d[2];
    *out_ptr.add(3) = params.d[3];
}

/// # Safety
/// - `x_ptr` must point to 4 u64s
/// - `y_ptr` must point to 4 u64s
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fn_add_c")]
pub unsafe extern "C" fn secp256k1_fn_add_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);
    let y: &[u64; 4] = &*(y_ptr as *const [u64; 4]);

    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *out_ptr.add(0) = params.d[0];
    *out_ptr.add(1) = params.d[1];
    *out_ptr.add(2) = params.d[2];
    *out_ptr.add(3) = params.d[3];
}

/// # Safety
/// - `x_ptr` must point to 4 u64s
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fn_neg_c")]
pub unsafe extern "C" fn secp256k1_fn_neg_c(
    x_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: x,
        b: &N_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *out_ptr.add(0) = params.d[0];
    *out_ptr.add(1) = params.d[1];
    *out_ptr.add(2) = params.d[2];
    *out_ptr.add(3) = params.d[3];
}

/// # Safety
/// - `x_ptr` must point to 4 u64s
/// - `y_ptr` must point to 4 u64s
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fn_sub_c")]
pub unsafe extern "C" fn secp256k1_fn_sub_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);
    let y: &[u64; 4] = &*(y_ptr as *const [u64; 4]);

    let mut params =
        SyscallArith256ModParams { a: y, b: &N_MINUS_ONE, c: x, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *out_ptr.add(0) = params.d[0];
    *out_ptr.add(1) = params.d[1];
    *out_ptr.add(2) = params.d[2];
    *out_ptr.add(3) = params.d[3];
}

/// # Safety
/// - `x_ptr` must point to 4 u64s
/// - `y_ptr` must point to 4 u64s
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fn_mul_c")]
pub unsafe extern "C" fn secp256k1_fn_mul_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);
    let y: &[u64; 4] = &*(y_ptr as *const [u64; 4]);

    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *out_ptr.add(0) = params.d[0];
    *out_ptr.add(1) = params.d[1];
    *out_ptr.add(2) = params.d[2];
    *out_ptr.add(3) = params.d[3];
}

/// # Safety
/// - `x_ptr` must point to 4 u64s (non-zero element)
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fn_inv_c")]
pub unsafe extern "C" fn secp256k1_fn_inv_c(
    x_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);

    // Hint the inverse
    let x_inv = fcall_secp256k1_fn_inv(
        x,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check that x·x_inv = 1 (N)
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &x_inv,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(*params.d, [0x1, 0x0, 0x0, 0x0]);

    *out_ptr.add(0) = x_inv[0];
    *out_ptr.add(1) = x_inv[1];
    *out_ptr.add(2) = x_inv[2];
    *out_ptr.add(3) = x_inv[3];
}
