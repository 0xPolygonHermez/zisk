use num_integer::Integer;
use num_bigint::BigUint;

use super::utils::biguint_from_u64_digits;

/// Perform the division of an unsigned integer by a u64
pub fn fcall_division(params: &[u64], results: &mut [u64]) -> i64 {
    let len_a = params[0] as usize;
    let a = &params[1..(1 + len_a)];
    let len_b = params[1 + len_a] as usize;
    let b = &params[(2 + len_a)..(2 + len_a + len_b)];

    let mut q = Vec::with_capacity(len_a);
    let mut r = Vec::with_capacity(len_b);
    division_into(a, b, &mut q, &mut r);

    let len_q = q.len();
    let len_r = r.len();

    results[0] = len_q as u64;
    results[1..(1 + len_q)].copy_from_slice(&q);
    results[1 + len_q] = len_r as u64;
    results[(2 + len_q)..(2 + len_q + len_r)].copy_from_slice(&r);

    (2 + len_q + len_r) as i64
}

fn division_into(a: &[u64], b: &[u64], q: &mut Vec<u64>, r: &mut Vec<u64>) {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);

    let (q_big, r_big) = a_big.div_rem(&b_big);

    let q_limbs = BigUint::to_u64_digits(&q_big);
    let r_limbs = BigUint::to_u64_digits(&r_big);

    // Round quotient length up to multiple of 4 (Since a >= b, quotient cannot be 0)
    let q_pad = q_limbs.len().div_ceil(4) * 4;
    q.extend_from_slice(&q_limbs);
    q.resize(q_pad, 0);

    // Round remainder length up to multiple of 4 (Remainder can be 0)
    let r_pad = if r_limbs.is_empty() { 4 } else { r_limbs.len().div_ceil(4) * 4 };
    r.extend_from_slice(&r_limbs);
    r.resize(r_pad, 0);
}
