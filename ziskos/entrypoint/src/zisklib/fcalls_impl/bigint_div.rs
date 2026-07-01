use num_integer::Integer;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, u64_digits_from_biguint};

/// Perform the division of an unsigned integer `a` by another unsigned integer `b`,
/// returning the quotient `q` and the remainder `r`, such that `a = b * q + r`
/// It assumes that `b != 0`
pub fn fcall_bigint_div(params: &[u64], results: &mut [u64]) -> i64 {
    let len_a = params[0] as usize;
    let a = &params[1..(1 + len_a)];
    let len_b = params[1 + len_a] as usize;
    let b = &params[(2 + len_a)..(2 + len_a + len_b)];

    let mut q = Vec::with_capacity(len_a);
    let mut r = Vec::with_capacity(len_b);
    bigint_div_into(a, b, &mut q, &mut r);

    let len_q = q.len();
    let len_r = r.len();

    results[0] = len_q as u64;
    results[1..(1 + len_q)].copy_from_slice(&q);
    results[1 + len_q] = len_r as u64;
    results[(2 + len_q)..(2 + len_q + len_r)].copy_from_slice(&r);

    (2 + len_q + len_r) as i64
}

pub fn bigint_div_into(a: &[u64], b: &[u64], q: &mut Vec<u64>, r: &mut Vec<u64>) {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);

    let (q_big, r_big) = a_big.div_rem(&b_big);

    let q_limbs = u64_digits_from_biguint(&q_big);
    let r_limbs = u64_digits_from_biguint(&r_big);

    // Round quotient length up to multiple of 4
    let q_pad = if q_limbs.is_empty() { 4 } else { q_limbs.len().div_ceil(4) * 4 };
    q.extend_from_slice(&q_limbs);
    q.resize(q_pad, 0);

    // Round remainder length up to multiple of 4
    let r_pad = if r_limbs.is_empty() { 4 } else { r_limbs.len().div_ceil(4) * 4 };
    r.extend_from_slice(&r_limbs);
    r.resize(r_pad, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_a_zero() {
        let a = [0, 0, 0, 0];
        let b = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x0];
        let mut q = Vec::new();
        let mut r = Vec::new();
        bigint_div_into(&a, &b, &mut q, &mut r);
        let expected_q = [0, 0, 0, 0];
        let expected_r = [0, 0, 0, 0];

        assert_eq!(q, expected_q);
        assert_eq!(r, expected_r);
    }

    #[test]
    fn test_div_a_eq_b() {
        let a = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x0];
        let b = a;
        let mut q = Vec::new();
        let mut r = Vec::new();
        bigint_div_into(&a, &b, &mut q, &mut r);
        let expected_q = [1, 0, 0, 0];
        let expected_r = [0, 0, 0, 0];

        assert_eq!(q, expected_q);
        assert_eq!(r, expected_r);
    }

    #[test]
    fn test_div_a_lt_b() {
        let a = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x0];
        let b = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x1];
        let mut q = Vec::new();
        let mut r = Vec::new();
        bigint_div_into(&a, &b, &mut q, &mut r);
        let expected_q = [0, 0, 0, 0];
        let expected_r = a;

        assert_eq!(q, expected_q);
        assert_eq!(r, expected_r);
    }

    #[test]
    fn test_div() {
        let a = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x3b4e3fc5e0d8b014];
        let b = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x0];
        let mut q = Vec::new();
        let mut r = Vec::new();
        bigint_div_into(&a, &b, &mut q, &mut r);
        let expected_q = [0x2868ebf5edfaecd5, 0x5, 0x0, 0x0];
        let expected_r = [0xdbb84a86764e268, 0xfd48d6ec2b636246, 0xadbb6db4207ffb8, 0x0];

        assert_eq!(q, expected_q);
        assert_eq!(r, expected_r);
    }
}
