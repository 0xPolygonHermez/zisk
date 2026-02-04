use crate::{
    syscalls::{
        syscall_secp256k1_add, syscall_secp256k1_dbl, SyscallPoint256, SyscallSecp256k1AddParams,
    },
    zisklib::{
        eq, fcall_msb_pos_256, fcall_secp256k1_ecdsa_verify, is_one, ONE_256, TWO_256, ZERO_256,
    },
};

use super::{
    constants::{E_B, G, G_X, G_Y, IDENTITY_X, IDENTITY_Y},
    curve::{
        secp256k1_decompress, secp256k1_double_scalar_mul_with_g, secp256k1_is_on_curve,
        secp256k1_triple_scalar_mul_with_g,
    },
    field::{
        secp256k1_fp_add, secp256k1_fp_inv, secp256k1_fp_mul, secp256k1_fp_sqrt,
        secp256k1_fp_square,
    },
    scalar::{
        secp256k1_fn_inv, secp256k1_fn_mul, secp256k1_fn_neg, secp256k1_fn_reduce, secp256k1_fn_sub,
    },
};

/// Verifies the signature (r, s) over the message hash z using the public key pk
/// Returns true if the signature is valid, false otherwise
pub fn secp256k1_ecdsa_verify(
    pk: &[u64; 8],
    z: &[u64; 4],
    r: &[u64; 4],
    s: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // Ecdsa verification computes (x, y) = [s⁻¹·z (mod n)]G + [s⁻¹·r (mod n)]PK
    // and checks that x ≡ r (mod n)
    // We can equivalently hint y, and verify that
    //   [z]G + [r]PK + [-s](r,y) == 𝒪,
    // saving us from expensive fn arithmetic

    // Hint the result
    let coords = fcall_secp256k1_ecdsa_verify(
        pk,
        z,
        r,
        s,
        #[cfg(feature = "hints")]
        hints,
    );
    let point = [r[0], r[1], r[2], r[3], coords[4], coords[5], coords[6], coords[7]];

    // Check the recovered point is valid
    assert!(secp256k1_is_on_curve(
        &point,
        #[cfg(feature = "hints")]
        hints,
    )); // Note: Identity point would be raised here

    // Check that [z]G + [r]PK + [-s](r,y) == 𝒪
    let neg_s = secp256k1_fn_neg(
        s,
        #[cfg(feature = "hints")]
        hints,
    );
    secp256k1_triple_scalar_mul_with_g(
        z,
        r,
        &neg_s,
        pk,
        &point,
        #[cfg(feature = "hints")]
        hints,
    )
    .is_none()
}

// ==================== C FFI Functions ====================

/// # Safety
/// - `pk_ptr` must point to 8 u64s
/// - `z_ptr`, `r_ptr`, `s_ptr` must point to 4 u64s each
///
/// Returns true if signature is valid
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_secp256k1_ecdsa_verify_c")]
pub unsafe extern "C" fn secp256k1_ecdsa_verify_c(
    pk_ptr: *const u64,
    z_ptr: *const u64,
    r_ptr: *const u64,
    s_ptr: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let pk: &[u64; 8] = &*(pk_ptr as *const [u64; 8]);
    let z: &[u64; 4] = &*(z_ptr as *const [u64; 4]);
    let r: &[u64; 4] = &*(r_ptr as *const [u64; 4]);
    let s: &[u64; 4] = &*(s_ptr as *const [u64; 4]);
    secp256k1_ecdsa_verify(
        pk,
        z,
        r,
        s,
        #[cfg(feature = "hints")]
        hints,
    )
}
