include!("../bindings.rs");

pub fn add_point_ec_c(
    dbl: u64,
    x1: &[u64; 4],
    y1: &[u64; 4],
    x2: &[u64; 4],
    y2: &[u64; 4],
    x3: &mut [u64; 4],
    y3: &mut [u64; 4],
) -> i32 {
    unsafe { AddPointEc(dbl, &x1[0], &y1[0], &x2[0], &y2[0], &mut x3[0], &mut y3[0]) }
}

pub fn add_point_ec_p_c(dbl: u64, p1: &[u64; 8], p2: &[u64; 8], p3: &mut [u64; 8]) -> i32 {
    unsafe { AddPointEcP(dbl, &p1[0], &p2[0], &mut p3[0]) }
}

pub fn inverse_fp_ec_c(params: &[u64; 32], result: &mut [u64; 32]) -> i32 {
    unsafe { InverseFpEc(&params[0], &mut result[0]) }
}

pub fn inverse_fn_ec_c(params: &[u64; 32], result: &mut [u64; 32]) -> i32 {
    unsafe { InverseFnEc(&params[0], &mut result[0]) }
}

pub fn sqrt_fp_ec_parity_c(params: &[u64; 32], result: &mut [u64; 32]) -> i32 {
    unsafe { SqrtFpEcParity(&params[0], params[8], &mut result[0]) }
}
