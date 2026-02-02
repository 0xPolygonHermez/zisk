use crate::zisklib::fcall_division;

use super::{add_short, mul_short, ShortScratch, U256};

/// Computes the remainder of a large number divided by a short number (initial call)
///
/// # Assumptions
/// - `len(a) > 0`
/// - `a` has no leading zeros (unless zero)
/// - `b > 0`
///
/// # Returns
/// The remainder: a mod b
///
/// # Note
/// Use this for the first reduction when `a` can be arbitrarily large.
/// For subsequent reductions in a loop, use `rem_short` with scratch space.
pub fn rem_short_init(
    a: &[U256],
    b: &U256,
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
        let a = a[0];
        if a.is_zero() || a.lt(b) {
            return a;
        } else if a.eq(b) {
            return U256::ZERO;
        }
    }
    // We can assume a > b from here on

    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma
    let a_flat = U256::slice_to_flat(a);

    // Hint the quotient and remainder
    let mut quo_flat = vec![0u64; len_a * 4];
    let mut rem_flat = [0u64; 4];
    let (limbs_quo, _) = fcall_division(
        a_flat,
        b.as_limbs(),
        &mut quo_flat,
        &mut rem_flat,
        #[cfg(feature = "hints")]
        hints,
    );
    let quo = U256::flat_to_slice(&quo_flat[..limbs_quo]);
    let rem = U256::from_u64s(&rem_flat);

    // Verify the division
    let mut q_b = vec![U256::ZERO; len_a + 1]; // The +1 is because mul_long and add_agtb are a general purpose functions
    let mut q_b_r = vec![U256::ZERO; len_a + 1];
    verify_division(
        a,
        b,
        quo,
        &rem,
        &mut q_b,
        &mut q_b_r,
        #[cfg(feature = "hints")]
        hints,
    );

    rem
}

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
        let a = a[0];
        if a.is_zero() || a.lt(b) {
            return a;
        } else if a.eq(b) {
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
