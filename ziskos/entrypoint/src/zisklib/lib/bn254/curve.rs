//! Operations on the BN254 curve E: y² = x³ + 3

use crate::zisklib::lib::utils::eq;

use super::{
    constants::E_B,
    fp::{add_fp_bn254, mul_fp_bn254, square_fp_bn254},
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
