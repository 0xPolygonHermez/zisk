use crate::fcall_division_short;

use super::{add_short, mul_short, U256};

/// Division of a large number (represented as an array of U256) by a short U256 number
///
/// It assumes that a > 0, b > 1
pub fn div_short(a: &[U256], b: &U256) -> (Vec<U256>, U256) {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(a.last().unwrap(), &U256::ZERO, "Input 'a' must not have leading zeros");
        assert!(b > &U256::ONE, "Input 'b' must be greater than one");
    }

    if len_a == 1 {
        let a = a[0];
        if a == U256::ZERO {
            // Return (q,r) = (0,0)
            return (vec![U256::ZERO], U256::ZERO);
        }

        // Check whether a < b or a == b
        // TODO: Do with hint and instructions?
        if a < *b {
            // Return (q,r) = (0,a)
            return (vec![U256::ZERO], a);
        } else if a == *b {
            // Return (q,r) = (1,0)
            return (vec![U256::ONE], U256::ZERO);
        }
    }

    // We can assume a > b from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let (quotient_flat, remainder_flat) =
        fcall_division_short(U256::slice_to_flat(a), b.as_ref().try_into().unwrap());
    let quotient = U256::slice_from_flat(&quotient_flat);
    let remainder = U256::from_u64s(&remainder_flat);

    // The quotient must satisfy 1 <= len(Q) <= len(inA)
    let len_quo = quotient.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(len_quo <= len_a, "Quotient length must be less than or equal to dividend length");
    assert_ne!(quotient[len_quo - 1], U256::ZERO, "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b = mul_short(&quotient, b);

    if remainder == U256::ZERO {
        // If the remainder is zero, then a must be equal to q·b
        assert_eq!(a, &q_b, "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then a must be equal to q·b + r and r < b
        assert_ne!(remainder, U256::ZERO, "Remainder must be non-zero");
        assert!(remainder < *b, "Remainder must be less than divisor");

        let q_b_r = add_short(&q_b, &remainder);
        assert_eq!(a, &q_b_r, "a != q·b + r");
    }

    (quotient, remainder)
}
