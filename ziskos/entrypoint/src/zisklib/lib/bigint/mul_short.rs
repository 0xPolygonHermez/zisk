use crate::syscalls::{
    syscall_arith256, syscall_arith256_mod, SyscallArith256ModParams, SyscallArith256Params,
};

use super::U256;

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

/// Computes `(a * b) mod modulus` for single-U256 operands
#[inline(always)]
pub fn mulmod_short(
    a: &U256,
    b: &U256,
    modulus: &U256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> U256 {
    #[cfg(debug_assertions)]
    {
        assert!(!modulus.is_zero(), "Input 'modulus' must not be zero");
    }

    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: a.as_limbs(),
        b: b.as_limbs(),
        c: U256::ZERO.as_limbs(),
        module: modulus.as_limbs(),
        d: &mut d,
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    U256::from_u64s(&d)
}
