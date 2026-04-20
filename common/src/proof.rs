use anyhow::{anyhow, Context, Result};
use proofman::{verify_snark_proof, SnarkProof, SnarkProtocol};
use proofman_util::VadcopFinalProof;
use proofman_verifier::{verify_vadcop_final, verify_vadcop_final_compressed};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const ZISK_PUBLICS: usize = 64;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ProgramVK {
    pub vk: Vec<u8>,
}

impl ProgramVK {
    pub fn new_from_publics(publics: &[u8]) -> Self {
        assert!(
            publics.len() >= 32,
            "Not enough bytes to extract program VK (expected at least 32 bytes)"
        );

        Self { vk: publics[0..32].to_vec() }
    }

    pub fn new_empty() -> Self {
        Self { vk: vec![0u8; 32] }
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

pub type ZiskVK = Vec<u8>;

/// Encode a Plonk `zisk_vk` blob: `[vk_len: u32 LE][vadcop_vk_bytes][plonk_vkey_json]`.
pub fn encode_plonk_zisk_vk(vadcop_vk: Vec<u8>, plonk_vkey: &PlonkVkey) -> Result<ZiskVK> {
    let plonk_json =
        serde_json::to_vec(plonk_vkey).context("Failed to serialize PlonkVkey to JSON")?;
    let vk_len = vadcop_vk.len() as u32;
    let mut bytes = Vec::with_capacity(4 + vadcop_vk.len() + plonk_json.len());
    bytes.extend_from_slice(&vk_len.to_le_bytes());
    bytes.extend_from_slice(&vadcop_vk);
    bytes.extend_from_slice(&plonk_json);
    Ok(bytes)
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

    pub fn new_empty() -> Self {
        Self { data: [0u8; ZISK_PUBLICS * 4].to_vec(), ptr: AtomicUsize::new(0) }
    }

    /// Create PublicValues from a serializable value.
    /// The value is serialized with bincode and stored in the public outputs as 64-bit chunks.
    pub fn write<T: serde::Serialize>(value: &T) -> Result<Self> {
        let serialized = bincode::serialize(value)
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
        let result: T = bincode::deserialize(&self.data[ptr..])
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))?;
        let nb_bytes = bincode::serialized_size(&result)
            .map_err(|e| anyhow::anyhow!("Failed to get serialized size: {}", e))?;
        self.ptr.store(ptr + nb_bytes as usize, Ordering::Relaxed);
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

    pub fn public_bytes(&self) -> Vec<u8> {
        let mut bytes = [0u8; ZISK_PUBLICS * 8];

        // Convert the 256 bytes back to ZISK_PUBLICS u64 values (padding upper 32 bits with zeros)
        for i in 0..ZISK_PUBLICS {
            let start = i * 4;
            let val32 = u32::from_le_bytes([
                self.data[start],
                self.data[start + 1],
                self.data[start + 2],
                self.data[start + 3],
            ]);
            let val64 = val32 as u64;
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&val64.to_le_bytes());
        }

        bytes.to_vec()
    }

    pub fn hash_solidity(&self, program_vk: &ProgramVK, vadcop_verkey: &[u8]) -> Vec<u8> {
        let bytes = self.bytes_solidity(program_vk, vadcop_verkey);

        // SHA-256
        let hash = Sha256::digest(&bytes);

        hash.to_vec()
    }
}

impl PublicValues {
    pub fn bytes_u64(&self, program_vk: &ProgramVK) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(program_vk.vk.len() + ZISK_PUBLICS * 8);

        bytes.extend(&program_vk.vk);
        bytes.extend(self.public_bytes());

