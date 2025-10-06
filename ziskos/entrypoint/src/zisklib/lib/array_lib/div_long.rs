use crate::fcall_division_long;

use super::{add_agtb, mul_long, U256};

/// Division of two large numbers (represented as arrays of U256)
///
/// It assumes that a > 0 and len(b) > 1
pub fn div_long(a: &[U256], b: &[U256]) -> (Vec<U256>, Vec<U256>) {
    let len_a = a.len();
    let len_b = b.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(len_b > 1, "Input 'b' must have more than one limb");
        assert_ne!(a.last().unwrap(), &U256::ZERO, "Input 'a' must not have leading zeros");
    }

    if len_a == 1 {
        let a = a[0];
        if a == U256::ZERO {
            // Return (q,r) = (0,0)
            return (vec![U256::ZERO], vec![U256::ZERO]);
        }

        // As len(b) > 1, we have a < b
        return (vec![U256::ZERO], vec![a]);
    } else if len_a < len_b {
        // We have a < b
        return (vec![U256::ZERO], a.to_vec());
    } else if len_a == len_b {
        // Check if a = b, a < b or a > b TODO: Do with hint and instructions?
        let mut equal = true;
        for i in (0..len_a).rev() {
            if a[i] < b[i] {
                // a < b
                return (vec![U256::ZERO], a.to_vec());
            } else if a[i] > b[i] {
                // a > b
                equal = false;
                break;
            }
        }

        if equal {
            // a == b
            return (vec![U256::ONE], vec![U256::ZERO]);
        }
    }

    // We can assume a > b from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let (quotient_flat, remainder_flat) =
        fcall_division_long(U256::slice_to_flat(a), U256::slice_to_flat(b));
    let quotient = U256::slice_from_flat(&quotient_flat);
    let remainder = U256::slice_from_flat(&remainder_flat);

    // Since len(a) >= len(b), the division a = q·b + r must satisfy:
    //      1] max{len(q·b), len(r)} <= len(a) => len(q) + len(b) - 1 <= len(q·b) <= len(a)
    //                                         =>                        len(r)   <= len(a)
    //      2] 1 <= len(r) <= len(b)

    // Check 1 <= len(q) <= len(a) - len(b) + 1
    let len_quo = quotient.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(
        len_quo <= len_a - len_b + 1,
        "Quotient length must be less than or equal to dividend length"
    );
    assert_ne!(quotient[len_quo - 1], U256::ZERO, "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b = mul_long(&quotient, b);

    // Check 1 <= len(r)
    let len_rem = remainder.len();
    assert!(len_rem > 0, "Remainder must have at least one limb");

    if len_rem == 1 && remainder[0] == U256::ZERO {
        // If the remainder is zero, then a must be equal to q·b
        assert_eq!(a, q_b, "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, check len(r) <= len(b)
        assert!(len_rem <= len_b, "Remainder length must be less than or equal to divisor length");
        assert_ne!(remainder[len_rem - 1], U256::ZERO, "Remainder must not have leading zeros");

        // We also must have r < b
        if len_rem == len_b {
            let mut less = false;
            for i in (0..len_b).rev() {
                if remainder[i] < b[i] {
                    less = true;
                    break;
                } else if remainder[i] > b[i] {
                    break;
                }
            }
            assert!(less, "Remainder must be less than divisor");
        }

        // As the remainder is non-zero, then a must be equal to q·b + r
        let q_b_r = add_agtb(&q_b, &remainder);
        assert_eq!(*a, q_b_r, "a != q·b + r");
    }

    (quotient, remainder)
}
