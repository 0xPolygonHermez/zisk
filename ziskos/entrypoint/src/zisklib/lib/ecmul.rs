use crate::{
    bn254_curve_add::{syscall_bn254_curve_add, SyscallBn254CurveAddParams},
    bn254_curve_dbl::syscall_bn254_curve_dbl,
    fcall_msb_pos_256,
    point256::SyscallPoint256,
};

use super::{
    bn254::{
        constants::{P_MINUS_ONE, R_MINUS_ONE},
        curve::is_on_curve_bn254,
    },
    utils::gt,
};

/// Given an scalar k and a point p, this function computes the point q = k·p.
///
/// Since the curve is E/Fp: y² = x³ + 3, there is no issue in representing the point at infinity as (0, 0).
///
/// It also returns an error code:
/// - 0: No error.
/// - 1: x-coordinate of p is larger than the curve's base field.
/// - 2: y-coordinate of p is larger than the curve's base field.
/// - 3: k is larger than the curve's scalar field.
/// - 4: p is not on the curve.
pub fn ecmul(k: &[u64; 4], p: &[u64; 8]) -> ([u64; 8], u8) {
    // Verify the coordinates of p
    let x1: [u64; 4] = p[0..4].try_into().unwrap();
    if gt(&x1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x1);

        return ([0u64; 8], 1);
    }

    let y1: [u64; 4] = p[4..8].try_into().unwrap();
    if gt(&y1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y1);

        return ([0u64; 8], 2);
    }

    // Verify k
    if gt(k, &R_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("k should be less than R_MINUS_ONE: {:?}, but got {:?}", R_MINUS_ONE, k);

        return ([0u64; 8], 3);
    }

    // Is p = 𝒪?
    if *p == [0, 0, 0, 0, 0, 0, 0, 0] {
        // Return 𝒪
        return ([0u64; 8], 0);
    }

    // Check if p is on curve: y² == x³ + 3 mod p
    if !is_on_curve_bn254(p) {
        #[cfg(debug_assertions)]
        println!("p is not on the curve");

        return ([0u64; 8], 4);
    }

    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0] => {
            // Return 𝒪
            return ([0u64; 8], 0);
        }
        [1, 0, 0, 0] => {
            // Return p
            return ([x1[0], x1[1], x1[2], x1[3], y1[0], y1[1], y1[2], y1[3]], 0);
        }
        [2, 0, 0, 0] => {
            // Return 2p
            let mut p = SyscallPoint256 { x: x1, y: y1 };
            syscall_bn254_curve_dbl(&mut p);
            return ([p.x[0], p.x[1], p.x[2], p.x[3], p.y[0], p.y[1], p.y[2], p.y[3]], 0);
        }
        _ => {}
    }

    // We can assume k > 2 from now on
    // Hint the length the binary representations of k
    // We will verify the output by recomposing k
    // Moreover, we should check that the first received bit is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(k, &[0, 0, 0, 0]);

    // Perform the loop, based on the binary representation of k

    // We do the first iteration separately
    let _max_limb = max_limb as usize;
    let k_bit = (k[_max_limb] >> max_bit) & 1;
    assert_eq!(k_bit, 1); // the first received bit should be 1

    // Start at P
    let mut q = SyscallPoint256 { x: x1, y: y1 };
    let mut k_rec = [0u64; 4];
    k_rec[_max_limb] |= 1 << max_bit;

    // Perform the rest of the loop
    let p = SyscallPoint256 { x: x1, y: y1 };
    let _max_bit = max_bit as usize;
    for i in (0..=_max_limb).rev() {
        let bit_len = if i == _max_limb { _max_bit - 1 } else { 63 };
        for j in (0..=bit_len).rev() {
            // Always double
            syscall_bn254_curve_dbl(&mut q);

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                let mut params = SyscallBn254CurveAddParams { p1: &mut q, p2: &p };
                syscall_bn254_curve_add(&mut params);

                // Reconstruct k
                k_rec[i] |= 1 << j;
            }
        }
    }

    // Check that the reconstructed k is equal to the input k
    assert_eq!(k_rec, *k);

    // Convert the result back to a single array
    let x3 = q.x;
    let y3 = q.y;
    ([x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]], 0)
}
