use crate::error::{CommonError, Result};
use proofman::{verify_snark_proof_with_vkey, SnarkProof, SnarkProtocol};
use proofman_common::ProofmanError;
use proofman_common::{PublicDefinition, PublicsInfo};
use proofman_verifier::VadcopFinalProof;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use zisk_verifier::{
    program_publics, GOLDILOCKS_ORDER, IS_VADCOP_FINAL_PROOF, PROGRAM_VK_LEN,
    VADCOP_FINAL_FLAG_LEN, ZISK_PUBLICS,
};

use crate::{HashMode, RandomSeed};

/// Leads a saved PLONK proof. A Vadcop proof is saved flat, whose first word is the 0/1
/// minimal marker, so no Vadcop file starts with this.
pub const PLONK_FILE_MAGIC: &[u8; 8] = b"ZISKPLNK";

/// Ceiling on what decoding a PLONK file may allocate. A real one (proof, vkey, publics)
/// is a few KB.
const PLONK_FILE_MAX_BYTES: usize = 1 << 20;

/// A BN254 PLONK proof: 9 G1 points (64 bytes each) and 6 field evaluations (32 each).
/// The native verifier aborts the process on any other length rather than failing.
pub const PLONK_PROOF_BYTES: usize = 9 * 64 + 6 * 32;

/// The PLONK circuit key the release's `PlonkVerifier.sol` was generated from, for the
/// families that have a PLONK wrap. `rootc` is a public input of that circuit, so one key
/// covers leaves and aggregates alike.
pub fn release_plonk_vkey(hash: &str) -> Option<&'static PlonkVkey> {
    static POSEIDON1: std::sync::OnceLock<PlonkVkey> = std::sync::OnceLock::new();
    match hash {
        "Poseidon1" => Some(POSEIDON1.get_or_init(|| {
            serde_json::from_str(include_str!("../keys/plonk_poseidon1.vkey.json"))
                .expect("the bundled release PLONK key parses")
        })),
        _ => None,
    }
}

/// The canonical representative of a Goldilocks element: the unique value in
/// `[0, p)` congruent to `word`. One subtraction suffices — `p > 2^63`, so every
/// u64 is below `2p`.
#[inline]
fn canonical(word: u64) -> u64 {
    if word >= GOLDILOCKS_ORDER {
        word - GOLDILOCKS_ORDER
    } else {
        word
    }
}

/// Canonicality plus the exact shape a *stored* body must have, including a PLONK proof's
/// length, which must be checked before it reaches the native verifier.
///
/// Length: a short vector would panic the fixed-offset slicing in `snark_publics_hash` /
/// `snark_inputs_bytes`, turning an untrusted proof into a crash rather than an `Err`.
/// Storage is flag-free by construction — `stark_publics` re-adds the flag at verify
/// time, so a 69-word body would reach the STARK verifier with 70 publics.
fn ensure_stored_body(body: &ProofBody) -> Result<()> {
    if let ProofBody::Plonk { proof_bytes, .. } = body {
        if proof_bytes.len() != PLONK_PROOF_BYTES {
            return Err(CommonError::InvalidProof(format!(
                "PLONK proof is {} bytes, expected {PLONK_PROOF_BYTES}",
                proof_bytes.len()
            )));
        }
    }
    let flag_free = PROGRAM_VK_LEN + ZISK_PUBLICS;
    let (ProofBody::Vadcop { publics_full, .. }
    | ProofBody::Plonk { publics_full, .. }
    | ProofBody::Pilfflonk { publics_full, .. }) = body;
    if publics_full.len() != flag_free {
        return Err(CommonError::InvalidProof(format!(
            "stored publics have {} field elements, expected {flag_free}",
            publics_full.len()
        )));
    }
    ensure_canonical_publics(publics_full)
}

/// What to pass when a proof fails under the release key: a custom setup's key for the
/// proof's own stage.
fn hint_setup_vk(trusted_setup_vk: Option<&[u64]>, compressed: bool) -> Option<String> {
    let stage = if compressed { "vadcop_final_compressed" } else { "vadcop_final" };
    trusted_setup_vk.is_none().then(|| {
        format!(
            "a proof from a non-release setup verifies under that setup's \
             `{stage}.verkey.json` (`--leaf-setup-vk`)"
        )
    })
}

/// Without this a proof holder could rewrite a stored public — `x` and `x + p` are one
/// field element to the STARK verifier but two different reported outputs.
fn ensure_canonical_publics(publics_full: &[u64]) -> Result<()> {
    let normalized = program_publics(publics_full);
    if normalized.len() != PROGRAM_VK_LEN + ZISK_PUBLICS {
        return Err(CommonError::InvalidProof(format!(
            "committed publics have {} field elements (after flag strip), expected {}",
            normalized.len(),
            PROGRAM_VK_LEN + ZISK_PUBLICS
        )));
    }
    if let Some(i) = publics_full.iter().position(|&w| w >= GOLDILOCKS_ORDER) {
        return Err(CommonError::InvalidProof(format!(
            "public {i} is not a canonical Goldilocks element: {} >= {GOLDILOCKS_ORDER}",
            publics_full[i]
        )));
    }
    Ok(())
}

/// Cache key for a built setup (per program + build flavor).
///
/// `hash_mode` is intentionally NOT part of the key: a worker/prover is started
/// against a single proving key whose hash family is fixed for the process
/// lifetime, so a given `hash_id` only ever resolves to one `hash_mode` within
/// a cache. Adding the mode would be dead discriminator. (If proving keys ever
/// become hot-swappable per process, revisit this.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetupKey {
    /// Hash identifier for the program, used to select the appropriate proving key and verification key.
    pub hash_id: String,
    /// Indicates whether the proof includes hints, which may require a different proving key.
    pub with_hints: bool,
    /// Indicates whether the proof is intended for emulator-only verification.
    pub emulator_only: bool,
}

impl SetupKey {
    /// Creates a new `SetupKey` instance.
    pub fn new(hash_id: impl Into<String>, with_hints: bool, emulator_only: bool) -> Self {
        Self { hash_id: hash_id.into(), with_hints, emulator_only }
    }
}

/// The `ProgramVK` struct represents the verification key for a program, consisting of a vector of u64 values.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ProgramVK {
    /// Verification key values for the program.
    pub vk: Vec<u64>,
    /// Hash mode.
    pub hash_mode: HashMode,
}

impl ProgramVK {
    /// Build from the first `PROGRAM_VK_LEN` u64 elements of a publics blob,
    /// recording the [`HashMode`] the verkey was produced under.
    ///
    /// # Panics
    ///
    /// Panics if `publics` has fewer than `PROGRAM_VK_LEN` elements.
    pub fn new_from_publics_with_mode(publics: &[u64], hash_mode: HashMode) -> Self {
        // Strip the recursion-layer `is_vadcop_final_proof` flag (present on a
        // full 69-wide vadcop_final publics vector) so the VK is read from the
        // flag-free `[vk | inputs]` view rather than `[flag | vk | inputs]`.
        let publics = program_publics(publics);
        assert!(
            publics.len() >= PROGRAM_VK_LEN,
            "Not enough u64 publics to extract program VK (expected at least {})",
            PROGRAM_VK_LEN
        );

        Self { vk: publics[..PROGRAM_VK_LEN].to_vec(), hash_mode }
    }

    /// Build from publics using the default [`HashMode`].
    pub fn new_from_publics(publics: &[u64]) -> Self {
        Self::new_from_publics_with_mode(publics, HashMode::default())
    }

    /// Creates a new `ProgramVK` instance with an empty verification key (filled with zeros).
    pub fn new_empty() -> Self {
        Self { vk: vec![0u64; PROGRAM_VK_LEN], hash_mode: HashMode::default() }
    }
}

/// Which flavor of Vadcop proof a [`ProofBody::Vadcop`] holds. This is the axis
/// that used to be a `minimal: bool`, split out so the `is_vadcop_final_proof`
/// public flag (present at index 0 of a full-width publics vector) has a single,
/// unambiguous value per variant instead of being guessed from the vector length:
///
/// | Variant    | publics layout                    | flag @0 |
/// |------------|-----------------------------------|---------|
/// | `Final`    | `[flag=1 \| vk(4) \| inputs(64)]` (69) | 1 (raw ZisK leaf) |
/// | `Recurser` | `[flag=0 \| vk(4) \| inputs(64)]` (69) | 0 (aggregator output) |
/// | `Minimal`  | `[vk(4) \| inputs(64)]` (68)          | none (compressed strips it) |
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VadcopKind {
    /// Raw ZisK vadcop_final proof — a recursion-tree leaf. Flag = 1.
    #[default]
    Final,
    /// Output of a recurser fold (aggregated proof). Same wire shape as `Final`
    /// but the flag is 0. Kept distinct so re-verification / re-folding stamps
    /// the correct flag value.
    Recurser,
    /// Minimal (compressed) proof: the `final_compressed` circuit strips the flag,
    /// so its publics are the flag-free `[vk | inputs]`.
    Minimal,
}

impl VadcopKind {
    /// The `is_vadcop_final_proof` value at public index 0, or `None` when the
    /// flavor carries no flag (minimal/compressed).
    pub fn flag(self) -> Option<u64> {
        match self {
            VadcopKind::Final => Some(IS_VADCOP_FINAL_PROOF),
            // An aggregator output forces the flag to 0 (see the recurser circuit).
            VadcopKind::Recurser => Some(0),
            VadcopKind::Minimal => None,
        }
    }

    /// Whether this is the minimal (compressed) proof — the STARK verifier and
    /// setup lookups that previously took a `minimal: bool` use this.
    pub fn is_minimal(self) -> bool {
        matches!(self, VadcopKind::Minimal)
    }

    /// Classify a RAW publics vector as it arrives from the prover/wire (before
    /// normalization): `Minimal` when flag-free (`PROGRAM_N_PUBLICS`, 68), else
    /// `Final`/`Recurser` by the `is_vadcop_final_proof` flag at index 0 (69).
    /// Falls back to `Final` for unexpected lengths (callers assert elsewhere).
    /// Used at ingest to capture the flag before it is stripped from
    /// `publics_full`.
    /// Best-effort classification by shape. Every non-zero flag reads as `Final`, so
    /// untrusted input must pin the flag itself rather than rely on this.
    pub fn from_publics_full(publics_full: &[u64]) -> Self {
        if publics_full.len() == VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN + ZISK_PUBLICS {
            if publics_full[0] == 0 {
                VadcopKind::Recurser
            } else {
                VadcopKind::Final
            }
        } else if publics_full.len() == PROGRAM_VK_LEN + ZISK_PUBLICS {
            VadcopKind::Minimal
        } else {
            VadcopKind::Final
        }
    }

    /// Build the STARK public vector for this proof from the canonical flag-free
    /// `publics_full` (`[program_vk | inputs]`): re-adds the
    /// `is_vadcop_final_proof` flag at index 0 for `Final`/`Recurser`, and
    /// returns the flag-free publics unchanged for `Minimal`. This is the exact
    /// vector the STARK verifier / next-layer witness commits to (full u64
    /// width), and the inverse of the ingest strip.
    pub fn stark_publics(self, publics_full: &[u64]) -> Vec<u64> {
        match self.flag() {
            Some(flag) => {
                let mut v = Vec::with_capacity(VADCOP_FINAL_FLAG_LEN + publics_full.len());
                v.push(flag);
                v.extend_from_slice(publics_full);
                v
            }
            None => publics_full.to_vec(),
        }
    }
}

/// Enumeration of supported proof types, used to distinguish between different proof generation and verification logic.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofKind {
    /// A STARKs proof.
    #[default]
    VadcopFinal,
    /// A minimal STARKs proof variant optimized for size.
    VadcopFinalMinimal,
    /// A Plonk SNARK proof.
    Plonk,
}

impl From<i32> for ProofKind {
    fn from(v: i32) -> Self {
        match v {
            1 => ProofKind::VadcopFinalMinimal,
            2 => ProofKind::Plonk,
            _ => ProofKind::VadcopFinal,
        }
    }
}

impl From<ProofKind> for i32 {
    fn from(k: ProofKind) -> Self {
        match k {
            ProofKind::VadcopFinal => 0,
            ProofKind::VadcopFinalMinimal => 1,
            ProofKind::Plonk => 2,
        }
    }
}

