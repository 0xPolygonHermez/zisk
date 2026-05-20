use anyhow::{anyhow, Context, Result};
use proofman::{verify_snark_proof, SnarkProof, SnarkProtocol};
use proofman_verifier::VadcopFinalProof;
use proofman_verifier::{
    expected_vadcop_final_compressed_proof_bytes, expected_vadcop_final_proof_bytes,
    verify_vadcop_final, verify_vadcop_final_compressed,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use zisk_verifier::{PROGRAM_VK_LEN, ZISK_PUBLICS};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetupKey {
    pub hash_id: String,
    pub with_hints: bool,
    pub emulator_only: bool,
}

impl SetupKey {
    pub fn new(hash_id: impl Into<String>, with_hints: bool, emulator_only: bool) -> Self {
        Self { hash_id: hash_id.into(), with_hints, emulator_only }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ProgramVK {
    pub vk: Vec<u64>,
}

impl ProgramVK {
    /// Build from the first `PROGRAM_VK_LEN` u64 elements of a publics blob.
    pub fn new_from_publics(publics: &[u64]) -> Self {
        assert!(
            publics.len() >= PROGRAM_VK_LEN,
            "Not enough u64 publics to extract program VK (expected at least {})",
            PROGRAM_VK_LEN
        );

        Self { vk: publics[..PROGRAM_VK_LEN].to_vec() }
    }

    pub fn new_empty() -> Self {
        Self { vk: vec![0u64; PROGRAM_VK_LEN] }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofKind {
    #[default]
    VadcopFinal,
    VadcopFinalMinimal,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlonkVkey {
    pub protocol: String,
    pub curve: String,
    #[serde(rename = "nPublic")]
    pub n_public: u32,
    pub power: u32,
    pub k1: String,
    pub k2: String,
    #[serde(rename = "Qm")]
    pub qm: [String; 3],
    #[serde(rename = "Ql")]
    pub ql: [String; 3],
    #[serde(rename = "Qr")]
    pub qr: [String; 3],
    #[serde(rename = "Qo")]
    pub qo: [String; 3],
    #[serde(rename = "Qc")]
    pub qc: [String; 3],
    #[serde(rename = "S1")]
    pub s1: [String; 3],
    #[serde(rename = "S2")]
    pub s2: [String; 3],
    #[serde(rename = "S3")]
    pub s3: [String; 3],
    #[serde(rename = "X_2")]
    pub x_2: [[String; 2]; 3],
    pub w: String,
}

impl PlonkVkey {
    /// Load PlonkVkey from a JSON file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path.as_ref()).with_context(|| {
            format!("failed to open file for loading PlonkVkey: {}", path.as_ref().display())
        })?;
        let vkey: PlonkVkey = serde_json::from_reader(file).with_context(|| {
            format!("failed to parse PlonkVkey JSON from {}", path.as_ref().display())
        })?;
        Ok(vkey)
    }

    /// Save PlonkVkey to a JSON file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path).with_context(|| {
            format!("failed to create file for saving PlonkVkey: {}", path.display())
        })?;

        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("failed to write PlonkVkey JSON to {}", path.display()))?;

        Ok(())
    }
}

