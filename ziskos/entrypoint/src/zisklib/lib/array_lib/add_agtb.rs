use crate::{
    adc256::{syscall_adc256, SyscallAdc256Params},
    add256::{syscall_add256, SyscallAdd256Params},
};

use super::U256;

/// Addition of two large numbers (represented as arrays of U256)
///
/// It assumes that len(a),len(b) > 0 and len(a) >= len(b)
pub fn add_agtb(a: &mut [U256], b: &[U256]) -> bool {
    let len_a = a.len();
    let len_b = b.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_b, 0, "Input 'b' must have at least one limb");
        assert!(len_a >= len_b, "Input 'a' must have at least len(b) limbs");
    }

    let mut carry = 0u64;

    // Start with a[0] + b[0]
    let mut params = SyscallAdd256Params { a: &a[0].clone(), b: &b[0], dl: &mut a[0], dh: &mut carry };
    syscall_add256(&mut params);

    for i in 1..len_b {
        if carry == 1 {
            // Compute a[i] + b[i] + carry
            let mut params =
                SyscallAdc256Params { a: &a[i].clone(), b: &b[i], dl: &mut a[i], dh: &mut carry };
            syscall_adc256(&mut params);
        } else {
            // Compute a[i] + b[i]
            let mut params =
                SyscallAdd256Params { a: &a[i].clone(), b: &b[i], dl: &mut a[i], dh: &mut carry };
            syscall_add256(&mut params);
        }
    }

    for i in len_b..len_a {
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
