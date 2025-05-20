use crate::{
    bn254_complex_add::{syscall_bn254_complex_add, SyscallBn254ComplexAddParams},
    bn254_complex_mul::{syscall_bn254_complex_mul, SyscallBn254ComplexMulParams},
    complex256::SyscallComplex256,
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
