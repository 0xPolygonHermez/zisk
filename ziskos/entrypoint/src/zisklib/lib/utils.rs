use crate::arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams};

pub fn from_be_bytes_to_u64_array(bytes: &[u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];
    for i in 0..4 {
        result[4 - 1 - i] = u64::from_be_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
    }
    result
}

pub fn from_u64_array_to_be_bytes(arr: &[u64; 4]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..4 {
        result[i * 8..(i + 1) * 8].copy_from_slice(&arr[4 - 1 - i].to_be_bytes());
    }
    result
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x > y`; otherwise, false.
pub fn gt(x: &[u64], y: &[u64]) -> bool {
    let len = x.len();
    assert_eq!(len, y.len(), "x and y must have the same length");

    for i in (0..len).rev() {
        if x[i] > y[i] {
            return true;
        } else if x[i] < y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x < y`; otherwise, false.    
pub fn lt(x: &[u64], y: &[u64]) -> bool {
    let len = x.len();
    assert_eq!(len, y.len(), "x and y must have the same length");

    for i in (0..len).rev() {
        if x[i] < y[i] {
            return true;
        } else if x[i] > y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x == y`; otherwise, false.
pub fn eq(x: &[u64], y: &[u64]) -> bool {
    let len = x.len();
    assert_eq!(len, y.len(), "x and y must have the same length");

    for i in 0..len {
        if x[i] != y[i] {
            return false;
        }
    }
    true
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
/// Performs all operations in RISC-V assembly for maximum performance
pub fn exp_power_of_two(x: &[u64; 4], module: &[u64; 4], power_log: usize) -> [u64; 4] {
    // x^1 = x
    if power_log == 0 {
        return *x;
    }

    let mut result = *x;
    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params = SyscallArith256ModParams {
            a: &result,
            b: &result,
            c: &zero,
            module,
            d: &mut [0u64; 4],
        };
        syscall_arith256_mod(&mut params);
        result = *params.d;
    }

    result
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
/// Performs all operations in RISC-V assembly for maximum performance
pub fn exp_power_of_two_self(x: &mut [u64; 4], module: &[u64; 4], power_log: usize) {
    if power_log == 0 {
        return;
    }

    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params =
            SyscallArith256ModParams { a: x, b: x, c: &zero, module, d: &mut [0u64; 4] };
        syscall_arith256_mod(&mut params);
        *x = *params.d;
    }
}
