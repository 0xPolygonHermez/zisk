use std::cmp::Ordering;

use crate::zisklib::fcall_division;

use super::{add_agtb, mul_long, U256};

/// Divides two large numbers
///
/// # Assumptions
/// - `len(a) > 0` and `len(b) > 0`
/// - `a` and `b` have no leading zeros (unless `a` being zero)
/// - `b > 0`
///
/// # Returns
/// A tuple of (quotient, remainder) where a = q·b + r
///
/// # Note
/// Not optimal for `len(b) == 1`, use `div_short` instead
pub fn div_long(a: &[U256], b: &[U256]) -> (Vec<U256>, Vec<U256>) {
    let len_a = a.len();
    let len_b = b.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(len_b, 0, "Input 'b' must have at least one limb");
        assert!(!b[len_b - 1].is_zero(), "Input 'b' must not have leading zeros");
        if len_a > 1 {
            assert!(!a[len_a - 1].is_zero(), "Input 'a' must not have leading zeros");
        }
    }

    // Check if a = b, a < b or a > b
    let comp = U256::compare_slices(a, b);
    if comp == Ordering::Less {
        return (vec![U256::ZERO], a.to_vec());
    } else if comp == Ordering::Equal {
        return (vec![U256::ONE], vec![U256::ZERO]);
    }
    // We can assume a > b from here on

    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma
    let a_flat = U256::slice_to_flat(a);
    let b_flat = U256::slice_to_flat(b);

    // Hint the quotient and remainder
    let mut quo_flat = vec![0u64; len_a * 4];
    let mut rem_flat = vec![0u64; len_b * 4];
    let (limbs_quo, limbs_rem) = fcall_division(a_flat, b_flat, &mut quo_flat, &mut rem_flat);
    let quo = U256::flat_to_slice(&quo_flat[..limbs_quo]);
    let rem = U256::flat_to_slice(&rem_flat[..limbs_rem]);

    // Verify the division

    // Since len(a) >= len(b), the division a = q·b + r must satisfy:
    //      1] max{len(q·b), len(r)} <= len(a) => len(q) + len(b) - 1 <= len(q·b) <= len(a)
    //                                         =>                        len(r)   <= len(a)
    //      2] 1 <= len(r) <= len(b)

    // Check 1 <= len(q) <= len(a) - len(b) + 1
    let len_quo = quo.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(
        len_quo <= len_a - len_b + 1,
        "Quotient length must be less than or equal to dividend length"
    );
    assert!(!quo[len_quo - 1].is_zero(), "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let mut q_b = vec![U256::ZERO; len_a + 1]; // The +1 is because mul_long is a general purpose function
    let q_b_len = mul_long(quo, b, &mut q_b);

    // Check 1 <= len(r)
    let len_rem = rem.len();
    assert!(len_rem > 0, "Remainder must have at least one limb");

    if rem[len_rem - 1].is_zero() {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b[..q_b_len]), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then we should check that a must be equal to q·b + r and r < b

        assert!(U256::lt_slices(rem, b), "Remainder must be less than divisor");

        let mut q_b_r = vec![U256::ZERO; len_a + 1]; // The +1 is because add_agtb is a general purpose function
        let q_b_r_len = add_agtb(&q_b[..q_b_len], rem, &mut q_b_r);
        assert!(U256::eq_slices(a, &q_b_r[..q_b_r_len]), "a != q·b + r");
    }

    (quo.to_vec(), rem.to_vec())
}
