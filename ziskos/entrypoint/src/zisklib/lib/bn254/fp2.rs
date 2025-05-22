use crate::{
    bn254_complex_add::{syscall_bn254_complex_add, SyscallBn254ComplexAddParams},
    bn254_complex_mul::{syscall_bn254_complex_mul, SyscallBn254ComplexMulParams},
    bn254_complex_sub::{syscall_bn254_complex_sub, SyscallBn254ComplexSubParams},
    complex256::SyscallComplex256,
    fcall_bn254_fp2_inv,
    zisklib::lib::utils::eq,
    P,
};

pub fn add_fp2_bn254(a: &[u64; 8], b: &[u64; 8]) -> [u64; 8] {
    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 = SyscallComplex256 { x: b[0..4].try_into().unwrap(), y: b[4..8].try_into().unwrap() };

    let mut params = SyscallBn254ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_add(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn dbl_fp2_bn254(a: &[u64; 8]) -> [u64; 8] {
    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 = SyscallComplex256 { x: f1.x, y: f1.y };

    let mut params = SyscallBn254ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_add(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn neg_fp2_bn254(a: &[u64; 8]) -> [u64; 8] {
    let mut f1 = SyscallComplex256 { x: P, y: P };
    let f2 = SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };

    let mut params = SyscallBn254ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_sub(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn sub_fp2_bn254(a: &[u64; 8], b: &[u64; 8]) -> [u64; 8] {
    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 = SyscallComplex256 { x: b[0..4].try_into().unwrap(), y: b[4..8].try_into().unwrap() };

    let mut params = SyscallBn254ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_sub(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn mul_fp2_bn254(a: &[u64; 8], b: &[u64; 8]) -> [u64; 8] {
    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 = SyscallComplex256 { x: b[0..4].try_into().unwrap(), y: b[4..8].try_into().unwrap() };

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn scalar_mul_fp2_bn254(a: &[u64; 8], b: &[u64; 4]) -> [u64; 8] {
    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 = SyscallComplex256 { x: b[0..4].try_into().unwrap(), y: [0, 0, 0, 0] };

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn square_fp2_bn254(a: &[u64; 8]) -> [u64; 8] {
    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 = SyscallComplex256 { x: f1.x, y: f1.y };

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}

pub fn inv_fp2_bn254(a: &[u64; 8]) -> [u64; 8] {
    // if a == 0, return 0
    if eq(a, &[0, 0, 0, 0, 0, 0, 0, 0]) {
        return [0, 0, 0, 0, 0, 0, 0, 0];
    }

    // if a != 0, return 1 / a

    // Remember that an element b ∈ Fp2 is the inverse of a ∈ Fp2 if and only if a·b = 1 in Fp2
    // We will therefore hint the inverse b and check the product with a is 1
    let inv = fcall_bn254_fp2_inv(a);

    let mut f1 =
        SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: a[4..8].try_into().unwrap() };
    let f2 =
        SyscallComplex256 { x: inv[0..4].try_into().unwrap(), y: inv[4..8].try_into().unwrap() };
    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(&mut params);
    assert_eq!(params.f1.x, [1, 0, 0, 0]);
    assert_eq!(params.f1.y, [0, 0, 0, 0]);

    inv
}

pub fn conjugate_fp2_bn254(a: &[u64; 8]) -> [u64; 8] {
    let mut f1 = SyscallComplex256 { x: a[0..4].try_into().unwrap(), y: P };
    let f2 = SyscallComplex256 { x: [0, 0, 0, 0], y: a[4..8].try_into().unwrap() };

    let mut params = SyscallBn254ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_sub(&mut params);
    let res_x = params.f1.x;
    let res_y = params.f1.y;
    [res_x[0], res_x[1], res_x[2], res_x[3], res_y[0], res_y[1], res_y[2], res_y[3]]
}
