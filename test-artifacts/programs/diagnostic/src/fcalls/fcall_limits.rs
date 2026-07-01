use ziskos::zisklib::{fcall_bigint_div, fcall_bin_decomp};

// From core/src/fcall.rs
const FCALL_PARAMS_MAX_SIZE: usize = 386;
const FCALL_RESULT_MAX_SIZE: usize = 8193;

pub fn diagnostic_fcall_limits() {
    diagnostic_max_fcall_params_size();
    diagnostic_max_fcall_result_size();
}

fn diagnostic_max_fcall_params_size() {
    let a = vec![0u64; 256];
    let mut b = vec![0u64; 128];
    b[0] = 1;
    assert_eq!(1 + a.len() + 1 + b.len(), FCALL_PARAMS_MAX_SIZE);

    let mut quo = vec![u64::MAX; 4];
    let mut rem = vec![u64::MAX; 4];
    let (len_quo, len_rem) = fcall_bigint_div(&a, &b, &mut quo, &mut rem);
    assert_eq!(len_quo, 4);
    assert_eq!(len_rem, 4);
    assert_eq!(quo, [0, 0, 0, 0]);
    assert_eq!(rem, [0, 0, 0, 0]);
}

fn diagnostic_max_fcall_result_size() {
    let mut exp = vec![0u64; (FCALL_RESULT_MAX_SIZE - 1) / 64];
    let last_exp_idx = exp.len() - 1;
    exp[last_exp_idx] = 1u64 << 63;

    let (len, bits) = fcall_bin_decomp(&exp);
    assert_eq!(1 + len, FCALL_RESULT_MAX_SIZE);
    assert_eq!(bits.len(), len);
    assert_eq!(bits[0], 1);
    assert!(bits[1..].iter().all(|&bit| bit == 0));
}
