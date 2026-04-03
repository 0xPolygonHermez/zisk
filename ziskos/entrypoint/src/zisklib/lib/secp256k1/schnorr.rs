//! BIP-340 Schnorr signature verification for secp256k1.
//! https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki

extern crate alloc;
use alloc::vec::Vec;

use crate::zisklib::{eq, gt, sha256, ZERO_256};

use super::{
    constants::{G, N, P},
    curve::{secp256k1_double_scalar_mul_with_g, secp256k1_lift_x, secp256k1_multi_scalar_mul},
    scalar::{secp256k1_fn_add, secp256k1_fn_mul, secp256k1_fn_neg, secp256k1_fn_reduce},
};

fn bytes_be_to_u64_le(bytes: &[u8; 32]) -> [u64; 4] {
    let mut r = [0u64; 4];
    for i in 0..4 {
        for j in 0..8 {
            r[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }
    r
}

/// BIP-340 `Verify(pk, m, sig)`. Arbitrary-length message, 32-byte big-endian pk/r/s.
pub fn secp256k1_schnorr_verify(
    msg: &[u8],
    pk_x: &[u8; 32],
    sig_r: &[u8; 32],
    sig_s: &[u8; 32],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let r = bytes_be_to_u64_le(sig_r);
    let s = bytes_be_to_u64_le(sig_s);
    let pk_x_le = bytes_be_to_u64_le(pk_x);

    if !gt(&P, &pk_x_le) {
        return false;
    }
    if !gt(&P, &r) {
        return false;
    }
    if !gt(&N, &s) {
        return false;
    }

    let point_p = match secp256k1_lift_x(
        &pk_x_le,
        false,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(pt) => pt,
        Err(_) => return false,
    };

    let tag = sha256(
        b"BIP0340/challenge",
        #[cfg(feature = "hints")]
        hints,
    );
    let mut buf = Vec::with_capacity(64 + 32 + 32 + msg.len());
    buf.extend_from_slice(&tag);
    buf.extend_from_slice(&tag);
    buf.extend_from_slice(sig_r);
    buf.extend_from_slice(pk_x);
    buf.extend_from_slice(msg);

    let e_hash = sha256(
        &buf,
        #[cfg(feature = "hints")]
        hints,
    );
    let e = secp256k1_fn_reduce(
        &bytes_be_to_u64_le(&e_hash),
        #[cfg(feature = "hints")]
        hints,
    );
    let neg_e = secp256k1_fn_neg(
        &e,
        #[cfg(feature = "hints")]
        hints,
    );

    let point_r = match secp256k1_double_scalar_mul_with_g(
        &s,
        &neg_e,
        &point_p,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(pt) => pt,
        None => return false,
    };

    if point_r[4] & 1 != 0 {
        return false;
    }

    eq(&[point_r[0], point_r[1], point_r[2], point_r[3]], &r)
}

/// # Safety
/// `pk_x` must point to 32 bytes, `sig` to 64 bytes.
/// `msg` must point to `msg_len` bytes, or may be null if `msg_len == 0`.
#[inline]
#[allow(dead_code)]
pub(crate) unsafe fn secp256k1_schnorr_verify_c(
    msg: *const u8,
    msg_len: usize,
    pk_x: *const u8,
    sig: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let msg_bytes: &[u8] =
        if msg_len == 0 { &[] } else { core::slice::from_raw_parts(msg, msg_len) };
    let pk_bytes: [u8; 32] = core::slice::from_raw_parts(pk_x, 32).try_into().unwrap();
    let sig_bytes: [u8; 64] = core::slice::from_raw_parts(sig, 64).try_into().unwrap();
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&sig_bytes[..32]);
    s.copy_from_slice(&sig_bytes[32..]);
    if secp256k1_schnorr_verify(
        msg_bytes,
        &pk_bytes,
        &r,
        &s,
        #[cfg(feature = "hints")]
        hints,
    ) {
        0
    } else {
        1
    }
}

/// BIP-340 batch verification using the multi-scalar multiplication approach.
/// Verifies multiple Schnorr signatures in a single Pippenger MSM.
///
/// Returns true if all signatures are valid.
/// Panics if the input slices have different lengths.
pub fn secp256k1_schnorr_batch_verify(
    msgs: &[&[u8]],
    pk_xs: &[&[u8; 32]],
    sig_rs: &[&[u8; 32]],
    sig_ss: &[&[u8; 32]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let u = msgs.len();
    if u == 0 {
        return true;
    }
    assert_eq!(pk_xs.len(), u);
    assert_eq!(sig_rs.len(), u);
    assert_eq!(sig_ss.len(), u);

    // For u=1, delegate to single verification (avoids extra lift_x(r))
    if u == 1 {
        return secp256k1_schnorr_verify(
            msgs[0],
            pk_xs[0],
            sig_rs[0],
            sig_ss[0],
            #[cfg(feature = "hints")]
            hints,
        );
    }

    let mut r_vals = Vec::with_capacity(u);
    let mut s_vals = Vec::with_capacity(u);
    let mut pk_vals = Vec::with_capacity(u);

    for i in 0..u {
        let r = bytes_be_to_u64_le(sig_rs[i]);
        let s = bytes_be_to_u64_le(sig_ss[i]);
        let pk = bytes_be_to_u64_le(pk_xs[i]);

        if !gt(&P, &pk) {
            return false;
        }
        if !gt(&P, &r) {
            return false;
        }
        if !gt(&N, &s) {
            return false;
        }

        r_vals.push(r);
        s_vals.push(s);
        pk_vals.push(pk);
    }

    let mut points_p = Vec::with_capacity(u);
    let mut points_r = Vec::with_capacity(u);

    for i in 0..u {
        let point_p = match secp256k1_lift_x(
            &pk_vals[i],
            false,
            #[cfg(feature = "hints")]
            hints,
        ) {
            Ok(pt) => pt,
            Err(_) => return false,
        };
        let point_r = match secp256k1_lift_x(
            &r_vals[i],
            false,
            #[cfg(feature = "hints")]
            hints,
        ) {
            Ok(pt) => pt,
            Err(_) => return false,
        };
        points_p.push(point_p);
        points_r.push(point_r);
    }

    let tag = sha256(
        b"BIP0340/challenge",
        #[cfg(feature = "hints")]
        hints,
    );
    let mut challenges = Vec::with_capacity(u);

    for i in 0..u {
        let mut buf = Vec::with_capacity(64 + 32 + 32 + msgs[i].len());
        buf.extend_from_slice(&tag);
        buf.extend_from_slice(&tag);
        buf.extend_from_slice(sig_rs[i]);
        buf.extend_from_slice(pk_xs[i]);
        buf.extend_from_slice(msgs[i]);
        let e_hash = sha256(
            &buf,
            #[cfg(feature = "hints")]
            hints,
        );
        let e = secp256k1_fn_reduce(
            &bytes_be_to_u64_le(&e_hash),
            #[cfg(feature = "hints")]
            hints,
        );
        challenges.push(e);
    }

    // Deterministic random coefficients seeded by all inputs.
    // SHA256 counter mode (equivalent security to BIP-340's recommended ChaCha20).
    let batch_tag = sha256(
        b"BIP0340/batch",
        #[cfg(feature = "hints")]
        hints,
    );
    let mut seed_buf = Vec::new();
    seed_buf.extend_from_slice(&batch_tag);
    seed_buf.extend_from_slice(&batch_tag);
    for pk in pk_xs {
        seed_buf.extend_from_slice(*pk);
    }
    for msg in msgs {
        let msg_len = (msg.len() as u64).to_le_bytes();
        seed_buf.extend_from_slice(&msg_len);
        seed_buf.extend_from_slice(msg);
    }
    for (r, s) in sig_rs.iter().zip(sig_ss.iter()) {
        seed_buf.extend_from_slice(*r);
        seed_buf.extend_from_slice(*s);
    }
    let seed = sha256(
        &seed_buf,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut coeffs = Vec::with_capacity(u);
    coeffs.push([1u64, 0, 0, 0]);
    for i in 1..u {
        let mut coeff_buf = [0u8; 36];
        coeff_buf[..32].copy_from_slice(&seed);
        coeff_buf[32..36].copy_from_slice(&((i - 1) as u32).to_le_bytes());
        let hash = sha256(
            &coeff_buf,
            #[cfg(feature = "hints")]
            hints,
        );
        let a = secp256k1_fn_reduce(
            &bytes_be_to_u64_le(&hash),
            #[cfg(feature = "hints")]
            hints,
        );
        if eq(&a, &ZERO_256) {
            return false;
        }
        coeffs.push(a);
    }

    // MSM batch equation: (Σ aᵢ·sᵢ)·G + Σ (-aᵢ)·Rᵢ + Σ (-aᵢ·eᵢ)·Pᵢ = O
    let mut s_total = s_vals[0];
    for i in 1..u {
        let ai_si = secp256k1_fn_mul(
            &coeffs[i],
            &s_vals[i],
            #[cfg(feature = "hints")]
            hints,
        );
        s_total = secp256k1_fn_add(
            &s_total,
            &ai_si,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    let mut msm_scalars = Vec::with_capacity(2 * u + 1);
    let mut msm_points = Vec::with_capacity(2 * u + 1);

    if !eq(&s_total, &ZERO_256) {
        msm_scalars.push(s_total);
        msm_points.push(G);
    }

    for i in 0..u {
        let neg_ai = secp256k1_fn_neg(
            &coeffs[i],
            #[cfg(feature = "hints")]
            hints,
        );
        let ai_ei = secp256k1_fn_mul(
            &coeffs[i],
            &challenges[i],
            #[cfg(feature = "hints")]
            hints,
        );
        let neg_ai_ei = secp256k1_fn_neg(
            &ai_ei,
            #[cfg(feature = "hints")]
            hints,
        );

        msm_scalars.push(neg_ai);
        msm_points.push(points_r[i]);

        if !eq(&neg_ai_ei, &ZERO_256) {
            msm_scalars.push(neg_ai_ei);
            msm_points.push(points_p[i]);
        }
    }

    secp256k1_multi_scalar_mul(
        &msm_scalars,
        &msm_points,
        #[cfg(feature = "hints")]
        hints,
    )
    .is_none()
}

/// C FFI for batch verification where all signatures share the same message.
/// `pk_xs`: `count * 32` contiguous bytes. `sigs`: `count * 64` contiguous bytes (r||s per sig).
/// For per-message batching, call `secp256k1_schnorr_batch_verify` from Rust.
/// Returns 0 on success, 1 on failure.
///
/// # Safety
/// `pk_xs` must point to `count * 32` bytes, `sigs` to `count * 64` bytes.
/// `msg` must point to `msg_len` bytes, or may be null if `msg_len == 0`.
#[inline]
#[allow(dead_code)]
pub(crate) unsafe fn secp256k1_schnorr_batch_verify_c(
    count: usize,
    msg: *const u8,
    msg_len: usize,
    pk_xs: *const u8,
    sigs: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let msg_bytes: &[u8] =
        if msg_len == 0 { &[] } else { core::slice::from_raw_parts(msg, msg_len) };

    let mut msgs_refs = Vec::with_capacity(count);
    let mut pk_refs = Vec::with_capacity(count);
    let mut r_refs = Vec::with_capacity(count);
    let mut s_refs = Vec::with_capacity(count);
    let mut pk_bufs = Vec::with_capacity(count);
    let mut r_bufs = Vec::with_capacity(count);
    let mut s_bufs = Vec::with_capacity(count);

    for i in 0..count {
        let pk_slice = core::slice::from_raw_parts(pk_xs.add(i * 32), 32);
        let sig_slice = core::slice::from_raw_parts(sigs.add(i * 64), 64);
        let mut pk_buf = [0u8; 32];
        let mut r_buf = [0u8; 32];
        let mut s_buf = [0u8; 32];
        pk_buf.copy_from_slice(pk_slice);
        r_buf.copy_from_slice(&sig_slice[..32]);
        s_buf.copy_from_slice(&sig_slice[32..]);
        pk_bufs.push(pk_buf);
        r_bufs.push(r_buf);
        s_bufs.push(s_buf);
    }

    for i in 0..count {
        msgs_refs.push(msg_bytes as &[u8]);
        pk_refs.push(&pk_bufs[i]);
        r_refs.push(&r_bufs[i]);
        s_refs.push(&s_bufs[i]);
    }

    if secp256k1_schnorr_batch_verify(
        &msgs_refs,
        &pk_refs,
        &r_refs,
        &s_refs,
        #[cfg(feature = "hints")]
        hints,
    ) {
        0
    } else {
        1
    }
}
