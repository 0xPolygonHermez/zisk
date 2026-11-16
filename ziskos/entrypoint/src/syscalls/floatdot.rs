//! `floatdot`: ordered dot product with an FP32 accumulator. Every term is a separately
//! rounded RNE FP32 multiply, then a separately rounded RNE FP32 add, in index order.
//! Inputs are FP32, BF16 or FP16, exactly embedded in FP32; there is no FP16 accumulation.

/// `floatdot` over packed FP32 bit patterns.
///
/// Each u64 packs two FP32 values, low half first. Both slices must contain exactly
/// ceil(n/2) words, n <= 4096, and flags may use only bits 0..4. For odd n the last
/// high half is ignored. Return value is final FP32 bits | (sticky flags << 32).
/// Empty calls preserve initial bits/flags. Words provide the required 8-byte alignment.
///
/// Rust execution/replay requires the matching FloatDot AIR and fresh proof setup.
/// Assembly, hints and opcode cost profiling remain unsupported.
pub fn syscall_floatdot_fp32(
    x: &[u64],
    y: &[u64],
    n: usize,
    initial: u32,
    flags: u8,
) -> Result<u64, &'static str> {
    if n > zisk_definitions::FLOATDOT_MAX_LEN || flags & !31 != 0 {
        return Err("floatdot invalid length or flags");
    }
    if x.len() != n.div_ceil(2) || y.len() != x.len() {
        return Err("floatdot needs exactly ceil(n/2) words in each vector");
    }
    if cfg!(any(feature = "hints", zisk_hints, zisk_hints_debug)) {
        return Err("floatdot hints support is not implemented");
    }
    #[cfg(not(zisk_guest))]
    {
        use zisk_precomp_helpers::{fp32_dot, Fp32Result};
        let unpack = |words: &[u64]| -> Vec<u32> {
            words.iter().flat_map(|&w| [w as u32, (w >> 32) as u32]).take(n).collect()
        };
        let result = fp32_dot(&unpack(x), &unpack(y), Fp32Result { bits: initial, flags })?;
        Ok(result.bits as u64 | ((result.flags as u64) << 32))
    }
    #[cfg(zisk_guest)]
    {
        let params = [x.as_ptr() as u64, y.as_ptr() as u64, n as u64, initial as u64, flags as u64];
        Ok(crate::ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_FLOATDOT_ID, params.as_ptr()))
    }
}

/// Legacy name of [`syscall_floatdot_fp32`], kept for existing FP32 callers.
pub fn syscall_fp32_dot(
    x: &[u64],
    y: &[u64],
    n: usize,
    initial: u32,
    flags: u8,
) -> Result<u64, &'static str> {
    syscall_floatdot_fp32(x, y, n, initial, flags)
}

/// `floatdot` over BF16 operands, with FP32 accumulation and separate RNE
/// multiply/add. Element i of x is `x[i * stride_x]`; same for y. Each BF16 value is
/// embedded exactly as `bits << 16`. Strides are element units in 0..=65535; n <= 4096.
/// The enclosing aligned 8-byte word of every element is read, so it must be mapped.
/// Return value and continuation are as for [`syscall_floatdot_fp32`].
pub fn syscall_floatdot_bf16(
    x: &[u16],
    stride_x: usize,
    y: &[u16],
    stride_y: usize,
    n: usize,
    initial: u32,
    flags: u8,
) -> Result<u64, &'static str> {
    dot16(1, x, stride_x, y, stride_y, n, initial, flags)
}

/// As [`syscall_floatdot_bf16`] for IEEE binary16 operands, exactly widened to binary32.
/// Subnormals are exact; a signaling NaN raises NV in the following FP32 multiply.
pub fn syscall_floatdot_fp16(
    x: &[u16],
    stride_x: usize,
    y: &[u16],
    stride_y: usize,
    n: usize,
    initial: u32,
    flags: u8,
) -> Result<u64, &'static str> {
    dot16(2, x, stride_x, y, stride_y, n, initial, flags)
}

#[allow(clippy::too_many_arguments)]
fn dot16(
    format: u64,
    x: &[u16],
    stride_x: usize,
    y: &[u16],
    stride_y: usize,
    n: usize,
    initial: u32,
    flags: u8,
) -> Result<u64, &'static str> {
    if n > zisk_definitions::FLOATDOT_MAX_LEN || flags & !31 != 0 {
        return Err("floatdot invalid length or flags");
    }
    if stride_x > 0xffff || stride_y > 0xffff {
        return Err("floatdot stride exceeds 65535 elements");
    }
    // Strides are bounded, so (n-1)*stride cannot overflow.
    let covers = |len: usize, stride: usize| n == 0 || (n - 1) * stride < len;
    if !covers(x.len(), stride_x) || !covers(y.len(), stride_y) {
        return Err("floatdot strided vector exceeds its slice");
    }
    if cfg!(any(feature = "hints", zisk_hints, zisk_hints_debug)) {
        return Err("floatdot hints support is not implemented");
    }
    #[cfg(not(zisk_guest))]
    {
        use zisk_precomp_helpers::{fp32_dot, FloatDotFormat, Fp32Result};
        let format = if format == 1 { FloatDotFormat::Bf16 } else { FloatDotFormat::Fp16 };
        let widen = |v: &[u16], stride: usize| -> Vec<u32> {
            (0..n).map(|i| format.widen(v[i * stride])).collect()
        };
        let result = fp32_dot(
            &widen(x, stride_x),
            &widen(y, stride_y),
            Fp32Result { bits: initial, flags },
        )?;
        Ok(result.bits as u64 | ((result.flags as u64) << 32))
    }
    #[cfg(zisk_guest)]
    {
        let params = [
            x.as_ptr() as u64 | (stride_x as u64) << 32,
            y.as_ptr() as u64 | (stride_y as u64) << 32,
            n as u64,
            initial as u64,
            flags as u64 | format << 8,
        ];
        Ok(crate::ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_FLOATDOT_ID, params.as_ptr()))
    }
}
