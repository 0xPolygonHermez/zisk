use crate::{
    adc256::{syscall_adc256, SyscallAdc256Params},
    add256::{syscall_add256, SyscallAdd256Params},
};

use super::U256;

/// Doubling of a large number (represented as an array of U256)
///
/// It assumes that a > 0
pub fn double(a: &[U256]) -> Vec<U256> {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(a.last().unwrap(), &U256::ZERO, "Input 'a' must not have leading zeros");
    }

    let mut out = vec![U256::ONE; len_a + 1];
    let mut carry = 0u64;

    // Start with a[0] + a[0]
    let mut params = SyscallAdd256Params { a: &a[0], b: &a[0], dl: &mut out[0], dh: &mut carry };
    syscall_add256(&mut params);

    for i in 1..len_a {
        if carry == 1 {
            // Compute a[i] + a[i] + carry
            let mut params =
                SyscallAdc256Params { a: &a[i], b: &a[i], dl: &mut out[i], dh: &mut carry };
            syscall_adc256(&mut params);
        } else {
            // Compute a[i] + a[i]
            let mut params =
                SyscallAdd256Params { a: &a[i], b: &a[i], dl: &mut out[i], dh: &mut carry };
            syscall_add256(&mut params);
        }
    }

    if carry == 0 {
        out.pop();
    }

    out
}