/// Verification key for a Plonk proof: the underlying Vadcop vkey plus the structured Plonk vkey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlonkVkBlob {
    pub vadcop_vk: Vec<u64>,
    pub plonk_vkey: PlonkVkey,
}

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

    pub fn new_empty() -> Self {
        Self { data: [0u8; ZISK_PUBLICS * 4].to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Create PublicValues from a serializable value.
    /// The value is serialized with bincode and stored in the public outputs as 64-bit chunks.
    pub fn write<T: serde::Serialize>(value: &T) -> Result<Self> {
        let serialized = bincode::serde::encode_to_vec(value, bincode::config::standard())
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;

        if serialized.len() > ZISK_PUBLICS * 4 {
            return Err(anyhow::anyhow!(
                "Serialized data too large: {} bytes (max {} bytes)",
                serialized.len(),
                ZISK_PUBLICS * 4
            ));
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

    pub fn write_abi<T: alloy_sol_types::SolValue>(value: &T) -> Result<Self> {
        let encoded = value.abi_encode();

        if encoded.len() > ZISK_PUBLICS * 4 {
            return Err(anyhow::anyhow!(
                "ABI encoded data too large: {} bytes (max {} bytes)",
                encoded.len(),
                ZISK_PUBLICS * 4
            ));
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
                .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))?;
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
            .map_err(|e| anyhow::anyhow!("ABI decoding failed: {}", e))?;
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

    pub fn hash_solidity(&self, program_vk: &ProgramVK, vadcop_verkey: &[u64]) -> Vec<u8> {
        let bytes = self.bytes_solidity(program_vk, vadcop_verkey);

        // SHA-256
        let hash = Sha256::digest(&bytes);

        hash.to_vec()
    }
}

impl PublicValues {
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
    Vadcop { proof: Vec<u64>, zisk_vk: Vec<u64>, minimal: bool },
    Plonk { proof_bytes: Vec<u8>, plonk_vk: Box<PlonkVkBlob> },
}

impl Default for ProofBody {
    fn default() -> Self {
        ProofBody::Vadcop { proof: Vec::new(), zisk_vk: vec![0u64; PROGRAM_VK_LEN], minimal: false }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub body: ProofBody,
    pub publics: PublicValues,
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
                    .context("Failed to serialize PlonkVkey to JSON")?;
                std::fs::write(&temp_file, &plonk_vkey_json).with_context(|| {
                    format!("Failed to write PlonkVkey to temporary file: {}", temp_file.display())
                })?;

                let result = verify_snark_proof(&snark_proof, &temp_file);

                if temp_file.exists() {
                    std::fs::remove_file(&temp_file).with_context(|| {
                        format!("Failed to delete temporary file: {}", temp_file.display())
                    })?;
                }

                result?;
                Ok(())
            }
            ProofBody::Vadcop { proof, zisk_vk, minimal } => {
                let minimal = *minimal;
                let expected_len = if minimal {
                    expected_vadcop_final_compressed_proof_bytes()
                } else {
                    expected_vadcop_final_proof_bytes()
                };
                if proof.len() * 8 != expected_len {
                    return Err(anyhow!(
                        "Malformed proof: expected {} bytes for {:?}, got {}",
                        expected_len,
                        self.proof_with_values.kind(),
                        proof.len() * 8
                    ));
                }

                let mut pubs_u64 = program_vk.vk.clone();
                pubs_u64.extend(publics.public_u64());
                let vadcop_final_proof = VadcopFinalProof::new(proof.clone(), pubs_u64, minimal);

                let is_valid = if minimal {
                    verify_vadcop_final_compressed(&vadcop_final_proof, zisk_vk)
                } else {
                    verify_vadcop_final(&vadcop_final_proof, zisk_vk)
                };

                if !is_valid {
                    Err(anyhow!("Zisk Proof was not verified"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Proof {
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

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut file = File::create(path.as_ref()).with_context(|| {
            format!("failed to create file for saving proof: {}", path.as_ref().display())
        })?;
        bincode::serde::encode_into_std_write(self, &mut file, bincode::config::standard())
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to save proof: {}", e))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path.as_ref()).with_context(|| {
            format!("failed to open file for loading proof: {}", path.as_ref().display())
        })?;
        let proof: Proof =
            bincode::serde::decode_from_std_read(&mut file, bincode::config::standard())
                .map_err(|e| anyhow::anyhow!("Failed to load proof: {}", e))?;
        Ok(proof)
    }

    pub fn get_vadcop_final_proof(&self) -> Result<VadcopFinalProof> {
        match &self.body {
            ProofBody::Vadcop { proof, minimal, .. } => {
                let mut pubs_u64 = self.program_vk.vk.clone();
                pubs_u64.extend(self.publics.public_u64());
                Ok(VadcopFinalProof::new(proof.clone(), pubs_u64, *minimal))
            }
            ProofBody::Plonk { .. } => Err(anyhow::anyhow!("Proof is not a Vadcop final proof")),
        }
    }

    pub fn get_proof_u64(&self) -> Result<Vec<u64>> {
        match &self.body {
            ProofBody::Vadcop { proof, zisk_vk, minimal } => {
                if self.program_vk.vk.len() != PROGRAM_VK_LEN {
                    return Err(anyhow!(
                        "Invalid program_vk length: expected {}, got {}",
                        PROGRAM_VK_LEN,
                        self.program_vk.vk.len()
                    ));
                }
                if zisk_vk.len() != PROGRAM_VK_LEN {
                    return Err(anyhow!(
                        "Invalid zisk_vk length: expected {}, got {}",
                        PROGRAM_VK_LEN,
                        zisk_vk.len()
                    ));
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
            ProofBody::Plonk { .. } => Err(anyhow!(
                "Proof not suitable for get_proof_u64. Only VadcopFinal and VadcopFinalMinimal proofs are supported."
            )),
        }
    }

    pub fn get_proof_bytes(&self) -> Result<Vec<u8>> {
        let words = self.get_proof_u64()?;
        let mut bytes = Vec::with_capacity(words.len() * 8);
        for w in &words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        Ok(bytes)
    }

    pub fn get_publics(&self) -> &PublicValues {
        &self.publics
    }

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
    ///
    /// # Returns
    ///
    /// A Proof containing the parsed proof, publics, and program VK
    pub fn new_from_vadcop_proof(proof: &[u64], minimal: bool, zisk_vk: Vec<u64>) -> Result<Self> {
        let vadcop_proof = VadcopFinalProof::new_from_proof(proof, minimal)
            .map_err(|e| anyhow::anyhow!("Failed to parse Vadcop proof: {}", e))?;

        Ok(Self {
            body: ProofBody::Vadcop { proof: vadcop_proof.proof, zisk_vk, minimal },
            publics: PublicValues::new_from_u64(&vadcop_proof.public_values),
            program_vk: ProgramVK::new_from_publics(&vadcop_proof.public_values),
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
            ProofBody::Vadcop { proof: vec![], zisk_vk: vec![0u64; PROGRAM_VK_LEN], minimal: true },
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
            },
            PublicValues::new_empty(),
            ProgramVK::new_from_publics(&[7, 8, 9, 10]),
        );

        original.save(&tmp).unwrap();
        let loaded = Proof::load(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(loaded.kind(), ProofKind::VadcopFinalMinimal);
        match loaded.body {
            ProofBody::Vadcop { proof, zisk_vk, minimal } => {
                assert_eq!(proof, vec![1, 2, 3, 4]);
                assert_eq!(zisk_vk, vec![10, 20, 30, 40]);
                assert!(minimal);
            }
            ProofBody::Plonk { .. } => panic!("expected Vadcop body after roundtrip"),
        }
        assert_eq!(loaded.program_vk.vk, vec![7, 8, 9, 10]);
    }

    #[test]
    fn proof_kind_derivation() {
        let vadcop = Proof::new(
            ProofBody::Vadcop { proof: vec![], zisk_vk: vec![], minimal: false },
            PublicValues::new_empty(),
            ProgramVK::new_empty(),
        );
        assert_eq!(vadcop.kind(), ProofKind::VadcopFinal);
        assert!(vadcop.is_empty());

        let minimal = Proof::new(
            ProofBody::Vadcop { proof: vec![1], zisk_vk: vec![], minimal: true },
            PublicValues::new_empty(),
            ProgramVK::new_empty(),
        );
        assert_eq!(minimal.kind(), ProofKind::VadcopFinalMinimal);
        assert!(!minimal.is_empty());
    }
}
