use crate::arith256::{syscall_arith256, SyscallArith256Params};

use super::U256;

/// Multiplication of a large number (represented as an array of U256) by a short U256 number
///
/// It assumes that a,b > 0
pub fn mul_short(a: &[U256], b: &U256) -> Vec<U256> {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(a.last().unwrap(), &U256::ZERO, "Input 'a' must not have leading zeros");
        assert_ne!(b, &U256::ZERO, "Input 'b' must be greater than zero");
    }

    let mut out = vec![U256::ZERO; len_a + 1];
    let mut carry = U256::ZERO;

    for i in 0..len_a {
        // Compute a[i]·b + carry
        let mut params = SyscallArith256Params {
            a: &a[i],
            b,
            c: &carry.clone(),
            dl: &mut out[i],
            dh: &mut carry,
        };
        syscall_arith256(&mut params);
    }

    if carry != U256::ZERO {
        out[len_a] = carry;
    } else {
        out.pop();
    }

    out
}
