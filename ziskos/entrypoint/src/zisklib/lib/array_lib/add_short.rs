use crate::{
    adc256::{syscall_adc256, SyscallAdc256Params},
    add256::{syscall_add256, SyscallAdd256Params},
};

use super::U256;

/// Addition of one large number (represented as an array of U256) and a short U256 number
///
/// It assumes that a,b > 0
pub fn add_short(a: &[U256], b: &U256) -> Vec<U256> {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(a.last().unwrap(), &U256::ZERO, "Input 'a' must not have leading zeros");
        assert_ne!(b, &U256::ZERO, "Input 'b' must be greater than zero");
    }

    let mut out = vec![U256::ONE; len_a + 1];
    let mut carry = 0u64;

    // Start with a[0] + b
    let mut params = SyscallAdd256Params { a: &a[0], b, dl: &mut out[0], dh: &mut carry };
    syscall_add256(&mut params);

    for i in 1..len_a {
        if carry == 1 {
            // Compute a[i] + carry
            let mut params =
                SyscallAdc256Params { a: &a[i], b: &U256::ZERO, dl: &mut out[i], dh: &mut carry };
            syscall_adc256(&mut params);
        } else {
            // No carry, just copy the rest of a
            out[i] = a[i];
        }
    }

    if carry == 0 {
        out.pop();
    }

    out
}
