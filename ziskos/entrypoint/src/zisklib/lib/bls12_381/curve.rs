//! Operations on the BLS12_381 curve E: y² = x³ + 4

use crate::{
    bls12_381_curve_add::{syscall_bls12_381_curve_add, SyscallBls12_381CurveAddParams},
    bls12_381_curve_dbl::syscall_bls12_381_curve_dbl,
    point::SyscallPoint384,
};

/// Addition of two non-zero and distinct points
pub unsafe fn add_bls12_381(p1: *mut u64, p2: *const u64) {
    let mut p1_point = SyscallPoint384 {
        x: core::ptr::read(p1.cast::<[u64; 6]>()),
        y: core::ptr::read(p1.add(6).cast::<[u64; 6]>()),
    };
    let p2_point = SyscallPoint384 {
        x: core::ptr::read(p2.cast::<[u64; 6]>()),
        y: core::ptr::read(p2.add(6).cast::<[u64; 6]>()),
    };

    let mut params = SyscallBls12_381CurveAddParams { p1: &mut p1_point, p2: &p2_point };
    syscall_bls12_381_curve_add(&mut params);

    core::ptr::write(p1.cast::<[u64; 6]>(), p1_point.x);
    core::ptr::write(p1.add(6).cast::<[u64; 6]>(), p1_point.y);
}

/// Doubling of a non-zero point
pub unsafe fn dbl_bls12_381(p: *mut u64) {
    let mut p_point = SyscallPoint384 {
        x: core::ptr::read(p.cast::<[u64; 6]>()),
        y: core::ptr::read(p.add(6).cast::<[u64; 6]>()),
    };

    syscall_bls12_381_curve_dbl(&mut p_point);

    core::ptr::write(p.cast::<[u64; 6]>(), p_point.x);
    core::ptr::write(p.add(6).cast::<[u64; 6]>(), p_point.y);
}
