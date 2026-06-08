//! BIP-340 Schnorr signature verification for secp256k1.
//! https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki

extern crate alloc;
use alloc::vec::Vec;

use crate::zisklib::{be_bytes_to_u64_4, eq, is_zero, lt, sha256, ZERO_256};

use super::{
    constants::{G, N, P},
    curve::{lift_x_secp256k1, multi_scalar_mul_secp256k1},
    glv::{glv_double_scalar_mul_with_g_secp256k1, glv_multi_scalar_mul_secp256k1},
    scalar::{add_fn_secp256k1, mul_fn_secp256k1, neg_fn_secp256k1, reduce_fn_secp256k1},
};

/// Batch size threshold above which the GLV expansion's per-scalar overhead exceeds the
/// savings it gives to Pippenger. Below this, GLV halves the bit budget and wins on the MSM;
/// above this, the non-GLV path with a wider Pippenger window is cheaper.
const SCHNORR_BATCH_GLV_THRESHOLD: usize = 512;

/// SHA256("BIP0340/challenge").
const TAG_CHALLENGE: [u8; 32] = [
    0x7B, 0xB5, 0x2D, 0x7A, 0x9F, 0xEF, 0x58, 0x32, 0x3E, 0xB1, 0xBF, 0x7A, 0x40, 0x7D, 0xB3, 0x82,
    0xD2, 0xF3, 0xF2, 0xD8, 0x1B, 0xB1, 0x22, 0x4F, 0x49, 0xFE, 0x51, 0x8F, 0x6D, 0x48, 0xD3, 0x7C,
];

/// SHA256("BIP0340/batch").
const TAG_BATCH: [u8; 32] = [
    0x77, 0x06, 0x39, 0x59, 0x84, 0x1F, 0xFA, 0x7B, 0x06, 0x15, 0x4E, 0xE0, 0x47, 0x50, 0x19, 0x40,
    0x36, 0x48, 0x7A, 0xB8, 0x91, 0x96, 0xD0, 0x6E, 0xC7, 0x3E, 0x75, 0x82, 0x90, 0x98, 0x41, 0xB5,
];