/// The `PlonkVkey` struct represents the verification key for a Plonk proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlonkVkey {
    /// Proof system identifier.
    pub protocol: String,
    /// Elliptic curve identifier.
    pub curve: String,
    /// Number of public inputs expected by the proof, which must match the number of public values provided during verification.
    #[serde(rename = "nPublic")]
    pub n_public: u32,
    /// Log₂ of the evaluation domain size: the circuit is padded to `n = 2^power` constraints.
    pub power: u32,
    /// First coset shift for the permutation argument. The three wire columns are mapped onto the
    /// cosets `H`, `k1·H`, `k2·H`, so `k1` (with `k2`) must yield cosets disjoint from `H` and from each other.
    pub k1: String,
    /// Second coset shift for the permutation argument (see `k1`).
    pub k2: String,
    /// KZG commitment to the multiplication selector polynomial `q_M` (G1 point).
    #[serde(rename = "Qm")]
    pub qm: [String; 3],
    /// KZG commitment to the left-wire selector polynomial `q_L` (G1 point).
    #[serde(rename = "Ql")]
    pub ql: [String; 3],
    /// KZG commitment to the right-wire selector polynomial `q_R` (G1 point).
    #[serde(rename = "Qr")]
    pub qr: [String; 3],
    /// KZG commitment to the output-wire selector polynomial `q_O` (G1 point).
    #[serde(rename = "Qo")]
    pub qo: [String; 3],
    /// KZG commitment to the constant selector polynomial `q_C` (G1 point).
    #[serde(rename = "Qc")]
    pub qc: [String; 3],
    /// KZG commitment to the first permutation polynomial `S_σ1`, encoding the copy constraints
    /// over the first wire column (G1 point).
    #[serde(rename = "S1")]
    pub s1: [String; 3],
    /// KZG commitment to the second permutation polynomial `S_σ2` (G1 point).
    #[serde(rename = "S2")]
    pub s2: [String; 3],
    /// KZG commitment to the third permutation polynomial `S_σ3` (G1 point).
    #[serde(rename = "S3")]
    pub s3: [String; 3],
    /// The SRS element `[x]₂` from the trusted setup, used as the G2 input to the final KZG pairing check.
    /// G2 point in projective coordinates over `Fp2`: 3 coordinates, each an `[c0, c1]` pair.
    #[serde(rename = "X_2")]
    pub x_2: [[String; 2]; 3],
    /// Generator of the evaluation domain `H`: a primitive `n`-th root of unity, with `n = 2^power`.
    pub w: String,
}

impl PlonkVkey {
    /// Load PlonkVkey from a JSON file
    ///
    /// # Errors
    ///
    /// - [`CommonError::Io`] if the file cannot be opened or read.
    /// - [`CommonError::Deserialization`] if the JSON cannot be parsed into a [`PlonkVkey`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path.as_ref()).map_err(|e| {
            CommonError::Io(format!(
                "failed to open file for loading PlonkVkey: {}: {e}",
                path.as_ref().display()
            ))
        })?;
        let vkey: PlonkVkey = serde_json::from_reader(file).map_err(|e| {
            CommonError::Deserialization(format!(
                "failed to parse PlonkVkey JSON from {}: {e}",
                path.as_ref().display()
            ))
        })?;
        Ok(vkey)
    }

    /// Save PlonkVkey to a JSON file
    ///
    /// # Errors
    ///
    /// - [`CommonError::Io`] if the parent directory or the file cannot be created.
    /// - [`CommonError::Serialization`] if the vkey cannot be serialized to JSON.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CommonError::Io(format!(
                    "failed to create parent directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        let file = File::create(path).map_err(|e| {
            CommonError::Io(format!(
                "failed to create file for saving PlonkVkey: {}: {e}",
                path.display()
            ))
        })?;

        serde_json::to_writer_pretty(file, self).map_err(|e| {
            CommonError::Serialization(format!("PlonkVkey JSON to {}: {e}", path.display()))
        })?;

        Ok(())
    }
}

/// Verification key for a Plonk proof: the underlying Vadcop vkey plus the structured Plonk vkey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlonkVkBlob {
    /// Vadcop verification key values.
    pub vadcop_vk: Vec<u64>,
    /// Structured Plonk verification key.
    pub plonk_vkey: PlonkVkey,
}

/// Public values for proof generation and verification.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PublicValues {
    data: Vec<u8>,
    #[serde(skip)]
    ptr: AtomicUsize,
}

impl Clone for PublicValues {
    fn clone(&self) -> Self {
        Self { data: self.data.clone(), ptr: AtomicUsize::new(self.ptr.load(Ordering::Relaxed)) }
    }
}

