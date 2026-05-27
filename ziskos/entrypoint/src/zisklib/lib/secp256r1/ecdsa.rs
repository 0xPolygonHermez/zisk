use crate::zisklib::{be_bytes_to_u64_4, eq, gt, is_zero};

use super::{
    constants::{IDENTITY, N_MINUS_ONE, P_MINUS_ONE},
    curve::{double_scalar_mul_with_g_secp256r1, is_on_curve_secp256r1},
    scalar::{inv_fn_secp256r1, mul_fn_secp256r1, reduce_fn_secp256r1},
};

/// Verifies the signature (r, s) over the message hash z using the public key pk.
/// Returns true if the signature is valid, false otherwise.
pub fn ecdsa_verify_secp256r1(
    pk: &[u64; 8],
    z: &[u64; 4],
    r: &[u64; 4],
    s: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // Range checks: r, s ∈ [1, n−1].
    if is_zero(r) || gt(r, &N_MINUS_ONE) {
        return false;
    }
    if is_zero(s) || gt(s, &N_MINUS_ONE) {
        return false;
    }

    // pk must not be the identity point.
    if eq(pk, &IDENTITY) {
        return false;
    }

    // pk must be a valid curve point with both coordinates in [0, p−1].
    let pk_x: [u64; 4] = [pk[0], pk[1], pk[2], pk[3]];
    let pk_y: [u64; 4] = [pk[4], pk[5], pk[6], pk[7]];
    if gt(&pk_x, &P_MINUS_ONE) || gt(&pk_y, &P_MINUS_ONE) {
        return false;
    }
    if !is_on_curve_secp256r1(
        pk,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return false;
    }

    // Compute u1 = z·s⁻¹ (mod n) and u2 = r·s⁻¹ (mod n).
    let s_inv = inv_fn_secp256r1(
        s,
        #[cfg(feature = "hints")]
        hints,
    );
    let u1 = mul_fn_secp256r1(
        z,
        &s_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let u2 = mul_fn_secp256r1(
        r,
        &s_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute (x, y) = [u1]·G + [u2]·PK.
    let point = match double_scalar_mul_with_g_secp256r1(
        &u1,
        &u2,
        pk,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(pt) => pt,
        None => return false, // Result is the point at infinity ⇒ invalid signature.
    };

    // Check that x ≡ r (mod n). Fast path: when x < n, x == r directly.
    let x: [u64; 4] = [point[0], point[1], point[2], point[3]];
    eq(&x, r)
        || eq(
            &reduce_fn_secp256r1(
                &x,
                #[cfg(feature = "hints")]
                hints,
            ),
            r,
        )
}

// ==================== C FFI Functions ====================

/// # Safety
/// - `msg_ptr` must point to 4 u64s
/// - `sig_ptr` must point to 8 u64s
/// - `pk_ptr` must point to 8 u64s
///
/// Returns true if signature is valid
#[inline]
pub(crate) unsafe fn secp256r1_ecdsa_verify_c(
    msg: *const u8,
    sig: *const u8,
    pk: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let msg_bytes: &[u8; 32] = &*(msg as *const [u8; 32]);
    let sig_bytes: &[u8; 64] = &*(sig as *const [u8; 64]);
    let pk_bytes: &[u8; 64] = &*(pk as *const [u8; 64]);

    // Parse r, s from big-endian bytes
    let r_bytes: [u8; 32] = sig_bytes[0..32].try_into().unwrap();
    let s_bytes: [u8; 32] = sig_bytes[32..64].try_into().unwrap();

    // Parse pk_x, pk_y from big-endian bytes
    let pk_x_bytes: [u8; 32] = pk_bytes[0..32].try_into().unwrap();
    let pk_y_bytes: [u8; 32] = pk_bytes[32..64].try_into().unwrap();

    // Convert to little-endian u64 limbs
    let z = be_bytes_to_u64_4(msg_bytes);
    let r = be_bytes_to_u64_4(&r_bytes);
    let s = be_bytes_to_u64_4(&s_bytes);
    let pk_x = be_bytes_to_u64_4(&pk_x_bytes);
    let pk_y = be_bytes_to_u64_4(&pk_y_bytes);

    let pk: [u64; 8] = [pk_x[0], pk_x[1], pk_x[2], pk_x[3], pk_y[0], pk_y[1], pk_y[2], pk_y[3]];
    ecdsa_verify_secp256r1(
        &pk,
        &z,
        &r,
        &s,
        #[cfg(feature = "hints")]
        hints,
    )
}
