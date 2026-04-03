/// Compute RIPEMD-160 hash
#[inline]
pub fn ripemd160(input: &[u8], #[cfg(feature = "hints")] _hints: &mut Vec<u64>) -> [u8; 32] {
    use ripemd::Digest;
    let mut hasher = ripemd::Ripemd160::new();
    hasher.update(input);

    let mut output = [0u8; 32];
    hasher.finalize_into((&mut output[12..]).into());
    output
}

/// C-compatible wrapper for full RIPEMD160 hash
///
/// # Safety
/// - `input` must point to at least `input_len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[inline]
pub(crate) unsafe fn ripemd160_c(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let input_slice = core::slice::from_raw_parts(input, input_len);
    let hash = ripemd160(
        input_slice,
        #[cfg(feature = "hints")]
        hints,
    );
    let output_slice = core::slice::from_raw_parts_mut(output, 32);
    output_slice.copy_from_slice(&hash);
}
