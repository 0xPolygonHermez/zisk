//! Pairing over BN254

use crate::zisklib::lib::utils::gt;

use super::{
    constants::P_MINUS_ONE,
    curve::is_on_curve_bn254,
    final_exp::final_exp_bn254,
    miller_loop::{miller_loop_batch_bn254, miller_loop_bn254},
    twist::{is_on_curve_twist_bn254, is_on_subgroup_twist_bn254},
};

/// Optimal Ate Pairing e: G1 x G2 -> GT over the BN254 curve
/// where G1 = E(Fp)[r] = E(Fp), G2 = E'(Fp2)[r] and GT = μ_r (the r-th roots of unity over Fp12*
/// the involved curves are E/Fp: y² = x³ + 3 and E'/Fp2: y² = x³ + 3/(9+u)
///  pairingBN254:
///          input: P ∈ G1 and Q ∈ G2
///          output: e(P,Q) ∈ GT
///
/// It also returns an error code:
/// - 0: No error.
/// - 1: x-coordinate of p is larger than the curve's base field.
/// - 2: y-coordinate of p is larger than the curve's base field.
/// - 3: x-coordinate real of q is larger than the curve's base field.
/// - 4: x-coordinate imaginary of q is larger than the curve's base field.
/// - 5: y-coordinate real of q is larger than the curve's base field.
/// - 6: y-coordinate imaginary of q is larger than the curve's base field.
/// - 7: p is not on the curve (nor on the subgroup).
/// - 8: q is not on the curve.
/// - 9: q is not on the subgroup.
pub fn pairing_bn254(p: &[u64; 8], q: &[u64; 16]) -> ([u64; 48], u8) {
    // Check p and q are valid

    // Verify the coordinates of p
    let x1: [u64; 4] = p[0..4].try_into().unwrap();
    if gt(&x1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x1);

        return ([0; 48], 1);
    }

    let y1: [u64; 4] = p[4..8].try_into().unwrap();
    if gt(&y1, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y1);

        return ([0; 48], 2);
    }

    // Verify the coordinates of q
    let x2_r: [u64; 4] = q[0..4].try_into().unwrap();
    if gt(&x2_r, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x2_r should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2_r);

        return ([0; 48], 3);
    }

    let x2_i: [u64; 4] = q[4..8].try_into().unwrap();
    if gt(&x2_i, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("x2_i should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2_i);

        return ([0; 48], 4);
    }

    let y2_r: [u64; 4] = q[8..12].try_into().unwrap();
    if gt(&y2_r, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y2_r should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2_r);

        return ([0; 48], 5);
    }

    let y2_i: [u64; 4] = q[12..16].try_into().unwrap();
    if gt(&y2_i, &P_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("y2_i should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2_i);

        return ([0; 48], 6);
    }

    // Is p = 𝒪?
    if *p == [0, 0, 0, 0, 0, 0, 0, 0] {
        // Is q = 𝒪?
        if *q == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
            // Both are 𝒪, then e(𝒪,𝒪) = 1
            let mut one = [0; 48];
            one[0] = 1;
            return (one, 0);
        } else {
            // Is q on the curve?
            if is_on_curve_twist_bn254(q) {
                // Is q on the subgroup G2?
                if is_on_subgroup_twist_bn254(q) {
                    // q is valid, then e(𝒪,q) = 1
                    let mut one = [0; 48];
                    one[0] = 1;
                    return (one, 0);
                } else {
                    #[cfg(debug_assertions)]
                    println!("q is not in the subgroup");

                    return ([0; 48], 9);
                }
            } else {
                #[cfg(debug_assertions)]
                println!("q is not on the curve");

                return ([0; 48], 8);
            }
        }
    }

    // Is Q = 𝒪?
    if *q == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
        // Is p on the curve?
        if is_on_curve_bn254(p) {
            // As p is on the curve, it is also in the subgroup
            // then e(p,𝒪) = 1
            let mut one = [0; 48];
            one[0] = 1;
            return (one, 0);
        } else {
            #[cfg(debug_assertions)]
            println!("p is not on the curve");

            return ([0; 48], 7);
        }
    }

    // If neither p nor q are 𝒪, we can check if they belong to the subgroup
    if !is_on_curve_bn254(p) {
        #[cfg(debug_assertions)]
        println!("p is not on the curve");

        return ([0; 48], 7);
    }

    if !is_on_curve_twist_bn254(q) {
        #[cfg(debug_assertions)]
        println!("q is not on the curve");

        return ([0; 48], 8);
    }

    if !is_on_subgroup_twist_bn254(q) {
        #[cfg(debug_assertions)]
        println!("q is not in the subgroup");

        return ([0; 48], 9);
    }

    // Miller loop
    let miller_loop = miller_loop_bn254(p, q);

    // Final exponentiation
    let final_exp = final_exp_bn254(&miller_loop);

    (final_exp, 0)
}

/// Computes the optimal Ate pairing for a batch of G1 and G2 points over the BN254 curve
/// and multiplies the results together, i.e.:
///     e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) ∈ GT
pub fn pairing_batch_bn254(g1_points: &[[u64; 8]], g2_points: &[[u64; 16]]) -> [u64; 48] {
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
        if *p == [0u64; 8] || *q == [0u64; 16] {
            // MillerLoop(P, 𝒪) = MillerLoop(𝒪, Q) = 1; we can skip
            continue;
        }

        g1_points_ml.push(*p);
        g2_points_ml.push(*q);
    }

    if g1_points_ml.is_empty() {
        // If all pairing computations were skipped, return 1
        let mut one = [0; 48];
        one[0] = 1;
        return one;
    }

    // Compute the Miller loop for the batch
    let miller_loop = miller_loop_batch_bn254(&g1_points_ml, &g2_points_ml);

    // Final exponentiation
    let res = final_exp_bn254(&miller_loop);

    res
}
