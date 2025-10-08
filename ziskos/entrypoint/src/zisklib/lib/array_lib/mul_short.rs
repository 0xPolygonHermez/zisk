use crate::arith256::{syscall_arith256, SyscallArith256Params};

use super::U256;

/// Multiplication of a large number (represented as an array of U256) by a short U256 number
///
/// It assumes that len(a) > 0 and len(out) >= len(a) + 1
pub fn mul_short(a: &[U256], b: &U256, out: &mut [U256]) {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(out.len() >= len_a + 1, "Output 'out' must have at least len(a) + 1 limbs");
    }

    // Start with a[0]·b
    let mut carry = U256::ZERO;
    let mut params = SyscallArith256Params {
        a: &a[0],
        b,
        c: &U256::ZERO,
        dl: &mut out[0],
        dh: &mut carry,
    };
    syscall_arith256(&mut params);

    for i in 1..len_a {
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

    out[len_a] = carry;
}
