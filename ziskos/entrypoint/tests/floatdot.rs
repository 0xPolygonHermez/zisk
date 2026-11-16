#![allow(unexpected_cfgs)]
use ziskos::syscalls::syscall_fp32_dot;
#[cfg(not(any(zisk_hints, zisk_hints_debug)))]
#[test]
fn packed_fp32_dot_returns_bits_flags_and_ignores_odd_tail() {
    assert_eq!(
        syscall_fp32_dot(
            &[0x3f800000_4b800000, 0x7f800001_cb800000],
            &[0x3f800000_3f800000, 0x7f800001_3f800000],
            3,
            0,
            8
        )
        .unwrap(),
        9u64 << 32
    );
    assert_eq!(syscall_fp32_dot(&[], &[], 0, 0xff800001, 31).unwrap(), (31u64 << 32) | 0xff800001);
}
#[cfg(not(any(zisk_hints, zisk_hints_debug)))]
#[test]
fn packed_fp32_dot_rejects_invalid_buffers_and_flags() {
    assert!(syscall_fp32_dot(&[], &[], 1, 0, 0).is_err());
    assert!(syscall_fp32_dot(&[0], &[], 1, 0, 0).is_err());
    assert!(syscall_fp32_dot(&[0], &[0], 3, 0, 0).is_err());
    assert!(syscall_fp32_dot(&[], &[], usize::MAX, 0, 0).is_err());
    assert!(syscall_fp32_dot(&[], &[], 0, 0, 32).is_err());
    assert!(syscall_fp32_dot(&[0], &[0], 0, 0, 0).is_err());
}

#[cfg(any(zisk_hints, zisk_hints_debug))]
#[test]
fn native_hint_configuration_rejects_even_empty_fp32_calls() {
    assert!(syscall_fp32_dot(&[], &[], 0, 0, 0).is_err());
}

#[cfg(not(any(zisk_hints, zisk_hints_debug)))]
#[test]
fn strided_16_bit_dots_widen_by_format() {
    use ziskos::syscalls::{syscall_floatdot_bf16, syscall_floatdot_fp16};
    // x = [1, _, 2, _, -3] at stride 2; y contiguous [3, 1, 1].
    let x_bf = [0x3f80, 0x7fa1, 0x4000, 0x7fa1, 0xc040];
    let y_bf = [0x4040, 0x3f80, 0x3f80];
    assert_eq!(syscall_floatdot_bf16(&x_bf, 2, &y_bf, 1, 3, 0, 0).unwrap(), 0x4000_0000);
    // Same bits as FP16: 1.875*2.125 + 2*1.875 - 2.125*1.875 = 3.75, all exact.
    assert_eq!(syscall_floatdot_fp16(&x_bf, 2, &y_bf, 1, 3, 0, 0).unwrap(), 0x4070_0000);
    let x_fp = [0x3c00, 0x4000, 0xc200];
    assert_eq!(syscall_floatdot_fp16(&x_fp, 1, &[0x3c00], 0, 3, 0, 8).unwrap(), 8u64 << 32);
    assert_eq!(
        syscall_floatdot_bf16(&[], 7, &[], 0, 0, 0xff800001, 24).unwrap(),
        (24u64 << 32) | 0xff800001
    );
}

#[cfg(not(any(zisk_hints, zisk_hints_debug)))]
#[test]
fn strided_16_bit_dots_reject_short_slices_strides_and_flags() {
    use ziskos::syscalls::{syscall_floatdot_bf16, syscall_floatdot_fp16};
    assert!(syscall_floatdot_bf16(&[0; 4], 2, &[0; 4], 1, 3, 0, 0).is_err(), "x needs index 4");
    assert!(syscall_floatdot_bf16(&[0; 5], 2, &[0; 2], 1, 3, 0, 0).is_err(), "y needs index 2");
    assert!(syscall_floatdot_fp16(&[0; 5], 0x10000, &[0; 5], 1, 1, 0, 0).is_err());
    assert!(syscall_floatdot_fp16(&[0; 5], 1, &[0; 5], 1, 1, 0, 32).is_err());
    assert!(syscall_floatdot_fp16(&[], 1, &[], 1, 4097, 0, 0).is_err());
    assert!(syscall_floatdot_bf16(&[0; 5], 2, &[0; 3], 1, 3, 0, 0).is_ok());
}
