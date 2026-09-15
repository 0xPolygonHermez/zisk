/// Hash families a proof can be produced under. One outside this list is rejected here
/// rather than dispatched, because the verifier panics on it and a guest panic aborts the run.
const HASH_FAMILIES: [&str; 3] = ["Poseidon1", "Poseidon2", "blake3"];

/// Family assumed when the caller names none. A proof carries no family tag, so proofs
/// from a Poseidon proving key must go through [verify_zisk_proof_with_hash].
const DEFAULT_HASH: &str = "blake3";

/// Verify a ZisK proof against a caller-supplied expected program and setup key, under
/// [DEFAULT_HASH]. The vk appended to `zisk_proof` is ignored.
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
    verify_zisk_proof_with_hash(zisk_proof, expected_setup_vk, expected_program_vk, DEFAULT_HASH)
}

/// [verify_zisk_proof] under `hash`, the family of the proving key the recursion was
/// generated against. Returns `false` for an unrecognized family.
pub fn verify_zisk_proof_with_hash(
    zisk_proof: &[u64],
    expected_setup_vk: &[u64],
    expected_program_vk: &[u64],
    hash: &str,
) -> bool {
    if !HASH_FAMILIES.contains(&hash) {
        return false;
    }
    if zisk_proof.len() < zisk_verifier::VADCOP_VK_LEN_WORDS {
        return false;
    }
    let (proof, _embedded_vk) =
        zisk_proof.split_at(zisk_proof.len() - zisk_verifier::VADCOP_VK_LEN_WORDS);

    match zisk_verifier::committed_program_vk(proof) {
        Some(vk) if vk == expected_program_vk => {}
        _ => return false,
    }

    zisk_verifier::verify_vadcop_final_proof(proof, expected_setup_vk, hash)
}

/// C-ABI wrapper around [verify_zisk_proof].
///
/// # Safety
/// - Every pointer must reference `*_len` initialized, 8-byte-aligned bytes with
///   `*_len` a multiple of 8; returns `false` otherwise.
/// - A null pointer returns `false` instead of being dereferenced.
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
    verify_zisk_proof_with_hash_c(
        zisk_proof,
        zisk_proof_len,
        expected_setup_vk,
        expected_setup_vk_len,
        expected_program_vk,
        expected_program_vk_len,
        DEFAULT_HASH.as_ptr(),
        DEFAULT_HASH.len(),
    )
}

/// C-ABI wrapper around [verify_zisk_proof_with_hash].
///
/// # Safety
/// - Every pointer must reference `*_len` initialized, 8-byte-aligned bytes with
///   `*_len` a multiple of 8; returns `false` otherwise. `hash` is plain bytes
///   (no NUL needed); non-UTF-8 returns `false`.
/// - A null pointer returns `false` instead of being dereferenced; `from_raw_parts`
///   forbids a null base even at length 0, so all four are checked before any slice.
/// - Both keys must be guest compile-time constants (see [verify_zisk_proof]).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_verify_zisk_proof_with_hash_c")]
pub unsafe extern "C" fn verify_zisk_proof_with_hash_c(
    zisk_proof: *const u8,
    zisk_proof_len: usize,
    expected_setup_vk: *const u8,
    expected_setup_vk_len: usize,
    expected_program_vk: *const u8,
    expected_program_vk_len: usize,
    hash: *const u8,
    hash_len: usize,
) -> bool {
    if zisk_proof.is_null()
        || expected_setup_vk.is_null()
        || expected_program_vk.is_null()
        || hash.is_null()
    {
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
    let Ok(hash) = core::str::from_utf8(core::slice::from_raw_parts(hash, hash_len)) else {
        return false;
    };
    verify_zisk_proof_with_hash(proof_words, setup_words, program_words, hash)
}
