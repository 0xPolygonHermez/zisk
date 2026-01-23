use crate::syscalls::{syscall_arith256, SyscallArith256Params};

use super::{rem_short, ShortScratch, U256};

/// Multiplies a large number by a short number: out = a · b
///
/// # Assumptions
/// - `len(a) > 0`
/// - `a` has no leading zeros (unless zero)
/// - `out` has at least `len(a) + 1` limbs
///
/// # Returns
/// The number of limbs in the result
pub fn mul_short(
    a: &[U256],
    b: &U256,
    out: &mut [U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> usize {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        if len_a > 1 {
            assert!(!a[len_a - 1].is_zero(), "Input 'a' must not have leading zeros");
        }
    }

    let mut carry = U256::ZERO;
    for i in 0..len_a {
        // Compute a[i]·b + carry
        let cin = carry;
        let mut params = SyscallArith256Params {
            a: a[i].as_limbs(),
            b: b.as_limbs(),
            c: cin.as_limbs(),
            dl: out[i].as_limbs_mut(),
            dh: carry.as_limbs_mut(),
        };
        syscall_arith256(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    if carry.is_zero() {
        len_a
    } else {
        out[len_a] = carry;
        len_a + 1
    }
}

/// Multiplies two single-limb numbers: returns (result, len)
///
/// # Returns
/// A tuple of (result array, number of limbs used)
pub fn mul_short_one_limb(
    a: &U256,
    b: &U256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([U256; 2], usize) {
    let mut out = [U256::ZERO; 2];

    // Compute a * b
    let mut dh = [0u64; 4];
    let mut mul_params = SyscallArith256Params {
        a: a.as_limbs(),
        b: b.as_limbs(),
        c: U256::ZERO.as_limbs(),
        dl: out[0].as_limbs_mut(),
        dh: &mut dh,
    };
    syscall_arith256(
        &mut mul_params,
        #[cfg(feature = "hints")]
        hints,
    );

    let len = if dh == [0u64; 4] {
        1
    } else {
        out[1] = U256::from_u64s(&dh);
        2
    };

    (out, len)
}

/// Multiplies two short numbers and reduces modulo a short modulus
///
/// # Assumptions
/// - `modulus > 0`
///
/// # Returns
/// The remainder: `(a · b) mod modulus`
pub fn mul_and_reduce_short(
    a: &U256,
    b: &U256,
    modulus: &U256,
    scratch: &mut ShortScratch,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> U256 {
    #[cfg(debug_assertions)]
    {
        assert!(!modulus.is_zero(), "Input 'modulus' must not be zero");
    }

    let (mul, len) = mul_short_one_limb(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    rem_short(
        &mul[..len],
        modulus,
        scratch,
        #[cfg(feature = "hints")]
        hints,
    )
}
