/// Verify a ZisK proof against a caller-supplied expected program and setup key.
/// The vk appended to `zisk_proof` is ignored.
///
/// The hash family is read from the proof's trailing tag, so a guest needs no build-time
/// configuration to verify a proof from any family. The tag is untrusted, but it is not
/// authority either: it only selects which verifier runs, and a wrong choice fails against
/// `expected_setup_vk`, whose const-tree root is built under the real family's own hash.
///
/// Both keys MUST be guest compile-time constants, never read from program input,
/// or verification is self-keyed and authenticates nothing. Both are needed:
/// `expected_setup_vk` is the STARK key, shared by every ZisK program, so alone it
/// proves only well-formedness; `expected_program_vk` is the identity committed in
/// the publics (ROM root for a leaf, recursion domain for an aggregate) and pins
/// *which* program ran. For an aggregate both are the recurser's own verkey, and
/// pinning them together is what makes its leaf allow-list transitive -- an
/// aggregated child is not allow-list checked in-circuit, so a proof from that
/// recurser can otherwise carry a subtree folded by a different one.
pub fn verify_zisk_proof(
    zisk_proof: &[u64],
    expected_setup_vk: &[u64],
    expected_program_vk: &[u64],
) -> bool {
    const TAIL: usize = zisk_verifier::VADCOP_VK_LEN_WORDS + zisk_verifier::HASH_TAG_LEN_WORDS;
    if zisk_proof.len() < TAIL {
        return false;
    }
    // Tail layout: [zisk_vk(4)][hash tag(1)].
    let (proof, tail) = zisk_proof.split_at(zisk_proof.len() - TAIL);
    let Some(hash) = zisk_verifier::hash_id_from_tag(tail[zisk_verifier::VADCOP_VK_LEN_WORDS])
    else {
        return false;
    };

    match zisk_verifier::committed_program_vk(proof) {
        Some(vk) if vk == expected_program_vk => {}
        _ => return false,
    }

    // An aggregate's declared domain must be the key it verifies under, or a subtree from
    // another recurser rides through. Mirrors `Proof::verify`: both keys are the
    // recurser's own verkey.
    //
    // A compressed proof cannot be classified -- `FinalCompressed` strips the flag -- so
    // this cannot fire for one. It does not need to: a fold verifies only under its own
    // recurser's key, so against a leaf's `expected_setup_vk` it fails outright. The gap
    // is a caller that pins a recurser key while expecting a different program VK, which
    // is why both keys must be the recurser's own for folds. Producers refuse to compress
    // an aggregate for the same reason.
    if zisk_verifier::committed_is_aggregate(proof) == Some(true)
        && expected_program_vk != expected_setup_vk
    {
        return false;
    }

    zisk_verifier::verify_vadcop_final_proof(proof, expected_setup_vk, hash)
}

/// C-ABI wrapper around [verify_zisk_proof].
///
/// # Safety
/// - Every pointer must reference `*_len` initialized, 8-byte-aligned bytes with
///   `*_len` a multiple of 8; returns `false` otherwise.
/// - A null pointer returns `false` instead of being dereferenced; `from_raw_parts`
///   forbids a null base even at length 0, so all three are checked before any slice.
/// - Both keys must be guest compile-time constants (see [verify_zisk_proof]).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_verify_zisk_proof_c")]
pub unsafe extern "C" fn verify_zisk_proof_c(
    zisk_proof: *const u8,
    zisk_proof_len: usize,
    expected_setup_vk: *const u8,
    expected_setup_vk_len: usize,
    expected_program_vk: *const u8,
    expected_program_vk_len: usize,
) -> bool {
    if zisk_proof.is_null() || expected_setup_vk.is_null() || expected_program_vk.is_null() {
        return false;
    }
    let proof_bytes = core::slice::from_raw_parts(zisk_proof, zisk_proof_len);
    let setup_vk_bytes = core::slice::from_raw_parts(expected_setup_vk, expected_setup_vk_len);
    let program_vk_bytes =
        core::slice::from_raw_parts(expected_program_vk, expected_program_vk_len);
    let (proof_prefix, proof_words, proof_suffix) = proof_bytes.align_to::<u64>();
    let (setup_prefix, setup_words, setup_suffix) = setup_vk_bytes.align_to::<u64>();
    let (program_prefix, program_words, program_suffix) = program_vk_bytes.align_to::<u64>();
    if !proof_prefix.is_empty()
        || !proof_suffix.is_empty()
        || !setup_prefix.is_empty()
        || !setup_suffix.is_empty()
        || !program_prefix.is_empty()
        || !program_suffix.is_empty()
    {
        return false;
    }
    verify_zisk_proof(proof_words, setup_words, program_words)
}
