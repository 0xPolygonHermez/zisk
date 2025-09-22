//! Pairing over BLS12-381 curve

use crate::zisklib::lib::utils::gt;

use super::{
    constants::P_MINUS_ONE,
    curve::{is_on_curve_bls12_381, is_on_subgroup_bls12_381},
    final_exp::final_exp_bls12_381,
    miller_loop::{miller_loop_batch_bls12_381, miller_loop_bls12_381},
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

/// Computes the optimal Ate pairing for a batch of G1 and G2 points over the bls12_381 curve
/// and multiplies the results together, i.e.:
///     e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) ∈ GT
pub fn pairing_batch_bls12_381(g1_points: &[[u64; 12]], g2_points: &[[u64; 24]]) -> [u64; 72] {
    // Since each e(Pi, Qi) := FinalExp(MillerLoop(Pi, Qi))
    // We have:
    //  e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) = FinalExp(MillerLoop(P₁, Q₁) · MillerLoop(P₂, Q₂) · ... · MillerLoop(Pₙ, Qₙ))
    // We can compute the Miller loop for each pair, multiplying the results together
    // and then just do the final exponentiation once at the end.

    let num_points = g1_points.len();
    assert_eq!(num_points, g2_points.len(), "Number of G1 and G2 points must be equal");

    // Miller loop and multiplication
    let mut g1_points_ml = Vec::with_capacity(num_points);
    let mut g2_points_ml = Vec::with_capacity(num_points);
    for (p, q) in g1_points.iter().zip(g2_points.iter()) {
        // Is p = 𝒪 or q = 𝒪?
        if *p == [0u64; 12] || *q == [0u64; 24] {
            // MillerLoop(P, 𝒪) = MillerLoop(𝒪, Q) = 1; we can skip
            continue;
        }

        g1_points_ml.push(*p);
        g2_points_ml.push(*q);
    }

    if g1_points_ml.is_empty() {
        // If all pairing computations were skipped, return 1
        let mut one = [0; 72];
        one[0] = 1;
        return one;
    }

    // Compute the Miller loop for the batch
    let miller_loop = miller_loop_batch_bls12_381(&g1_points_ml, &g2_points_ml);

    // Final exponentiation
    final_exp_bls12_381(&miller_loop)
}
