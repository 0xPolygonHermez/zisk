use crate::{
    adc256::{syscall_adc256, SyscallAdc256Params},
    add256::{syscall_add256, SyscallAdd256Params},
};

use super::U256;

/// Addition of one large number (represented as an array of U256) and a short U256 number
///
/// It assumes that len(a) > 0
pub fn add_short(a: &mut [U256], b: &U256) -> bool {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
    }

    let mut carry = 0u64;

    // Start with a[0] + b
    let mut params = SyscallAdd256Params { a: &a[0].clone(), b, dl: &mut a[0], dh: &mut carry };
    syscall_add256(&mut params);

    for i in 1..len_a {
        if carry == 1 {
            // Compute a[i] + carry
            let mut params =
                SyscallAdc256Params { a: &a[i].clone(), b: &U256::ZERO, dl: &mut a[i], dh: &mut carry };
            syscall_adc256(&mut params);
        } else {
            break;
        }
    }

    carry == 1
}
