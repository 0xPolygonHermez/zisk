use crate::{
    syscalls::{
        syscall_secp256k1_add, syscall_secp256k1_dbl, SyscallPoint256, SyscallSecp256k1AddParams,
    },
    zisklib::{
        eq, fcall_msb_pos_256, fcall_secp256k1_ecdsa_verify, is_one, lt, ONE_256, TWO_256, ZERO_256,
    },
};

use super::{
    constants::{E_B, G, G_X, G_Y, IDENTITY_X, IDENTITY_Y, N, N_HALF_PLUS_ONE, P},
    curve::{
        secp256k1_decompress, secp256k1_double_scalar_mul_with_g, secp256k1_is_on_curve,
        secp256k1_scalar_mul, secp256k1_triple_scalar_mul_with_g,
    },
    field::{
        secp256k1_fp_add, secp256k1_fp_inv, secp256k1_fp_mul, secp256k1_fp_sqrt,
        secp256k1_fp_square,
    },
    scalar::{
        secp256k1_fn_add, secp256k1_fn_inv, secp256k1_fn_mul, secp256k1_fn_neg,
        secp256k1_fn_reduce, secp256k1_fn_sub,
    },
};

use tiny_keccak::{Hasher, Keccak};

/// Ecrecover result codes
pub const ECRECOVER_SUCCESS: u8 = 0;
pub const ECRECOVER_ERR_INVALID_R: u8 = 1;
pub const ECRECOVER_ERR_INVALID_S: u8 = 2;
pub const ECRECOVER_ERR_INVALID_RECID: u8 = 3;
pub const ECRECOVER_ERR_POINT_NOT_ON_CURVE: u8 = 4;
pub const ECRECOVER_ERR_RECOVERY_FAILED: u8 = 5;

