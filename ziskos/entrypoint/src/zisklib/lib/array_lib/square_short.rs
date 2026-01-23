use crate::syscalls::{syscall_arith256, SyscallArith256Params};

use super::{rem_short, ShortScratch, U256};

/// Squares a single-limb number: returns (result, len)
///
/// # Assumptions
/// - `a > 0`
///
/// # Returns
/// A tuple of (result array, number of limbs used)
pub fn square_short(
    a: &U256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([U256; 2], usize) {
    #[cfg(debug_assertions)]
    {
        assert!(!a.is_zero(), "Input 'a' must not have leading zeros");
    }

    let mut out = [U256::ZERO; 2];

    // Compute a * a
    let mut dh = [0u64; 4];
    let mut sq_params = SyscallArith256Params {
        a: a.as_limbs(),
        b: a.as_limbs(),
        c: U256::ZERO.as_limbs(),
        dl: out[0].as_limbs_mut(),
        dh: &mut dh,
    };
    syscall_arith256(
        &mut sq_params,
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

/// Squares a short number and reduces modulo a short modulus
///
/// # Assumptions
/// - `modulus > 0`
///
/// # Returns
/// The remainder: a² mod modulus
pub fn square_and_reduce_short(
    a: &U256,
    modulus: &U256,
    scratch: &mut ShortScratch,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> U256 {
    #[cfg(debug_assertions)]
    {
        assert!(!modulus.is_zero(), "Input 'modulus' must not be zero");
    }

    let (sq, len) = square_short(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    rem_short(
        &sq[..len],
        modulus,
        scratch,
        #[cfg(feature = "hints")]
        hints,
    )
}
