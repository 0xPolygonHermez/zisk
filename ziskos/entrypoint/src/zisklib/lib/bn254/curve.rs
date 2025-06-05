//! Operations on the BN254 curve E: y² = x³ + 3

use crate::{
    bn254_curve_add::{syscall_bn254_curve_add, SyscallBn254CurveAddParams},
    bn254_curve_dbl::syscall_bn254_curve_dbl,
    fcall_msb_pos_256,
    point256::SyscallPoint256,
    zisklib::lib::utils::eq,
};

use super::{
    constants::E_B,
    fp::{add_fp_bn254, inv_fp_bn254, mul_fp_bn254, square_fp_bn254},
};

/// Check if a point `p` is on the BN254 curve
pub fn is_on_curve_bn254(p: &[u64; 8]) -> bool {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // p in E iff y² == x³ + 3
    let lhs = square_fp_bn254(&y);
    let mut rhs = square_fp_bn254(&x);
    rhs = mul_fp_bn254(&rhs, &x);
    rhs = add_fp_bn254(&rhs, &E_B);
    eq(&lhs, &rhs)
}

/// Converts a point `p` on the BN254 curve from Jacobian coordinates to affine coordinates
pub fn to_affine_bn254(p: &[u64; 12]) -> Option<[u64; 8]> {
    let z: [u64; 4] = p[8..12].try_into().unwrap();

    // Check if p is the point at infinity
    if z == [0u64; 4] {
        // Point at infinity cannot be converted to affine
        return None;
    }

    // Check if p is already in affine coordinates
    if z == [1u64, 0, 0, 0] {
        return Some([p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]]);
    }

    let zinv = inv_fp_bn254(&z);
    let zinv_sq = square_fp_bn254(&zinv);

    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    let x_res = mul_fp_bn254(&x, &zinv_sq);
    let mut y_res = mul_fp_bn254(&y, &zinv_sq);
    y_res = mul_fp_bn254(&y_res, &zinv);

    Some([x_res[0], x_res[1], x_res[2], x_res[3], y_res[0], y_res[1], y_res[2], y_res[3]])
}

/// Adds two points `p1` and `p2` on the BN254 curve
pub fn add_bn254(p1: &[u64; 8], p2: &[u64; 8]) -> [u64; 8] {
    // Check if p1 is the point at infinity
    if *p1 == [0u64; 8] {
        return *p2;
    }

    // Check if p2 is the point at infinity
    if *p2 == [0u64; 8] {
        return *p1;
    }

    let x1: [u64; 4] = p1[0..4].try_into().unwrap();
    let y1: [u64; 4] = p1[4..8].try_into().unwrap();
    let x2: [u64; 4] = p2[0..4].try_into().unwrap();
    let y2: [u64; 4] = p2[4..8].try_into().unwrap();
    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling

            // Convert the input points to SyscallPoint256
            let mut p1 = SyscallPoint256 { x: x1, y: y1 };

            // Call the syscall to double the point
            syscall_bn254_curve_dbl(&mut p1);

            // Convert the result back to a single array
            let x3 = p1.x;
            let y3 = p1.y;
            return [x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]];
        } else {
            // Return 𝒪
            return [0u64; 8];
        }
    }

    // As p1 != p2,-p2, compute the addition

    // Convert the input points to SyscallPoint256
    let mut p1 = SyscallPoint256 { x: x1, y: y1 };
    let p2 = SyscallPoint256 { x: x2, y: y2 };

    // Call the syscall to add the two points
    let mut params = SyscallBn254CurveAddParams { p1: &mut p1, p2: &p2 };
    syscall_bn254_curve_add(&mut params);

    // Convert the result back to a single array
    let x3 = params.p1.x;
    let y3 = params.p1.y;
    [x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]]
}

/// Multiplies a point `p` on the BN254 curve by a scalar `k` on the BN254 scalar field
pub fn mul_bn254(p: &[u64; 8], k: &[u64; 4]) -> [u64; 8] {
    // Is p = 𝒪?
    if *p == [0u64; 8] {
        // Return 𝒪
        return [0u64; 8];
    }

    let x1: [u64; 4] = p[0..4].try_into().unwrap();
    let y1: [u64; 4] = p[4..8].try_into().unwrap();

    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0] => {
            // Return 𝒪
            return [0u64; 8];
        }
        [1, 0, 0, 0] => {
            // Return p
            return [x1[0], x1[1], x1[2], x1[3], y1[0], y1[1], y1[2], y1[3]];
        }
        [2, 0, 0, 0] => {
            // Return 2p
            let mut p = SyscallPoint256 { x: x1, y: y1 };
            syscall_bn254_curve_dbl(&mut p);
            return [p.x[0], p.x[1], p.x[2], p.x[3], p.y[0], p.y[1], p.y[2], p.y[3]];
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
    [x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]]
}