/// Recover the public key point from an ECDSA signature
///
/// The recovery formula is:
/// R = (r, y) where y is recovered from r using the curve equation
/// PK = r⁻¹ * (s*R - z*G) = u1*G + u2*R where u1 = -z*r⁻¹ and u2 = s*r⁻¹
///
/// # Arguments
/// * `r` - Signature r value as [u64; 4] little-endian
/// * `s` - Signature s value as [u64; 4] little-endian
/// * `z` - Message hash as [u64; 4] little-endian
/// * `recid` - Recovery ID (0, 1, 2, or 3)
///
/// # Returns
/// * `Ok([u64; 8])` - Recovered public key (x, y coordinates)
/// * `Err(u8)` - Error code
// TODO: Use triple scalar mul!
pub fn secp256k1_ecrecover_point(
    r: &[u64; 4],
    s: &[u64; 4],
    z: &[u64; 4],
    recid: u8,
    require_low_s: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 8], u8> {
    // Validate recid
    // The recid is a value in the range [0, 3]
    // However, the upper two values (2 and 3) are, representing infinity values, invalid
    if recid > 1 {
        return Err(ECRECOVER_ERR_INVALID_RECID);
    }

    // Validate r
    if !is_valid_r(r) {
        return Err(ECRECOVER_ERR_INVALID_R);
    }

    // Validate s
    if require_low_s {
        if !is_valid_s_low(s) {
            return Err(ECRECOVER_ERR_INVALID_S);
        }
    } else if !is_valid_s(s) {
        return Err(ECRECOVER_ERR_INVALID_S);
    }

    // Determine the x-coordinate of R
    let x = *r;

    // Recover the y-coordinate from x
    // y² = x³ + 7
    let y_is_odd = (recid & 1) == 1;
    let (rx, ry) = secp256k1_decompress(
        &x,
        y_is_odd,
        #[cfg(feature = "hints")]
        hints,
    )
    .map_err(|_| ECRECOVER_ERR_POINT_NOT_ON_CURVE)?;

    let r_point = [rx[0], rx[1], rx[2], rx[3], ry[0], ry[1], ry[2], ry[3]];

    // Compute r_inv = r⁻¹ (mod N)
    let r_inv = secp256k1_fn_inv(
        r,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute u1 = -z * r_inv (mod N)
    let neg_z = secp256k1_fn_neg(
        z,
        #[cfg(feature = "hints")]
        hints,
    );
    let u1 = secp256k1_fn_mul(
        &neg_z,
        &r_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute u2 = s * r_inv (mod N)
    let u2 = secp256k1_fn_mul(
        s,
        &r_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute PK = u1*G + u2*R
    let pk = secp256k1_double_scalar_mul_with_g(
        &u1,
        &u2,
        &r_point,
        #[cfg(feature = "hints")]
        hints,
    )
    .ok_or(ECRECOVER_ERR_RECOVERY_FAILED)?;

    Ok(pk)
}

/// Recover the Ethereum address from an ECDSA signature (for precompile)
///
/// # Arguments
/// * `r` - Signature r value as [u64; 4] little-endian
/// * `s` - Signature s value as [u64; 4] little-endian
/// * `z` - Message hash as [u64; 4] little-endian
/// * `recid` - Recovery ID (0 or 1)
///
/// # Returns
/// * `Ok([u8; 32])` - First 12 bytes are 0, last 20 bytes are the Ethereum address
/// * `Err(u8)` - Error code
pub fn secp256k1_ecrecover(
    r: &[u64; 4],
    s: &[u64; 4],
    z: &[u64; 4],
    recid: u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u8; 32], u8> {
    // Recover the public key point
    let pk = secp256k1_ecrecover_point(
        r,
        s,
        z,
        recid,
        false,
        #[cfg(feature = "hints")]
        hints,
    )?;

    Ok(pubkey_to_address(&pk))
}

/// Recover the Ethereum address from an ECDSA signature (for tx recovery with low S)
///
/// # Arguments
/// * `r` - Signature r value as [u64; 4] little-endian
/// * `s` - Signature s value as [u64; 4] little-endian
/// * `z` - Message hash as [u64; 4] little-endian
/// * `recid` - Recovery ID (0 or 1)
///
/// # Returns
/// * `Ok([u8; 32])` - First 12 bytes are 0, last 20 bytes are the Ethereum address
/// * `Err(u8)` - Error code
pub fn secp256k1_ecrecover_tx(
    r: &[u64; 4],
    s: &[u64; 4],
    z: &[u64; 4],
    recid: u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u8; 32], u8> {
    // Recover the public key point
    let pk = secp256k1_ecrecover_point(
        r,
        s,
        z,
        recid,
        true,
        #[cfg(feature = "hints")]
        hints,
    )?;

    Ok(pubkey_to_address(&pk))
}

/// Convert a public key point to an Ethereum address
fn pubkey_to_address(pk: &[u64; 8]) -> [u8; 32] {
    let x = [pk[0], pk[1], pk[2], pk[3]];
    let y = [pk[4], pk[5], pk[6], pk[7]];

    let x_bytes = u64_le_to_bytes_be(&x);
    let y_bytes = u64_le_to_bytes_be(&y);

    // Concatenate x and y for hashing (64 bytes)
    let mut pk_bytes = [0u8; 64];
    pk_bytes[0..32].copy_from_slice(&x_bytes);
    pk_bytes[32..64].copy_from_slice(&y_bytes);

    // Hash with keccak256
    let mut hasher = Keccak::v256();
    hasher.update(&pk_bytes);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // Return with first 12 bytes zeroed (Ethereum address is last 20 bytes)
    let mut result = [0u8; 32];
    result[12..32].copy_from_slice(&hash[12..32]);

    result
}

/// C-compatible wrapper for secp256k1_ecrecover
///
/// # Safety
/// - `sig` must point to at least 64 bytes (r || s, big-endian)
/// - `msg` must point to at least 32 bytes (message hash, big-endian)
/// - `output` must point to a writable buffer of at least 32 bytes
///
/// # Arguments
/// - `sig` - 64 bytes: r (32 bytes) || s (32 bytes), big-endian
/// - `recid` - Recovery ID (0 or 1)
/// - `msg` - 32 bytes message hash, big-endian
/// - `output` - Output buffer for the recovered address (32 bytes)
/// - `require_low_s` - If true, require s <= N/2 (for tx recovery); if false, allow s < N (for precompile)
///
/// # Returns
/// - 0 = success
/// - 1 = invalid r (not in [1, N))
/// - 2 = invalid s (not in [1, N) or [1, N/2] depending on require_low_s)
/// - 3 = invalid recid (not 0 or 1)
/// - 4 = point not on curve (no valid y for given r)
/// - 5 = recovery failed (result is point at infinity)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_ecrecover_c")]
pub unsafe extern "C" fn secp256k1_ecrecover_c(
    sig: *const u8,
    recid: u8,
    msg: *const u8,
    output: *mut u8,
    require_low_s: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let sig_bytes: &[u8; 64] = &*(sig as *const [u8; 64]);
    let msg_bytes: &[u8; 32] = &*(msg as *const [u8; 32]);
    let output_bytes: &mut [u8; 32] = &mut *(output as *mut [u8; 32]);

    // Parse r, s, z from big-endian bytes
    let r_bytes: [u8; 32] = sig_bytes[0..32].try_into().unwrap();
    let s_bytes: [u8; 32] = sig_bytes[32..64].try_into().unwrap();

    let r = bytes_be_to_u64_le(&r_bytes);
    let s = bytes_be_to_u64_le(&s_bytes);
    let z = bytes_be_to_u64_le(msg_bytes);

    // Perform ecrecover
    match secp256k1_ecrecover_point(
        &r,
        &s,
        &z,
        recid,
        require_low_s,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(pk) => {
            let result = pubkey_to_address(&pk);
            output_bytes.copy_from_slice(&result);
            ECRECOVER_SUCCESS
        }
        Err(code) => {
            output_bytes.fill(0);
            code
        }
    }
}

/// Convert big-endian bytes to little-endian u64 limbs (32 bytes -> [u64; 4])
fn bytes_be_to_u64_le(bytes: &[u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];
    for i in 0..4 {
        for j in 0..8 {
            result[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }
    result
}

/// Convert little-endian u64 limbs to big-endian bytes ([u64; 4] -> 32 bytes)
fn u64_le_to_bytes_be(limbs: &[u64; 4]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..4 {
        for j in 0..8 {
            result[i * 8 + j] = ((limbs[3 - i] >> (8 * (7 - j))) & 0xff) as u8;
        }
    }
    result
}

/// Check if r is valid: 0 < r < N
fn is_valid_r(r: &[u64; 4]) -> bool {
    if *r == ZERO_256 {
        return false;
    }
    if lt(r, &N) {
        return true;
    }
    false
}

/// Check if s is valid for precompile: 0 < s < N
fn is_valid_s(s: &[u64; 4]) -> bool {
    if *s == ZERO_256 {
        return false;
    }
    if lt(s, &N) {
        return true;
    }
    false
}

/// Check if s is valid for tx recovery (low S): 0 < s <= N/2
fn is_valid_s_low(s: &[u64; 4]) -> bool {
    if *s == ZERO_256 {
        return false;
    }
    if lt(s, &N_HALF_PLUS_ONE) {
        return true;
    }
    false
}
