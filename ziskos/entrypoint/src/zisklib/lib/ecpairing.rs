use super::{
    bn254::{
        constants::P_MINUS_ONE,
        curve::is_on_curve_bn254,
        final_exp::final_exp_bn254,
        fp12::mul_fp12_bn254,
        miller_loop::miller_loop_bn254,
        twist::{is_on_curve_twist_bn254, is_on_subgroup_twist_bn254},
    },
    utils::gt,
};

/// Performs a pairing product check over the BN254 curve.
///
/// Given n pairs of elliptic curve points:
/// - `P₁, ..., Pₙ ∈ G1` (BN254 curve over Fp)
/// - `Q₁, ..., Qₙ ∈ G2` (twist curve over Fp²),
///
/// where:
/// - G1 = E(Fp)\[r\], the group of r-torsion points on E/Fp: y² = x³ + 3,
/// - G2 = E'(Fp²)\[r\], the group of r-torsion points on E'/Fp²: y² = x³ + 3 / (9 + u),
///
/// This function checks whether:
/// ```text
///     e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) == 1 in GT
/// ```
/// where:
/// - e: G1 × G2 → GT is the optimal Ate pairing,
/// - GT = μ_r, the group of r-th roots of unity in Fp¹²*.
///
/// The input is a flattened array of length `8 * n`, representing n pairs of G1 and G2 points:
/// - Each G1 point is 1 × `[u64; 8]`
/// - Each G2 point is 1 × `[u64; 16]` (typically, Fp² x/y each need 2 × [u64; 4])
///
/// ### Returns
/// - `bool`: true if the pairing product is 1, false otherwise.
/// - `u8`: Error code:
///   - `0`: success
///   - `1`: generic error
pub fn ecpairing(g1_points: &[[u64; 8]], g2_points: &[[u64; 16]]) -> (bool, u8) {
    // Since each e(Pi, Qi) := FinalExp(MillerLoop(Pi, Qi))
    // We have:
    //  e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) = FinalExp(MillerLoop(P₁, Q₁) · MillerLoop(P₂, Q₂) · ... · MillerLoop(Pₙ, Qₙ))
    // We can compute the Miller loop for each pair, multiplying the results together
    // and then just do the final exponentiation once at the end.

    let num_points = g1_points.len();
    assert_eq!(num_points, g2_points.len(), "Number of G1 and G2 points must be equal");

    // Miller loop and multiplication
    let mut acc = [0; 48];
    acc[0] = 1;
    for (p, q) in g1_points.iter().zip(g2_points.iter()) {
        // Check p and q are valid

        // Verify the coordinates of p
        let x1: [u64; 4] = p[0..4].try_into().unwrap();
        if gt(&x1, &P_MINUS_ONE) {
            #[cfg(debug_assertions)]
            println!("x1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x1);

            return (false, 1);
        }

        let y1: [u64; 4] = p[4..8].try_into().unwrap();
        if gt(&y1, &P_MINUS_ONE) {
            #[cfg(debug_assertions)]
            println!("y1 should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y1);

            return (false, 1);
        }

        // Verify the coordinates of q
        let x2_r: [u64; 4] = q[0..4].try_into().unwrap();
        if gt(&x2_r, &P_MINUS_ONE) {
            #[cfg(debug_assertions)]
            println!("x2_r should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2_r);

            return (false, 1);
        }

        let x2_i: [u64; 4] = q[4..8].try_into().unwrap();
        if gt(&x2_i, &P_MINUS_ONE) {
            #[cfg(debug_assertions)]
            println!("x2_i should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, x2_i);

            return (false, 1);
        }

        let y2_r: [u64; 4] = q[8..12].try_into().unwrap();
        if gt(&y2_r, &P_MINUS_ONE) {
            #[cfg(debug_assertions)]
            println!("y2_r should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2_r);

            return (false, 1);
        }

        let y2_i: [u64; 4] = q[12..16].try_into().unwrap();
        if gt(&y2_i, &P_MINUS_ONE) {
            #[cfg(debug_assertions)]
            println!("y2_i should be less than P_MINUS_ONE: {:?}, but got {:?}", P_MINUS_ONE, y2_i);

            return (false, 1);
        }

        // Is p = 𝒪?
        if *p == [0, 0, 0, 0, 0, 0, 0, 0] {
            // Is q = 𝒪?
            if *q == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
                // Both are 𝒪, then MillerLoop(𝒪, 𝒪) = 1; we can skip
                continue;
            } else {
                // Is q on the curve?
                if is_on_curve_twist_bn254(q) {
                    // Is q on the subgroup G2?
                    if is_on_subgroup_twist_bn254(q) {
                        // q is valid, but MillerLoop(𝒪, q) = 1; we can skip
                        continue;
                    } else {
                        #[cfg(debug_assertions)]
                        println!("q is not in the subgroup");

                        return (false, 1);
                    }
                } else {
                    #[cfg(debug_assertions)]
                    println!("q is not on the curve");

                    return (false, 1);
                }
            }
        }

        // Is Q = 𝒪?
        if *q == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
            // Is p on the curve?
            if is_on_curve_bn254(p) {
                // As p is on the curve, it is also in the subgroup
                // MillerLoop(p, 𝒪) = 1; we can skip
                continue;
            } else {
                #[cfg(debug_assertions)]
                println!("p is not on the curve");

                return (false, 1);
            }
        }

        // If neither p nor q are 𝒪, we can check if they belong to the subgroup
        if !is_on_curve_bn254(p) {
            #[cfg(debug_assertions)]
            println!("p is not on the curve");

            return (false, 1);
        }

        if !is_on_curve_twist_bn254(q) {
            #[cfg(debug_assertions)]
            println!("q is not on the curve");

            return (false, 1);
        }

        if !is_on_subgroup_twist_bn254(q) {
            #[cfg(debug_assertions)]
            println!("q is not in the subgroup");

            return (false, 1);
        }

        // Miller loop
        let miller_loop = miller_loop_bn254(p, q);

        // Update result with the new miller loop result
        acc = mul_fp12_bn254(&acc, &miller_loop); // TODO: The mul is sparse, so we can optimize this further
    }

    // Final exponentiation
    let acc = final_exp_bn254(&acc);

    // Check if the result is equal to 1
    let mut one = [0; 48];
    one[0] = 1;
    let is_satisfied = acc == one;
    (is_satisfied, 0)
}
