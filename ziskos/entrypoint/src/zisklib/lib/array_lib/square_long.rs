use crate::syscalls::{
    syscall_add256, syscall_arith256, SyscallAdd256Params, SyscallArith256Params,
};

use super::{rem_long, LongScratch, U256};

/// Squares a large number: out = a²
///
/// # Assumptions
/// - `len(a) > 0`
/// - `a` has no leading zeros
/// - `out` has at least `2 * len(a)` limbs
///
/// # Returns
/// The number of limbs in the result
///
/// # Note
/// Not optimal for `len(a) == 1`, use `square_short` instead
pub fn square_long(
    a: &[U256],
    out: &mut [U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> usize {
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

    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        if len_a > 1 {
            assert!(!a[len_a - 1].is_zero(), "Input 'a' must not have leading zeros");
        }
    }

    // Step 1: Compute all diagonal terms a[i] * a[i]
    for i in 0..len_a {
        // Compute the diagonal:
        //      a[i]·a[i] = dh·B + dl
        // and set out[2 * i] = dl and out[2 * i + 1] = dh
        let mut ai_ai = SyscallArith256Params {
            a: a[i].as_limbs(),
            b: a[i].as_limbs(),
            c: U256::ZERO.as_limbs(),
            dl: out[2 * i].as_limbs_mut(),
            dh: &mut [0, 0, 0, 0],
        };
        syscall_arith256(
            &mut ai_ai,
            #[cfg(feature = "hints")]
            hints,
        );

        out[2 * i + 1] = U256::from_u64s(ai_ai.dh);
    }

    // Step 2: Compute all cross terms 2·a[i]·a[j] for i < j
    for i in 0..len_a {
        for j in (i + 1)..len_a {
            // First compute a[i]·a[j] = h₁·B + l₁
            let mut ai_aj = SyscallArith256Params {
                a: a[i].as_limbs(),
                b: a[j].as_limbs(),
                c: U256::ZERO.as_limbs(),
                dl: &mut [0, 0, 0, 0],
                dh: &mut [0, 0, 0, 0],
            };
            syscall_arith256(
                &mut ai_aj,
                #[cfg(feature = "hints")]
                hints,
            );

            // Double the result 2·a[i]·a[j]

            // Start by doubling the lower chunk: 2·l₁ = [1/0]·B + l₂
            let mut dbl_low =
                SyscallAdd256Params { a: ai_aj.dl, b: ai_aj.dl, cin: 0, c: &mut [0, 0, 0, 0] };
            let dbl_low_carry = syscall_add256(
                &mut dbl_low,
                #[cfg(feature = "hints")]
                hints,
            );

            // Next, double the higher chunk: 2·h₁·B = [1/0]·B² + h₂·B
            let mut dbl_high =
                SyscallAdd256Params { a: ai_aj.dh, b: ai_aj.dh, cin: 0, c: &mut [0, 0, 0, 0] };
            let dbl_high_carry = syscall_add256(
                &mut dbl_high,
                #[cfg(feature = "hints")]
                hints,
            );

            // If there's a carry from doubling the low part, add it to the high part
            if dbl_low_carry != 0 {
                let a_in = *dbl_high.c;
                let mut add = SyscallAdd256Params {
                    a: &a_in,
                    b: U256::ZERO.as_limbs(),
                    cin: 1,
                    c: dbl_high.c,
                };
                let _carry = syscall_add256(
                    &mut add,
                    #[cfg(feature = "hints")]
                    hints,
                );

                debug_assert!(_carry == 0, "Unexpected carry in intermediate addition");
            }

            // The result is expressed as: dbl_high.dh·B² + dbl_high.dl·B + dbl_low.dl

            // Now update out[i+j], out[i+j+1] and out[i+j+2] with this result

            // Update out[i+j]
            let mut add_low = SyscallAdd256Params {
                a: out[i + j].as_limbs(),
                b: dbl_low.c,
                cin: 0,
                c: &mut [0, 0, 0, 0],
            };
            let add_low_carry = syscall_add256(
                &mut add_low,
                #[cfg(feature = "hints")]
                hints,
            );
            out[i + j] = U256::from_u64s(add_low.c);

            if add_low_carry != 0 {
                let a_in = out[i + j + 1];
                let mut add = SyscallAdd256Params {
                    a: a_in.as_limbs(),
                    b: U256::ZERO.as_limbs(),
                    cin: 1,
                    c: out[i + j + 1].as_limbs_mut(),
                };
                let add_carry = syscall_add256(
                    &mut add,
                    #[cfg(feature = "hints")]
                    hints,
                );

                if add_carry != 0 {
                    let a_in = out[i + j + 2];
                    let mut add2 = SyscallAdd256Params {
                        a: a_in.as_limbs(),
                        b: U256::ZERO.as_limbs(),
                        cin: 1,
                        c: out[i + j + 2].as_limbs_mut(),
                    };
                    let _carry = syscall_add256(
                        &mut add2,
                        #[cfg(feature = "hints")]
                        hints,
                    );

                    debug_assert!(_carry == 0, "Unexpected carry in intermediate addition");
                }
            }

            // Update out[i+j+1]
            let mut add_mid = SyscallAdd256Params {
                a: out[i + j + 1].as_limbs(),
                b: dbl_high.c,
                cin: 0,
                c: &mut [0, 0, 0, 0],
            };
            let add_mid_carry = syscall_add256(
                &mut add_mid,
                #[cfg(feature = "hints")]
                hints,
            );
            out[i + j + 1] = U256::from_u64s(add_mid.c);

            if add_mid_carry != 0 {
                let a_in = out[i + j + 2];
                let mut add = SyscallAdd256Params {
                    a: a_in.as_limbs(),
                    b: U256::ZERO.as_limbs(),
                    cin: 1,
                    c: out[i + j + 2].as_limbs_mut(),
                };
                let _carry = syscall_add256(
                    &mut add,
                    #[cfg(feature = "hints")]
                    hints,
                );

                debug_assert!(_carry == 0, "Unexpected carry in intermediate addition");
            }

            // Update out[i+j+2]
            if dbl_high_carry != 0 {
                let a_in = out[i + j + 2];
                let mut add = SyscallAdd256Params {
                    a: a_in.as_limbs(),
                    b: U256::ZERO.as_limbs(),
                    cin: 1,
                    c: out[i + j + 2].as_limbs_mut(),
                };
                let _carry = syscall_add256(
                    &mut add,
                    #[cfg(feature = "hints")]
                    hints,
                );

                debug_assert!(_carry == 0, "Unexpected carry in intermediate addition");
            }
        }
    }

    if out[2 * len_a - 1].is_zero() {
        2 * len_a - 1
    } else {
        2 * len_a
    }
}

/// Squares a large number and reduces modulo a large modulus
///
/// # Assumptions
/// - `len(modulus) > 0`
/// - `modulus > 0`
/// - `modulus` has no leading zeros
///
/// # Returns
/// The remainder: a² mod modulus
pub fn square_and_reduce_long(
    a: &[U256],
    modulus: &[U256],
    scratch: &mut LongScratch,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<U256> {
    #[cfg(debug_assertions)]
    {
        let len_m = modulus.len();
        assert_ne!(len_m, 0, "Input 'modulus' must have at least one limb");
        assert!(!modulus[len_m - 1].is_zero(), "Input 'modulus' must not have leading zeros");
    }

    let sq_len = square_long(
        a,
        &mut scratch.mul,
        #[cfg(feature = "hints")]
        hints,
    );

    rem_long(
        &scratch.mul[..sq_len],
        modulus,
        &mut scratch.rem,
        #[cfg(feature = "hints")]
        hints,
    )
}
