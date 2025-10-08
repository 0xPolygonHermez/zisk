use super::{U256, rem_short, rem_long, mul_long, mul_short};


/// Multiplication of two large numbers (represented as arrays of U256) followed by reduction modulo a third large number
/// 
/// It assumes that modulus > 0 and modulus has no leading zeros
pub fn mul_and_reduce(a: &[U256], b: &[U256], modulus: &[U256], out: &mut [U256]) {
    let len_m = modulus.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_m, 0, "Input 'modulus' must have at least one limb");
        assert_ne!(modulus.last().unwrap(), &U256::ZERO, "Input 'modulus' must not have leading zeros");
        assert!(out.len() >= len_m, "Output 'out' must have at least len(modulus) limbs");
    }

    let len_mul = a.len() + b.len();
    let mut mul = vec![U256::ZERO; len_mul];
    if b.len() == 1 {
        mul_short(a, &b[0], &mut mul);
    } else {
        mul_long(a, b, &mut mul);
    };

    // If a·b < modulus, then the result is just a·b
    if U256::lt_slices(&mul, modulus) {
        out[..len_mul].copy_from_slice(&mul);
        return;
    }
    
    if len_m == 1 {
        // If modulus has only one limb, we can use short division
        out[0] = rem_short(&mul, &modulus[0]);
    } else {
        // Otherwise, use long division
        let r = rem_long(&mul, modulus);
        out[..r.len()].copy_from_slice(&r);
    }
}