        bytes
    }

    pub fn bytes_solidity(&self, program_vk: &ProgramVK, vadcop_verkey: &[u8]) -> Vec<u8> {
        let mut prefix = [0u8; 32];
        for (i, chunk) in program_vk.vk.chunks_exact(8).enumerate() {
            let val = u64::from_le_bytes(chunk.try_into().unwrap());
            prefix[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }

        let mut bytes = prefix.to_vec();
        bytes.extend_from_slice(&self.data);
        let mut suffix = [0u8; 32];
        for (i, chunk) in vadcop_verkey.chunks_exact(8).enumerate() {
            let val = u64::from_le_bytes(chunk.try_into().unwrap());
            suffix[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }
        bytes.extend(&suffix);
        bytes
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub proof_kind: ProofKind,
    pub proof_bytes: Vec<u8>,
    pub publics: PublicValues,
    pub program_vk: ProgramVK,
    pub zisk_vk: ZiskVK,
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
        let zisk_vk = &self.proof_with_values.zisk_vk;

        match self.proof_with_values.proof_kind {
            ProofKind::Plonk => {
                let proof_bytes = &self.proof_with_values.proof_bytes;
                let protocol_id = match self.proof_with_values.proof_kind {
                    ProofKind::Plonk => SnarkProtocol::Plonk.protocol_id(),
                    _ => unreachable!(),
                };

                // Parse blob: [vk_len: u32 LE][vadcop_vk_bytes][plonk_vkey_json]
                if zisk_vk.len() < 4 {
                    return Err(anyhow::anyhow!("zisk_vk too short for Plonk proof"));
                }
                let vk_len = u32::from_le_bytes(zisk_vk[0..4].try_into().unwrap()) as usize;
                if zisk_vk.len() < 4 + vk_len {
                    return Err(anyhow::anyhow!("zisk_vk truncated"));
                }
                let vadcop_vk = &zisk_vk[4..4 + vk_len];
                let plonk_vkey_json = &zisk_vk[4 + vk_len..];

                let pubs = publics.bytes_solidity(program_vk, vadcop_vk);

                let hash = Sha256::digest(&pubs).to_vec();

                let snark_proof = SnarkProof {
                    proof_bytes: proof_bytes.clone(),
                    public_bytes: pubs,
                    public_snark_bytes: hash,
                    protocol_id,
                };

                let temp_dir = std::env::temp_dir();
                let temp_file = temp_dir.join(format!("plonk_vkey_{}.json", std::process::id()));

                std::fs::write(&temp_file, plonk_vkey_json).with_context(|| {
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
            ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal => {
                let minimal = self.proof_with_values.proof_kind != ProofKind::VadcopFinal;
                let proof_bytes = &self.proof_with_values.proof_bytes;
                let mut pubs = program_vk.vk.clone();
                pubs.extend(publics.public_bytes());
                let vadcop_final_proof = VadcopFinalProof::new(proof_bytes.clone(), pubs, minimal);

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
    pub fn new(
        proof_kind: ProofKind,
        proof_bytes: Vec<u8>,
        publics: PublicValues,
        program_vk: ProgramVK,
        zisk_vk: ZiskVK,
    ) -> Self {
        Self { proof_kind, proof_bytes, publics, program_vk, zisk_vk }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        bincode::serialize_into(
            File::create(path.as_ref()).with_context(|| {
                format!("failed to create file for saving proof: {}", path.as_ref().display())
            })?,
            self,
        )
        .map_err(Into::into)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path.as_ref()).with_context(|| {
            format!("failed to open file for loading proof: {}", path.as_ref().display())
        })?;
        let proof: Proof = bincode::deserialize_from(file)?;
        Ok(proof)
    }

    pub fn get_vadcop_final_proof(&self) -> Result<VadcopFinalProof> {
        match self.proof_kind {
            ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal => {
                let minimal = self.proof_kind == ProofKind::VadcopFinalMinimal;
                let mut pubs = self.program_vk.vk.clone();
                pubs.extend(self.publics.public_bytes());
                Ok(VadcopFinalProof::new(self.proof_bytes.clone(), pubs, minimal))
            }
            _ => Err(anyhow::anyhow!("Proof is not a Vadcop final proof")),
        }
    }

    pub fn get_proof_bytes(&self) -> Vec<u8> {
        match self.proof_kind {
            ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal => {
                let minimal = self.proof_kind == ProofKind::VadcopFinalMinimal;

                let mut pubs = self.program_vk.vk.clone();
                pubs.extend(self.publics.public_bytes());

                // Format: [minimal(8)][pubs_len(8)][pubs][proof_bytes][zisk_vk]
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&(minimal as u64).to_le_bytes());
                bytes.extend_from_slice(&(ZISK_PUBLICS + 4).to_le_bytes());
                bytes.extend_from_slice(&pubs);
                bytes.extend_from_slice(&self.proof_bytes);
                bytes.extend_from_slice(&self.zisk_vk);

                bytes
            }
            _ => panic!("Proof not suitable for get_proof_bytes. Only VadcopFinal and VadcopFinalMinimal proofs are supported."),
        }
    }

    pub fn get_publics(&self) -> &PublicValues {
        &self.publics
    }

    pub fn get_program_vk(&self) -> &ProgramVK {
        &self.program_vk
    }

    pub fn get_vk(&self) -> &ZiskVK {
        &self.zisk_vk
    }

    /// Create Proof directly from a Vadcop proof byte array.
    ///
    /// This method parses the proof format (n_publics, publics..., proof...) and extracts
    /// the public values and program VK directly, without creating an intermediate VadcopFinalProof.
    ///
    /// # Parameters
    ///
    /// * `proof` - The proof as a slice of u64 values
    /// * `minimal` - Whether the proof is minimal
    ///
    /// # Returns
    ///
    /// A Proof containing the parsed proof, publics, and program VK
    pub fn new_from_vadcop_proof(proof: &[u64], minimal: bool, zisk_vk: Vec<u8>) -> Result<Self> {
        let vadcop_proof = VadcopFinalProof::new_from_proof(proof, minimal)
            .map_err(|e| anyhow::anyhow!("Failed to parse Vadcop proof: {}", e))?;

        let proof_kind =
            if minimal { ProofKind::VadcopFinalMinimal } else { ProofKind::VadcopFinal };

        Ok(Self {
            proof_kind,
            proof_bytes: vadcop_proof.proof,
            publics: PublicValues::new(&vadcop_proof.public_values),
            program_vk: ProgramVK::new_from_publics(&vadcop_proof.public_values),
            zisk_vk,
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
