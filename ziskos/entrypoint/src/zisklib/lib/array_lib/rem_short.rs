use crate::fcall_division;

use super::{add_short, mul_short, U256};

/// Division of a large number (represented as an array of U256) by a short U256 number
///
/// It assumes that a > 0, b > 1
pub fn rem_short(a: &[U256], b: &U256) -> U256 {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(b > &U256::ONE, "Input 'b' must be greater than one");
    }

    if len_a == 1 {
        let a = a[0];
        if a == U256::ZERO {
            // Return r = 0
            return U256::ZERO;
        }

        // Check whether a < b or a == b
        // TODO: Do with hint and instructions?
        if a < *b {
            // Return r = a
            return a;
        } else if a == *b {
            // Return r = 0
            return U256::ZERO;
        }
    }

    // Check if a = b, a < b or a > b
    let comp = U256::compare_slices(a, &[*b]);
    if comp == std::cmp::Ordering::Less {
        // a < b. Return r = a
        return a[0];
    } else if comp == std::cmp::Ordering::Equal {
        // a == b. Return r = 0
        return U256::ZERO;
    }

    // We can assume a > b from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let (quo_flat, rem_flat) =
        fcall_division(U256::slice_to_flat(a), b.as_ref().try_into().unwrap());
    let quo = U256::slice_from_flat(&quo_flat);
    let rem = U256::slice_from_flat(&rem_flat)[0];

    // The quotient must satisfy 1 <= len(Q) <= len(inA)
    let len_quo = quo.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(len_quo <= len_a, "Quotient length must be less than or equal to dividend length");
    assert_ne!(quo[len_quo - 1], U256::ZERO, "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let len_qb = len_quo + 1;
    let mut q_b = vec![U256::ZERO; len_qb];
    mul_short(&quo, b, &mut q_b);

    if rem == U256::ZERO {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then a must be equal to q·b + r and r < b
        assert_ne!(rem, U256::ZERO, "Remainder must be non-zero");
        assert!(rem < *b, "Remainder must be less than divisor");

        let carry = add_short(&mut q_b, &rem);
        if !carry {
            assert!(U256::eq_slices(a, &q_b), "a != q·b + r");
        } else {
            for i in 0..len_qb {
                assert_eq!(a[i], q_b[i], "a != q·b + r");
            }
            assert_eq!(a[len_a - 1], U256::ONE, "a != q·b + r");
        }
    }

    rem
}
