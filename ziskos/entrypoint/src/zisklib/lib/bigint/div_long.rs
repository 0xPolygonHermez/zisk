use core::cmp::Ordering;

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

use crate::scratch_accelerators::{
    new_scratch_vec_filled, new_scratch_vec_filled_z, scratch_vec_from_slice, ScratchVec,
};

use crate::zisklib::fcall_bigint_div;

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
pub fn div_long(
    a: &[U256],
    b: &[U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (ScratchVec<U256>, ScratchVec<U256>) {
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
        return (new_scratch_vec_filled_z(1, U256::ZERO), scratch_vec_from_slice(a));
    } else if comp == Ordering::Equal {
        return (new_scratch_vec_filled(1, U256::ONE), new_scratch_vec_filled_z(1, U256::ZERO));
    }
    // We can assume a > b from here on

    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma
    let a_flat = U256::slice_to_flat(a);
    let b_flat = U256::slice_to_flat(b);

    // Hint the quotient and remainder
    let mut quo_flat = new_scratch_vec_filled_z(len_a * 4, 0u64);
    let mut rem_flat = new_scratch_vec_filled_z(len_b * 4, 0u64);
    let (limbs_quo, limbs_rem) = fcall_bigint_div(
        a_flat,
        b_flat,
        &mut quo_flat,
        &mut rem_flat,
        #[cfg(feature = "hints")]
        hints,
    );
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
    let mut q_b = new_scratch_vec_filled_z(len_a + 1, U256::ZERO); // The +1 is because mul_long is a general purpose function
    let q_b_len = mul_long(
        quo,
        b,
        &mut q_b,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check 1 <= len(r)
    let len_rem = rem.len();
    assert!(len_rem > 0, "Remainder must have at least one limb");

    if rem[len_rem - 1].is_zero() {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b[..q_b_len]), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then we should check that a must be equal to q·b + r and r < b

        assert!(U256::lt_slices(rem, b), "Remainder must be less than divisor");

        let mut q_b_r = new_scratch_vec_filled_z(len_a + 1, U256::ZERO); // The +1 is because add_agtb is a general purpose function
        let q_b_r_len = add_agtb(
            &q_b[..q_b_len],
            rem,
            &mut q_b_r,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(U256::eq_slices(a, &q_b_r[..q_b_r_len]), "a != q·b + r");
    }

    (scratch_vec_from_slice(quo), scratch_vec_from_slice(rem))
}
