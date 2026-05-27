use crate::zisklib::{
    be_bytes_to_u64_4, eq, glv_double_scalar_mul_with_g_secp256k1, gt, u64_4_to_be_bytes, ZERO_256,
};

use super::{
    constants::N_MINUS_ONE,
    curve::{is_on_curve_secp256k1, lift_x_secp256k1},
    scalar::{inv_fn_secp256k1, mul_fn_secp256k1, neg_fn_secp256k1, reduce_fn_secp256k1},
};

/// ECDSA recover result codes
pub const ECDSA_RECOVER_SUCCESS: u8 = 0;
pub const ECDSA_RECOVER_ERR_INVALID_R: u8 = 1;
pub const ECDSA_RECOVER_ERR_INVALID_S: u8 = 2;
pub const ECDSA_RECOVER_ERR_INVALID_RECID: u8 = 3;
pub const ECDSA_RECOVER_ERR_POINT_NOT_ON_CURVE: u8 = 4;
pub const ECDSA_RECOVER_ERR_RECOVERY_FAILED: u8 = 5;

/// Verifies the signature (r, s) over the message hash z using the public key pk
pub fn ecdsa_verify_secp256k1(
    pk: &[u64; 8],
    z: &[u64; 4],
    r: &[u64; 4],
    s: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // pk must be on the curve
    if !is_on_curve_secp256k1(
        pk,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return false;
    }

    // Compute u1 = z·s⁻¹ (mod n) and u2 = r·s⁻¹ (mod n).
    let s_inv = inv_fn_secp256k1(
        s,
        #[cfg(feature = "hints")]
        hints,
    );
    let u1 = mul_fn_secp256k1(
        z,
        &s_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let u2 = mul_fn_secp256k1(
        r,
        &s_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute (x, y) = [u1]G + [u2]PK.
    let point = match glv_double_scalar_mul_with_g_secp256k1(
        &u1,
        &u2,
        pk,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(pt) => pt,
        None => return false, // Result is the point at infinity ⇒ invalid signature.
    };

    // Check that x = r (mod n). Fast path: x < n, so x == r directly.
    let x: [u64; 4] = [point[0], point[1], point[2], point[3]];
    eq(&x, r)
        || eq(
            &reduce_fn_secp256k1(
                &x,
                #[cfg(feature = "hints")]
                hints,
            ),
            r,
        )
}

/// Recover the public key point from an ECDSA signature (r, s) over the message hash z and recovery id recid
///
/// # Returns
/// - 0 = success
/// - 1 = invalid r (not in [1, N))
/// - 2 = invalid s (not in [1, N))
/// - 3 = invalid recid (not 0 or 1)
/// - 4 = point not on curve
/// - 5 = recovery failed
pub fn ecdsa_recover_secp256k1(
    r: &[u64; 4],
    s: &[u64; 4],
    z: &[u64; 4],
    recid: u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 8], u8> {
    // Validate r
    if *r == ZERO_256 || gt(r, &N_MINUS_ONE) {
        return Err(ECDSA_RECOVER_ERR_INVALID_R);
    }

    // Validate s
    if *s == ZERO_256 || gt(s, &N_MINUS_ONE) {
        return Err(ECDSA_RECOVER_ERR_INVALID_S);
    }

    // Validate recid
    if recid > 1 {
        return Err(ECDSA_RECOVER_ERR_INVALID_RECID);
    }

    // Lift R from its x-coordinate (= r) and the parity bit (= recid & 1).
    let y_is_odd = (recid & 1) == 1;
    let r_point = lift_x_secp256k1(
        r,
        y_is_odd,
        #[cfg(feature = "hints")]
        hints,
    )
    .map_err(|_| ECDSA_RECOVER_ERR_POINT_NOT_ON_CURVE)?;

    // Compute u1, u2 such that u1 = -z·r⁻¹ (mod n), u2 = s·r⁻¹ (mod n).
    let r_inv = inv_fn_secp256k1(
        r,
        #[cfg(feature = "hints")]
        hints,
    );
    let neg_z = neg_fn_secp256k1(
        z,
        #[cfg(feature = "hints")]
        hints,
    );
    let u1 = mul_fn_secp256k1(
        &neg_z,
        &r_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let u2 = mul_fn_secp256k1(
        s,
        &r_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // PK = [u1]G + [u2]R.
    match glv_double_scalar_mul_with_g_secp256k1(
        &u1,
        &u2,
        &r_point,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(pk) => Ok(pk),
        None => Err(ECDSA_RECOVER_ERR_RECOVERY_FAILED), // Recovered point is identity
    }
}

// ==================== C FFI Functions ====================

/// ECDSA signature verification with little-endian u64 limb inputs.
/// Returns 1 if the signature is valid, 0 otherwise.
///
/// # Safety
/// - `pk_ptr` must point to a valid `[u64; 8]` array (public key x ‖ y, little-endian limbs)
/// - `z_ptr` must point to a valid `[u64; 4]` array (message hash, little-endian limbs)
/// - `r_ptr` must point to a valid `[u64; 4]` array (signature r, little-endian limbs)
/// - `s_ptr` must point to a valid `[u64; 4]` array (signature s, little-endian limbs)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_ecdsa_verify_c")]
pub unsafe extern "C" fn ecdsa_verify_secp256k1_c(
    pk_ptr: *const u64,
    z_ptr: *const u64,
    r_ptr: *const u64,
    s_ptr: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let pk = &*(pk_ptr as *const [u64; 8]);
    let z = &*(z_ptr as *const [u64; 4]);
    let r = &*(r_ptr as *const [u64; 4]);
    let s = &*(s_ptr as *const [u64; 4]);
    ecdsa_verify_secp256k1(
        pk,
        z,
        r,
        s,
        #[cfg(feature = "hints")]
        hints,
    ) as u8
}

/// ECDSA signature verification with big-endian byte inputs.
///
/// # Safety
/// - `sig` must point to at least 64 bytes (r || s, big-endian)
/// - `msg` must point to at least 32 bytes (message hash, big-endian)
/// - `pk` must point to at least 64 bytes (x || y, big-endian)
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn secp256k1_ecdsa_verify_bytes_c(
    sig: *const u8,
    msg: *const u8,
    pk: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let sig_bytes: &[u8; 64] = &*(sig as *const [u8; 64]);
    let msg_bytes: &[u8; 32] = &*(msg as *const [u8; 32]);
    let pk_bytes: &[u8; 64] = &*(pk as *const [u8; 64]);

    // Parse r, s from big-endian bytes
    let r_bytes: [u8; 32] = sig_bytes[0..32].try_into().unwrap();
    let s_bytes: [u8; 32] = sig_bytes[32..64].try_into().unwrap();

    // Parse pk_x, pk_y from big-endian bytes
    let pk_x_bytes: [u8; 32] = pk_bytes[0..32].try_into().unwrap();
    let pk_y_bytes: [u8; 32] = pk_bytes[32..64].try_into().unwrap();

    // Convert to little-endian u64 limbs
    let r = be_bytes_to_u64_4(&r_bytes);
    let s = be_bytes_to_u64_4(&s_bytes);
    let z = be_bytes_to_u64_4(msg_bytes);
    let pk_x = be_bytes_to_u64_4(&pk_x_bytes);
    let pk_y = be_bytes_to_u64_4(&pk_y_bytes);

    let pk_arr: [u64; 8] = [pk_x[0], pk_x[1], pk_x[2], pk_x[3], pk_y[0], pk_y[1], pk_y[2], pk_y[3]];
    ecdsa_verify_secp256k1(
        &pk_arr,
        &z,
        &r,
        &s,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// C-compatible wrapper for ecdsa_recover_secp256k1
///
/// # Safety
/// - `sig` must point to at least 64 bytes (r || s, big-endian)
/// - `msg` must point to at least 32 bytes (message hash, big-endian)
/// - `output` must point to a writable buffer of 64 bytes
///
/// # Arguments
/// - `sig` - 64 bytes: r (32 bytes) || s (32 bytes), big-endian
/// - `recid` - Recovery ID (0 or 1)
/// - `msg` - 32 bytes message hash, big-endian
/// - `output` - Output buffer for the recovered public key (64 bytes)
///
/// # Returns
/// - `Ok([u64; 8])` - Recovered pubkey if recovery is successful
/// - `Err(u8)` - Error code
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn secp256k1_ecdsa_recover_c(
    sig: *const u8,
    recid: u8,
    msg: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let sig_bytes: &[u8; 64] = &*(sig as *const [u8; 64]);
    let msg_bytes: &[u8; 32] = &*(msg as *const [u8; 32]);
    let output_bytes: &mut [u8; 64] = &mut *(output as *mut [u8; 64]);

    // Parse r, s, z from big-endian bytes
    let r_bytes: [u8; 32] = sig_bytes[0..32].try_into().unwrap();
    let s_bytes: [u8; 32] = sig_bytes[32..64].try_into().unwrap();

    let r = be_bytes_to_u64_4(&r_bytes);
    let s = be_bytes_to_u64_4(&s_bytes);
    let z = be_bytes_to_u64_4(msg_bytes);

    // Perform ecrecover
    match ecdsa_recover_secp256k1(
        &r,
        &s,
        &z,
        recid,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(pk) => {
            // pk is [u64; 8]: x in limbs [0..4] and y in limbs [4..8], little-endian
            let x = [pk[0], pk[1], pk[2], pk[3]];
            let y = [pk[4], pk[5], pk[6], pk[7]];
            output_bytes[..32].copy_from_slice(&u64_4_to_be_bytes(&x));
            output_bytes[32..].copy_from_slice(&u64_4_to_be_bytes(&y));
            ECDSA_RECOVER_SUCCESS
        }
        Err(code) => code,
    }
}
