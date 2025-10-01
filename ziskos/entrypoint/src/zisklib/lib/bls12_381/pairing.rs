//! Pairing over BLS12-381 curve

use crate::zisklib::lib::utils::gt;

use super::{
    constants::P_MINUS_ONE,
    curve::{is_on_curve_bls12_381, is_on_subgroup_bls12_381},
    final_exp::final_exp_bls12_381,
    miller_loop::miller_loop_bls12_381,
    twist::{is_on_curve_twist_bls12_381, is_on_subgroup_twist_bls12_381},
};

/// Optimal Ate Pairing e: G1 x G2 -> GT over the BLS12-381 curve
/// where G1 = E(Fp)[r] = E(Fp), G2 = E'(Fp2)[r] and GT = μ_r (the r-th roots of unity over Fp12*)
/// the involved curves are E/Fp: y² = x³ + 4 and E'/Fp2: y² = x³ + 4·(1+u)
///  pairingBLS12-381:
///          input: P ∈ G1 and Q ∈ G2
///          output: e(P,Q) ∈ GT
///
/// It also returns an error code:
/// -  0: No error.
/// -  1: x-coordinate of p is larger than the curve's base field.
/// -  2: y-coordinate of p is larger than the curve's base field.
/// -  3: x-coordinate real of q is larger than the curve's base field.
/// -  4: x-coordinate imaginary of q is larger than the curve's base field.
/// -  5: y-coordinate real of q is larger than the curve's base field.
/// -  6: y-coordinate imaginary of q is larger than the curve's base field.
/// -  7: p is not on the curve.
/// -  8: p is not on the subgroup.
/// -  9: q is not on the curve.
/// - 10: q is not on the subgroup.
pub fn pairing_bls12_381(p: &[u64; 12], q: &[u64; 24]) -> ([u64; 72], u8) {
    // TODO: One can assume that points are valid!!!
    // Check p and q are valid

    // Verify the coordinates of p
    let x1: [u64; 6] = p[0..6].try_into().unwrap();
    if gt(&x1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x1);

        return ([0; 72], 1);
    }

    let y1: [u64; 6] = p[6..12].try_into().unwrap();
    if gt(&y1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y1);

        return ([0; 72], 2);
    }

    // Verify the coordinates of q
    let x2_r: [u64; 6] = q[0..6].try_into().unwrap();
    if gt(&x2_r, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x2_r should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2_r);

        return ([0; 72], 3);
    }

    let x2_i: [u64; 6] = q[6..12].try_into().unwrap();
    if gt(&x2_i, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x2_i should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2_i);

        return ([0; 72], 4);
    }

    let y2_r: [u64; 6] = q[12..18].try_into().unwrap();
    if gt(&y2_r, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y2_r should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2_r);

        return ([0; 72], 5);
    }

    let y2_i: [u64; 6] = q[18..24].try_into().unwrap();
    if gt(&y2_i, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y2_i should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2_i);

        return ([0; 72], 6);
    }

    // Is p = 𝒪?
    if *p == [0; 12] {
        // Is q = 𝒪?
        if *q == [0; 24] {
            // Both are 𝒪, then e(𝒪,𝒪) = 1
            let mut one = [0; 72];
            one[0] = 1;
            return (one, 0);
        } else {
            // Is q on the curve?
            if is_on_curve_twist_bls12_381(q) {
                // Is q on the subgroup G2?
                if is_on_subgroup_twist_bls12_381(q) {
                    // q is valid, then e(𝒪,q) = 1
                    let mut one = [0; 72];
                    one[0] = 1;
                    return (one, 0);
                } else {
                    #[cfg(debug_assertions)]
                    println!("q is not in the subgroup");

                    return ([0; 72], 10);
                }
            } else {
                #[cfg(debug_assertions)]
                println!("q is not on the curve");

                return ([0; 72], 9);
            }
        }
    }

    // Is Q = 𝒪?
    if *q == [0; 24] {
        // Is p on the curve?
        if is_on_curve_bls12_381(p) {
            // Is p on the subgroup G1?
            if is_on_subgroup_bls12_381(p) {
                // p is valid, then e(p,𝒪) = 1
                let mut one = [0; 72];
                one[0] = 1;
                return (one, 0);
            } else {
                #[cfg(debug_assertions)]
                println!("p is not in the subgroup");

                return ([0; 72], 8);
            }
        } else {
            #[cfg(debug_assertions)]
            println!("p is not on the curve");

            return ([0; 72], 7);
        }
    }

    // If neither p nor q are 𝒪, we can check if they belong to the subgroup
    if !is_on_curve_bls12_381(p) {
        #[cfg(debug_assertions)]
        println!("p is not on the curve");

        return ([0; 72], 7);
    }

    if !is_on_subgroup_bls12_381(p) {
        #[cfg(debug_assertions)]
        println!("p is not in the subgroup");

        return ([0; 72], 8);
    }

    if !is_on_curve_twist_bls12_381(q) {
        #[cfg(debug_assertions)]
        println!("q is not on the curve");

        return ([0; 72], 9);
    }

    if !is_on_subgroup_twist_bls12_381(q) {
        #[cfg(debug_assertions)]
        println!("q is not in the subgroup");

        return ([0; 72], 10);
    }

    // Miller loop
    let miller_loop = miller_loop_bls12_381(p, q);

    // Final exponentiation
    let final_exp = final_exp_bls12_381(&miller_loop);

    (final_exp, 0)
}

/// Checks whether the pairing of two G1 and two G2 points are equivalent, i.e.,
///     e(P₁, Q₁) == e(P₂, Q₂)
pub fn pairing_verify_bls12_381(
    p1: &[u64; 12],
    q1: &[u64; 24],
    p2: &[u64; 12],
    q2: &[u64; 24],
) -> bool {
    let (res, err) = pairing_bls12_381(p1, q1);
    if err != 0 {
        return false;
    }

    let (res2, err2) = pairing_bls12_381(p2, q2);
    if err2 != 0 {
        return false;
    }

    res == res2
}