impl PublicValues {
    /// Build from the full proof publics byte blob.
    ///
    /// # Panics
    ///
    /// Panics if `publics_bytes` is not exactly `ZISK_PUBLICS * 8 + 32` bytes long.
    pub fn new(publics_bytes: &[u8]) -> Self {
        assert!(
            publics_bytes.len() == ZISK_PUBLICS * 8 + 32,
            "Not enough bytes to fill PublicValues"
        );

        let mut data = [0u8; ZISK_PUBLICS * 4];
        for (i, chunk) in publics_bytes[32..].chunks_exact(8).enumerate() {
            let v32 = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
            data[i * 4..(i + 1) * 4].copy_from_slice(&v32.to_le_bytes());
        }

        Self { data: data.to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Build from the full proof publics u64 blob: `[program_vk(4)][publics(ZISK_PUBLICS)]`.
    /// Each public u64 is truncated to its low 32 bits (matching `public_u64()`).
    ///
    /// Truncation is only sound because non-canonical encodings are rejected at ingest
    /// (`ensure_canonical_publics`, and the verifier's own check): `x` and `x + p` are
    /// one field element but differ in their low 32 bits, so without that rejection the
    /// reported outputs would not be pinned by the verified statement. Callers must not
    /// treat truncation alone as a security boundary.
    ///
    /// # Panics
    ///
    /// Panics if `publics` does not contain exactly `ZISK_PUBLICS + PROGRAM_VK_LEN` elements.
    pub fn new_from_u64(publics: &[u64]) -> Self {
        // Accept either the flag-free app view (`[vk | inputs]`, 68) or a full
        // vadcop_final vector (`[flag | vk | inputs]`, 69); strip the flag first.
        let publics = program_publics(publics);
        assert!(
            publics.len() == ZISK_PUBLICS + PROGRAM_VK_LEN,
            "Expected {} u64 publics, got {}",
            ZISK_PUBLICS + PROGRAM_VK_LEN,
            publics.len()
        );

        let mut data = [0u8; ZISK_PUBLICS * 4];
        for (i, &val) in publics[PROGRAM_VK_LEN..].iter().enumerate() {
            data[i * 4..(i + 1) * 4].copy_from_slice(&(canonical(val) as u32).to_le_bytes());
        }

        Self { data: data.to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Creates a new `PublicValues` instance with empty data and a reset pointer.
    pub fn new_empty() -> Self {
        Self { data: [0u8; ZISK_PUBLICS * 4].to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Create PublicValues from a serializable value.
    /// The value is serialized with bincode and stored in the public outputs as 64-bit chunks.
    ///
    /// # Errors
    ///
    /// - [`CommonError::Serialization`] if the value cannot be serialized with bincode.
    /// - [`CommonError::Invalid`] if the serialized data exceeds `ZISK_PUBLICS * 4` bytes.
    pub fn write<T: serde::Serialize>(value: &T) -> Result<Self> {
        let serialized = bincode::serde::encode_to_vec(value, bincode::config::standard())
            .map_err(|e| CommonError::Serialization(e.to_string()))?;

        if serialized.len() > ZISK_PUBLICS * 4 {
            return Err(CommonError::Invalid(format!(
                "Serialized data too large: {} bytes (max {} bytes)",
                serialized.len(),
                ZISK_PUBLICS * 4
            )));
        }

        let mut data = [0u8; ZISK_PUBLICS * 4];
        // Chunk into 8-byte (u64) values
        for (i, chunk) in serialized.chunks(4).enumerate() {
            // copy chunk into 32-bit slot, padding with zeros if chunk < 4 bytes
            let mut buf = [0u8; 4];
            buf[..chunk.len()].copy_from_slice(chunk);
            data[i * 4..(i + 1) * 4].copy_from_slice(&buf);
        }

        Ok(Self { data: data.to_vec(), ptr: AtomicUsize::new(0) })
    }

    /// Create PublicValues from an ABI-encodable value.
    /// The value is ABI-encoded and stored in the public outputs as 32-bit chunks.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Invalid`] if the ABI-encoded data exceeds `ZISK_PUBLICS * 4` bytes.
    pub fn write_abi<T: alloy_sol_types::SolValue>(value: &T) -> Result<Self> {
        let encoded = value.abi_encode();

        if encoded.len() > ZISK_PUBLICS * 4 {
            return Err(CommonError::Invalid(format!(
                "ABI encoded data too large: {} bytes (max {} bytes)",
                encoded.len(),
                ZISK_PUBLICS * 4
            )));
        }

        let mut data = [0u8; ZISK_PUBLICS * 4];
        for (i, chunk) in encoded.chunks(4).enumerate() {
            // copy chunk into 32-bit slot, padding with zeros if chunk < 4 bytes
            let mut buf = [0u8; 4];
            buf[..chunk.len()].copy_from_slice(chunk);
            data[i * 4..(i + 1) * 4].copy_from_slice(&buf);
        }

        Ok(Self { data: data.to_vec(), ptr: AtomicUsize::new(0) })
    }

    /// Reset the reading pointer to the beginning.
    pub fn head(&self) {
        self.ptr.store(0, Ordering::Relaxed);
    }

    /// Read raw bytes from public outputs.
    pub fn read_slice(&self, slice: &mut [u8]) {
        let ptr = self.ptr.load(Ordering::Relaxed);
        slice.copy_from_slice(&self.data[ptr..ptr + slice.len()]);
        self.ptr.store(ptr + slice.len(), Ordering::Relaxed);
    }

    /// Deserialize a value from public outputs.
    /// The value must have been previously written with bincode serialization using `commit()`.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Deserialization`] if the stored bytes cannot be decoded into `T`.
    pub fn read<T: serde::Serialize + serde::de::DeserializeOwned>(&self) -> Result<T> {
        let ptr = self.ptr.load(Ordering::Relaxed);
        let (result, nb_bytes): (T, usize) =
            bincode::serde::decode_from_slice(&self.data[ptr..], bincode::config::standard())
                .map_err(|e| CommonError::Deserialization(e.to_string()))?;
        self.ptr.store(ptr + nb_bytes, Ordering::Relaxed);
        Ok(result)
    }

    /// Decode an ABI-encoded value from public outputs.
    /// The value must have been previously written with ABI encoding using `write_abi()`.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::AbiDecoding`] if the stored bytes cannot be ABI-decoded into `T`.
    pub fn read_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        let ptr = self.ptr.load(Ordering::Relaxed);
        let decoded = T::abi_decode(&self.data[ptr..])
            .map_err(|e| CommonError::AbiDecoding(e.to_string()))?;
        let encoded_size = decoded.abi_encode().len();
        self.ptr.store(ptr + encoded_size, Ordering::Relaxed);
        Ok(decoded)
    }

    /// Public values as `ZISK_PUBLICS` u64 elements (each is a u32 widened to u64).
    pub fn public_u64(&self) -> Vec<u64> {
        (0..ZISK_PUBLICS)
            .map(|i| {
                let start = i * 4;
                u32::from_le_bytes([
                    self.data[start],
                    self.data[start + 1],
                    self.data[start + 2],
                    self.data[start + 3],
                ]) as u64
            })
            .collect()
    }

    /// Hash the public values using Solidity-compatible encoding.
    pub fn hash_solidity(&self, program_vk: &ProgramVK, vadcop_verkey: &[u64]) -> Vec<u8> {
        let bytes = self.bytes_solidity(program_vk, vadcop_verkey);

        // SHA-256
        let hash = Sha256::digest(&bytes);

        hash.to_vec()
    }
}

impl PublicValues {
    /// Convert the public values into a byte vector formatted for Solidity hashing.
    pub fn bytes_solidity(&self, program_vk: &ProgramVK, vadcop_verkey: &[u64]) -> Vec<u8> {
        let mut prefix = [0u8; PROGRAM_VK_LEN * 8];
        for (i, val) in program_vk.vk.iter().enumerate() {
            prefix[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }

        let mut bytes = prefix.to_vec();
        bytes.extend_from_slice(&self.data);
        let mut suffix = [0u8; PROGRAM_VK_LEN * 8];
        for (i, val) in vadcop_verkey.iter().enumerate() {
            suffix[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }
        bytes.extend(&suffix);
        bytes
    }
}

/// ZisK's publics as the final SNARK circuit lays them out: the program VK, then the
/// `ZISK_PUBLICS` user outputs, one 64-bit chunk each.
fn zisk_publics_info() -> &'static PublicsInfo {
    static INFO: std::sync::OnceLock<PublicsInfo> = std::sync::OnceLock::new();
    INFO.get_or_init(|| {
        let section = |name: &str, initial_pos, n_values, verification_key| PublicDefinition {
            name: name.to_string(),
            initial_pos,
            n_values,
            chunks: [1, 64],
            verification_key,
        };
        PublicsInfo {
            n_publics: PROGRAM_VK_LEN + ZISK_PUBLICS,
            definitions: vec![
                section("rom_root", 0, PROGRAM_VK_LEN, true),
                section("inputs", PROGRAM_VK_LEN, ZISK_PUBLICS, false),
            ],
            has_program_vk: true,
        }
    })
}

/// The user outputs as the SNARK circuit hashes them, 8 little-endian bytes each: the
/// on-chain `publicValues`, not the u32 `PublicValues.data`. See
/// `PublicsInfo::solidity_public_bytes`.
///
/// # Panics
///
/// Panics if `publics_full` is not a `[flag? | program_vk | inputs]` vector.
pub fn snark_inputs_bytes(publics_full: &[u64]) -> Vec<u8> {
    zisk_publics_info().solidity_public_bytes(publics_full).expect("publics_full has ZisK's layout")
}

/// The public input the SNARK commits to, as 32 big-endian bytes: `getSha256Inputs` over
/// `publics_full` and `rootc`, the key the wrap verified under (vadcop_final's for a leaf,
/// the recurser's own for an aggregate). See `PublicsInfo::snark_public_digest`.
///
/// # Panics
///
/// Panics if `publics_full` is not a `[flag? | program_vk | inputs]` vector, or `rootc` is
/// shorter than `PROGRAM_VK_LEN`.
pub fn snark_publics_hash(publics_full: &[u64], rootc: &[u64]) -> Vec<u8> {
    let rootc: &[u64; PROGRAM_VK_LEN] =
        rootc.get(..PROGRAM_VK_LEN).and_then(|r| r.try_into().ok()).expect("rootc too short");
    zisk_publics_info()
        .snark_public_digest(publics_full, rootc)
        .expect("publics_full has ZisK's layout")
        .to_vec()
}

/// Kind-tagged proof payload.
///
/// Publics are stored as full-width u64 field elements: a recurser proof's
/// publics exceed 32 bits, and the recursion round-trip / snark hash must use
/// the exact committed elements. The u32 `PublicValues` view (guest API,
/// Solidity encoding) is derived on demand via [`Proof::publics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofBody {
    /// A recursive (Vadcop) proof.
    Vadcop {
        /// Proof, as a flat vector of field elements.
        proof: Vec<u64>,
        /// ZisK verification key.
        zisk_vk: Vec<u64>,
        /// Which vadcop flavor this is (Final / Recurser / Minimal). Owns the
        /// `is_vadcop_final_proof` flag value (1 / 0 / none); the flag is NOT
        /// stored in `publics_full` — STARK paths re-add it via
        /// [`VadcopKind::stark_publics`].
        kind: VadcopKind,
        /// Hash family the proof was generated with.
        hash: String,
        /// Canonical flag-free program publics `[program_vk(4) | inputs(64)]`
        /// (always 68), at full u64 width. The recursion-layer
        /// `is_vadcop_final_proof` flag lives in `kind`, not here.
        publics_full: Vec<u64>,
    },
    /// A Plonk proof for on-chain verification.
    Plonk {
        /// Serialized proof bytes.
        proof_bytes: Vec<u8>,
        /// The keys the producer wrapped under. Untrusted metadata: verification uses the
        /// caller's or the release keys. Boxed so the Vadcop variant stays small.
        plonk_vk: Box<PlonkVkBlob>,
        /// u32 view of the publics, for the Solidity calldata layout.
        publics: PublicValues,
        /// Full-width publics; the snark's `publicsHash` is computed over these
        /// (the u32 `publics` view loses a recurser proof's high bits).
        publics_full: Vec<u64>,
        /// The stamped `rootCVadcopFinal` committed into `publicsHash`:
        /// vadcop_final verkey for a plain proof, recurser verkey for an
        /// aggregated one. Not derivable from `publics_full` (see `Proof::verify`).
        rootc: Vec<u64>,
    },
    /// A pilfflonk SNARK proof. It is the final SNARK of a blake3 key, and is also set up for the
    /// poseidon families with `--final-snark pilfflonk`. Appended after `Plonk`: bincode records
    /// the variant by position, so files of the other variants decode as before.
    Pilfflonk {
        /// Serialized proof bytes.
        proof_bytes: Vec<u8>,
        /// The `pilfflonk.vkey.json` the proof verifies under, as its JSON. Unlike a PLONK vkey it
        /// has no fixed fields to deserialize into, and the verifier reads it as a file.
        vkey_json: String,
        /// u32 view of the publics.
        publics: PublicValues,
        /// Full-width publics; the snark's public digest is computed over these.
        publics_full: Vec<u64>,
        /// The stamped `rootCVadcopFinal` committed into the digest, as in `Plonk`.
        rootc: Vec<u64>,
    },
}

impl Default for ProofBody {
    fn default() -> Self {
        ProofBody::Vadcop {
            proof: Vec::new(),
            zisk_vk: vec![0u64; PROGRAM_VK_LEN],
            kind: VadcopKind::Final,
            hash: String::new(),
            publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
        }
    }
}

/// A struct representing a proof.
///
/// Serialized with bincode, which records no field names, lengths or version: fields decode
/// positionally. New fields therefore only go at the end, and [`Proof::from_bytes`] is the one
/// decoder that knows which trailing fields a file may lack. Decode through it, never with
/// `bincode` directly.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// The data of the proof.
    pub body: ProofBody,
    /// The program verification key.
    pub program_vk: ProgramVK,
    /// Host randomness seed of the execution this proof attests to.
    ///
    /// Unauthenticated metadata: verification ignores it, as the values derived from it are
    /// free witness values. `None` for proofs that do not come from a single execution
    /// (aggregates) and for proofs written by ZisK versions that predate this field.
    pub random_seed: Option<RandomSeed>,
}

/// Builder for customizing verification parameters before calling verify.
///
/// This builder allows you to override the publics or program VK
/// that will be used during verification. If not overridden, the values from
/// the proof itself will be used.
///
/// # Examples
///
/// ```ignore
/// // Use default values from proof
/// proof.verify()?;
///
/// // Override publics only
/// proof.with_publics(&custom_publics).verify()?;
///
/// // Override program VK only
/// proof.with_program_vk(&custom_program_vk).verify()?;
///
/// // Override both
/// proof.with_publics(&custom_publics).with_program_vk(&custom_program_vk).verify()?;
/// ```
pub struct ZiskVerifyBuilder<'a> {
    proof_with_values: &'a Proof,
    override_publics: Option<&'a PublicValues>,
    override_program_vk: Option<&'a ProgramVK>,
    trusted_plonk_vk: Option<&'a PlonkVkey>,
    trusted_pilfflonk_vkey: Option<&'a str>,
    trusted_setup_vk: Option<&'a [u64]>,
    /// The caller vouches `trusted_setup_vk` is a setup's leaf key, not a recurser's.
    leaf_setup_vk: bool,
}

impl<'a> ZiskVerifyBuilder<'a> {
    fn new(proof_with_values: &'a Proof) -> Self {
        Self {
            proof_with_values,
            override_publics: None,
            override_program_vk: None,
            trusted_plonk_vk: None,
            trusted_pilfflonk_vkey: None,
            trusted_setup_vk: None,
            leaf_setup_vk: false,
        }
    }

    /// Override the publics used for verification.
    pub fn with_publics(mut self, publics: &'a PublicValues) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key used for verification.
    pub fn with_program_vk(mut self, program_vk: &'a ProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Trusted PLONK circuit key (`final.verkey.json`). If unset, the release key for the
    /// proof's family ([`release_plonk_vkey`]). The copy the proof carries is never used.
    pub fn with_plonk_vk(mut self, plonk_vkey: &'a PlonkVkey) -> Self {
        self.trusted_plonk_vk = Some(plonk_vkey);
        self
    }

    /// Trusted pilfflonk key, the JSON of its `pilfflonk.vkey.json`. No release publishes one
    /// yet, so a pilfflonk proof needs it; the key the proof carries is never used.
    pub fn with_pilfflonk_vkey(mut self, vkey_json: &'a str) -> Self {
        self.trusted_pilfflonk_vkey = Some(vkey_json);
        self
    }

    /// Trusted recursion key (4 u64 limbs): a recurser's verkey for an aggregated proof,
    /// or a release leaf key. If unset, a plain proof verifies under the release key for its
    /// family and stage (`zisk_verifier::vadcop_vk`); an aggregated one requires it. The copy
    /// the proof carries is never used.
    ///
    /// A PLONK proof does not say whether it wraps a leaf or a fold, so under a key that is
    /// not a release leaf key it must declare that key as its recursion domain. Pass a
    /// custom setup's leaf key with [`with_leaf_setup_vk`](Self::with_leaf_setup_vk).
    pub fn with_setup_vk(mut self, setup_vk: &'a [u64]) -> Self {
        self.trusted_setup_vk = Some(setup_vk);
        self.leaf_setup_vk = false;
        self
    }

    /// A non-release setup's leaf key: its `vadcop_final.verkey.json`, or
    /// `vadcop_final_compressed.verkey.json` for a compressed proof. Aggregates are refused.
    pub fn with_leaf_setup_vk(mut self, setup_vk: &'a [u64]) -> Self {
        self.trusted_setup_vk = Some(setup_vk);
        self.leaf_setup_vk = true;
        self
    }

    /// The statement to verify, flag-free `[program_vk | inputs]`: the committed publics at
    /// full u64 width, with each override written over its own slots. Only the overridden
    /// inputs are u32-wide; an untouched recurser input keeps its full width.
    fn statement(&self, publics_full: &[u64]) -> Vec<u64> {
        let mut statement = program_publics(publics_full).to_vec();
        if let Some(pv) = self.override_program_vk {
            statement[..PROGRAM_VK_LEN].copy_from_slice(&pv.vk);
        }
        if let Some(publics) = self.override_publics {
            statement[PROGRAM_VK_LEN..].copy_from_slice(&publics.public_u64());
        }
        statement
    }

    /// The public digest a SNARK proof must carry, rebuilt from the statement and the root it
    /// verifies under; `name` is the protocol, for errors. A SNARK wraps an uncompressed
    /// vadcop_final proof.
    fn snark_public_digest(
        &self,
        publics_full: &[u64],
        stored_rootc: &[u64],
        name: &str,
    ) -> Result<Vec<u8>> {
        // The family only picks the release keys; an override's mode is irrelevant.
        let hash = self.proof_with_values.program_vk.hash_mode.as_str();
        // A recurser stores its own key in the program VK slots, so an unpinned
        // proof stamped with its own domain claims to be an aggregate.
        let committed = &program_publics(publics_full)[..PROGRAM_VK_LEN];
        let aggregate = stored_rootc == committed;
        let rootc = self.setup_vk_to_verify_under(hash, false, aggregate.then_some(committed))?;
        let rootc: &[u64] = &rootc;
        // export-solidity-calldata reads the stored copy.
        if stored_rootc != rootc {
            return Err(CommonError::InvalidProof(format!(
                "stored `rootc` does not match the setup vk the proof is verified under{}",
                hint_setup_vk(self.trusted_setup_vk, false)
                    .map(|h| format!("; {h}"))
                    .unwrap_or_default()
            )));
        }
        // The wrap does not commit the leaf flag, so a fold carrying another
        // recurser's subtree reads as a leaf of its own key. Unless that key is a
        // leaf key, the statement's domain must be the key, as for a Vadcop fold.
        let leaf_key =
            self.leaf_setup_vk || zisk_verifier::vadcop_vk(hash, false).is_some_and(|k| k == rootc);
        let statement = self.statement(publics_full);
        let declared = &statement[..PROGRAM_VK_LEN];
        if !leaf_key && declared != rootc {
            return Err(CommonError::InvalidProof(format!(
                "{name} proof declares recursion domain {declared:?} but verifies under \
                 {rootc:?}; pass a non-release leaf key with with_leaf_setup_vk \
                 (`--leaf-setup-vk`)"
            )));
        }

        // snarkjs uses only these bytes; `public_bytes` is unused.
        Ok(snark_publics_hash(&statement, rootc))
    }

    /// The vadcop_final key to verify under: the caller's, else the release constant for a
    /// leaf of this family and stage. Never the copy the proof carries, which would make
    /// verification self-keyed.
    fn setup_vk_to_verify_under(
        &self,
        hash: &str,
        compressed: bool,
        aggregate_domain: Option<&[u64]>,
    ) -> Result<std::borrow::Cow<'a, [u64]>> {
        if let Some(vk) = self.trusted_setup_vk {
            if self.leaf_setup_vk && aggregate_domain.is_some() {
                return Err(CommonError::InvalidProof(
                    "an aggregated proof cannot verify under a leaf setup key; pass its \
                     recurser's key with with_setup_vk (`--setup-vk`)"
                        .to_string(),
                ));
            }
            return Ok(std::borrow::Cow::Borrowed(vk));
        }
        if let Some(domain) = aggregate_domain {
            return Err(CommonError::InvalidProof(format!(
                "a recurser proof verifies under its recurser's key, which no release \
                 publishes; pass the verkey of the recurser you trust with with_setup_vk \
                 (`--setup-vk`). The proof claims {domain:?}"
            )));
        }
        zisk_verifier::vadcop_vk(hash, compressed).map(|k| k.to_vec().into()).ok_or_else(|| {
            CommonError::InvalidProof(format!(
                "no published vadcop_final key for {hash:?} (compressed: {compressed}); pass \
                 one with with_setup_vk"
            ))
        })
    }

    /// Verify the proof using the configured parameters.
    ///
    /// This method uses the overridden values if provided, otherwise falls back
    /// to the values stored in the proof, except for the setup key: see
    /// [`with_setup_vk`](Self::with_setup_vk).
    ///
    /// # Errors
    ///
    /// - [`CommonError::NotVerified`] if the proof is well-formed but does not verify.
    /// - [`CommonError::InvalidProof`] if the proof is malformed or its hash family
    ///   does not match the verification key.
    /// - [`CommonError::Serialization`] if the PLONK key cannot be serialized (Plonk).
    pub fn verify(self) -> Result<()> {
        // A successful verify() is the signal callers trust before reading
        // `publics()`, and `Proof::new` builds bodies the ingest checks never see.
        ensure_stored_body(&self.proof_with_values.body)?;

        let program_vk = self.override_program_vk.unwrap_or(&self.proof_with_values.program_vk);

        // Spliced into the committed publics below, so a short one would shear the rest of
        // the statement out of place (or panic `public_u64`'s fixed-offset reads).
        if let Some(pv) = self.override_program_vk {
            if pv.vk.len() != PROGRAM_VK_LEN {
                return Err(CommonError::InvalidProof(format!(
                    "program vk override must have exactly {PROGRAM_VK_LEN} u64 limbs, got {}",
                    pv.vk.len()
                )));
            }
            // The field verifier reads x and x+p as one element, so a non-canonical limb
            // would match a key it does not equal. Pinning a key means pinning its bytes.
            if !zisk_verifier::publics_are_canonical(&pv.vk) {
                return Err(CommonError::InvalidProof(
                    "program vk override has a non-canonical Goldilocks limb".to_string(),
                ));
            }
        }
        if let Some(vk) = self.trusted_setup_vk {
            if vk.len() != PROGRAM_VK_LEN {
                return Err(CommonError::InvalidProof(format!(
                    "setup vk must have exactly {PROGRAM_VK_LEN} u64 limbs, got {}",
                    vk.len()
                )));
            }
            if !zisk_verifier::publics_are_canonical(vk) {
                return Err(CommonError::InvalidProof(
                    "setup vk override has a non-canonical Goldilocks limb".to_string(),
                ));
            }
        }
        if let Some(pv) = self.override_publics {
            if pv.data.len() != ZISK_PUBLICS * 4 {
                return Err(CommonError::InvalidProof(format!(
                    "publics override must be {} bytes ({ZISK_PUBLICS} u32 values), got {}",
                    ZISK_PUBLICS * 4,
                    pv.data.len()
                )));
            }
        }

        match &self.proof_with_values.body {
            ProofBody::Plonk { proof_bytes, publics_full, rootc: stored_rootc, .. } => {
                if self.trusted_pilfflonk_vkey.is_some() {
                    return Err(CommonError::InvalidProof(
                        "a pilfflonk verification key was pinned (with_pilfflonk_vkey) for a PLONK proof"
                            .to_string(),
                    ));
                }
                let public_snark_bytes =
                    self.snark_public_digest(publics_full, stored_rootc, "PLONK")?;
                let hash = self.proof_with_values.program_vk.hash_mode.as_str();

                let plonk_vkey = match self.trusted_plonk_vk {
                    Some(vkey) => vkey,
                    None => release_plonk_vkey(hash).ok_or_else(|| {
                        CommonError::InvalidProof(format!(
                            "no published PLONK key for {hash:?}; pass one with with_plonk_vk \
                             (`--plonk-vk`)"
                        ))
                    })?,
                };

                let snark_proof = SnarkProof {
                    proof_bytes: proof_bytes.clone(),
                    public_bytes: Vec::new(),
                    public_snark_bytes: public_snark_bytes.clone(),
                    protocol_id: SnarkProtocol::Plonk.protocol_id(),
                };
                let plonk_vkey_json = serde_json::to_string(plonk_vkey)
                    .map_err(|e| CommonError::Serialization(format!("PlonkVkey to JSON: {e}")))?;
                // The digest is the statement recomputed above, so pin it.
                verify_snark_proof_with_vkey(
                    &snark_proof,
                    &plonk_vkey_json,
                    Some(&public_snark_bytes),
                )
                .map_err(|e| match e {
                    // The proof was checked and rejected.
                    ProofmanError::InvalidProof(reason) => {
                        tracing::warn!("PLONK proof not verified: {reason}");
                        if self.trusted_plonk_vk.is_none() {
                            tracing::warn!(
                                "a PLONK proof from a non-release setup verifies under that \
                                 setup's final.verkey.json (`--plonk-vk`)"
                            );
                        }
                        CommonError::NotVerified
                    }
                    // snarkjs could not run (missing, no temp dir, ...): nothing was checked.
                    e => CommonError::Invalid(format!("PLONK verification could not run: {e}")),
                })
            }
            ProofBody::Pilfflonk { proof_bytes, publics_full, rootc: stored_rootc, .. } => {
                if self.trusted_plonk_vk.is_some() {
                    return Err(CommonError::InvalidProof(
                        "a PLONK verification key was pinned (with_plonk_vk) for a pilfflonk proof"
                            .to_string(),
                    ));
                }
                let public_snark_bytes =
                    self.snark_public_digest(publics_full, stored_rootc, "pilfflonk")?;
                // The key the proof carries is metadata only, as for PLONK.
                let vkey_json = self.trusted_pilfflonk_vkey.ok_or_else(|| {
                    CommonError::InvalidProof(
                        "no published pilfflonk key; pass one with with_pilfflonk_vkey \
                         (`--pilfflonk-vk`)"
                            .to_string(),
                    )
                })?;

                let snark_proof = SnarkProof {
                    proof_bytes: proof_bytes.clone(),
                    public_bytes: Vec::new(),
                    public_snark_bytes: public_snark_bytes.clone(),
                    protocol_id: SnarkProtocol::Pilfflonk.protocol_id(),
                };
                // The digest is the statement recomputed above, so pin it.
                verify_snark_proof_with_vkey(&snark_proof, vkey_json, Some(&public_snark_bytes))
                    .map_err(|e| {
                        tracing::debug!("snark proof verification failed: {e}");
                        CommonError::NotVerified
                    })
            }
            ProofBody::Vadcop { proof, kind, hash, publics_full, zisk_vk } => {
                let kind = *kind;

                // A pinned PLONK key can't gate a non-PLONK proof; the Vadcop path would
                // ignore it, so reject rather than silently verify as if it were pinned.
                if self.trusted_plonk_vk.is_some() {
                    return Err(CommonError::InvalidProof(
                        "a PLONK verification key was pinned (with_plonk_vk) for a non-PLONK proof"
                            .to_string(),
                    ));
                }
                if self.trusted_pilfflonk_vkey.is_some() {
                    return Err(CommonError::InvalidProof(
                        "a pilfflonk verification key was pinned (with_pilfflonk_vkey) for a \
                         non-pilfflonk proof"
                            .to_string(),
                    ));
                }

                if program_vk.hash_mode.as_str() != hash {
                    return Err(CommonError::InvalidProof(format!(
                        "verkey hash mode {} does not match proof hash family {hash:?}",
                        program_vk.hash_mode.as_str()
                    )));
                }

                let declared = &program_publics(publics_full)[..PROGRAM_VK_LEN];
                let setup_vk = self.setup_vk_to_verify_under(
                    hash,
                    kind.is_minimal(),
                    (kind == VadcopKind::Recurser).then_some(declared),
                )?;
                let setup_vk: &[u64; PROGRAM_VK_LEN] = setup_vk.as_ref().try_into().unwrap();

                // The STARK transcript is over `[flag? | program_vk | inputs]`.
                let pubs_u64 = kind.stark_publics(&self.statement(publics_full));
                let vadcop_final_proof =
                    VadcopFinalProof::new(proof.clone(), pubs_u64, kind.is_minimal(), hash.clone());

                // The one acceptance policy, shared with guests: shape, canonical publics,
                // leaf flag, an aggregate's domain (which makes a recurser's allow-list
                // transitive over the fold tree), u32 leaf outputs, then the STARK.
                match zisk_verifier::verify(&vadcop_final_proof, setup_vk) {
                    Ok(_) => Ok(()),
                    Err(zisk_verifier::VerifyError::InvalidProof) => {
                        // The claimed key is untrusted; it only tells a genuine proof from
                        // another setup apart from a broken one.
                        if zisk_vk.as_slice() != setup_vk.as_slice() {
                            if let Some(hint) =
                                hint_setup_vk(self.trusted_setup_vk, kind.is_minimal())
                            {
                                tracing::warn!("{hint}");
                            }
                        }
                        Err(CommonError::NotVerified)
                    }
                    Err(e) => Err(CommonError::InvalidProof(format!(
                        "{:?} proof: {e}",
                        self.proof_with_values.kind()
                    ))),
                }
            }
        }
    }
}

impl Proof {
    /// Creates a new `Proof` from a body and program verification key, with no seed.
    ///
    /// When the proof attests to an execution, chain [`Proof::with_random_seed`] to keep its
    /// seed; otherwise a proof saved from it cannot be replayed.
    pub fn new(body: ProofBody, program_vk: ProgramVK) -> Self {
        Self { body, program_vk, random_seed: None }
    }

    /// Sets the seed of the execution this proof attests to. Takes an `Option` too, so callers
    /// can pass through a seed they may not have.
    pub fn with_random_seed(mut self, seed: impl Into<Option<RandomSeed>>) -> Self {
        self.random_seed = seed.into();
        self
    }

    /// The u32 `PublicValues` view, derived from the body's native publics.
    ///
    /// Truncates each full-width public to its low 32 bits (the guest/Solidity ABI).
    ///
    /// Derived from `publics_full` for both flavors. The stored `Plonk.publics` copy is
    /// deserialized independently of the statement `verify()` actually hashes, so
    /// returning it would let a modified proof verify while reporting other outputs.
    ///
    /// **Only meaningful on a proof that passed [`Proof::load`] or [`Proof::verify`].** A
    /// verification with [`with_publics`](Proof::with_publics) or
    /// [`with_program_vk`](Proof::with_program_vk) verifies the caller's values, not these:
    /// read the overrides back, not this view.
    /// A misshapen body has no committed statement to report, and what comes back is
    /// zero-filled — indistinguishable from a proof whose outputs really are zero. It is
    /// not evidence of anything. Reach for [`Proof::try_publics`] whenever the proof's
    /// provenance is unknown; it rejects such a body instead of answering.
    ///
    /// The buffer stays full width rather than empty so that `public_u64`, `read` and
    /// `read_slice` keep their fixed-offset reads in bounds.
    pub fn publics(&self) -> PublicValues {
        let committed = program_publics(self.committed_publics());
        if committed.len() == PROGRAM_VK_LEN + ZISK_PUBLICS {
            return PublicValues::new_from_u64(committed);
        }
        let mut data = [0u8; ZISK_PUBLICS * 4];
        for (i, &val) in committed.iter().skip(PROGRAM_VK_LEN).take(ZISK_PUBLICS).enumerate() {
            data[i * 4..(i + 1) * 4].copy_from_slice(&(canonical(val) as u32).to_le_bytes());
        }
        PublicValues { data: data.to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// [`Proof::publics`], but rejects a body whose committed publics are not a
    /// well-formed, canonical `[program_vk(4) | inputs(ZISK_PUBLICS)]` vector.
    ///
    /// `Proof::new` and the raw bincode decode paths build bodies the ingest checks never
    /// see, so this is the accessor to reach for when the proof's provenance is unknown.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] if the committed publics are the wrong
    /// length or hold a non-canonical Goldilocks element.
    pub fn try_publics(&self) -> Result<PublicValues> {
        let committed = self.committed_publics();
        ensure_canonical_publics(committed)?;
        Ok(PublicValues::new_from_u64(committed))
    }

    /// The full-width (u64) publics `[program_vk(4)][user(ZISK_PUBLICS)]` for a
    /// Vadcop proof — the untruncated field elements the proof committed to.
    /// Used by the recursion round-trip; `None` for Plonk (no u64 form exists).
    pub fn publics_full(&self) -> Option<&[u64]> {
        match &self.body {
            ProofBody::Vadcop { publics_full, .. } => Some(publics_full),
            ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. } => None,
        }
    }

    /// Derive the `ProofKind` from the body discriminant.
    pub fn kind(&self) -> ProofKind {
        match &self.body {
            ProofBody::Vadcop { kind: VadcopKind::Minimal, .. } => ProofKind::VadcopFinalMinimal,
            ProofBody::Vadcop { .. } => ProofKind::VadcopFinal,
            ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. } => ProofKind::Plonk,
        }
    }

    /// Whether the underlying proof payload is empty (used to detect non-prove flows).
    pub fn is_empty(&self) -> bool {
        match &self.body {
            ProofBody::Vadcop { proof, .. } => proof.is_empty(),
            ProofBody::Plonk { proof_bytes, .. } | ProofBody::Pilfflonk { proof_bytes, .. } => {
                proof_bytes.is_empty()
            }
        }
    }

    /// The bytes [`save`](Self::save) writes.
    ///
    /// A Vadcop proof is ZisK's flat layout, [`get_proof_bytes`](Self::get_proof_bytes):
    /// the same bytes guests, `zisk_verifier::decode_saved` and ethproofs take. A PLONK
    /// proof is [`PLONK_FILE_MAGIC`] followed by the bincode `Proof`.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] if a Vadcop proof's publics are misshapen or
    /// non-canonical, or its `program_vk` is not the one its publics and hash family commit
    /// to (the flat layout derives it from them); otherwise the errors of
    /// [`get_proof_u64`](Self::get_proof_u64).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        match &self.body {
            ProofBody::Vadcop { hash, .. } => {
                // A flagged or non-canonical vector would be written as a file `load` refuses.
                ensure_stored_body(&self.body)?;
                self.ensure_program_vk_committed()?;
                if self.program_vk.hash_mode.as_str() != hash {
                    return Err(CommonError::InvalidProof(
                        "program_vk's hash mode differs from the proof's hash family, which the \
                         saved form cannot carry"
                            .to_string(),
                    ));
                }
                self.get_proof_bytes()
            }
            ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. } => {
                ensure_stored_body(&self.body)?;
                self.ensure_program_vk_committed()?;
                let mut bytes = PLONK_FILE_MAGIC.to_vec();
                bytes.extend(
                    bincode::serde::encode_to_vec(self, bincode::config::standard())
                        .map_err(|e| CommonError::Io(format!("Failed to encode proof: {e}")))?,
                );
                Ok(bytes)
            }
        }
    }

    /// `program_vk` is a public field; a saved proof must not carry one that differs from the
    /// program VK its publics commit to.
    fn ensure_program_vk_committed(&self) -> Result<()> {
        if self.program_vk.vk.as_slice()
            != &program_publics(self.committed_publics())[..PROGRAM_VK_LEN]
        {
            return Err(CommonError::InvalidProof(
                "program_vk differs from the one the publics commit to".to_string(),
            ));
        }
        Ok(())
    }

    /// Decode [`to_bytes`](Self::to_bytes) output.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] if the bytes are neither a well-formed flat
    /// Vadcop proof nor a PLONK file.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if let Some(rest) = bytes.strip_prefix(PLONK_FILE_MAGIC) {
            let (proof, read): (Proof, usize) = bincode::serde::decode_from_slice(
                rest,
                bincode::config::standard().with_limit::<PLONK_FILE_MAX_BYTES>(),
            )
            .map_err(|e| CommonError::InvalidProof(format!("Malformed PLONK proof: {e}")))?;
            if read != rest.len() {
                return Err(CommonError::InvalidProof(format!(
                    "{} trailing bytes after the PLONK proof",
                    rest.len() - read
                )));
            }
            if !matches!(proof.body, ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. }) {
                return Err(CommonError::InvalidProof(
                    "a PLONK file must hold a SNARK proof; Vadcop proofs are saved flat"
                        .to_string(),
                ));
            }
            // bincode will happily decode a non-canonical or misshapen `publics_full`.
            ensure_stored_body(&proof.body)?;
            proof.ensure_program_vk_committed()?;
            return Ok(proof);
        }

        let saved = zisk_verifier::decode_saved(bytes)
            .map_err(|e| CommonError::InvalidProof(format!("Malformed proof: {e}")))?;
        let VadcopFinalProof { proof, public_values, compressed, hash } = saved.proof;
        // `decode_saved` pins the flag to 0 or 1.
        let kind = match (compressed, public_values.first()) {
            (true, _) => VadcopKind::Minimal,
            (false, Some(&IS_VADCOP_FINAL_PROOF)) => VadcopKind::Final,
            (false, _) => VadcopKind::Recurser,
        };
        let publics_full = program_publics(&public_values).to_vec();
        let mode = hash.parse().map_err(|e| CommonError::InvalidProof(format!("{e}")))?;
        let program_vk = ProgramVK::new_from_publics_with_mode(&publics_full, mode);
        Ok(Proof::new(
            ProofBody::Vadcop {
                proof,
                zisk_vk: saved.claimed_vadcop_vk.to_vec(),
                kind,
                hash,
                publics_full,
            },
            program_vk,
        ))
    }

    /// Save the proof to a file: [`to_bytes`](Self::to_bytes).
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Io`] if the parent directory or file cannot be created or
    /// written, or the errors of [`to_bytes`](Self::to_bytes).
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let bytes = self.to_bytes()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CommonError::Io(format!(
                    "failed to create parent directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        std::fs::write(path, bytes).map_err(|e| {
            CommonError::Io(format!("failed to save proof to {}: {e}", path.display()))
        })
    }

    /// Load a proof saved by [`save`](Self::save): [`from_bytes`](Self::from_bytes).
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Io`] if the file cannot be read, or the errors of
    /// [`from_bytes`](Self::from_bytes).
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| {
            CommonError::Io(format!(
                "failed to open file for loading proof: {}: {e}",
                path.display()
            ))
        })?;
        Self::from_bytes(&bytes)
    }

    /// The committed `publics_full` (`[program_vk | inputs]`) for either flavor —
    /// the single source of truth the getters and verification derive from.
    fn committed_publics(&self) -> &[u64] {
        match &self.body {
            ProofBody::Vadcop { publics_full, .. }
            | ProofBody::Plonk { publics_full, .. }
            | ProofBody::Pilfflonk { publics_full, .. } => publics_full,
        }
    }

    /// Extract a `VadcopFinalProof` from the proof body.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] if the proof is not a Vadcop final proof.
    pub fn get_vadcop_final_proof(&self) -> Result<VadcopFinalProof> {
        match &self.body {
            ProofBody::Vadcop { proof, kind, hash, publics_full, .. } => {
                // `stark_publics` re-adds the flag, so a stored vector that already
                // carries one would come back 70 wide. Vadcop storage is flag-free by
                // construction, but `Proof::new` and raw deserialization never check
                // that, and this conversion feeds proofman directly.
                ensure_stored_body(&self.body)?;

                // The STARK layer commits to the full-width publics INCLUDING the
                // is_vadcop_final_proof flag; `kind.stark_publics` re-adds it to
                // the canonical flag-free `publics_full` (full u64 width — the
                // truncated u32 view would break re-verification).
                Ok(VadcopFinalProof::new(
                    proof.clone(),
                    kind.stark_publics(publics_full),
                    kind.is_minimal(),
                    hash.clone(),
                ))
            }
            ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. } => {
                Err(CommonError::InvalidProof("Proof is not a Vadcop final proof".to_string()))
            }
        }
    }

    /// The `VadcopFinalProof` to fold into a recursion step.
    ///
    /// Refuses a compressed proof. `FinalCompressed` strips the `is_vadcop_final_proof`
    /// flag, so the recurser — which reads slot 0 as that flag and expects the 69-word
    /// layout — would take the first program-VK limb for the flag and shift the whole
    /// statement by one. A limb of 0 or 1 makes that misread silent. Compression is a
    /// terminal step, not a foldable one.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] if the proof is compressed, or is not a
    /// Vadcop final proof.
    pub fn get_vadcop_final_proof_to_aggregate(&self) -> Result<VadcopFinalProof> {
        if let ProofBody::Vadcop { kind, .. } = &self.body {
            if kind.is_minimal() {
                return Err(CommonError::InvalidProof(
                    "a compressed (minimal) proof cannot be aggregated: compression strips \
                     the is_vadcop_final_proof flag the aggregator reads at public slot 0, \
                     and it cannot be recovered. Produce the leaf uncompressed \
                     (ProofKind::VadcopFinal) if you intend to fold it — note the embedded \
                     client defaults to ProofKind::VadcopFinalMinimal"
                        .to_string(),
                ));
            }
        }
        self.get_vadcop_final_proof()
    }

    /// Get the proof data as a vector of u64 values.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] if the program or Zisk verification key
    /// has an unexpected length, or if the proof is not a Vadcop proof.
    pub fn get_proof_u64(&self) -> Result<Vec<u64>> {
        match &self.body {
            ProofBody::Vadcop { proof, zisk_vk, kind, hash, publics_full } => {
                if self.program_vk.vk.len() != PROGRAM_VK_LEN {
                    return Err(CommonError::InvalidProof(format!(
                        "Invalid program_vk length: expected {}, got {}",
                        PROGRAM_VK_LEN,
                        self.program_vk.vk.len()
                    )));
                }
                if zisk_vk.len() != PROGRAM_VK_LEN {
                    return Err(CommonError::InvalidProof(format!(
                        "Invalid zisk_vk length: expected {}, got {}",
                        PROGRAM_VK_LEN,
                        zisk_vk.len()
                    )));
                }

                // The serialized STARK public vector must carry the
                // is_vadcop_final_proof flag (its Fiat-Shamir transcript is over
                // the full [flag? | vk | inputs]); `kind.stark_publics` re-adds it
                // to the canonical flag-free `publics_full`, at full u64 width (no
                // u32 truncation). Minimal proofs stay flag-free (68).
                let stark_publics = kind.stark_publics(publics_full);
                let n_publics = stark_publics.len();

                // The family travels with the proof so a reader needs no side channel to
                // learn which verifier to run. Routing metadata, not authority: a wrong
                // tag fails against the reader's expected verification key.
                let tag = zisk_verifier::hash_tag(hash).ok_or_else(|| {
                    CommonError::InvalidProof(format!("unrecognized proof hash family {hash:?}"))
                })?;

                // Format: [minimal(1)][n_publics(1)][flag?|vk|inputs][proof][zisk_vk(4)][tag(1)]
                let mut words =
                    Vec::with_capacity(2 + n_publics + proof.len() + zisk_vk.len() + 1);
                words.push(kind.is_minimal() as u64);
                words.push(n_publics as u64);
                words.extend_from_slice(&stark_publics);
                words.extend_from_slice(proof);
                words.extend_from_slice(zisk_vk);
                words.push(tag);

                Ok(words)
            }
            ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. } => Err(CommonError::InvalidProof(
                "Proof not suitable for get_proof_u64. Only VadcopFinal and VadcopFinalMinimal proofs are supported.".to_string()
            )),
        }
    }

    /// Get the proof data as a vector of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::InvalidProof`] under the same conditions as
    /// [`get_proof_u64`](Self::get_proof_u64), which this method builds upon.
    pub fn get_proof_bytes(&self) -> Result<Vec<u8>> {
        let words = self.get_proof_u64()?;
        let mut bytes = Vec::with_capacity(words.len() * 8);
        for w in &words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        Ok(bytes)
    }

    /// Returns the u32 `PublicValues` view of this proof's publics.
    pub fn get_publics(&self) -> PublicValues {
        self.publics()
    }

    /// The program verification key committed by this proof.
    ///
    /// Derived from the committed `publics_full` (the ROM-root limbs, reduced to canonical
    /// representatives) rather than the stored `program_vk` copy, which is untrusted
    /// metadata. `hash_mode` is carried through unchanged.
    pub fn get_program_vk(&self) -> ProgramVK {
        // `take` rather than `[..PROGRAM_VK_LEN]`: this getter is reachable on an
        // unverified proof (verify()/load() enforce the length, but a hand-built `Proof`
        // bypasses them), so a short committed vector must not panic.
        ProgramVK {
            vk: program_publics(self.committed_publics())
                .iter()
                .take(PROGRAM_VK_LEN)
                .map(|&w| canonical(w))
                .collect(),
            hash_mode: self.program_vk.hash_mode,
        }
    }

    /// Create Proof directly from a Vadcop proof u64 array.
    ///
    /// This method parses the proof format (n_publics, publics..., proof...) and extracts
    /// the public values and program VK directly, without creating an intermediate VadcopFinalProof.
    ///
    /// # Parameters
    ///
    /// * `proof` - The proof as a slice of u64 values
    /// * `minimal` - Whether the proof is minimal
    /// * `zisk_vk` - The Vadcop verification key (4 u64s)
    /// * `hash` - Hash family the proof was generated with (e.g. "Poseidon1" / "Poseidon2")
    ///
    /// # Returns
    ///
    /// A Proof containing the parsed proof, publics, and program VK, with no seed: the flat
    /// u64 form (as produced by [`Proof::get_proof_u64`]) does not carry one. When the proof
    /// attests to an execution, chain [`Proof::with_random_seed`] to keep its seed; otherwise a
    /// proof saved from it cannot be replayed.
    ///
    /// # Errors
    ///
    /// - [`CommonError::InvalidProof`] if `zisk_vk` has an unexpected length or the
    ///   proof bytes cannot be parsed.
    /// - [`CommonError::Invalid`] if `hash` is not a recognized proof hash family.
    pub fn new_from_vadcop_proof(
        proof: &[u64],
        minimal: bool,
        zisk_vk: Vec<u64>,
        hash: String,
    ) -> Result<Self> {
        if zisk_vk.len() != PROGRAM_VK_LEN {
            return Err(CommonError::InvalidProof(format!(
                "Invalid zisk_vk length: expected {}, got {}",
                PROGRAM_VK_LEN,
                zisk_vk.len()
            )));
        }

        // `from_str` is case-insensitive; downstream comparison and dispatch are exact
        // string matches, so keep the canonical spelling rather than the caller's.
        let hash_mode = hash.parse::<HashMode>().map_err(|e| {
            CommonError::Invalid(format!("unrecognized proof hash family {hash:?}: {e}"))
        })?;
        let hash = hash_mode.as_str().to_string();

        // `new_from_proof` accepts any count that fits the slice, and a short one then
        // trips `ProgramVK::new_from_publics_with_mode`'s assert. Pin the stage's exact
        // width first so malformed input is an `InvalidProof`, not a panic.
        let expected_n_publics = zisk_verifier::expected_n_publics(minimal);
        match proof.first() {
            Some(&n) if n == expected_n_publics as u64 => {}
            Some(&n) => {
                return Err(CommonError::InvalidProof(format!(
                    "proof declares {n} publics, expected {expected_n_publics} for this stage"
                )))
            }
            None => {
                return Err(CommonError::InvalidProof(
                    "Vadcop proof is empty, cannot read its public count".to_string(),
                ))
            }
        }

        let vadcop_proof =
            VadcopFinalProof::new_from_proof(proof, minimal, hash.clone()).map_err(|e| {
                CommonError::InvalidProof(format!("Failed to parse Vadcop proof: {}", e))
            })?;

        ensure_canonical_publics(&vadcop_proof.public_values)?;

        let program_vk =
            ProgramVK::new_from_publics_with_mode(&vadcop_proof.public_values, hash_mode);

        // Classify by the raw publics, then normalize ONCE to the flag-free
        // `[vk | inputs]` view. A minimal proof is already flag-free; a
        // Final/Recurser proof carries the `is_vadcop_final_proof` flag at index
        // 0, which is captured in `kind` and stripped from stored `publics_full`.
        let kind = if minimal {
            VadcopKind::Minimal
        } else {
            // Not `from_publics_full`: it maps every non-zero flag to `Final`, so a flag
            // of 2 would be accepted and silently change the committed statement.
            match vadcop_proof.public_values[0] {
                0 => VadcopKind::Recurser,
                IS_VADCOP_FINAL_PROOF => VadcopKind::Final,
                other => {
                    return Err(CommonError::InvalidProof(format!(
                        "is_vadcop_final_proof must be 0 or {IS_VADCOP_FINAL_PROOF}, got {other}"
                    )))
                }
            }
        };
        let publics_full = program_publics(&vadcop_proof.public_values).to_vec();

        Ok(Self {
            body: ProofBody::Vadcop {
                proof: vadcop_proof.proof,
                zisk_vk,
                kind,
                hash,
                // Canonical flag-free `[program_vk(4) | inputs(64)]` (68), full
                // u64 width. The flag lives in `kind`; STARK/serialization paths
                // re-add it via `kind.flag()`.
                publics_full,
            },
            program_vk,
            random_seed: None,
        })
    }

    /// Verify the proof using the default values stored in this instance.
    ///
    /// For custom verification with overridden values, use the builder methods:
    /// - `with_publics()` to override public values
    /// - `with_program_vk()` to override program verification key
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Default verification
    /// proof.verify()?;
    ///
    /// // Custom verification with overridden publics
    /// proof.with_publics(&custom_publics).verify()?;
    ///
    /// // Custom verification with multiple overrides
    /// proof.with_publics(&custom_publics).with_program_vk(&custom_program_vk).verify()?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`ZiskVerifyBuilder::verify`], which this method delegates to.
    pub fn verify(&self) -> Result<()> {
        ZiskVerifyBuilder::new(self).verify()
    }

    /// Start a custom verification with no overrides applied yet.
    pub fn verify_builder(&self) -> ZiskVerifyBuilder<'_> {
        ZiskVerifyBuilder::new(self)
    }

    /// Start building a custom verification by overriding the public values.
    ///
    /// Returns a builder that allows chaining additional overrides before calling `verify()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// proof.with_publics(&custom_publics).verify()?;
    /// proof.with_publics(&custom_publics).with_program_vk(&custom_program_vk).verify()?;
    /// ```
    pub fn with_publics<'a>(&'a self, publics: &'a PublicValues) -> ZiskVerifyBuilder<'a> {
        ZiskVerifyBuilder::new(self).with_publics(publics)
    }

    /// Start building a custom verification by overriding the program verification key.
    ///
    /// Returns a builder that allows chaining additional overrides before calling `verify()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// proof.with_program_vk(&custom_program_vk).verify()?;
    /// proof.with_program_vk(&custom_program_vk).with_publics(&custom_publics).verify()?;
    /// ```
    pub fn with_program_vk<'a>(&'a self, program_vk: &'a ProgramVK) -> ZiskVerifyBuilder<'a> {
        ZiskVerifyBuilder::new(self).with_program_vk(program_vk)
    }

    /// Start a custom verification with the trusted PLONK circuit key. See
    /// [`ZiskVerifyBuilder::with_plonk_vk`].
    pub fn with_plonk_vk<'a>(&'a self, plonk_vkey: &'a PlonkVkey) -> ZiskVerifyBuilder<'a> {
        ZiskVerifyBuilder::new(self).with_plonk_vk(plonk_vkey)
    }

    /// Start a custom verification with the trusted pilfflonk key (the JSON of its
    /// `pilfflonk.vkey.json`). See [`ZiskVerifyBuilder::with_pilfflonk_vkey`].
    pub fn with_pilfflonk_vkey<'a>(&'a self, vkey_json: &'a str) -> ZiskVerifyBuilder<'a> {
        ZiskVerifyBuilder::new(self).with_pilfflonk_vkey(vkey_json)
    }

    /// Start a custom verification with the trusted recursion setup key. See
    /// [`ZiskVerifyBuilder::with_setup_vk`].
    pub fn with_setup_vk<'a>(&'a self, setup_vk: &'a [u64]) -> ZiskVerifyBuilder<'a> {
        ZiskVerifyBuilder::new(self).with_setup_vk(setup_vk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_returns_err_for_malformed_vadcop_final_minimal() {
        let result = Proof::new(
            ProofBody::Vadcop {
                proof: vec![],
                zisk_vk: vec![0u64; PROGRAM_VK_LEN],
                kind: VadcopKind::Minimal,
                hash: "Poseidon2".to_string(),
                publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
            },
            ProgramVK::new_empty(),
        )
        .verify();

        assert!(result.is_err(), "expected Err for malformed proof, got {:?}", result);
    }

    #[test]
    fn verify_returns_err_for_malformed_vadcop_final() {
        let result = Proof::new(
            ProofBody::Vadcop {
                proof: vec![],
                zisk_vk: vec![0u64; PROGRAM_VK_LEN],
                kind: VadcopKind::Final,
                hash: "Poseidon2".to_string(),
                publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
            },
            ProgramVK::new_empty(),
        )
        .verify();

        assert!(result.is_err(), "expected Err for malformed proof, got {:?}", result);
    }

    /// `[n_publics][publics][proof]`, the layout `new_from_vadcop_proof` ingests.
    fn serialized_vadcop(publics: &[u64]) -> Vec<u64> {
        let mut v = vec![publics.len() as u64];
        v.extend_from_slice(publics);
        v.extend_from_slice(&[0u64; 8]);
        v
    }

    /// A short count would otherwise reach `ProgramVK::new_from_publics_with_mode`,
    /// whose assert panics instead of returning the documented `InvalidProof`.
    #[test]
    fn new_from_vadcop_proof_rejects_a_wrong_public_count() {
        let err = Proof::new_from_vadcop_proof(
            &serialized_vadcop(&[1, 2]),
            false,
            vec![1, 2, 3, 4],
            "Poseidon2".to_string(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("declares 2 publics"), "got: {err}");
    }

    /// `from_publics_full` maps every non-zero flag to `Final`; an out-of-range flag must
    /// be refused rather than silently reinterpreted.
    #[test]
    fn new_from_vadcop_proof_rejects_an_out_of_range_flag() {
        let mut publics = vec![0u64; VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN + ZISK_PUBLICS];
        publics[0] = 2;
        let err = Proof::new_from_vadcop_proof(
            &serialized_vadcop(&publics),
            false,
            vec![1, 2, 3, 4],
            "Poseidon2".to_string(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("is_vadcop_final_proof"), "got: {err}");
    }

    #[test]
    fn new_from_vadcop_proof_rejects_a_non_canonical_public() {
        let mut publics = vec![0u64; VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN + ZISK_PUBLICS];
        publics[0] = IS_VADCOP_FINAL_PROOF;
        publics[VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN] = GOLDILOCKS_ORDER;
        let err = Proof::new_from_vadcop_proof(
            &serialized_vadcop(&publics),
            false,
            vec![1, 2, 3, 4],
            "Poseidon2".to_string(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("canonical"), "got: {err}");
    }

    /// `verify()` hashes `publics_full`, so the reported view must come from there and
    /// not from the independently deserialized `Plonk.publics` copy.
    #[test]
    fn plonk_publics_come_from_the_committed_vector() {
        let mut publics_full = vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS];
        publics_full[PROGRAM_VK_LEN] = 0xAABB;

        // A stored copy that disagrees with the committed statement.
        let mut lying = PublicValues::new_empty();
        lying.data[0..4].copy_from_slice(&0xDEADu32.to_le_bytes());

        let proof = Proof::new(
            ProofBody::Plonk {
                proof_bytes: vec![],
                plonk_vk: Box::new(PlonkVkBlob {
                    vadcop_vk: vec![0u64; PROGRAM_VK_LEN],
                    plonk_vkey: dummy_plonk_vkey(),
                }),
                publics: lying,
                publics_full,
                rootc: vec![0u64; PROGRAM_VK_LEN],
            },
            ProgramVK::new_empty(),
        );

        assert_eq!(&proof.publics().data[0..4], &0xAABBu32.to_le_bytes());
    }

    /// An override is spliced into the verified statement, so the domain guard must read
    /// the overridden VK — otherwise it guards limbs nothing is verified against.
    #[test]
    fn recurser_domain_check_follows_the_program_vk_override() {
        let proof = vadcop_proof(VadcopKind::Recurser, flag_free_publics([1, 2, 3, 4]));
        let foreign = ProgramVK { vk: vec![9, 9, 9, 9], hash_mode: HashMode::Poseidon2 };
        let err =
            proof.with_program_vk(&foreign).with_setup_vk(&[1, 2, 3, 4]).verify().unwrap_err();
        assert!(err.to_string().contains("recursion domain"), "got: {err}");
    }

    /// Compression strips the flag the recurser reads at slot 0, so folding a compressed
    /// proof would take the first VK limb for the flag and shift the statement by one.
    #[test]
    fn a_compressed_proof_cannot_be_aggregated() {
        let minimal = vadcop_proof(VadcopKind::Minimal, flag_free_publics([1, 2, 3, 4]));
        let err = minimal.get_vadcop_final_proof_to_aggregate().unwrap_err();
        assert!(err.to_string().contains("cannot be aggregated"), "got: {err}");

        // The uncompressed flavors still fold, carrying the 69-word layout.
        for kind in [VadcopKind::Final, VadcopKind::Recurser] {
            let p = vadcop_proof(kind, flag_free_publics([1, 2, 3, 4]));
            let vfp = p.get_vadcop_final_proof_to_aggregate().unwrap();
            assert_eq!(
                vfp.public_values.len(),
                VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN + ZISK_PUBLICS
            );
        }
    }

    /// `publics()` must stay in bounds for a body that never passed load/verify, and
    /// `try_publics()` must refuse to answer for it at all.
    #[test]
    fn try_publics_rejects_what_publics_can_only_guess() {
        let misshapen = Proof::new(
            ProofBody::Plonk {
                proof_bytes: vec![],
                plonk_vk: Box::new(PlonkVkBlob {
                    vadcop_vk: vec![0u64; PROGRAM_VK_LEN],
                    plonk_vkey: dummy_plonk_vkey(),
                }),
                publics: PublicValues::new_empty(),
                publics_full: vec![1, 2],
                rootc: vec![0u64; PROGRAM_VK_LEN],
            },
            ProgramVK::new_empty(),
        );
        assert!(misshapen.try_publics().is_err());
        // Still safe to call, and its fixed-offset reads stay in bounds.
        assert_eq!(misshapen.publics().public_u64().len(), ZISK_PUBLICS);

        let ok = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        assert_eq!(ok.try_publics().unwrap().data, ok.publics().data);
    }

    /// A compressed blake3 proof of the wrong size is a malformed shape, not a
    /// failed statement, so it must not come back as `NotVerified`.
    #[test]
    fn verify_reports_a_misshapen_blake3_compressed_proof_as_malformed() {
        let mut proof = vadcop_proof(VadcopKind::Minimal, flag_free_publics([1, 2, 3, 4]));
        if let ProofBody::Vadcop { hash, .. } = &mut proof.body {
            *hash = "blake3".to_string();
        }
        proof.program_vk.hash_mode = HashMode::Blake3;

        let err = proof.verify().unwrap_err();
        assert!(matches!(err, CommonError::InvalidProof(_)), "got: {err:?}");
        assert!(err.to_string().contains("the stage's proofs are"), "got: {err}");
    }

    /// `stark_publics` re-adds the flag, so a stored vector that already carries one
    /// would reach proofman 70 wide. `Proof::new` bypasses the ingest checks.
    #[test]
    fn get_vadcop_final_proof_rejects_an_already_flagged_body() {
        let mut flagged = vec![0u64; VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN + ZISK_PUBLICS];
        flagged[0] = IS_VADCOP_FINAL_PROOF;
        let proof = vadcop_proof(VadcopKind::Final, flagged);

        let err = proof.get_vadcop_final_proof().unwrap_err();
        assert!(matches!(err, CommonError::InvalidProof(_)), "got: {err:?}");

        // A short vector must not reach proofman either.
        let short = vadcop_proof(VadcopKind::Final, vec![0u64; 3]);
        assert!(short.get_vadcop_final_proof().is_err());

        // The well-formed flag-free body still converts, and gains the flag exactly once.
        let ok = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let vfp = ok.get_vadcop_final_proof().unwrap();
        assert_eq!(vfp.public_values.len(), VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN + ZISK_PUBLICS);
        assert_eq!(vfp.public_values[0], IS_VADCOP_FINAL_PROOF);
    }

    /// A structurally valid (not cryptographically meaningful) PLONK vkey.
    fn dummy_plonk_vkey() -> PlonkVkey {
        let g1 = || ["0".to_string(), "0".to_string(), "1".to_string()];
        let g2 = || {
            [
                ["0".to_string(), "0".to_string()],
                ["0".to_string(), "0".to_string()],
                ["1".to_string(), "0".to_string()],
            ]
        };
        PlonkVkey {
            protocol: "plonk".to_string(),
            curve: "bn128".to_string(),
            n_public: 1,
            power: 1,
            k1: "2".to_string(),
            k2: "3".to_string(),
            qm: g1(),
            ql: g1(),
            qr: g1(),
            qo: g1(),
            qc: g1(),
            s1: g1(),
            s2: g1(),
            s3: g1(),
            x_2: g2(),
            w: "1".to_string(),
        }
    }

    /// A flag-free Vadcop proof whose publics are well-shaped, for the guard tests.
    fn vadcop_proof(kind: VadcopKind, publics_full: Vec<u64>) -> Proof {
        Proof::new(
            ProofBody::Vadcop {
                proof: vec![0u64; 8],
                zisk_vk: vec![1, 2, 3, 4],
                kind,
                hash: "Poseidon2".to_string(),
                publics_full,
            },
            ProgramVK::new_from_publics_with_mode(&[1, 2, 3, 4], HashMode::Poseidon2),
        )
    }

    fn flag_free_publics(vk: [u64; PROGRAM_VK_LEN]) -> Vec<u64> {
        let mut p = vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS];
        p[..PROGRAM_VK_LEN].copy_from_slice(&vk);
        p
    }

    /// The Vadcop path ignores a PLONK key, so pinning one must be refused rather than
    /// silently verified as if the pin had applied.
    #[test]
    fn verify_rejects_a_plonk_key_pinned_on_a_vadcop_proof() {
        let proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let vkey = dummy_plonk_vkey();
        let err = proof.with_plonk_vk(&vkey).verify().unwrap_err();
        assert!(err.to_string().contains("PLONK verification key was pinned"), "got: {err}");
    }

    /// The field verifier reads x and x+p as one element, so a non-canonical override
    /// would pin a key by value it does not equal byte-for-byte.
    #[test]
    fn verify_rejects_non_canonical_program_vk_override() {
        let proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let shifted =
            ProgramVK { vk: vec![1 + GOLDILOCKS_ORDER, 2, 3, 4], hash_mode: HashMode::Poseidon2 };
        let err = proof.with_program_vk(&shifted).verify().unwrap_err();
        assert!(err.to_string().contains("non-canonical"), "got: {err}");
    }

    #[test]
    fn verify_rejects_non_canonical_setup_vk_override() {
        let proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let err = proof.with_setup_vk(&[1 + GOLDILOCKS_ORDER, 2, 3, 4]).verify().unwrap_err();
        assert!(err.to_string().contains("non-canonical"), "got: {err}");
    }

    /// A fold must verify under the domain it declares, or a subtree from another
    /// recurser rides through.
    #[test]
    fn verify_rejects_a_recurser_proof_verifying_outside_its_domain() {
        let proof = vadcop_proof(VadcopKind::Recurser, flag_free_publics([9, 9, 9, 9]));
        let err = proof.with_setup_vk(&[1, 2, 3, 4]).verify().unwrap_err();
        assert!(err.to_string().contains("recursion domain"), "got: {err}");
    }

    /// A non-canonical public is refused on both sides: `save` will not write one, and
    /// `load` rejects a file that carries one anyway.
    #[test]
    fn load_rejects_a_non_canonical_stored_public() {
        let mut publics = flag_free_publics([1, 2, 3, 4]);
        publics[PROGRAM_VK_LEN] = GOLDILOCKS_ORDER;
        let proof = vadcop_proof(VadcopKind::Final, publics);
        assert!(proof.to_bytes().unwrap_err().to_string().contains("canonical"));

        let err = Proof::from_bytes(&proof.get_proof_bytes().unwrap()).unwrap_err();
        assert!(err.to_string().contains("canonical"), "got: {err}");
    }

    #[test]
    fn load_rejects_a_leaf_flag_other_than_0_or_1() {
        let proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let mut bytes = proof.get_proof_bytes().unwrap();
        bytes[16..24].copy_from_slice(&2u64.to_le_bytes());
        let err = Proof::from_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("leaf flag"), "got: {err}");
    }

    #[test]
    fn serialized_proof_carries_the_hash_tag_last() {
        let proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let words = proof.get_proof_u64().unwrap();
        assert_eq!(words.last().copied(), zisk_verifier::hash_tag("Poseidon2"));
    }

    /// A VK override keeps every other slot at full u64 width; rebuilding from the u32
    /// `PublicValues` view would truncate a recurser proof's inputs.
    #[test]
    fn a_program_vk_override_keeps_wide_publics() {
        let mut publics = flag_free_publics([1, 2, 3, 4]);
        publics[PROGRAM_VK_LEN] = 1 << 40;
        let proof = vadcop_proof(VadcopKind::Final, publics);
        let vk = ProgramVK { vk: vec![9, 9, 9, 9], hash_mode: HashMode::Poseidon2 };

        let statement = proof.with_program_vk(&vk).statement(proof.publics_full().unwrap());
        assert_eq!(&statement[..PROGRAM_VK_LEN], &[9, 9, 9, 9]);
        assert_eq!(statement[PROGRAM_VK_LEN], 1 << 40, "wide public must not be truncated");
    }

    /// A wrong-length `setup_vk` must return an error, not panic in
    /// `snark_publics_hash` (which slices on `PROGRAM_VK_LEN`).
    #[test]
    fn plonk_verify_rejects_wrong_len_setup_vk() {
        let proof = plonk_proof(flag_free_publics([0; 4]), vec![0; 4], HashMode::Poseidon1);
        // 3 limbs instead of PROGRAM_VK_LEN (4).
        let err = proof.with_setup_vk(&[1u64, 2, 3]).verify().unwrap_err();
        assert!(
            matches!(err, CommonError::InvalidProof(_)),
            "expected InvalidProof for wrong-length setup vk, got {err:?}"
        );
    }

    fn save_and_load(proof: &Proof) -> (Vec<u8>, Result<Proof>) {
        // Pilfflonk and PLONK share a kind, so the kind alone would race parallel tests.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp =
            std::env::temp_dir().join(format!("proof_roundtrip_{}_{n}.bin", std::process::id()));
        proof.save(&tmp).unwrap();
        let bytes = std::fs::read(&tmp).unwrap();
        let loaded = Proof::load(&tmp);
        std::fs::remove_file(&tmp).ok();
        (bytes, loaded)
    }

    /// One Vadcop format: what `save` writes is `get_proof_bytes`, the bytes guests and
    /// ethproofs take, and `load` restores every kind from them.
    #[test]
    fn a_saved_vadcop_proof_is_the_flat_layout_for_every_kind() {
        for kind in [VadcopKind::Final, VadcopKind::Recurser, VadcopKind::Minimal] {
            let mut publics = flag_free_publics([1, 2, 3, 4]);
            publics[PROGRAM_VK_LEN] = 1 << 40; // a recurser's outputs may be wide
            let original = vadcop_proof(kind, publics.clone());

            let (bytes, loaded) = save_and_load(&original);
            assert_eq!(bytes, original.get_proof_bytes().unwrap(), "{kind:?}");
            let saved = zisk_verifier::decode_saved(&bytes).unwrap();
            assert_eq!(saved.proof.compressed, kind.is_minimal(), "{kind:?}");

            let loaded = loaded.unwrap();
            assert_eq!(loaded.program_vk.vk, vec![1, 2, 3, 4], "{kind:?}");
            assert_eq!(loaded.program_vk.hash_mode, HashMode::Poseidon2, "{kind:?}");
            match loaded.body {
                ProofBody::Vadcop { proof, zisk_vk, kind: k, hash, publics_full } => {
                    assert_eq!(proof, vec![0u64; 8], "{kind:?}");
                    assert_eq!(zisk_vk, vec![1, 2, 3, 4], "{kind:?}");
                    assert_eq!(k, kind);
                    assert_eq!(hash, "Poseidon2", "{kind:?}");
                    assert_eq!(publics_full, publics, "{kind:?}");
                }
                ProofBody::Plonk { .. } | ProofBody::Pilfflonk { .. } => {
                    panic!("expected a Vadcop body")
                }
            }
        }
    }

    fn pilfflonk_proof() -> Proof {
        Proof::new(
            ProofBody::Pilfflonk {
                proof_bytes: vec![5, 6, 7],
                vkey_json: "{\"protocol\":\"pilfflonk\"}".to_string(),
                publics: PublicValues::new_empty(),
                publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
                rootc: vec![0u64; PROGRAM_VK_LEN],
            },
            ProgramVK::new_empty(),
        )
    }

    /// A pilfflonk body is a SNARK proof: it takes the `Plonk` kind, and its bytes and key
    /// survive a save and a load.
    #[test]
    fn pilfflonk_proof_is_a_snark_kind_and_roundtrips() {
        let original = pilfflonk_proof();
        assert_eq!(original.kind(), ProofKind::Plonk);
        assert!(!original.is_empty());

        let (bytes, loaded) = save_and_load(&original);
        assert!(bytes.starts_with(PLONK_FILE_MAGIC));
        match loaded.unwrap().body {
            ProofBody::Pilfflonk { proof_bytes, vkey_json, .. } => {
                assert_eq!(proof_bytes, vec![5, 6, 7]);
                assert_eq!(vkey_json, "{\"protocol\":\"pilfflonk\"}");
            }
            other => panic!("expected a Pilfflonk body after roundtrip, got {other:?}"),
        }
    }

    /// The pins are per protocol: a key of one is refused on a proof of another, rather than
    /// ignored as if the pin had applied.
    #[test]
    fn verify_rejects_a_pin_of_the_wrong_protocol() {
        let vkey = dummy_plonk_vkey();
        let err = pilfflonk_proof().with_plonk_vk(&vkey).verify().unwrap_err();
        assert!(err.to_string().contains("PLONK verification key was pinned"), "got: {err}");

        let vadcop = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        let err = vadcop.with_pilfflonk_vkey("{}").verify().unwrap_err();
        assert!(err.to_string().contains("pilfflonk verification key was pinned"), "got: {err}");

        let plonk = Proof::new(
            ProofBody::Plonk {
                proof_bytes: vec![0; PLONK_PROOF_BYTES],
                plonk_vk: Box::new(PlonkVkBlob {
                    vadcop_vk: vec![0u64; PROGRAM_VK_LEN],
                    plonk_vkey: dummy_plonk_vkey(),
                }),
                publics: PublicValues::new_empty(),
                publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
                rootc: vec![0u64; PROGRAM_VK_LEN],
            },
            ProgramVK::new_empty(),
        );
        let err = plonk.with_pilfflonk_vkey("{}").verify().unwrap_err();
        assert!(err.to_string().contains("pilfflonk verification key was pinned"), "got: {err}");
    }

    /// As for PLONK, a wrong-length root is an error and not a slicing panic.
    #[test]
    fn pilfflonk_verify_rejects_wrong_len_setup_vk() {
        let err = pilfflonk_proof().with_setup_vk(&[1u64, 2, 3]).verify().unwrap_err();
        assert!(matches!(err, CommonError::InvalidProof(_)), "got {err:?}");
    }

    /// The flat layout derives `program_vk` from the publics, so a proof whose
    /// `program_vk` says otherwise must not be saved as if it agreed.
    #[test]
    fn save_refuses_a_program_vk_the_publics_do_not_commit_to() {
        let mut proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        proof.program_vk = ProgramVK::new_from_publics(&[7, 8, 9, 10]);
        let err = proof.to_bytes().unwrap_err();
        assert!(err.to_string().contains("program_vk"), "got: {err}");
    }

    /// Neither a flagged publics vector nor a `hash_mode` the body contradicts can be
    /// saved: the file would not load, or would load as a different proof.
    #[test]
    fn save_refuses_what_the_flat_layout_would_not_round_trip() {
        let mut flagged = vec![IS_VADCOP_FINAL_PROOF];
        flagged.extend(flag_free_publics([1, 2, 3, 4]));
        assert!(vadcop_proof(VadcopKind::Final, flagged).to_bytes().is_err());

        let mut proof = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        proof.program_vk.hash_mode = HashMode::Poseidon1;
        let err = proof.to_bytes().unwrap_err();
        assert!(err.to_string().contains("program_vk"), "got: {err}");
    }

    /// With no key passed, a leaf verifies under the release key, never the one it carries:
    /// a real proof whose claimed key is garbage still verifies, and a tampered one fails.
    #[test]
    fn verify_defaults_to_the_release_key_not_the_proofs_own() {
        let bytes = include_bytes!("../../verifier/tests/fixtures/poseidon2_compressed.bin");
        let mut proof = Proof::from_bytes(bytes).unwrap();
        if let ProofBody::Vadcop { zisk_vk, .. } = &mut proof.body {
            *zisk_vk = vec![1, 2, 3, 4];
        }
        proof.verify().unwrap();

        if let ProofBody::Vadcop { proof: words, .. } = &mut proof.body {
            words[7] ^= 1;
        }
        assert!(matches!(proof.verify().unwrap_err(), CommonError::NotVerified));
    }

    /// No release publishes a recurser's key, so an aggregate must be given one.
    #[test]
    fn verify_requires_a_key_for_a_recurser_proof() {
        let proof = vadcop_proof(VadcopKind::Recurser, flag_free_publics([9, 9, 9, 9]));
        let err = proof.verify().unwrap_err();
        assert!(err.to_string().contains("recurser's key"), "got: {err}");
    }

    fn plonk_proof(publics_full: Vec<u64>, rootc: Vec<u64>, hash_mode: HashMode) -> Proof {
        Proof::new(
            ProofBody::Plonk {
                proof_bytes: vec![5; PLONK_PROOF_BYTES],
                plonk_vk: Box::new(PlonkVkBlob {
                    vadcop_vk: vec![0u64; PROGRAM_VK_LEN],
                    plonk_vkey: dummy_plonk_vkey(),
                }),
                publics: PublicValues::new_empty(),
                publics_full,
                rootc,
            },
            ProgramVK { vk: vec![0u64; PROGRAM_VK_LEN], hash_mode },
        )
    }

    #[test]
    fn verify_rejects_a_plonk_stored_rootc_that_differs_from_the_setup_vk() {
        let proof = plonk_proof(flag_free_publics([0; 4]), vec![9; 4], HashMode::Blake3);
        let err = proof.with_setup_vk(&[1, 2, 3, 4]).verify().unwrap_err();
        assert!(err.to_string().contains("stored `rootc`"), "got: {err}");
    }

    #[test]
    fn plonk_verify_asks_for_the_recurser_key_on_an_aggregate() {
        let proof = plonk_proof(flag_free_publics([7, 7, 7, 7]), vec![7; 4], HashMode::Blake3);
        let err = proof.verify().unwrap_err();
        assert!(err.to_string().contains("recurser's key"), "got: {err}");
    }

    /// The native verifier aborts on any other length, so it must never reach it.
    #[test]
    fn a_plonk_proof_of_the_wrong_length_is_refused_before_the_native_verifier() {
        for len in [0, 3, PLONK_PROOF_BYTES - 1, PLONK_PROOF_BYTES + 1] {
            let mut proof = plonk_proof(flag_free_publics([0; 4]), vec![9; 4], HashMode::Poseidon1);
            if let ProofBody::Plonk { proof_bytes, .. } = &mut proof.body {
                *proof_bytes = vec![0; len];
            }
            let err = proof.with_setup_vk(&[9; 4]).verify().unwrap_err();
            assert!(err.to_string().contains("PLONK proof is"), "{len}: {err}");
            let mut bytes = PLONK_FILE_MAGIC.to_vec();
            bytes.extend(
                bincode::serde::encode_to_vec(&proof, bincode::config::standard()).unwrap(),
            );
            assert!(Proof::from_bytes(&bytes).is_err(), "{len}");
        }
    }

    /// Under a key that is not a release leaf key, a PLONK proof cannot tell a leaf from a
    /// fold carrying another recurser's subtree, so its domain must be that key.
    #[test]
    fn plonk_under_a_recurser_key_must_declare_it_as_its_domain() {
        let recurser = [7u64; 4];
        let foreign =
            plonk_proof(flag_free_publics([3; 4]), recurser.to_vec(), HashMode::Poseidon1);
        let err = foreign.with_setup_vk(&recurser).verify().unwrap_err();
        assert!(err.to_string().contains("recursion domain"), "got: {err}");

        // Vouched as a custom setup's leaf key, the same statement passes the policy and
        // fails only at the dummy PLONK proof.
        let err = foreign.verify_builder().with_leaf_setup_vk(&recurser).verify().unwrap_err();
        assert!(matches!(err, CommonError::NotVerified), "got: {err:?}");

        // A leaf key never verifies an aggregate.
        let own = plonk_proof(flag_free_publics(recurser), recurser.to_vec(), HashMode::Poseidon1);
        let err = own.verify_builder().with_leaf_setup_vk(&recurser).verify().unwrap_err();
        assert!(err.to_string().contains("leaf setup key"), "got: {err}");
    }

    /// The contract's fixture verifies with no key passed, under the release PLONK key.
    #[test]
    fn the_contract_fixture_verifies_under_the_release_plonk_key() {
        let bytes = include_bytes!("../../zisk-contracts/test/plonk_proof.bin");
        Proof::from_bytes(bytes).unwrap().verify().unwrap();
    }

    /// `program_vk` is public, so a PLONK file is bound to its publics like a Vadcop one.
    #[test]
    fn a_plonk_program_vk_must_match_its_publics() {
        let mut proof =
            plonk_proof(flag_free_publics([1, 2, 3, 4]), vec![9; 4], HashMode::Poseidon1);
        proof.program_vk.vk = vec![1, 2, 3, 4];
        let bytes = proof.to_bytes().unwrap();
        assert!(Proof::from_bytes(&bytes).is_ok());

        proof.program_vk.vk = vec![7, 7, 7, 7];
        assert!(proof.to_bytes().unwrap_err().to_string().contains("program_vk"));
        let mut forged = PLONK_FILE_MAGIC.to_vec();
        forged.extend(bincode::serde::encode_to_vec(&proof, bincode::config::standard()).unwrap());
        assert!(Proof::from_bytes(&forged).unwrap_err().to_string().contains("program_vk"));
    }

    #[test]
    fn save_refuses_a_plonk_body_load_would_reject() {
        let proof = plonk_proof(vec![0; 3], vec![9; 4], HashMode::Blake3);
        assert!(proof.to_bytes().is_err());
    }

    #[test]
    fn a_saved_plonk_proof_round_trips_behind_its_magic() {
        let original = plonk_proof(flag_free_publics([0; 4]), vec![9; 4], HashMode::Poseidon1);
        let (bytes, loaded) = save_and_load(&original);
        assert!(bytes.starts_with(PLONK_FILE_MAGIC));
        match loaded.unwrap().body {
            ProofBody::Plonk { proof_bytes, rootc, .. } => {
                assert_eq!(proof_bytes, vec![5; PLONK_PROOF_BYTES]);
                assert_eq!(rootc, vec![9u64; PROGRAM_VK_LEN]);
            }
            other => panic!("expected a PLONK body, got {other:?}"),
        }

        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(Proof::from_bytes(&trailing).is_err());

        // A Vadcop proof smuggled behind the magic is refused: Vadcop has one format.
        let mut smuggled = PLONK_FILE_MAGIC.to_vec();
        let vadcop = vadcop_proof(VadcopKind::Final, flag_free_publics([1, 2, 3, 4]));
        smuggled
            .extend(bincode::serde::encode_to_vec(&vadcop, bincode::config::standard()).unwrap());
        assert!(Proof::from_bytes(&smuggled).is_err());
    }

    #[test]
    fn proof_kind_derivation() {
        let vadcop = Proof::new(
            ProofBody::Vadcop {
                proof: vec![],
                zisk_vk: vec![],
                kind: VadcopKind::Final,
                hash: "Poseidon2".to_string(),
                publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
            },
            ProgramVK::new_empty(),
        );
        assert_eq!(vadcop.kind(), ProofKind::VadcopFinal);
        assert!(vadcop.is_empty());

        let minimal = Proof::new(
            ProofBody::Vadcop {
                proof: vec![1],
                zisk_vk: vec![],
                kind: VadcopKind::Minimal,
                hash: "Poseidon2".to_string(),
                publics_full: vec![0u64; PROGRAM_VK_LEN + ZISK_PUBLICS],
            },
            ProgramVK::new_empty(),
        );
        assert_eq!(minimal.kind(), ProofKind::VadcopFinalMinimal);
        assert!(!minimal.is_empty());
    }
}
