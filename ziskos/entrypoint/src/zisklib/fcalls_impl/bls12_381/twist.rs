use super::fp2_inv::{
    bls12_381_fp2_dbl, bls12_381_fp2_inv, bls12_381_fp2_mul, bls12_381_fp2_scalar_mul,
    bls12_381_fp2_square, bls12_381_fp2_sub,
};

/// Computes the coefficients (𝜆,𝜇) of a line passing through points (x1,y1),(x2,y2)
pub fn fcall_bls12_381_twist_add_line_coeffs(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let x1: &[u64; 12] = &params[0..12].try_into().unwrap();
    let y1: &[u64; 12] = &params[12..24].try_into().unwrap();
    let x2: &[u64; 12] = &params[24..36].try_into().unwrap();
    let y2: &[u64; 12] = &params[36..48].try_into().unwrap();

    // Compute the line coefficients
    let (lambda, mu) = bls12_381_twist_add_line_coeffs(x1, y1, x2, y2);

    // Store the result
    results[0..12].copy_from_slice(&lambda);
    results[12..24].copy_from_slice(&mu);

    24
}

pub fn bls12_381_twist_add_line_coeffs(
    x1: &[u64; 12],
    y1: &[u64; 12],
    x2: &[u64; 12],
    y2: &[u64; 12],
) -> ([u64; 12], [u64; 12]) {
    // Compute 𝜆 = (y2 - y1)/(x2 - x1)
    let mut lambda = bls12_381_fp2_inv(&bls12_381_fp2_sub(x2, x1));
    lambda = bls12_381_fp2_mul(&lambda, &bls12_381_fp2_sub(y2, y1));

    // Compute 𝜇 = y - 𝜆x
    let mu = bls12_381_fp2_sub(y1, &bls12_381_fp2_mul(&lambda, x1));

    (lambda, mu)
}

/// Computes the coefficients (𝜆,𝜇) of the tangent line at the point (x,y)
pub fn fcall_bls12_381_twist_dbl_line_coeffs(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let x: &[u64; 12] = &params[0..12].try_into().unwrap();
    let y: &[u64; 12] = &params[12..24].try_into().unwrap();

    // Compute the line coefficients
    let (lambda, mu) = bls12_381_twist_dbl_line_coeffs(x, y);

    // Store the result
    results[0..12].copy_from_slice(&lambda);
    results[12..24].copy_from_slice(&mu);

    24
}

pub fn bls12_381_twist_dbl_line_coeffs(x: &[u64; 12], y: &[u64; 12]) -> ([u64; 12], [u64; 12]) {
    // Compute 𝜆 = 3x²/2y
    let mut lambda = bls12_381_fp2_inv(&bls12_381_fp2_dbl(y));
    let x_sq = bls12_381_fp2_square(x);
    lambda = bls12_381_fp2_mul(&lambda, &bls12_381_fp2_scalar_mul(&x_sq, &[3, 0, 0, 0, 0, 0]));

    // Compute 𝜇 = y - 𝜆x
    let mu = bls12_381_fp2_sub(y, &bls12_381_fp2_mul(&lambda, x));

    (lambda, mu)
}
