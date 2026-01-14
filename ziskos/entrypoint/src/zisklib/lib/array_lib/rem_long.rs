use std::cmp::Ordering;

use crate::zisklib::fcall_division;

use super::{add_agtb, mul_long, RemLongScratch, U256};

/// Computes the remainder of two large numbers (initial call)
///
/// # Assumptions
/// - `len(a) > 0` and `len(b) > 0`
/// - `a` and `b` have no leading zeros (unless `a` being zero)
/// - `b > 0`
///
/// # Returns
/// The remainder: a mod b
///
/// # Note
/// Use this for the first reduction when `a` can be arbitrarily large.
/// For subsequent reductions in a loop, use `rem_long` with scratch space.
pub fn rem_long_init(
    a: &[U256],
    b: &[U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<U256> {
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
        return a.to_vec();
    } else if comp == Ordering::Equal {
        return vec![U256::ZERO];
    }
    // We can assume a > b from here on

    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma
    let a_flat = U256::slice_to_flat(a);
    let b_flat = U256::slice_to_flat(b);

    // Hint the quotient and remainder
    let mut quo_flat = vec![0u64; len_a * 4];
    let mut rem_flat = vec![0u64; len_b * 4];
    let (limbs_quo, limbs_rem) = fcall_division(
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
    let mut q_b = vec![U256::ZERO; len_a + 1]; // The +1 is because mul_long and add_agtb are a general purpose functions
    let mut q_b_r = vec![U256::ZERO; len_a + 1];
    verify_division(
        a,
        b,
        quo,
        rem,
        &mut q_b,
        &mut q_b_r,
        #[cfg(feature = "hints")]
        hints,
    );

    rem.to_vec()
}

/// Computes the remainder of two large numbers (with scratch)
///
/// # Assumptions
/// - `len(a) > 0` and `len(b) > 0`
/// - `a` and `b` have no leading zeros (unless `a` being zero)
/// - `b > 0`
///
/// # Returns
/// The remainder: a mod b
///
/// # Note
/// Not optimal for `len(b) == 1`, use `rem_short` instead
pub fn rem_long(
    a: &[U256],
    b: &[U256],
    scratch: &mut RemLongScratch,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<U256> {
    #[cfg(debug_assertions)]
    {
        let len_a = a.len();
        let len_b = b.len();
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
        return a.to_vec();
    } else if comp == Ordering::Equal {
        return vec![U256::ZERO];
    }
    // We can assume a > b from here on

    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma
    let a_flat = U256::slice_to_flat(a);
    let b_flat = U256::slice_to_flat(b);

    // Hint the quotient and remainder
    let (limbs_quo, limbs_rem) = fcall_division(
        a_flat,
        b_flat,
        &mut scratch.quo,
        &mut scratch.rem,
        #[cfg(feature = "hints")]
        hints,
    );
    let quo = U256::flat_to_slice(&scratch.quo[..limbs_quo]);
    let rem = U256::flat_to_slice(&scratch.rem[..limbs_rem]);

    // Verify the division
    verify_division(
        a,
        b,
        quo,
        rem,
        &mut scratch.q_b,
        &mut scratch.q_b_r,
        #[cfg(feature = "hints")]
        hints,
    );

    rem.to_vec()
}

/// Verify that a = q·b + r
#[inline(always)]
fn verify_division(
    a: &[U256],
    b: &[U256],
    quo: &[U256],
    rem: &[U256],
    q_b: &mut [U256],
    q_b_r: &mut [U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let len_a = a.len();
    let len_b = b.len();
    let len_quo = quo.len();
    let len_rem = rem.len();

    // Since len(a) >= len(b), the division a = q·b + r must satisfy:
    //      1] max{len(q·b), len(r)} <= len(a) => len(q) + len(b) - 1 <= len(q·b) <= len(a)
    //                                         =>                        len(r)   <= len(a)
    //      2] 1 <= len(r) <= len(b)

    // Check 1 <= len(q) <= len(a) - len(b) + 1
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(
        len_quo <= len_a - len_b + 1,
        "Quotient length must be less than or equal to dividend length"
    );
    assert!(!quo[len_quo - 1].is_zero(), "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b_len = mul_long(
        quo,
        b,
        q_b,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check 1 <= len(r)
    assert!(len_rem > 0, "Remainder must have at least one limb");

    if rem[len_rem - 1].is_zero() {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b[..q_b_len]), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then we should check that a must be equal to q·b + r and r < b

        assert!(U256::lt_slices(rem, b), "Remainder must be less than divisor");

        let q_b_r_len = add_agtb(
            &q_b[..q_b_len],
            rem,
            q_b_r,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(U256::eq_slices(a, &q_b_r[..q_b_r_len]), "a != q·b + r");
    }
}
