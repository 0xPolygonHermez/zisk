//! BLS signature verification over BLS12-381 (IETF [draft-irtf-cfrg-bls-signature-06](https://www.ietf.org/archive/id/draft-irtf-cfrg-bls-signature-06.html)).
//!
//! Implements the minimal-pubkey-size **Basic** ciphersuite variant.
//!

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

use crate::zisklib::{lib::utils::eq, mul_fp12_bls12_381};

use super::{
    constants::{G1_GENERATOR, G1_IDENTITY},
    curve::{
        add_complete_safe_bls12_381, decompress_bls12_381, is_on_subgroup_bls12_381, neg_bls12_381,
    },
    hash_to_curve::hash_to_curve_g2_bls12_381,
    pairing::{pairing_bls12_381, pairing_check_safe_bls12_381},
    twist::{decompress_twist_bls12_381, is_on_subgroup_twist_bls12_381},
};

/// Domain separation tag for the basic scheme minimal-pubkey-size ciphersuite
pub const BLS_DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

/// Single BLS signature verification (CoreVerify, IETF draft §2.7).
pub fn bls_verify_bls12_381(
    pk_compressed: &[u8; 48],
    msg: &[u8],
    sig_compressed: &[u8; 96],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<bool, &'static str> {
    // Signature decompression and validation
    let r = decompress_and_validate_sig(
        sig_compressed,
        #[cfg(feature = "hints")]
        hints,
    )?;

    // Public key decompression and validation
    let pk = decompress_and_validate_pk(
        pk_compressed,
        #[cfg(feature = "hints")]
        hints,
    )?;

    // Hash the message to a curve point in G2
    let q = hash_to_curve_g2_bls12_381(
        msg,
        BLS_DST,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute e(PK, H(msg)) and e(G1, sig) and compare
    let c1 = pairing_bls12_381(
        &pk,
        &q,
        #[cfg(feature = "hints")]
        hints,
    );
    let c2 = pairing_bls12_381(
        &G1_GENERATOR,
        &r,
        #[cfg(feature = "hints")]
        hints,
    );

    if eq(&c1, &c2) {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Aggregate verification, Basic scheme (AggregateVerify, IETF draft §3.1.1).
pub fn bls_aggregate_verify_bls12_381(
    pks_compressed: &[[u8; 48]],
    msgs: &[&[u8]],
    sig_compressed: &[u8; 96],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<bool, &'static str> {
    // To avoid rogue key attacks, we ensure that all messages are distinct
    for i in 0..msgs.len() {
        for j in (i + 1)..msgs.len() {
            if msgs[i] == msgs[j] {
                return Err("Duplicate messages are not allowed in aggregate verification");
            }
        }
    }

    bls_core_aggregate_verify_bls12_381(
        pks_compressed,
        msgs,
        sig_compressed,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Aggregate verification (CoreAggregateVerify, IETF draft §2.9).
/// Verifies one aggregate signature against `n` (PK, msg) pairs.
pub fn bls_core_aggregate_verify_bls12_381(
    pks_compressed: &[[u8; 48]],
    msgs: &[&[u8]],
    sig_compressed: &[u8; 96],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<bool, &'static str> {
    // Preconditions
    if pks_compressed.is_empty() {
        return Err("Empty input");
    }
    if pks_compressed.len() != msgs.len() {
        return Err("Length mismatch between public keys and messages");
    }

    // Signature decompression and validation
    let r = decompress_and_validate_sig(
        sig_compressed,
        #[cfg(feature = "hints")]
        hints,
    )?;

    // Group the n input messages into l distinct messages
    // Aggregate the pks of the same message to l sets of pks
    // Compute the pairings and accumulate the result in c1
    let n = pks_compressed.len();
    let mut c1: [u64; 72] = {
        let mut tmp = [0u64; 72];
        tmp[0] = 1; // Identity element of Fp12
        tmp
    };
    for i in 0..n {
        // Find a non-seen message
        let mut seen_earlier = false;
        for k in 0..i {
            if msgs[i] == msgs[k] {
                seen_earlier = true;
                break;
            }
        }
        if seen_earlier {
            continue;
        }

        // Public key decompression and validation
        let mut aggregate = decompress_and_validate_pk(
            &pks_compressed[i],
            #[cfg(feature = "hints")]
            hints,
        )?;

        // Scan for duplicate messages and aggregate their pks
        let mut group_size: usize = 1;
        for j in (i + 1)..n {
            if msgs[j] == msgs[i] {
                // Public key decompression and validation
                let next = decompress_and_validate_pk(
                    &pks_compressed[j],
                    #[cfg(feature = "hints")]
                    hints,
                )?;

                // Aggregate the pk
                aggregate = add_complete_safe_bls12_381(
                    &aggregate,
                    &next,
                    #[cfg(feature = "hints")]
                    hints,
                )
                .map_err(|_| "Error during public key aggregation")?;
                group_size += 1;
            }
        }

        // If the group has more than one PK, check that the aggregate is not the
        // point at infinity
        if group_size > 1 && eq(&aggregate, &G1_IDENTITY) {
            return Err("Aggregate public key is the point at infinity");
        }

        // Hash the message to a curve point in G2
        let q = hash_to_curve_g2_bls12_381(
            msgs[i],
            BLS_DST,
            #[cfg(feature = "hints")]
            hints,
        );

        // Compute the pairing:
        //      e(agg_i, H(m_i)) = e(Σ_{k ∈ group_i} pk_k, H(m_i)) = ∏_{k ∈ group_i} e(pk_k, H(m_i))
        // and accumulate the result in c1
        let pairing_result = pairing_bls12_381(
            &aggregate,
            &q,
            #[cfg(feature = "hints")]
            hints,
        );
        c1 = mul_fp12_bls12_381(
            &c1,
            &pairing_result,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // Compute e(G1, sig)
    let c2 = pairing_bls12_381(
        &G1_GENERATOR,
        &r,
        #[cfg(feature = "hints")]
        hints,
    );

    if eq(&c1, &c2) {
        Ok(true)
    } else {
        Ok(false)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decompresses a signature and checks that it belongs to the correct subgroup.
fn decompress_and_validate_sig(
    sig_compressed: &[u8; 96],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 24], &'static str> {
    // Decompress the signature
    let (sig, is_inf) = decompress_twist_bls12_381(
        sig_compressed,
        #[cfg(feature = "hints")]
        hints,
    )
    .map_err(|_| "Failed to decompress signature")?;

    // Check that it belongs to the correct subgroup
    if !is_inf
        && !is_on_subgroup_twist_bls12_381(
            &sig,
            #[cfg(feature = "hints")]
            hints,
        )
    {
        return Err("Signature is not in the correct subgroup");
    }

    Ok(sig)
}

/// Decompresses a public key and runs KeyValidate: rejects identity and points
/// outside the prime-order G1 subgroup.
fn decompress_and_validate_pk(
    pk_compressed: &[u8; 48],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 12], &'static str> {
    // Decompress the public key
    let (pk, pk_is_inf) = decompress_bls12_381(
        pk_compressed,
        #[cfg(feature = "hints")]
        hints,
    )
    .map_err(|_| "Failed to decompress public key")?;

    // Reject the point at infinity
    if pk_is_inf {
        return Err("Public key is the point at infinity");
    }

    // Check that it belongs to the correct subgroup
    if !is_on_subgroup_bls12_381(
        &pk,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return Err("Public key is not in the correct subgroup");
    }

    Ok(pk)
}
