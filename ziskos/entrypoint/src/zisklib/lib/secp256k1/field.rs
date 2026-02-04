use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{fcall_secp256k1_fp_inv, fcall_secp256k1_fp_sqrt, lt},
};

use super::constants::{NQR, P, P_MINUS_ONE};

pub fn secp256k1_fp_reduce(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if lt(x, &P) {
        return *x;
    }

    // x·1 + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

pub fn secp256k1_fp_add(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

pub fn secp256k1_fp_negate(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

pub fn secp256k1_fp_mul(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

pub fn secp256k1_fp_mul_scalar(
    x: &[u64; 4],
    scalar: u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·scalar + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[scalar, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

pub fn secp256k1_fp_square(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Inverts a non-zero element `x`
pub fn secp256k1_fp_inv(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // Hint the inverse
    let x_inv = fcall_secp256k1_fp_inv(
        x,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check that x·x_inv = 1 (P)
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &x_inv,
        c: &[0, 0, 0, 0],
        module: &P,
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

pub fn secp256k1_fp_sqrt(
    x: &[u64; 4],
    parity: u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    // Hint the sqrt
    let hint = fcall_secp256k1_fp_sqrt(
        x,
        parity,
        #[cfg(feature = "hints")]
        hints,
    );
    let is_qr = hint[0] == 1;
    let sqrt = hint[1..5].try_into().unwrap();

    // Compute sqrt * sqrt
    let mut params = SyscallArith256ModParams {
        a: &sqrt,
        b: &sqrt,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    if is_qr {
        // Check that sqrt * sqrt == x
        assert_eq!(*params.d, *x);
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = secp256k1_fp_mul(
            x,
            &NQR,
            #[cfg(feature = "hints")]
            hints,
        );
        assert_eq!(*params.d, nqr);
        (sqrt, false)
    }
}

// ==================== C FFI Functions ====================

/// # Safety
/// - `x_ptr` must point to 4 u64s
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fp_reduce_c")]
pub unsafe extern "C" fn secp256k1_fp_reduce_c(
    x_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);

    if lt(x, &P) {
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
        module: &P,
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
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fp_add_c")]
pub unsafe extern "C" fn secp256k1_fp_add_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);
    let y: &[u64; 4] = &*(y_ptr as *const [u64; 4]);

    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
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
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fp_negate_c")]
pub unsafe extern "C" fn secp256k1_fp_negate_c(
    x_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &P,
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
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fp_mul_c")]
pub unsafe extern "C" fn secp256k1_fp_mul_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);
    let y: &[u64; 4] = &*(y_ptr as *const [u64; 4]);

    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
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
/// - `scalar` is a single u64 value
/// - `out_ptr` must point to at least 4 u64s
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_fp_mul_scalar_c")]
pub unsafe extern "C" fn secp256k1_fp_mul_scalar_c(
    x_ptr: *const u64,
    scalar: u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x: &[u64; 4] = &*(x_ptr as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[scalar, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &P,
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
