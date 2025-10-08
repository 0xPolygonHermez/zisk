use crate::{
    adc256::{syscall_adc256, SyscallAdc256Params},
    add256::{syscall_add256, SyscallAdd256Params},
    arith256::{syscall_arith256, SyscallArith256Params},
};

use super::{U256, rem_short, rem_long};

/// Squaring of a large number (represented as an array of U256)
//                                        a3    a2    a1      a0
//                                      * a3    a2    a1      a0
//         ------------------------------------------------------- 0
//                               Y       2*a0*a2   2*a0*a1  a0*a0
//         ------------------------------------------------------- 1
//               2*a1*a3+Z    2*a1*a2     a1*a1        X      0
//         ------------------------------------------------------- 2
//  Z   Y     2*a2*a3   a2*a2        X      X          0      0
//         ------------------------------------------------------- 3
//    a3*a3     X        X          X           0      0      0
//         ------------------------------------------------------- 4
//                          RESULT
pub fn square(a: &[U256], out: &mut [U256]) {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(out.len() >= 2 * len_a, "Output 'out' must have at least 2 * len(a) limbs");
    }

    // Step 1: Compute all diagonal terms a[i] * a[i]
    for i in 0..len_a {
        // Compute the diagonal:
        //      a[i]·a[i] = dh·B + dl
        // and set out[2 * i] = dl and out[2 * i + 1] = dh
        let mut ai_ai = SyscallArith256Params {
            a: &a[i],
            b: &a[i],
            c: &[0, 0, 0, 0],
            dl: &mut out[2 * i],
            dh: &mut [0, 0, 0, 0],
        };
        syscall_arith256(&mut ai_ai);

        out[2 * i + 1] = U256::from_u64s(ai_ai.dh);
    }

    // Step 2: Compute all cross terms 2·a[i]·a[j] for i < j
    for i in 0..len_a {
        for j in (i + 1)..len_a {
            // First compute a[i]·a[j] = h₁·B + l₁
            let mut ai_aj = SyscallArith256Params {
                a: &a[i],
                b: &a[j],
                c: &[0, 0, 0, 0],
                dl: &mut [0, 0, 0, 0],
                dh: &mut [0, 0, 0, 0],
            };
            syscall_arith256(&mut ai_aj);

            // Double the result 2·a[i]·a[j]

            // Start by doubling the lower chunk: 2·l₁ = [1/0]·B + l₂
            let mut dbl_low = SyscallAdd256Params {
                a: &ai_aj.dl.clone(),
                b: &ai_aj.dl.clone(),
                dl: &mut [0, 0, 0, 0],
                dh: &mut 0,
            };
            syscall_add256(&mut dbl_low);

            // Next, double the higher chunk: 2·h₁·B = [1/0]·B² + h₂·B
            let mut dbl_high = SyscallAdd256Params {
                a: &ai_aj.dh.clone(),
                b: &ai_aj.dh.clone(),
                dl: &mut [0, 0, 0, 0],
                dh: &mut 0,
            };
            syscall_add256(&mut dbl_high);

            // If there's a carry from doubling the low part, add it to the high part
            if *dbl_low.dh != 0 {
                let mut adc = SyscallAdc256Params {
                    a: &dbl_high.dl.clone(),
                    b: &[0, 0, 0, 0],
                    dl: dbl_high.dl,
                    dh: &mut 0,
                };
                syscall_adc256(&mut adc);

                debug_assert!(*adc.dh == 0, "Unexpected carry in intermediate addition");
            }

            // The result is expressed as: dbl_high.dh·B² + dbl_high.dl·B + dbl_low.dl

            // Now update out[i+j], out[i+j+1] and out[i+j+2] with this result

            // Update out[i+j]
            let mut add_low = SyscallAdd256Params {
                a: &out[i + j].clone(),
                b: dbl_low.dl,
                dl: &mut [0, 0, 0, 0],
                dh: &mut 0,
            };
            syscall_add256(&mut add_low);
            out[i + j] = U256::from_u64s(add_low.dl);

            if *add_low.dh != 0 {
                let mut adc = SyscallAdc256Params {
                    a: &out[i + j + 1].clone(),
                    b: &[0, 0, 0, 0],
                    dl: &mut out[i + j + 1],
                    dh: &mut 0,
                };
                syscall_adc256(&mut adc);

                if adc.dh != &0 {
                    let mut adc2 = SyscallAdc256Params {
                        a: &out[i + j + 2].clone(),
                        b: &[0, 0, 0, 0],
                        dl: &mut out[i + j + 2],
                        dh: &mut 0,
                    };
                    syscall_adc256(&mut adc2);

                    debug_assert!(*adc2.dh == 0, "Unexpected carry in intermediate addition");
                }
            }

            // Update out[i+j+1]
            let mut add_mid = SyscallAdd256Params {
                a: &out[i + j + 1].clone(),
                b: dbl_high.dl,
                dl: &mut [0, 0, 0, 0],
                dh: &mut 0,
            };
            syscall_add256(&mut add_mid);
            out[i + j + 1] = U256::from_u64s(add_mid.dl);

            if *add_mid.dh != 0 {
                let mut adc = SyscallAdc256Params {
                    a: &out[i + j + 2].clone(),
                    b: &[0, 0, 0, 0],
                    dl: &mut out[i + j + 2],
                    dh: &mut 0,
                };
                syscall_adc256(&mut adc);

                debug_assert!(*adc.dh == 0, "Unexpected carry in intermediate addition");
            }

            // Update out[i+j+2]
            if *dbl_high.dh != 0 {
                let mut adc = SyscallAdc256Params {
                    a: &out[i + j + 2].clone(),
                    b: &[0, 0, 0, 0],
                    dl: &mut out[i + j + 2],
                    dh: &mut 0,
                };
                syscall_adc256(&mut adc);

                debug_assert!(*adc.dh == 0, "Unexpected carry in intermediate addition");
            }
        }
    }
}

pub fn square_and_reduce(a: &[U256], modulus: &[U256], out: &mut [U256]) {
    let len_m = modulus.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_m, 0, "Input 'modulus' must have at least one limb");
        assert_ne!(modulus.last().unwrap(), &U256::ZERO, "Input 'modulus' must not have leading zeros");
        assert!(out.len() >= len_m, "Output 'out' must have at least len(modulus) limbs");
    }

    let len_sq = a.len() * 2;
    let mut sq = vec![U256::ZERO; len_sq];
    square(a, &mut sq);
    
    // If a·b < modulus, then the result is just a·b
    if U256::lt_slices(&sq, modulus) {
        out[..len_sq].copy_from_slice(&sq);
        return;
    }

    if len_m == 1 {
        // If modulus has only one limb, we can use short division
        out[0] = rem_short(&sq, &modulus[0]);
    } else {
        // Otherwise, use long division
        let r = rem_long(&sq, modulus);
        out[..r.len()].copy_from_slice(&r);
    }
}
