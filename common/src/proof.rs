use crate::error::{CommonError, Result};
use proofman::{verify_snark_proof, SnarkProof, SnarkProtocol};
use proofman_verifier::verifier;
use proofman_verifier::VadcopFinalProof;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use zisk_verifier::{PROGRAM_VK_LEN, ZISK_PUBLICS};

use crate::HashMode;

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
    pub fn new_from_publics_with_mode(publics: &[u64], hash_mode: HashMode) -> Self {
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
    /// Structured Plonk verification key. This is boxed to avoid bloating the size of the `ProofBody` enum, since Plonk proofs are less common and the vkey is large.
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
    pub fn new_from_u64(publics: &[u64]) -> Self {
        assert!(
            publics.len() == ZISK_PUBLICS + PROGRAM_VK_LEN,
            "Expected {} u64 publics, got {}",
            ZISK_PUBLICS + PROGRAM_VK_LEN,
            publics.len()
        );

        let mut data = [0u8; ZISK_PUBLICS * 4];
        for (i, &val) in publics[PROGRAM_VK_LEN..].iter().enumerate() {
            data[i * 4..(i + 1) * 4].copy_from_slice(&(val as u32).to_le_bytes());
        }

        Self { data: data.to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Creates a new `PublicValues` instance with empty data and a reset pointer.
    pub fn new_empty() -> Self {
        Self { data: [0u8; ZISK_PUBLICS * 4].to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Create PublicValues from a serializable value.
    /// The value is serialized with bincode and stored in the public outputs as 64-bit chunks.
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

/// Kind-tagged proof payload. The Vadcop variant is u64-native; Plonk is byte-shaped.
///
/// The Plonk vkey blob is boxed so the enum doesn't carry ~880 bytes of inline Plonk
/// vkey strings on the Vadcop variant — Vadcop is the common case and most-cloned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofBody {
    /// Vadcop proof variant.
    Vadcop {
        /// The proof data as a vector of u64 values.
        proof: Vec<u64>,
        /// The Zisk verification key as a vector of u64 values.
        zisk_vk: Vec<u64>,
        /// Indicates whether the proof is minimal.
        minimal: bool,
        /// Proof hash.
        hash: String,
    },
    /// Plonk proof variant.
    Plonk {
        /// The proof data as a vector of bytes.
        proof_bytes: Vec<u8>,
        /// The Plonk verification key.
        plonk_vk: Box<PlonkVkBlob>,
    },
}

impl Default for ProofBody {
    fn default() -> Self {
        ProofBody::Vadcop {
            proof: Vec::new(),
            zisk_vk: vec![0u64; PROGRAM_VK_LEN],
            minimal: false,
            hash: String::new(),
        }
    }
}

/// A struct representing a proof.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// The data of the proof.
    pub body: ProofBody,
    /// The public values associated with the proof.
    pub publics: PublicValues,
    /// The program verification key associated with the proof.
    pub program_vk: ProgramVK,
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
/// proof.publics(&custom_publics).verify()?;
///
/// // Override program VK only
/// proof.program_vk(&custom_program_vk).verify()?;
///
/// // Override both
/// proof.publics(&custom_publics).program_vk(&custom_program_vk).verify()?;
/// ```
pub struct ZiskVerifyBuilder<'a> {
    proof_with_values: &'a Proof,
    override_publics: Option<&'a PublicValues>,
    override_program_vk: Option<&'a ProgramVK>,
}

impl<'a> ZiskVerifyBuilder<'a> {
    fn new(proof_with_values: &'a Proof) -> Self {
        Self { proof_with_values, override_publics: None, override_program_vk: None }
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

    /// Verify the proof using the configured parameters.
    ///
    /// This method uses the overridden values if provided, otherwise falls back
    /// to the values stored in the proof.
    pub fn verify(self) -> Result<()> {
        let publics = self.override_publics.unwrap_or(&self.proof_with_values.publics);
        let program_vk = self.override_program_vk.unwrap_or(&self.proof_with_values.program_vk);

        match &self.proof_with_values.body {
            ProofBody::Plonk { proof_bytes, plonk_vk } => {
                let pubs = publics.bytes_solidity(program_vk, &plonk_vk.vadcop_vk);
                let hash = Sha256::digest(&pubs).to_vec();

                let snark_proof = SnarkProof {
                    proof_bytes: proof_bytes.clone(),
                    public_bytes: pubs,
                    public_snark_bytes: hash,
                    protocol_id: SnarkProtocol::Plonk.protocol_id(),
                };

                let temp_dir = std::env::temp_dir();
                // Concurrent verify() calls in one process otherwise race on the tempfile.
                let unique_id = format!(
                    "{}_{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                );
                let temp_file = temp_dir.join(format!("plonk_vkey_{}.json", unique_id));

                let plonk_vkey_json = serde_json::to_vec(&plonk_vk.plonk_vkey)
                    .map_err(|e| CommonError::Serialization(format!("PlonkVkey to JSON: {e}")))?;
                std::fs::write(&temp_file, &plonk_vkey_json).map_err(|e| {
                    CommonError::Io(format!(
                        "Failed to write PlonkVkey to temporary file: {}: {e}",
                        temp_file.display()
                    ))
                })?;

                let result = verify_snark_proof(&snark_proof, &temp_file);

                if temp_file.exists() {
                    std::fs::remove_file(&temp_file).map_err(|e| {
                        CommonError::Io(format!(
                            "Failed to delete temporary file: {}: {e}",
                            temp_file.display()
                        ))
                    })?;
                }

                result.map_err(|e| {
                    CommonError::Invalid(format!("snark proof verification failed: {e}"))
                })?;
                Ok(())
            }
            ProofBody::Vadcop { proof, zisk_vk, minimal, hash } => {
                let minimal = *minimal;

                if program_vk.hash_mode.as_str() != hash {
                    return Err(CommonError::InvalidProof(format!(
                        "verkey hash mode {} does not match proof hash family {hash:?}",
                        program_vk.hash_mode.as_str()
                    )));
                }

                let v = verifier(hash);
                let expected_len = if minimal {
                    v.expected_vadcop_final_compressed_proof_bytes()
                } else {
                    v.expected_vadcop_final_proof_bytes()
                };
                if proof.len() * 8 != expected_len {
                    return Err(CommonError::InvalidProof(format!(
                        "Malformed proof: expected {} bytes for {:?}, got {}",
                        expected_len,
                        self.proof_with_values.kind(),
                        proof.len() * 8
                    )));
                }

                let mut pubs_u64 = program_vk.vk.clone();
                pubs_u64.extend(publics.public_u64());
                let vadcop_final_proof =
                    VadcopFinalProof::new(proof.clone(), pubs_u64, minimal, hash.clone());

                let is_valid = if minimal {
                    v.verify_vadcop_final_compressed(&vadcop_final_proof, zisk_vk)
                } else {
                    v.verify_vadcop_final(&vadcop_final_proof, zisk_vk)
                };

                if !is_valid {
                    Err(CommonError::NotVerified)
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Proof {
    /// Creates a new `Proof` instance with the provided body, public values, and program verification key.
    pub fn new(body: ProofBody, publics: PublicValues, program_vk: ProgramVK) -> Self {
        Self { body, publics, program_vk }
    }

    /// Derive the `ProofKind` from the body discriminant.
    pub fn kind(&self) -> ProofKind {
        match &self.body {
            ProofBody::Vadcop { minimal: true, .. } => ProofKind::VadcopFinalMinimal,
            ProofBody::Vadcop { minimal: false, .. } => ProofKind::VadcopFinal,
            ProofBody::Plonk { .. } => ProofKind::Plonk,
        }
    }

    /// Whether the underlying proof payload is empty (used to detect non-prove flows).
    pub fn is_empty(&self) -> bool {
        match &self.body {
            ProofBody::Vadcop { proof, .. } => proof.is_empty(),
            ProofBody::Plonk { proof_bytes, .. } => proof_bytes.is_empty(),
        }
    }

    /// Save the proof to a file using bincode serialization.
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

        let mut file = File::create(path).map_err(|e| {
            CommonError::Io(format!(
                "failed to create file for saving proof: {}: {e}",
                path.display()
            ))
        })?;
        bincode::serde::encode_into_std_write(self, &mut file, bincode::config::standard())
            .map(|_| ())
            .map_err(|e| CommonError::Io(format!("Failed to save proof: {}", e)))
    }

    /// Load a proof from a file using bincode deserialization.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path.as_ref()).map_err(|e| {
            CommonError::Io(format!(
                "failed to open file for loading proof: {}: {e}",
                path.as_ref().display()
            ))
        })?;
        let proof: Proof =
            bincode::serde::decode_from_std_read(&mut file, bincode::config::standard())
                .map_err(|e| CommonError::Io(format!("Failed to load proof: {}", e)))?;
        Ok(proof)
    }

    /// Extract a `VadcopFinalProof` from the proof body.
    pub fn get_vadcop_final_proof(&self) -> Result<VadcopFinalProof> {
        match &self.body {
            ProofBody::Vadcop { proof, minimal, hash, .. } => {
                let mut pubs_u64 = self.program_vk.vk.clone();
                pubs_u64.extend(self.publics.public_u64());
                Ok(VadcopFinalProof::new(proof.clone(), pubs_u64, *minimal, hash.clone()))
            }
            ProofBody::Plonk { .. } => {
                Err(CommonError::InvalidProof("Proof is not a Vadcop final proof".to_string()))
            }
        }
    }

    /// Get the proof data as a vector of u64 values.
    pub fn get_proof_u64(&self) -> Result<Vec<u64>> {
        match &self.body {
            ProofBody::Vadcop { proof, zisk_vk, minimal, .. } => {
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

                let publics = self.publics.public_u64();
                let n_publics = self.program_vk.vk.len() + publics.len();

                // Format: [minimal(1)][n_publics(1)][program_vk][publics][proof][zisk_vk]
                let mut words = Vec::with_capacity(2 + n_publics + proof.len() + zisk_vk.len());
                words.push(*minimal as u64);
                words.push(n_publics as u64);
                words.extend_from_slice(&self.program_vk.vk);
                words.extend(publics);
                words.extend_from_slice(proof);
                words.extend_from_slice(zisk_vk);

                Ok(words)
            }
            ProofBody::Plonk { .. } => Err(CommonError::InvalidProof(
                "Proof not suitable for get_proof_u64. Only VadcopFinal and VadcopFinalMinimal proofs are supported.".to_string()
            )),
        }
    }

    /// Get the proof data as a vector of bytes.
    pub fn get_proof_bytes(&self) -> Result<Vec<u8>> {
        let words = self.get_proof_u64()?;
        let mut bytes = Vec::with_capacity(words.len() * 8);
        for w in &words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        Ok(bytes)
    }

    /// Get a reference to the public values associated with this proof.
    pub fn get_publics(&self) -> &PublicValues {
        &self.publics
    }

    /// Get a reference to the program verification key associated with this proof.
    pub fn get_program_vk(&self) -> &ProgramVK {
        &self.program_vk
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
    /// A Proof containing the parsed proof, publics, and program VK
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

        let hash_mode = hash.parse::<HashMode>().map_err(|e| {
            CommonError::Invalid(format!("unrecognized proof hash family {hash:?}: {e}"))
        })?;

        let vadcop_proof = VadcopFinalProof::new_from_proof(proof, minimal, hash.clone())
            .map_err(|e| CommonError::InvalidProof(format!("Failed to parse Vadcop proof: {}", e)))?;

        Ok(Self {
            body: ProofBody::Vadcop { proof: vadcop_proof.proof, zisk_vk, minimal, hash },
            publics: PublicValues::new_from_u64(&vadcop_proof.public_values),
            program_vk: ProgramVK::new_from_publics_with_mode(
                &vadcop_proof.public_values,
                hash_mode,
            ),
        })
    }

    /// Verify the proof using the default values stored in this instance.
    ///
    /// For custom verification with overridden values, use the builder methods:
    /// - `publics()` to override public values
    /// - `program_vk()` to override program verification key
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Default verification
    /// proof.verify()?;
    ///
    /// // Custom verification with overridden publics
    /// proof.publics(&custom_publics).verify()?;
    ///
    /// // Custom verification with multiple overrides
    /// proof.publics(&custom_publics).program_vk(&custom_program_vk).verify()?;
    /// ```
    pub fn verify(&self) -> Result<()> {
        ZiskVerifyBuilder::new(self).verify()
    }

    /// Start building a custom verification by overriding the public values.
    ///
    /// Returns a builder that allows chaining additional overrides before calling `verify()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// proof.publics(&custom_publics).verify()?;
    /// proof.publics(&custom_publics).program_vk(&custom_program_vk).verify()?;
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
    /// proof.program_vk(&custom_program_vk).verify()?;
    /// proof.program_vk(&custom_program_vk).publics(&custom_publics).verify()?;
    /// ```
    pub fn with_program_vk<'a>(&'a self, program_vk: &'a ProgramVK) -> ZiskVerifyBuilder<'a> {
        ZiskVerifyBuilder::new(self).with_program_vk(program_vk)
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
                minimal: true,
                hash: "Poseidon2".to_string(),
            },
            PublicValues::new_empty(),
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
                minimal: false,
                hash: "Poseidon2".to_string(),
            },
            PublicValues::new_empty(),
            ProgramVK::new_empty(),
        )
        .verify();

        assert!(result.is_err(), "expected Err for malformed proof, got {:?}", result);
    }

    #[test]
    fn proof_save_load_roundtrip_vadcop() {
        let tmp = std::env::temp_dir().join(format!("proof_roundtrip_{}.bin", std::process::id()));
        let original = Proof::new(
            ProofBody::Vadcop {
                proof: vec![1, 2, 3, 4],
                zisk_vk: vec![10, 20, 30, 40],
                minimal: true,
                hash: "Poseidon2".to_string(),
            },
            PublicValues::new_empty(),
            ProgramVK::new_from_publics(&[7, 8, 9, 10]),
        );

        original.save(&tmp).unwrap();
        let loaded = Proof::load(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(loaded.kind(), ProofKind::VadcopFinalMinimal);
        match loaded.body {
            ProofBody::Vadcop { proof, zisk_vk, minimal, hash } => {
                assert_eq!(proof, vec![1, 2, 3, 4]);
                assert_eq!(zisk_vk, vec![10, 20, 30, 40]);
                assert!(minimal);
                assert_eq!(hash, "Poseidon2");
            }
            ProofBody::Plonk { .. } => panic!("expected Vadcop body after roundtrip"),
        }
        assert_eq!(loaded.program_vk.vk, vec![7, 8, 9, 10]);
    }

    #[test]
    fn proof_kind_derivation() {
        let vadcop = Proof::new(
            ProofBody::Vadcop {
                proof: vec![],
                zisk_vk: vec![],
                minimal: false,
                hash: "Poseidon2".to_string(),
            },
            PublicValues::new_empty(),
            ProgramVK::new_empty(),
        );
        assert_eq!(vadcop.kind(), ProofKind::VadcopFinal);
        assert!(vadcop.is_empty());

        let minimal = Proof::new(
            ProofBody::Vadcop {
                proof: vec![1],
                zisk_vk: vec![],
                minimal: true,
                hash: "Poseidon2".to_string(),
            },
            PublicValues::new_empty(),
            ProgramVK::new_empty(),
        );
        assert_eq!(minimal.kind(), ProofKind::VadcopFinalMinimal);
        assert!(!minimal.is_empty());
    }
}
