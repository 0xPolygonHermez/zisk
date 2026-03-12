use crate::syscalls::{syscall_add256, SyscallAdd256Params};

use super::U256;

/// Adds two large numbers: out = a + b
///
/// # Assumptions
/// - `len(a) >= len(b) > 0`
/// - `a` and `b` have no leading zeros (unless zero)
/// - `out` has at least `len(a) + 1` limbs
///
/// # Returns
/// The number of limbs in the result
pub fn add_agtb(
    a: &[U256],
    b: &[U256],
    out: &mut [U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> usize {
    let len_a = a.len();
    let len_b = b.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_b, 0, "Input 'b' must have at least one limb");
        assert!(len_a >= len_b, "Input 'a' must have at least as many limbs as 'b'");
        if len_a > 1 {
            assert!(!a[len_a - 1].is_zero(), "Input 'a' must not have leading zeros");
        }
        if len_b > 1 {
            assert!(!b[len_b - 1].is_zero(), "Input 'b' must not have leading zeros");
        }
    }

    // Start with a[0] + b[0]
    let mut params = SyscallAdd256Params {
        a: a[0].as_limbs(),
        b: b[0].as_limbs(),
        cin: 0,
        c: out[0].as_limbs_mut(),
    };
    let mut carry = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    for i in 1..len_b {
        // Compute a[i] + b[i] + carry
        let mut params = SyscallAdd256Params {
            a: a[i].as_limbs(),
            b: b[i].as_limbs(),
            cin: carry,
            c: out[i].as_limbs_mut(),
        };
        carry = syscall_add256(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    for i in len_b..len_a {
        if carry == 1 {
            // Compute a[i] + carry
            let mut params = SyscallAdd256Params {
                a: a[i].as_limbs(),
                b: U256::ZERO.as_limbs(),
                cin: 1,
                c: out[i].as_limbs_mut(),
            };
            carry = syscall_add256(
                &mut params,
                #[cfg(feature = "hints")]
                hints,
            );
        } else {
            // Directly copy a[i] to out[i]
            out[i] = a[i];
        }
    }

    if carry == 0 {
        len_a
    } else {
        out[len_a] = U256::ONE;
        len_a + 1
    }
}
