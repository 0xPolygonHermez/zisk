use ziskos::zisklib::{fcall_bigint_div, fcall_bin_decomp};

pub fn diagnostic_bigint() {
    diagnostic_bin_decomp();
    diagnostic_bigint_div();
}

fn diagnostic_bin_decomp() {
    // 10 = 0b1010  →  bits [1, 0, 1, 0] after stripping leading zeros.
    let (len, bits) = fcall_bin_decomp(&[10]);
    assert_eq!(len, 4);
    assert_eq!(bits.as_slice(), &[1, 0, 1, 0]);
}

fn diagnostic_bigint_div() {
    // 10 / 3 = (3, 1).  Output lengths are rounded up to a multiple of 4.
    let a: [u64; 4] = [10, 0, 0, 0];
    let b: [u64; 4] = [3, 0, 0, 0];
    let mut quo = [0u64; 4];
    let mut rem = [0u64; 4];
    let (len_quo, len_rem) = fcall_bigint_div(&a, &b, &mut quo, &mut rem);
    assert_eq!(len_quo, 4);
    assert_eq!(len_rem, 4);
    assert_eq!(quo, [3, 0, 0, 0]);
    assert_eq!(rem, [1, 0, 0, 0]);
}
