use super::bn254_fp2::{
    bn254_fp2_dbl, bn254_fp2_inv, bn254_fp2_mul, bn254_fp2_scalar_mul, bn254_fp2_square,
    bn254_fp2_sub,
};

/// Computes the coefficients (𝜆,𝜇) of a line passing through points (x1,y1),(x2,y2)
pub fn fcall_bn254_twist_add_line_coeffs(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let x1: &[u64; 8] = &params[0..8].try_into().unwrap();
    let y1: &[u64; 8] = &params[8..16].try_into().unwrap();
    let x2: &[u64; 8] = &params[16..24].try_into().unwrap();
    let y2: &[u64; 8] = &params[24..32].try_into().unwrap();

    // Compute 𝜆 = (y2 - y1)/(x2 - x1)
    let mut lambda = bn254_fp2_inv(&bn254_fp2_sub(x2, x1));
    lambda = bn254_fp2_mul(&lambda, &bn254_fp2_sub(y2, y1));

    // Compute 𝜇 = y - 𝜆x
    let mu = bn254_fp2_sub(y1, &bn254_fp2_mul(&lambda, x1));

    // Store the result
    results[0..8].copy_from_slice(&lambda);
    results[8..16].copy_from_slice(&mu);

    16
}

/// Computes the coefficients (𝜆,𝜇) of the tangent line at the point (x,y)
pub fn fcall_bn254_twist_dbl_line_coeffs(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let x: &[u64; 8] = &params[0..8].try_into().unwrap();
    let y: &[u64; 8] = &params[8..16].try_into().unwrap();

    // Compute 𝜆 = 3x²/2y
    let mut lambda = bn254_fp2_inv(&bn254_fp2_dbl(y));
    let x_sq = bn254_fp2_square(x);
    lambda = bn254_fp2_mul(&lambda, &bn254_fp2_scalar_mul(&x_sq, &[3, 0, 0, 0]));

    // Compute 𝜇 = y - 𝜆x
    let mu = bn254_fp2_sub(y, &bn254_fp2_mul(&lambda, x));

    // Store the result
    results[0..8].copy_from_slice(&lambda);
    results[8..16].copy_from_slice(&mu);

    16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_line() {
        let p = [
            0x66f0731159b54cd6,
            0xb630013739a5a053,
            0x31045e15f3a54bc2,
            0x214275f5c7d57155,
            0xfaf80b929d13179a,
            0xf63689aef8ecc6ff,
            0x26ffe67c5b2f3a49,
            0x04d4ad74230d1e83,
            0x46246b07a2ce41fd,
            0x65cd5922607deeee,
            0xe4ae145fac34c502,
            0x1e977a2280041e87,
            0x20ca11200df6b3c4,
            0x00bed9e88dfb7f8d,
            0x735adb5c7981edda,
            0x226adef094e4c626,
        ];
        let q = [
            0x4b70ada95bc43412,
            0x38f6cc990d30c020,
            0xca7d1f2becd3258a,
            0x2f9041da70888180,
            0x8d940679d41b2409,
            0xb28d0f4c5ea7672c,
            0xaa05b19dfad3217a,
            0x04ff3ef00c3f7d32,
            0x0cf3024d5172b33a,
            0xb3f5b354255ea1ee,
            0x70f37619880ce080,
            0x0e35dfd0b8edaa9c,
            0xf0e610b9d6ba7228,
            0x8d4202db12ceed20,
            0xdab0c37f22e05f42,
            0x172945c562cea2c7,
        ];

        let mut params = [0u64; 32];
        params[0..16].copy_from_slice(&p);
        params[16..32].copy_from_slice(&q);
        let mut results = [0; 16];
        fcall_bn254_twist_add_line_coeffs(&params, &mut results);

        let expected_lambda = [
            0x70a3dd9659d4661d,
            0x272dad27777b65c9,
            0x0d3ed5d3d8417100,
            0x28b3fb64bf5e0593,
            0x84591f2f3fcbbf52,
            0x14fd5d4745900016,
            0xf620661dd1c5db97,
            0x0352e891aa056e3a,
        ];
        let expected_mu = [
            0x7ae5d34cb3796d62,
            0x72e9885302380fda,
            0x90ba3e6a5edbad26,
            0x0da370e47b9854d6,
            0x337d5300e9a1f793,
            0x8e74c5f9836fb364,
            0x9207b0b313b312b5,
            0x263c38b6fef528c5,
        ];
        assert_eq!(results[0..8], expected_lambda);
        assert_eq!(results[8..16], expected_mu);
    }

    #[test]
    fn test_dbl_line() {
        let p = [
            0x66f0731159b54cd6,
            0xb630013739a5a053,
            0x31045e15f3a54bc2,
            0x214275f5c7d57155,
            0xfaf80b929d13179a,
            0xf63689aef8ecc6ff,
            0x26ffe67c5b2f3a49,
            0x04d4ad74230d1e83,
            0x46246b07a2ce41fd,
            0x65cd5922607deeee,
            0xe4ae145fac34c502,
            0x1e977a2280041e87,
            0x20ca11200df6b3c4,
            0x00bed9e88dfb7f8d,
            0x735adb5c7981edda,
            0x226adef094e4c626,
        ];

        let mut results = [0; 16];
        fcall_bn254_twist_dbl_line_coeffs(&p, &mut results);

        let expected_lambda = [
            0xfa23df0596bf5ac0,
            0x1d60eabc30697e27,
            0xde847f8d09ff3261,
            0x0d2b35469ba57c1a,
            0x0e441461c8b02f6c,
            0x43ea3964b1f2af60,
            0x371d248d3d09e45f,
            0x260ac06e4d6faf7d,
        ];
        let expected_mu = [
            0xf1e9e54da61ae409,
            0x1473486505dd6aeb,
            0xb6cb8f0ad3d51eea,
            0x2e6234f03865fd67,
            0xab884e4411c6b07b,
            0x0fafd74a66389f1c,
            0x9b91b3503e4834d0,
            0x0bb2e0552b697667,
        ];
        assert_eq!(results[0..8], expected_lambda);
        assert_eq!(results[8..16], expected_mu);
    }
}
