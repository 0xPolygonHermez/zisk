use crate::{
    bn254_curve_add::{syscall_bn254_curve_add, SyscallBn254CurveAddParams},
    bn254_curve_dbl::syscall_bn254_curve_dbl,
    point256::SyscallPoint256,
};

use super::{
    bn254::{constants::P_MINUS_ONE, curve::is_on_curve_bn254},
    utils::{eq, gt},
};

/// Given two points `p1` and `p2` on the BN254 curve, this function computes the sum of the two points.
///
/// Since the curve is E/Fp: y² = x³ + 3, there is no issue in representing the point at infinity as (0, 0).
///
/// It also returns an error code:
/// - 0: No error.
/// - 1: x-coordinate of p1 is larger than the curve's base field.
/// - 2: y-coordinate of p1 is larger than the curve's base field.
/// - 3: x-coordinate of p2 is larger than the curve's base field.
/// - 4: y-coordinate of p2 is larger than the curve's base field.
/// - 5: p1 is not on the curve.
/// - 6: p2 is not on the curve.
pub fn ecadd(p1: &[u64; 8], p2: &[u64; 8]) -> ([u64; 8], u8) {
    // Verify the coordinates of p1
    let x1: [u64; 4] = p1[0..4].try_into().unwrap();
    if gt(&x1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x1);

        return ([0u64; 8], 1);
    }

    let y1: [u64; 4] = p1[4..8].try_into().unwrap();
    if gt(&y1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y1);

        return ([0u64; 8], 2);
    }

    // Verify the coordinates of p2
    let x2: [u64; 4] = p2[0..4].try_into().unwrap();
    if gt(&x2, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x2 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2);

        return ([0u64; 8], 3);
    }

    let y2: [u64; 4] = p2[4..8].try_into().unwrap();
    if gt(&y2, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y2 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2);

        return ([0u64; 8], 4);
    }

    // Is p1 = 𝒪?
    if *p1 == [0, 0, 0, 0, 0, 0, 0, 0] {
        // Is p2 = 𝒪?
        if *p2 == [0, 0, 0, 0, 0, 0, 0, 0] {
            // Return 𝒪
            return ([0u64; 8], 0);
        } else {
            // Is p2 on the curve?
            if is_on_curve_bn254(p2) {
                // Return p2
                return (*p2, 0);
            } else {
                #[cfg(debug_assertions)]
                println!("p2 is not on the curve");

                return ([0u64; 8], 6);
            }
        }
    }

    // Is p2 = 𝒪?
    if *p2 == [0, 0, 0, 0, 0, 0, 0, 0] {
        // Is p1 on the curve?
        // p1 in E iff y² == x³ + 3 (mod p)
        if is_on_curve_bn254(p1) {
            // Return p1
            return (*p1, 0);
        } else {
            #[cfg(debug_assertions)]
            println!("p1 is not on the curve");

            return ([0u64; 8], 5);
        }
    }

    // Is p1 on the curve?
    if !is_on_curve_bn254(p1) {
        #[cfg(debug_assertions)]
        println!("p1 is not on the curve");

        return ([0u64; 8], 5);
    }

    // Is p2 on the curve?
    if !is_on_curve_bn254(p2) {
        #[cfg(debug_assertions)]
        println!("p2 is not on the curve");

        return ([0u64; 8], 6);
    }

    // From here, three posibilities:
    //  - p1 = p2
    //  - p1 = -p2
    //  - p1 != p2

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
            return ([x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]], 0);
        } else {
            // Return 𝒪
            return ([0u64; 8], 0);
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
    ([x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]], 0)
}
