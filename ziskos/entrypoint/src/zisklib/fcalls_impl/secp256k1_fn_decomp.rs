use lazy_static::lazy_static;
use lll_rs::{
    lll::biglll::lattice_reduce,
    matrix::Matrix,
    vector::BigVector,
};
use rug::{integer::Order, Integer};

lazy_static! {
    pub static ref N: Integer = Integer::from_str_radix(
        "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
        16
    )
    .unwrap();
    pub static ref LAMBDA: Integer = Integer::from_str_radix(
        "5363ad4cc05c30e0a5261c028812645a122e22ea20816678df02967c1b23bd72",
        16
    )
    .unwrap();
}

pub fn fcall_secp256k1_fn_decomp(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let k1: &[u64; 4] = &params[0..4].try_into().unwrap();
    let k2: &[u64; 4] = &params[4..8].try_into().unwrap();

    // Perform the inversion using fn inversion
    let [x1, y1, x2, y2, z, t] = secp256k1_fn_decomp(k1, k2);

    // Store the result
    results[0..4].copy_from_slice(&x1);
    results[4..8].copy_from_slice(&y1);
    results[8..12].copy_from_slice(&x2);
    results[12..16].copy_from_slice(&y2);
    results[16..20].copy_from_slice(&z);
    results[20..24].copy_from_slice(&t);

    24
}

fn secp256k1_fn_decomp(k1: &[u64; 4], k2: &[u64; 4]) -> [[u64; 4]; 6] {
    /* 
    Compute a reduced basis for the lattice:
        | r  0   0  0  0  0|
        | 0  r   0  0  0  0|
        | 0  0   r  0  0  0|
        | 0  0   0  r  0  0|
        | 0  0   0  0  r  0|
        | 0  0   0  0  0  r|
        |-λ  1   0  0  0  0|
        | 0  0  -λ  1  0  0|
        |k1  0  k2  0  1  0|
        | 0  0   0  0 -λ  1|
    */
    let mut basis: Matrix<BigVector> = Matrix::init(10, 6);
    basis[0] = BigVector::from_vector(vec![
        N.clone(),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
    ]);

    basis[1] = BigVector::from_vector(vec![
        Integer::from(0),
        N.clone(),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
    ]);

    basis[2] = BigVector::from_vector(vec![
        Integer::from(0),
        Integer::from(0),
        N.clone(),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
    ]);

    basis[3] = BigVector::from_vector(vec![
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        N.clone(),
        Integer::from(0),
        Integer::from(0),
    ]);

    basis[4] = BigVector::from_vector(vec![
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        N.clone(),
        Integer::from(0),
    ]);

    basis[5] = BigVector::from_vector(vec![
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        N.clone(),
    ]);

    basis[6] = BigVector::from_vector(vec![
        -LAMBDA.clone(),
        Integer::from(1),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
    ]);

    basis[7] = BigVector::from_vector(vec![
        Integer::from(0),
        Integer::from(0),
        -LAMBDA.clone(),
        Integer::from(1),
        Integer::from(0),
        Integer::from(0),
    ]);

    basis[8] = BigVector::from_vector(vec![
        Integer::from_digits(k1, Order::Lsf),
        Integer::from(0),
        Integer::from_digits(k2, Order::Lsf),
        Integer::from(0),
        Integer::from(1),
        Integer::from(0),
    ]);

    basis[9] = BigVector::from_vector(vec![
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        Integer::from(0),
        -LAMBDA.clone(),
        Integer::from(1),
    ]);

    lattice_reduce(&mut basis);

    // Find the solution in the reduced basis
    let mut solution = &basis[0];
    for i in 1..10 {
        if solution[4] == Integer::from(0) && solution[5] == Integer::from(0) {
            solution = &basis[i];
        } else {
            break;
        }
    }

    // Reduce modulo N and return
    let mut x1: Vec<u64> = solution[0].clone().modulo(&N).to_digits(Order::Lsf);
    let mut y1: Vec<u64> = solution[1].clone().modulo(&N).to_digits(Order::Lsf);
    let mut x2: Vec<u64> = solution[2].clone().modulo(&N).to_digits(Order::Lsf);
    let mut y2: Vec<u64> = solution[3].clone().modulo(&N).to_digits(Order::Lsf);
    let mut z: Vec<u64> = solution[4].clone().modulo(&N).to_digits(Order::Lsf);
    let mut t: Vec<u64> = solution[5].clone().modulo(&N).to_digits(Order::Lsf);
    x1.resize(4, 0);
    y1.resize(4, 0);
    x2.resize(4, 0);
    y2.resize(4, 0);
    z.resize(4, 0);
    t.resize(4, 0);

    return [
        x1.try_into().unwrap(),
        y1.try_into().unwrap(),
        x2.try_into().unwrap(),
        y2.try_into().unwrap(),
        z.try_into().unwrap(),
        t.try_into().unwrap(),
    ];
}

#[cfg(test)]
mod tests {
    use rug::rand::RandState;

    use super::*;

    #[test]
    fn test_decomposition() {
        let mut rand = RandState::new();
        for i in 0..10 {
            println!("Testing number i: {}", i);
            let k1_int: Integer  = N.clone().random_below(&mut rand);
            let k2_int: Integer  = N.clone().random_below(&mut rand);

            let mut k1 = k1_int.to_digits(Order::Lsf);
            let mut k2 = k2_int.to_digits(Order::Lsf);
            k1.resize(4, 0);
            k2.resize(4, 0);
            let params = [k1[0], k1[1], k1[2], k1[3], k2[0], k2[1], k2[2], k2[3]];
            let mut results = [0; 24];
            fcall_secp256k1_fn_decomp(&params, &mut results);
            let x1 = Integer::from_digits(&results[0..4], Order::Lsf);
            let y1 = Integer::from_digits(&results[4..8], Order::Lsf);
            let x2 = Integer::from_digits(&results[8..12], Order::Lsf);
            let y2 = Integer::from_digits(&results[12..16], Order::Lsf);
            let z = Integer::from_digits(&results[16..20], Order::Lsf);
            let t = Integer::from_digits(&results[20..24], Order::Lsf);

            let den = (z + t * LAMBDA.clone()).invert(&N).expect("Denominator should not be zero");
            let r1 = ((x1 + y1 * LAMBDA.clone()) * den.clone()).modulo(&N);
            let r2 = ((x2 + y2 * LAMBDA.clone()) * den).modulo(&N);
            assert_eq!(r1, k1_int);
            assert_eq!(r2, k2_int);
        }
    }
}
