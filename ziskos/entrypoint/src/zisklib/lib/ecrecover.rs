use tiny_keccak::{Hasher, Keccak};

use crate::point256::SyscallPoint256;

use super::{
    secp256k1::{
        constants::{E_B, N_MINUS_ONE},
        curve::secp256k1_double_scalar_mul_with_g,
        field::{secp256k1_fp_add, secp256k1_fp_mul, secp256k1_fp_sqrt, secp256k1_fp_square},
        scalar::{secp256k1_fn_inv, secp256k1_fn_mul, secp256k1_fn_neg},
    },
    utils::gt,
};

/// Given a hash `hash`, a recovery parity `v` and a signature (`r`, `s`),
/// this function computes the address that signed the hash.
///
/// It also returns an error code:
/// - 0: No error
/// - 1: r should be greater than 0
/// - 2: r should be less than `N_MINUS_ONE`
/// - 3: s should be greater than 0
/// - 4: s should be less than `N_MINUS_ONE`
/// - 5: The recovery id should be either 0 or 1
/// - 6: No square root found for `y_sq`
/// - 7: The public key is the point at infinity
pub fn ecrecover(sig: &[u8; 65], msg: &[u8; 32]) -> ([u8; 20], u8) {
    // Extract the signature components (from BEu8 to LEu64)
    let mut r = [0u64; 4];
    let mut s = [0u64; 4];
    for i in 0..32 {
        let pos = 3 - i / 8;
        let shift = 8 * (7 - (i % 8));
        r[pos] |= (sig[i] as u64) << shift;
        s[pos] |= (sig[32 + i] as u64) << shift;
    }

    // Check r is in the range [1, n-1]
    if r == [0, 0, 0, 0] {
        #[cfg(debug_assertions)]
        println!("r should be greater than 0");

        return ([0u8; 20], 1);
    } else if gt(&r, &N_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("r should be less than N_MINUS_ONE: {:?}, but got {:?}", N_MINUS_ONE, r);

        return ([0u8; 20], 2);
    }

    // Check s is either in the range [1, n-1] (precompiled) or [1, (n-1)/2] (tx):
    if s == [0, 0, 0, 0] {
        #[cfg(debug_assertions)]
        println!("s should be greater than 0");

        return ([0u8; 20], 3);
    } else if gt(&s, &N_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("s should be less than N_MINUS_ONE: {:?}, but got {:?}", N_MINUS_ONE, s);

        return ([0u8; 20], 4);
    }

    // Extract the parity: 0 indicates that y is even, 1 indicates that y is odd
    let parity = sig[64] as u64;

    // Check the recovery id is a bit
    if parity > 1 {
        #[cfg(debug_assertions)]
        println!("parity should be either 0 or 1, but got {:?}", parity);

        return ([0u8; 20], 5);
    }

    // Get the hash
    let mut hash = [0u64; 4];
    for i in 0..32 {
        hash[3 - i / 8] |= (msg[i] as u64) << (8 * (7 - (i % 8)));
    }

    // In Ethereum, signatures where the x-coordinate of the resulting point is
    // greater than N are considered invalid. Hence, r = x as integers

    // Calculate the y-coordinate of the point: y = sqrt(x³ + 7)
    let x_sq = secp256k1_fp_square(&r);
    let x_cb = secp256k1_fp_mul(&x_sq, &r);
    let y_sq = secp256k1_fp_add(&x_cb, &E_B);
    let (y, has_sqrt) = secp256k1_fp_sqrt(&y_sq, parity);
    if !has_sqrt {
        return ([0u8; 20], 6);
    }

    // Check the received parity of the y-coordinate is correct, otherwise MAP
    let y_parity = y[0] & 1;
    assert_eq!(y_parity, parity);

    // Calculate the public key

    // Compute k1 = (-hash * r_inv) % N
    let r_inv = secp256k1_fn_inv(&r);
    let mul = secp256k1_fn_mul(&hash, &r_inv);
    let k1 = secp256k1_fn_neg(&mul);

    // Compute k2 = (s * r_inv) % N
    let k2 = secp256k1_fn_mul(&s, &r_inv);

    // Calculate the public key
    let p = SyscallPoint256 { x: r, y };
    let (pk_is_infinity, pk) = secp256k1_double_scalar_mul_with_g(&k1, &k2, &p);
    if pk_is_infinity {
        #[cfg(debug_assertions)]
        println!("The public key is the point at infinity");

        return ([0u8; 20], 7);
    }

    // Compute the hash of the public key
    let mut buf = [0u8; 64];
    for i in 0..4 {
        buf[i * 8..(i + 1) * 8].copy_from_slice(&pk.x[3 - i].to_be_bytes());
        buf[32 + i * 8..32 + (i + 1) * 8].copy_from_slice(&pk.y[3 - i].to_be_bytes());
    }

    let mut pk_hash = [0u8; 32];
    let mut keccak = Keccak::v256();
    keccak.update(&buf);
    keccak.finalize(&mut pk_hash);

    // Return the right-most 20 bytes of the hash
    let mut addr = [0u8; 20];
    for i in 0..20 {
        addr[i] = pk_hash[12 + i];
    }
    (addr, 0)
}
