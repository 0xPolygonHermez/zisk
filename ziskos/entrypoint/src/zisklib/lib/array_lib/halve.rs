use crate::fcall_division_short;

use super::{add_short, double, U256};

/// Halving of a large number (represented as an array of U256)
///
/// It assumes that a > 0
pub fn halve(a: &[U256]) -> Vec<U256> {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(a.last().unwrap(), &U256::ZERO, "Input 'a' must not have leading zeros");
    }

    if len_a == 1 {
        let a = a[0];
        if a < U256::TWO {
            // Return q = 0
            return vec![U256::ZERO];
        }

        // Check whether a == 2
        if a == U256::TWO {
            // Return q == 1
            return vec![U256::ONE];
        }
    }

    // We can assume a > 2 from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let (quotient_flat, remainder_flat) =
        fcall_division_short(U256::slice_to_flat(a), U256::TWO.as_ref().try_into().unwrap());
    let quotient = U256::slice_from_flat(&quotient_flat);
    let remainder = U256::from_u64s(&remainder_flat);

    // The quotient must satisfy 1 <= len(Q) <= len(inA)
    let len_quo = quotient.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(len_quo <= len_a, "Quotient length must be less than or equal to dividend length");
    assert_ne!(quotient[len_quo - 1], U256::ZERO, "Quotient must not have leading zeros");

    // Double the quotient
    let q_2 = double(&quotient);

    if remainder == U256::ZERO {
        // If the remainder is zero, then a must be equal to 2·q
        assert_eq!(a, &q_2, "Remainder is zero, but a != 2·q");
    } else {
        // If the remainder is non-zero, then a must be equal to 2·q + 1
        let q_2_1 = add_short(&q_2, &U256::ONE);
        assert_eq!(a, &q_2_1, "a != 2·q + 1");
    }

    quotient
}
