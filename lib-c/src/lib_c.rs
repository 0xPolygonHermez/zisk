#![allow(unused_variables)]

include!("../bindings.rs");

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
macro_rules! run_on_linux {
    ($body:expr) => {
        0
    };
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
macro_rules! run_on_linux {
    ($body:expr) => {
        unsafe { $body }
    };
}

pub fn add_point_ec_c(
    dbl: u64,
    x1: &[u64; 4],
    y1: &[u64; 4],
    x2: &[u64; 4],
    y2: &[u64; 4],
    x3: &mut [u64; 4],
    y3: &mut [u64; 4],
) -> i32 {
    run_on_linux!(AddPointEc(dbl, &x1[0], &y1[0], &x2[0], &y2[0], &mut x3[0], &mut y3[0]))
}

pub fn add_point_ec_p_c(dbl: u64, p1: &[u64; 8], p2: &[u64; 8], p3: &mut [u64; 8]) -> i32 {
    run_on_linux!(AddPointEcP(dbl, &p1[0], &p2[0], &mut p3[0]))
}

pub fn secp256k1_fp_inv_c(params: &[u64], result: &mut [u64]) -> i32 {
    run_on_linux!(InverseFpEc(&params[0], &mut result[0]))
}

pub fn secp256k1_fn_inv_c(params: &[u64], result: &mut [u64]) -> i32 {
    run_on_linux!(InverseFnEc(&params[0], &mut result[0]))
}

pub fn secp256k1_fp_parity_sqrt_c(params: &[u64], parity: u64, result: &mut [u64]) -> i32 {
    run_on_linux!(SqrtFpEcParity(&params[0], parity, &mut result[0]))
}

pub fn inverse_fp_ec_c(params: &[u64; 32], result: &mut [u64; 32]) -> i32 {
    run_on_linux!(InverseFpEc(&params[0], &mut result[0]))
}

pub fn inverse_fn_ec_c(params: &[u64; 32], result: &mut [u64; 32]) -> i32 {
    run_on_linux!(InverseFnEc(&params[0], &mut result[0]))
}

pub fn sqrt_fp_ec_parity_c(params: &[u64; 32], result: &mut [u64; 32]) -> i32 {
    run_on_linux!(SqrtFpEcParity(&params[0], params[8], &mut result[0]))
}

pub fn arith256_c(
    a: &[u64; 4],
    b: &[u64; 4],
    c: &[u64; 4],
    dl: &mut [u64; 4],
    dh: &mut [u64; 4],
) -> i32 {
    run_on_linux!(Arith256(&a[0], &b[0], &c[0], &mut dl[0], &mut dh[0]))
}

pub fn arith256_mod_c(
    a: &[u64; 4],
    b: &[u64; 4],
    c: &[u64; 4],
    module: &mut [u64; 4],
    d: &mut [u64; 4],
) -> i32 {
    run_on_linux!(Arith256(&a[0], &b[0], &c[0], &mut module[0], &mut d[0]))
}

pub fn add256(a: &[u64; 4], b: &[u64; 4], cin: u64, c: &mut [u64; 4]) -> i32 {
    run_on_linux!(Add256(&a[0], &b[0], cin, &mut c[0]))
}
