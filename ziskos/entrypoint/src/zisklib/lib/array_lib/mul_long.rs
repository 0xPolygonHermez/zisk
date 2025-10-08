use crate::{
    adc256::{syscall_adc256, SyscallAdc256Params},
    add256::{syscall_add256, SyscallAdd256Params},
    arith256::{syscall_arith256, SyscallArith256Params},
};

use super::U256;

/// Multiplication of two large numbers (represented as arrays of U256)
///
/// It assumes that len(a),len(b) > 0 and len(out) >= len(a) + len(b)
pub fn mul_long(a: &[U256], b: &[U256], out: &mut [U256]) {
    let len_a = a.len();
    let len_b = b.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert_ne!(len_b, 0, "Input 'b' must have at least one limb");
        assert!(out.len() >= len_a + len_b, "Output 'out' must have at least len(a) + len(b) limbs");
    }

    // Start with a[0]·b[0]
    let mut params = SyscallArith256Params {
        a: &a[0],
        b: &b[0],
        c: &[0, 0, 0, 0],
        dl: &mut out[0],
        dh: &mut [0, 0, 0, 0],
    };
    syscall_arith256(&mut params);

    // Propagate the carry
    out[1] = U256::from_u64s(params.dh);

    // Finish the first row
    for j in 1..len_b {
        // Compute a[0]·b[j] + out[j]
        let mut params = SyscallArith256Params {
            a: &a[0],
            b: &b[j],
            c: &out[j].clone(),
            dl: &mut out[j],
            dh: &mut [0, 0, 0, 0],
        };
        syscall_arith256(&mut params);

        // Propagate the carry
        out[j + 1] = U256::from_u64s(params.dh);
    }

    // Finish the remaining rows
    for i in 1..len_a {
        let mut carry_flag = 0u64;
        for j in 0..(len_b - 1) {
            // Compute a[i]·b[j] + out[i + j]
            let mut params_arith = SyscallArith256Params {
                a: &a[i],
                b: &b[j],
                c: &out[i + j].clone(),
                dl: &mut [0, 0, 0, 0],
                dh: &mut [0, 0, 0, 0],
            };
            syscall_arith256(&mut params_arith);

            // Set the result
            out[i + j] = U256::from_u64s(params_arith.dl);

            if carry_flag == 1 {
                let mut params_adc = SyscallAdc256Params {
                    a: &params_arith.dh.clone(),
                    b: &[0, 0, 0, 0],
                    dl: params_arith.dh,
                    dh: &mut 0,
                };
                syscall_adc256(&mut params_adc);

                debug_assert!(*params_adc.dh == 0, "Unexpected carry in intermediate addition");
            }

            // Update out[i+j+1] with carry
            let mut params_add = SyscallAdd256Params {
                a: &out[i + j + 1].clone(),
                b: params_arith.dh,
                dl: &mut out[i + j + 1],
                dh: &mut carry_flag,
            };
            syscall_add256(&mut params_add);
        }

        // Last chunk isolated

        // Compute a[i]·b[len_b - 1] + out[i + len_b - 1]
        let mut params_arith = SyscallArith256Params {
            a: &a[i],
            b: &b[len_b - 1],
            c: &out[i + len_b - 1].clone(),
            dl: &mut out[i + len_b - 1],
            dh: &mut [0, 0, 0, 0],
        };
        syscall_arith256(&mut params_arith);

        if carry_flag == 1 {
            let mut params_add = SyscallAdc256Params {
                a: &params_arith.dh.clone(),
                b: &[0, 0, 0, 0],
                dl: params_arith.dh,
                dh: &mut 0,
            };
            syscall_adc256(&mut params_add);

            debug_assert!(*params_add.dh == 0, "Unexpected carry in intermediate addition");
        }

        // Set out[i+j+1] = carry
        out[i + len_b] = U256::from_u64s(params_arith.dh);
    }
}