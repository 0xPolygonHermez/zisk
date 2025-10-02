use num_integer::Integer;

use super::utils::{from_limbs_le_dyn, to_limbs_le_dyn};

/// Perform the division of an unsigned integer by a u64
pub fn fcall_division_long(params: &[u64], results: &mut [u64]) -> i64 {
    let len_a = params[0] as usize;
    let a = &params[1..(1 + len_a)];
    let len_b = params[1 + len_a] as usize;
    let b = &params[(2 + len_a)..(2 + len_a + len_b)];

    let (quo, rem) = division_long(a, b);

    let len_quo = quo.len();
    results[0] = len_quo as u64;
    results[1..(1 + len_quo)].copy_from_slice(&quo);
    let len_rem = rem.len();
    results[1 + len_quo] = len_rem as u64;
    results[(2 + len_quo)..(2 + len_quo + len_rem)].copy_from_slice(&rem);

    2 + len_quo as i64 + len_rem as i64
}

fn division_long(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let a_big = from_limbs_le_dyn(a);
    let b_big = from_limbs_le_dyn(b);

    let (quo_big, rem_big) = a_big.div_rem(&b_big);

    let quo_limbs = to_limbs_le_dyn(&quo_big);
    let rem_limbs = to_limbs_le_dyn(&rem_big);

    let quo_len = quo_limbs.len().div_ceil(4) * 4; // Round up to multiple of 4
    let mut quo = quo_limbs.clone();
    quo.resize(quo_len, 0);

    let rem_len_needed = rem_limbs.len().div_ceil(4) * 4; // Round up to multiple of 4
    let rem_len = if rem_len_needed == 0 { 4 } else { rem_len_needed };
    let mut rem = rem_limbs.clone();
    rem.resize(rem_len, 0);

    (quo, rem)
}
