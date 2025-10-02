use num_integer::Integer;

use super::utils::{from_limbs_le_dyn, to_limbs_le_dyn};

/// Perform the division of an unsigned integer by a u64
pub fn fcall_division_short(params: &[u64], results: &mut [u64]) -> i64 {
    let len_a = params[0] as usize;
    let a = &params[1..(1 + len_a)];
    let b = &params[(1 + len_a)..(1 + len_a + 4)];

    let (quo, rem) = division_short(a, b.try_into().unwrap());

    let len_quo = quo.len();
    results[0] = len_quo as u64;
    results[1..(1 + len_quo)].copy_from_slice(&quo);
    results[(1 + len_quo)..(1 + len_quo + 4)].copy_from_slice(&rem);

    1 + len_quo as i64 + 4
}

fn division_short(a: &[u64], b: &[u64; 4]) -> (Vec<u64>, [u64; 4]) {
    let a_big = from_limbs_le_dyn(a);
    let b_big = from_limbs_le_dyn(b);

    let (quo_big, rem_big) = a_big.div_rem(&b_big);

    let quo_limbs = to_limbs_le_dyn(&quo_big);
    let rem_limbs = to_limbs_le_dyn(&rem_big);

    let quo_len = quo_limbs.len().div_ceil(4) * 4; // Round up to multiple of 4
    let mut quo = quo_limbs.clone();
    quo.resize(quo_len, 0);

    let mut rem = [0u64; 4];
    rem[..rem_limbs.len()].copy_from_slice(&rem_limbs[..]);

    (quo, rem)
}
