use ziskos::syscalls::{DOT_BF16_MAX_N, SyscallDotBf16Params, bf16_from_f32, syscall_dot_bf16};

pub fn project(
    activations: &[u32],
    columns: usize,
    matrices: &[&[u8]],
    outputs: &mut [&mut [f32]],
) {
    assert!(columns > 0 && columns.is_multiple_of(4) && columns as u64 <= DOT_BF16_MAX_N);
    assert!(activations.len().is_multiple_of(columns));
    assert_eq!(matrices.len(), outputs.len());
    let row_bytes = columns.checked_mul(2).expect("BF16 row size overflow");
    // dot_bf16 reads its vectors as aligned 64-bit words, and a Vec<u16> is only 2-byte aligned:
    // the BF16 activations are packed four per u64 (little endian, as the guest), which is 8-byte
    // aligned. Each row is columns / 4 whole words, so every row starts aligned too
    let row_words = columns / 4;
    let inputs: Vec<u64> = activations
        .chunks_exact(4)
        .map(|values| {
            values.iter().rev().fold(0u64, |word, &bits| {
                word << 16 | u64::from(bf16_from_f32(f32::from_bits(bits)))
            })
        })
        .collect();
    let positions = inputs.len() / row_words;

    for (&matrix, output) in matrices.iter().zip(outputs) {
        assert_eq!(matrix.as_ptr().align_offset(8), 0, "unaligned BF16 weights");
        assert!(matrix.len().is_multiple_of(row_bytes));
        let rows = matrix.len() / row_bytes;
        assert!(rows > 0);
        assert_eq!(rows.checked_mul(positions), Some(output.len()));

        for (input, output_row) in inputs.chunks_exact(row_words).zip(output.chunks_exact_mut(rows))
        {
            for (weights, score) in matrix.chunks_exact(row_bytes).zip(output_row) {
                let params = SyscallDotBf16Params {
                    x: weights.as_ptr().cast(),
                    y: input.as_ptr().cast(),
                    n: columns as u64,
                    flags: 0,
                    block: 0,
                    out: core::ptr::null_mut(),
                };
                // SAFETY: both aligned operands hold exactly n BF16 values; scalar mode doesn't use out.
                let dot = unsafe { syscall_dot_bf16(&params) };
                *score = f32::from_bits(u32::from(bf16_from_f32(dot)) << 16);
            }
        }
    }
}