/// Single Schnorr signature verification per BIP-340 `Verify(pk, m, sig)`.
/// Returns true if the signature is valid.
pub fn schnorr_verify_secp256k1(
    msg: &[u8],
    pk_x: &[u8; 32],
    sig_r: &[u8; 32],
    sig_s: &[u8; 32],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let r = be_bytes_to_u64_4(sig_r);
    let s = be_bytes_to_u64_4(sig_s);
    let pk_x_le = be_bytes_to_u64_4(pk_x);

    // Range checks: pk_x,r ∈ [0, P), s ∈ [0, N).
    if !lt(&pk_x_le, &P) {
        return false;
    }
    if !lt(&r, &P) {
        return false;
    }
    if !lt(&s, &N) {
        return false;
    }

    // Lift pk_x to the full curve point P (even y per BIP-340).
    let point_p = match lift_x_secp256k1(
        &pk_x_le,
        false,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(pt) => pt,
        Err(_) => return false,
    };

    // Challenge:
    //      e = int(SHA(tag || tag || r || pk || msg)) mod n
    // where tag = SHA("BIP0340/challenge")
    let mut buf = Vec::with_capacity(64 + 32 + 32 + msg.len());
    buf.extend_from_slice(&TAG_CHALLENGE);
    buf.extend_from_slice(&TAG_CHALLENGE);
    buf.extend_from_slice(sig_r);
    buf.extend_from_slice(pk_x);
    buf.extend_from_slice(msg);

    let e_hash = sha256(
        &buf,
        #[cfg(feature = "hints")]
        hints,
    );
    let e = reduce_fn_secp256k1(
        &be_bytes_to_u64_4(&e_hash),
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute R = s·G + (-e)·P and verify R's x-coordinate and parity against r.
    let neg_e = neg_fn_secp256k1(
        &e,
        #[cfg(feature = "hints")]
        hints,
    );
    let point_r = match glv_double_scalar_mul_with_g_secp256k1(
        &s,
        &neg_e,
        &point_p,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(pt) => pt,
        None => return false,
    };

    // Fail if R has odd y
    if point_r[4] & 1 != 0 {
        return false;
    }

    // Finally, check that R's x-coordinate matches r
    eq(&[point_r[0], point_r[1], point_r[2], point_r[3]], &r)
}

/// BIP-340 batch verification using the multi-scalar multiplication approach.
/// Verifies multiple Schnorr signatures in a single Pippenger MSM.
///
/// Returns true if all signatures are valid.
pub fn schnorr_batch_verify_secp256k1(
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
    debug_assert_eq!(pk_xs.len(), u);
    debug_assert_eq!(sig_rs.len(), u);
    debug_assert_eq!(sig_ss.len(), u);

    // For u=1, delegate to single verification
    if u == 1 {
        return schnorr_verify_secp256k1(
            msgs[0],
            pk_xs[0],
            sig_rs[0],
            sig_ss[0],
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // Parse + range-check each tuple (pk_x, r ∈ [0, P), s ∈ [0, N)).
    let mut pk_vals = Vec::with_capacity(u);
    let mut r_vals = Vec::with_capacity(u);
    let mut s_vals = Vec::with_capacity(u);
    for i in 0..u {
        let pk = be_bytes_to_u64_4(pk_xs[i]);
        let r = be_bytes_to_u64_4(sig_rs[i]);
        let s = be_bytes_to_u64_4(sig_ss[i]);

        if !lt(&pk, &P) {
            return false;
        }
        if !lt(&r, &P) {
            return false;
        }
        if !lt(&s, &N) {
            return false;
        }

        pk_vals.push(pk);
        r_vals.push(r);
        s_vals.push(s);
    }

    // Lift each pk_x and r to a full curve point.
    let mut points_p = Vec::with_capacity(u);
    let mut points_r = Vec::with_capacity(u);
    for i in 0..u {
        let point_p = match lift_x_secp256k1(
            &pk_vals[i],
            false,
            #[cfg(feature = "hints")]
            hints,
        ) {
            Ok(pt) => pt,
            Err(_) => return false,
        };

        let point_r = match lift_x_secp256k1(
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

    // Per-signature challenge:
    //      eᵢ = int(SHA(tag || tag || rᵢ || pkᵢ || msgᵢ)) mod n
    // where tag = SHA("BIP0340/challenge")
    let mut challenges = Vec::with_capacity(u);
    for i in 0..u {
        let mut buf = Vec::with_capacity(64 + 32 + 32 + msgs[i].len());
        buf.extend_from_slice(&TAG_CHALLENGE);
        buf.extend_from_slice(&TAG_CHALLENGE);
        buf.extend_from_slice(sig_rs[i]);
        buf.extend_from_slice(pk_xs[i]);
        buf.extend_from_slice(msgs[i]);

        let e_hash = sha256(
            &buf,
            #[cfg(feature = "hints")]
            hints,
        );
        let e = reduce_fn_secp256k1(
            &be_bytes_to_u64_4(&e_hash),
            #[cfg(feature = "hints")]
            hints,
        );

        challenges.push(e);
    }

    // Now we generate n-1 random coefficients by iterative hashing:
    //      a₀ = 1
    //      aᵢ = int(SHA(seed || i)) mod n for i = 1..u
    // where seed = SHA(tag || tag || pk₀ || pk₁ || ... || msg₀ || msg₁ || ... || r₀ || s₀ || r₁ || s₁ || ...),
    // and tag = SHA("BIP0340/batch").
    // NOTE: BIP-340 doesn't specify how to generate the coefficients, but they must be derived from all inputs
    //       in a deterministic way. This approach is simple and efficient.

    // Compute the seed
    let mut seed_buf = Vec::new();
    seed_buf.extend_from_slice(&TAG_BATCH);
    seed_buf.extend_from_slice(&TAG_BATCH);
    for pk in pk_xs {
        seed_buf.extend_from_slice(*pk);
    }
    for msg in msgs {
        // Hash the length of the message as well to avoid boundary collisions, e.g.:
        //  msgs = [b"abc", b"defgh"] →  concat = b"abcdefgh"
        //  msgs = [b"abcd", b"efgh"] →  concat = b"abcdefgh"
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

    // Compute the random coefficients as:
    //      a₀ = 1
    //      aᵢ = int(SHA(seed || i)) mod n for i = 1..u
    let mut coeffs = Vec::with_capacity(u);
    coeffs.push([1u64, 0, 0, 0]); // a₀ = 1
    for i in 1..u {
        let mut coeff_buf = [0u8; 36];
        coeff_buf[..32].copy_from_slice(&seed);
        coeff_buf[32..36].copy_from_slice(&(i as u32).to_le_bytes());
        let hash = sha256(
            &coeff_buf,
            #[cfg(feature = "hints")]
            hints,
        );
        let a = reduce_fn_secp256k1(
            &be_bytes_to_u64_4(&hash),
            #[cfg(feature = "hints")]
            hints,
        );
        // With a 256-bit modulus, the probability of a = 0 is slightly less than 2⁻²⁵⁶.
        // We don't check it in release builds
        debug_assert!(!is_zero(&a));

        coeffs.push(a);
    }

    // MSM batch equation: (Σ aᵢ·sᵢ)·G = Σ (aᵢ)·Rᵢ + Σ (aᵢ·eᵢ)·Pᵢ <==> (Σ aᵢ·sᵢ)·G + Σ (-aᵢ)·Rᵢ + Σ ((-aᵢ)·eᵢ)·Pᵢ = O
    let mut s_total = s_vals[0];
    for i in 1..u {
        let ai_si = mul_fn_secp256k1(
            &coeffs[i],
            &s_vals[i],
            #[cfg(feature = "hints")]
            hints,
        );
        s_total = add_fn_secp256k1(
            &s_total,
            &ai_si,
            #[cfg(feature = "hints")]
            hints,
        );
    }
    debug_assert!(!is_zero(&s_total));

    let mut msm_scalars = Vec::with_capacity(2 * u + 1);
    let mut msm_points = Vec::with_capacity(2 * u + 1);
    msm_scalars.push(s_total);
    msm_points.push(G);
    for i in 0..u {
        let neg_ai = neg_fn_secp256k1(
            &coeffs[i],
            #[cfg(feature = "hints")]
            hints,
        );
        let neg_ai_ei = mul_fn_secp256k1(
            &neg_ai,
            &challenges[i],
            #[cfg(feature = "hints")]
            hints,
        );

        msm_scalars.push(neg_ai);
        msm_points.push(points_r[i]);

        debug_assert!(!is_zero(&neg_ai_ei));
        msm_scalars.push(neg_ai_ei);
        msm_points.push(points_p[i]);
    }

    // For small-to-moderate batches, GLV expansion halves the bit budget and pays for itself.
    // For very large batches the per-input GLV overhead (decompose + φ + sign-adjust) exceeds
    // the MSM saving, so the plain Pippenger over 256-bit scalars is cheaper.
    let result = if u <= SCHNORR_BATCH_GLV_THRESHOLD {
        glv_multi_scalar_mul_secp256k1(
            &msm_scalars,
            &msm_points,
            #[cfg(feature = "hints")]
            hints,
        )
    } else {
        multi_scalar_mul_secp256k1(
            &msm_scalars,
            &msm_points,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    result.is_none()
}

// ==================== C FFI Functions ====================

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
    if schnorr_verify_secp256k1(
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

/// C FFI for batch verification where all signatures share the same message.
/// `pk_xs`: `count * 32` contiguous bytes. `sigs`: `count * 64` contiguous bytes (r||s per sig).
/// For per-message batching, call `schnorr_batch_verify_secp256k1` from Rust.
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

    if schnorr_batch_verify_secp256k1(
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
