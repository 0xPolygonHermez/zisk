//! Hash-to-curve for BLS12-381 G2

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

use crate::zisklib::lib::sha256::sha256;

use super::{
    fp::{add_fp_bls12_381, mul_fp_bls12_381},
    map_to_curve::map_to_curve_g2_no_cofactor_bls12_381,
    twist::{add_complete_safe_twist_bls12_381, clear_cofactor_twist_bls12_381},
};

/// Hash an arbitrary byte string to a point in the BLS12-381 G2 prime-order
pub fn hash_to_curve_g2_bls12_381(
    msg: &[u8],
    dst: &[u8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    // Hash to field to get 2 Fp2 elements (u0, u1)
    let u = hash_to_field_fp2_count2_bls12_381(
        msg,
        dst,
        #[cfg(feature = "hints")]
        hints,
    );

    // Map u0 to a point on the curve Q0
    let q0 = map_to_curve_g2_no_cofactor_bls12_381(
        &u[0],
        #[cfg(feature = "hints")]
        hints,
    )
    .expect("hash_to_field output is reduced mod p");

    // Map u1 to a point on the curve Q1
    let q1 = map_to_curve_g2_no_cofactor_bls12_381(
        &u[1],
        #[cfg(feature = "hints")]
        hints,
    )
    .expect("hash_to_field output is reduced mod p");

    // R = Q0 + Q1
    let r = add_complete_safe_twist_bls12_381(
        &q0,
        &q1,
        #[cfg(feature = "hints")]
        hints,
    )
    .expect("Q0 and Q1 are on the curve by construction");

    // Clear cofactor
    clear_cofactor_twist_bls12_381(
        &r,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Expand a message to `len_in_bytes` uniformly random bytes
fn expand_message_xmd_sha256(
    msg: &[u8],
    dst: &[u8],
    len_in_bytes: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<u8> {
    const B_IN_BYTES: usize = 32; // SHA-256 output length in bytes
    const S_IN_BYTES: usize = 64; // SHA-256 input block length in bytes

    assert!(dst.len() <= 255, "DST too long for expand_message_xmd");
    assert!(len_in_bytes <= 0xFFFF, "len_in_bytes exceeds its limit");

    let ell = len_in_bytes.div_ceil(B_IN_BYTES);
    assert!(ell <= 255, "expand_message_xmd requires ell <= 255");

    // DST_prime = DST || I2OSP(len(DST), 1)
    let mut dst_prime = Vec::with_capacity(dst.len() + 1);
    dst_prime.extend_from_slice(dst);
    dst_prime.push(dst.len() as u8);

    // msg_prime = I2OSP(0, s_in_bytes) || msg || I2OSP(len_in_bytes, 2) || I2OSP(0, 1) || DST_prime
    let mut msg_prime = Vec::with_capacity(S_IN_BYTES + msg.len() + 3 + dst_prime.len());
    msg_prime.extend(core::iter::repeat(0u8).take(S_IN_BYTES));
    msg_prime.extend_from_slice(msg);
    msg_prime.push((len_in_bytes >> 8) as u8);
    msg_prime.push((len_in_bytes & 0xff) as u8);
    msg_prime.push(0);
    msg_prime.extend_from_slice(&dst_prime);

    // b_0 = H(msg_prime)
    let b_0 = sha256(
        &msg_prime,
        #[cfg(feature = "hints")]
        hints,
    );

    // b_1 = H(b_0 || I2OSP(1, 1) || DST_prime)
    let mut buf: Vec<u8> = Vec::with_capacity(B_IN_BYTES + 1 + dst_prime.len());
    buf.extend_from_slice(&b_0);
    buf.push(1);
    buf.extend_from_slice(&dst_prime);
    let mut b_prev = sha256(
        &buf,
        #[cfg(feature = "hints")]
        hints,
    );

    // b_i = H(strxor(b_0, b_{i-1}) || I2OSP(i, 1) || DST_prime), for i in 2..=ell
    let mut uniform_bytes = Vec::with_capacity(B_IN_BYTES * ell);
    uniform_bytes.extend_from_slice(&b_prev);
    for i in 2..=ell {
        let mut xored = [0u8; B_IN_BYTES];
        for j in 0..B_IN_BYTES {
            xored[j] = b_0[j] ^ b_prev[j];
        }
        buf.clear();
        buf.extend_from_slice(&xored);
        buf.push(i as u8);
        buf.extend_from_slice(&dst_prime);
        b_prev = sha256(
            &buf,
            #[cfg(feature = "hints")]
            hints,
        );
        uniform_bytes.extend_from_slice(&b_prev);
    }

    uniform_bytes.truncate(len_in_bytes);
    uniform_bytes
}

/// Reduces a 64-byte big-endian integer to an Fp element via the
/// split-and-shift trick: each 32-byte half is already < 2^256 < p, so we just
/// compute `hi * 2^256 + lo (mod p)` with one Fp multiplication and one Fp add.
fn os2ip_64_be_mod_p(bytes: &[u8; 64], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 6] {
    const R256_FP: [u64; 6] = [0, 0, 0, 0, 1, 0]; // 2^256 mod p

    let mut d_hi = [0u64; 6];
    let mut d_lo = [0u64; 6];
    for i in 0..4 {
        for j in 0..8 {
            d_hi[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
            d_lo[3 - i] |= (bytes[32 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }
    let shifted = mul_fp_bls12_381(
        &d_hi,
        &R256_FP,
        #[cfg(feature = "hints")]
        hints,
    );
    add_fp_bls12_381(
        &shifted,
        &d_lo,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Hash an arbitrary-length byte string to two Fp2 elements
fn hash_to_field_fp2_count2_bls12_381(
    msg: &[u8],
    dst: &[u8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [[u64; 12]; 2] {
    const L: usize = 64; // ceil((ceil(log2(p)) + k) / 8) = ceil((381 + 128) / 8)
    const M: usize = 2; // Fp2 has degree 2 over Fp
    const COUNT: usize = 2; // Output 2 Fp2 elements for the G2 suite

    // Expand the message to 256 uniformly random bytes
    let uniform = expand_message_xmd_sha256(
        msg,
        dst,
        L * M * COUNT,
        #[cfg(feature = "hints")]
        hints,
    );

    // Parse the output as 4 big-endian 64-byte integers,
    // reduce each mod p to get 4 Fp elements,
    // and pack those into 2 Fp2 elements
    let mut result = [[0u64; 12]; COUNT];
    for (i, fp2) in result.iter_mut().enumerate() {
        for j in 0..M {
            let off = L * (j + i * M);
            let chunk: &[u8; L] = uniform[off..off + L].try_into().unwrap();
            let e_j = os2ip_64_be_mod_p(
                chunk,
                #[cfg(feature = "hints")]
                hints,
            );
            let limb_off = j * 6;
            fp2[limb_off..limb_off + 6].copy_from_slice(&e_j);
        }
    }
    result
}
