//! Module for operations on the BN254 curve E: y² = x³ + 3

use crate::zisklib::lib::utils::eq;

use super::{
    constants::E_B,
    fp::{add_fp_bn254, mul_fp_bn254, square_fp_bn254},
};

pub fn is_on_curve_bn254(p: &[u64; 8]) -> bool {
    // p in E iff y² == x³ + 3
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();
    let x_sq = square_fp_bn254(&x);
    let x_cubed = mul_fp_bn254(&x_sq, &x);
    let x_cubed_plus_b = add_fp_bn254(&x_cubed, &E_B);
    let y_sq = square_fp_bn254(&y);
    eq(&x_cubed_plus_b, &y_sq)
}
