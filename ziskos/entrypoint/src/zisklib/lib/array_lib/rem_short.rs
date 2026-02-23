use crate::zisklib::fcall_division;

use super::{add_short, mul_short, ShortScratch, U256};

/// Computes the remainder of a large number divided by a short number (with scratch)
///
/// # Assumptions
/// - `len(a) > 0`
/// - `a` has no leading zeros (unless zero)
/// - `b > 0`
///
/// # Returns
/// The remainder: a mod b
pub fn rem_short(
    a: &[U256],
    b: &U256,
    scratch: &mut ShortScratch,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> U256 {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(!b.is_zero(), "Input 'b' must be greater than zero");
        if len_a > 1 {
            assert!(!a[len_a - 1].is_zero(), "Input 'a' must not have leading zeros");
        }
    }

    // Check if a = b, a < b or a > b
    if len_a == 1 {
        let a0 = &a[0];
        let cmp = a0.compare(b);
        if cmp < 0 {
            return *a0;
        } else if cmp == 0 {
            return U256::ZERO;
        }
    }
    // We can assume a > b from here on

    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma
    let a_flat = U256::slice_to_flat(a);

    // Hint the quotient and remainder
    let (limbs_quo, _) = fcall_division(
        a_flat,
        b.as_limbs(),
        &mut scratch.quo,
        &mut scratch.rem,
        #[cfg(feature = "hints")]
        hints,
    );
    let quo = U256::flat_to_slice(&scratch.quo[..limbs_quo]);
    let rem = U256::from_u64s(&scratch.rem);

    // Verify the division
    verify_division(
        a,
        b,
        quo,
        &rem,
        &mut scratch.q_b,
        &mut scratch.q_b_r,
        #[cfg(feature = "hints")]
        hints,
    );

    rem
}

/// Verify that a = q·b + r
#[inline(always)]
fn verify_division(
    a: &[U256],
    b: &U256,
    quo: &[U256],
    rem: &U256,
    q_b: &mut [U256],
    q_b_r: &mut [U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let len_a = a.len();
    let len_quo = quo.len();

    // The quotient must satisfy 1 <= len(Q) <= len(inA)
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(len_quo <= len_a, "Quotient length must be less than or equal to dividend length");
    assert!(!quo[len_quo - 1].is_zero(), "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b_len = mul_short(
        quo,
        b,
        q_b,
        #[cfg(feature = "hints")]
        hints,
    );

    if rem.is_zero() {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b[..q_b_len]), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then we should check that a must be equal to q·b + r and r < b
        assert!(rem.lt(b), "Remainder must be less than divisor");

        let q_b_r_len = add_short(
            &q_b[..q_b_len],
            rem,
            q_b_r,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(U256::eq_slices(a, &q_b_r[..q_b_r_len]), "a != q·b + r");
    }
}
