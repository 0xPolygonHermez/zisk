mod asm;
mod backend;
mod emu;
pub use asm::*;
use backend::*;
pub use emu::*;
use proofman::{
    AggProofs, ExecutionInfo, ProvePhase, ProvePhaseInputs, ProvePhaseResult, SnarkProtocol,
};
use proofman_common::ProofOptions;
use proofman_util::VadcopFinalProof;
use sha2::{Digest, Sha256};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    time::Duration,
};
use zisk_common::ElfBinaryLike;
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    ExecutorStatsHandle, ZiskExecutionResult,
};

pub struct ZiskExecuteResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub publics: ZiskPublics,
}

impl ZiskExecuteResult {
    pub fn new(execution: ZiskExecutionResult, duration: Duration, publics: &[u8]) -> Self {
        Self { execution, duration, publics: ZiskPublics::new(publics) }
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.publics
    }

    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.publics.read()
    }

    pub fn get_execution_steps(&self) -> &u64 {
        &self.execution.steps
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
    }
}

pub struct ZiskVerifyConstraintsResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub stats: ExecutorStatsHandle,
    pub publics: ZiskPublics,
}

impl ZiskVerifyConstraintsResult {
    pub fn new(
        execution: ZiskExecutionResult,
        duration: Duration,
        stats: ExecutorStatsHandle,
        publics: &[u8],
    ) -> Self {
        Self { execution, duration, stats, publics: ZiskPublics::new(publics) }
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.publics
    }

    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.publics.read()
    }

    pub fn get_execution_steps(&self) -> &u64 {
        &self.execution.steps
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskProgramVK {
    pub vk: Vec<u8>,
}

impl ZiskProgramVK {
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

#[derive(Debug, Clone)]
pub struct ProofOpts {
    pub aggregation: bool,
    pub verify_proofs: bool,
    pub rma: bool,
    pub minimal_memory: bool,
    pub output_dir_path: Option<PathBuf>,
    pub save_proofs: bool,
}

impl Default for ProofOpts {
    fn default() -> Self {
        Self {
            aggregation: true,
            verify_proofs: false,
            rma: false,
            minimal_memory: false,
            output_dir_path: None,
            save_proofs: false,
        }
    }
}

impl ProofOpts {
    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir_path = Some(path);
        self
    }

    pub fn save_proofs(mut self) -> Self {
        self.save_proofs = true;
        self
    }

    pub fn verify_proofs(mut self) -> Self {
        self.verify_proofs = true;
        self
    }

    pub fn minimal_memory(mut self) -> Self {
        self.minimal_memory = true;
        self
    }

    pub fn no_aggregation(mut self) -> Self {
        self.aggregation = false;
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProofMode {
    VadcopFinal,
    VadcopFinalCompressed,
    Snark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZiskProof {
    Null(),
    VadcopFinal(Vec<u8>),
    VadcopFinalCompressed(Vec<u8>),
    Plonk(Vec<u8>),
    Fflonk(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskVadcopFinalProof {
    pub proof: Vec<u8>,
    pub compressed: bool,
}

impl ZiskVadcopFinalProof {
    pub fn new(proof: Vec<u8>, compressed: bool) -> Self {
        Self { proof, compressed }
    }

    pub fn save(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to create file for saving Vadcop Final proof: {}: {}",
                    path.display(),
                    e
                ),
            )
        })?;

        bincode::serialize_into(file, self)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path.as_ref()).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to open file for loading proof: {}: {}",
                    path.as_ref().display(),
                    e
                ),
            )
        })?;
        let proof: ZiskVadcopFinalProof = bincode::deserialize_from(file)?;
        Ok(proof)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskSnarkProof {
    pub proof: Vec<u8>,
    pub protocol_id: u64,
}

impl ZiskSnarkProof {
    pub fn new(proof: Vec<u8>, protocol_id: u64) -> Self {
        Self { proof, protocol_id }
    }

    pub fn save(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Failed to create file for saving SNARK proof: {}: {}", path.display(), e),
            )
        })?;

        bincode::serialize_into(file, self)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path.as_ref()).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to open file for loading SNARK proof: {}: {}",
                    path.as_ref().display(),
                    e
                ),
            )
        })?;
        let proof: ZiskSnarkProof = bincode::deserialize_from(file)?;
        Ok(proof)
    }
}

impl ZiskProof {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        match self {
            ZiskProof::Null() => Err(anyhow::anyhow!("No proof to save")),
            ZiskProof::VadcopFinal(proof) | ZiskProof::VadcopFinalCompressed(proof) => {
                let compressed = matches!(self, ZiskProof::VadcopFinalCompressed(_));
                let zisk_proof = ZiskVadcopFinalProof::new(proof.clone(), compressed);
                zisk_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
            ZiskProof::Plonk(snark_proof) | ZiskProof::Fflonk(snark_proof) => {
                let protocol_id = match self {
                    ZiskProof::Plonk(_) => SnarkProtocol::Plonk.protocol_id(),
                    ZiskProof::Fflonk(_) => SnarkProtocol::Fflonk.protocol_id(),
                    _ => unreachable!(),
                };
                let snark_proof = ZiskSnarkProof::new(snark_proof.clone(), protocol_id);
                snark_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<ZiskProof> {
        if let Ok(vadcop_proof) = ZiskVadcopFinalProof::load(path.as_ref()) {
            let proof = if vadcop_proof.compressed {
                ZiskProof::VadcopFinalCompressed(vadcop_proof.proof)
            } else {
                ZiskProof::VadcopFinal(vadcop_proof.proof)
            };
            return Ok(proof);
        }

        if let Ok(snark_proof) = ZiskSnarkProof::load(path.as_ref()) {
            let proof = match SnarkProtocol::from_protocol_id(snark_proof.protocol_id)? {
                SnarkProtocol::Plonk => ZiskProof::Plonk(snark_proof.proof),
                SnarkProtocol::Fflonk => ZiskProof::Fflonk(snark_proof.proof),
            };
            return Ok(proof);
        }

        Err(anyhow::anyhow!("Failed to load proof: unsupported format or corrupted file"))
    }
}

pub const ZISK_PUBLICS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskPublics {
    data: Vec<u8>,
    ptr: Cell<usize>,
}

impl ZiskPublics {
    pub fn new(publics_bytes: &[u8]) -> Self {
        assert!(
            publics_bytes.len() == ZISK_PUBLICS * 8 + 32,
            "Not enough bytes to fill ZiskPublics"
        );

        let mut data = [0u8; ZISK_PUBLICS * 4];
        for (i, chunk) in publics_bytes[32..].chunks_exact(8).enumerate() {
            let v32 = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
            data[i * 4..(i + 1) * 4].copy_from_slice(&v32.to_le_bytes());
        }

        Self { data: data.to_vec(), ptr: Cell::new(0) }
    }

    pub fn new_empty() -> Self {
        Self { data: [0u8; ZISK_PUBLICS * 4].to_vec(), ptr: Cell::new(0) }
    }

    /// Create ZiskPublics from a serializable value.
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

        Ok(Self { data: data.to_vec(), ptr: Cell::new(0) })
    }

    /// Reset the reading pointer to the beginning.
    pub fn head(&self) {
        self.ptr.set(0);
    }

    /// Read raw bytes from public outputs.
    pub fn read_slice(&self, slice: &mut [u8]) {
        let ptr = self.ptr.get();
        slice.copy_from_slice(&self.data[ptr..ptr + slice.len()]);
        self.ptr.set(ptr + slice.len());
    }

    /// Deserialize a value from public outputs.
    /// The value must have been previously written with bincode serialization using `commit()`.
    pub fn read<T: serde::Serialize + serde::de::DeserializeOwned>(&self) -> Result<T> {
        let ptr = self.ptr.get();
        let result: T = bincode::deserialize(&self.data[ptr..])
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))?;
        let nb_bytes = bincode::serialized_size(&result)
            .map_err(|e| anyhow::anyhow!("Failed to get serialized size: {}", e))?;
        self.ptr.set(ptr + nb_bytes as usize);
        Ok(result)
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

    pub fn public_bytes_solidity(&self) -> Vec<u8> {
        let mut bytes = [0u8; ZISK_PUBLICS * 4];

        for i in 0..ZISK_PUBLICS {
            let start = i * 4;
            let val32 = u32::from_le_bytes([
                self.data[start],
                self.data[start + 1],
                self.data[start + 2],
                self.data[start + 3],
            ]);
            bytes[i * 4..(i + 1) * 4].copy_from_slice(&val32.to_be_bytes());
        }

        bytes.to_vec()
    }

    pub fn hash_solidity(&self, program_vk: &ZiskProgramVK, vadcop_verkey: &[u8]) -> Vec<u8> {
        let bytes = self.bytes_solidity(program_vk, vadcop_verkey);

        // SHA-256
        let hash = Sha256::digest(&bytes);

        hash.to_vec()
    }
}

impl ZiskPublics {
    pub fn bytes_u64(&self, program_vk: &ZiskProgramVK) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(program_vk.vk.len() + ZISK_PUBLICS * 8);

        bytes.extend(&program_vk.vk);
        bytes.extend(self.public_bytes());

        bytes
    }

    pub fn bytes_solidity(&self, program_vk: &ZiskProgramVK, vadcop_verkey: &[u8]) -> Vec<u8> {
        let mut prefix = [0u8; 32];
        for (i, chunk) in program_vk.vk.chunks_exact(8).enumerate() {
            let val = u64::from_le_bytes(chunk.try_into().unwrap());
            prefix[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }

        let mut bytes = prefix.to_vec();
        bytes.extend(self.public_bytes_solidity());
        let mut suffix = [0u8; 32];
        for (i, chunk) in vadcop_verkey.chunks_exact(8).enumerate() {
            let val = u64::from_le_bytes(chunk.try_into().unwrap());
            suffix[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }
        bytes.extend(&suffix);
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskProofWithPublicValues {
    pub proof: ZiskProof,
    pub publics: ZiskPublics,
    pub program_vk: ZiskProgramVK,
}

impl ZiskProofWithPublicValues {
    pub fn new(proof: ZiskProof, publics: ZiskPublics, program_vk: ZiskProgramVK) -> Self {
        Self { proof, publics, program_vk }
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
        let proof_with_publics: ZiskProofWithPublicValues = bincode::deserialize_from(file)?;
        Ok(proof_with_publics)
    }

    pub fn get_vadcop_final_proof(&self) -> Result<VadcopFinalProof> {
        match &self.proof {
            ZiskProof::VadcopFinal(proof_bytes) | ZiskProof::VadcopFinalCompressed(proof_bytes) => {
                let compressed = matches!(self.proof, ZiskProof::VadcopFinalCompressed(_));
                let mut pubs = self.program_vk.vk.clone();
                pubs.extend(self.publics.public_bytes());
                Ok(VadcopFinalProof::new(proof_bytes.clone(), pubs, compressed))
            }

            _ => Err(anyhow::anyhow!("Proof is not a Vadcop final proof")),
        }
    }

    pub fn get_proof(&self) -> &ZiskProof {
        &self.proof
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.publics
    }

    pub fn get_program_vk(&self) -> &ZiskProgramVK {
        &self.program_vk
    }

    /// Create ZiskProofWithPublicValues directly from a Vadcop proof byte array.
    ///
    /// This method parses the proof format (n_publics, publics..., proof...) and extracts
    /// the public values and program VK directly, without creating an intermediate VadcopFinalProof.
    ///
    /// # Parameters
    ///
    /// * `proof` - The proof as a slice of u64 values
    /// * `compressed` - Whether the proof is compressed
    ///
    /// # Returns
    ///
    /// A ZiskProofWithPublicValues containing the parsed proof, publics, and program VK
    pub fn new_from_vadcop_proof(proof: &[u64], compressed: bool) -> Result<Self> {
        let vadcop_proof = VadcopFinalProof::new_from_proof(proof, compressed)
            .map_err(|e| anyhow::anyhow!("Failed to parse Vadcop proof: {}", e))?;

        let zisk_proof = if compressed {
            ZiskProof::VadcopFinalCompressed(vadcop_proof.proof)
        } else {
            ZiskProof::VadcopFinal(vadcop_proof.proof)
        };

        Ok(Self {
            proof: zisk_proof,
            publics: ZiskPublics::new(&vadcop_proof.public_values),
            program_vk: ZiskProgramVK::new_from_publics(&vadcop_proof.public_values),
        })
    }
}

pub struct ZiskProveResult {
    execution: ZiskExecutionResult,
    duration: Duration,
    stats: ExecutorStatsHandle,
    proof_id: Option<String>,
    proof_with_publics: ZiskProofWithPublicValues,
}

impl ZiskProveResult {
    pub fn new(
        execution: ZiskExecutionResult,
        duration: Duration,
        stats: ExecutorStatsHandle,
        proof_id: Option<String>,
        proof_with_publics: ZiskProofWithPublicValues,
    ) -> Self {
        Self { execution, duration, stats, proof_id, proof_with_publics }
    }

    pub fn new_null(
        execution: ZiskExecutionResult,
        duration: Duration,
        stats: ExecutorStatsHandle,
    ) -> Self {
        Self {
            execution,
            duration,
            stats,
            proof_id: None,
            proof_with_publics: ZiskProofWithPublicValues {
                proof: ZiskProof::Null(),
                publics: ZiskPublics::new_empty(),
                program_vk: ZiskProgramVK::new_empty(),
            },
        }
    }

    pub fn get_stats(&self) -> &ExecutorStatsHandle {
        &self.stats
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
    }

    pub fn get_execution_steps(&self) -> &u64 {
        &self.execution.steps
    }

    pub fn get_proof_id(&self) -> Option<&String> {
        self.proof_id.as_ref()
    }

    pub fn get_proof(&self) -> &ZiskProof {
        &self.proof_with_publics.proof
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.proof_with_publics.publics
    }

    pub fn get_program_vk(&self) -> &ZiskProgramVK {
        &self.proof_with_publics.program_vk
    }

    pub fn get_proof_with_publics(&self) -> &ZiskProofWithPublicValues {
        &self.proof_with_publics
    }

    pub fn save_proof_with_publics(&self, path: impl AsRef<Path>) -> Result<()> {
        self.proof_with_publics.save(path)
    }

    /// Deserialize a value from public outputs.
    /// The value must have been previously written with bincode serialization using `commit()`.
    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.proof_with_publics.publics.read()
    }
}

pub type ZiskPhaseResult = ProvePhaseResult;

pub struct ZiskAggPhaseResult {
    pub agg_proofs: Vec<AggProofs>,
}

pub trait ProverEngine {
    fn setup(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK>;

    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()>;

    fn set_hints_stream(&self, hints_stream: StreamSource) -> Result<()>;

    fn executed_steps(&self) -> u64;

    fn get_execution_info(&self) -> Result<ExecutionInfo>;

    fn execute(&self, stdin: ZiskStdin, output_path: Option<PathBuf>) -> Result<ZiskExecuteResult>;

    fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)>;

    fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult>;

    fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult>;

    fn vk(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK>;

    fn verify(&self, proof: &ZiskProof, publics: &ZiskPublics, vk: &ZiskProgramVK) -> Result<()>;

    fn prove_debug(&self, stdin: ZiskStdin, proof_options: ProofOpts) -> Result<ZiskProveResult>;

    fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult>;

    fn prove_snark(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues>;

    fn compress(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues>;

    fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult>;

    fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>>;

    fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()>;
}

pub trait ZiskBackend: Send + Sync {
    type Prover: ProverEngine + Send + Sync;
}

pub struct ZiskProver<C: ZiskBackend> {
    pub prover: C::Prover,
}

impl<C: ZiskBackend> ZiskProver<C> {
    /// Create a new ZiskProver with the given prover engine.
    pub fn new(prover: C::Prover) -> Self {
        Self { prover }
    }

    pub fn setup(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        self.prover.setup(elf)
    }

    /// Set the standard input for the current proof.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.prover.set_stdin(stdin)
    }

    pub fn set_hints_stream(&self, hints_stream: StreamSource) -> Result<()> {
        self.prover.set_hints_stream(hints_stream)
    }

    /// Get the world rank of the prover. The world rank is the rank of the prover in the global MPI context.
    /// If MPI is not used, this will always return 0.
    pub fn world_rank(&self) -> i32 {
        self.prover.world_rank()
    }

    /// Get the local rank of the prover. The local rank is the rank of the prover in the local MPI context.
    /// If MPI is not used, this will always return 0.
    pub fn local_rank(&self) -> i32 {
        self.prover.local_rank()
    }

    /// Get the number of executed steps by the prover after a proof generation or execution.
    pub fn executed_steps(&self) -> u64 {
        self.prover.executed_steps()
    }

    pub fn get_execution_info(&self) -> Result<ExecutionInfo> {
        self.prover.get_execution_info()
    }

    /// Execute the prover with the given standard input and output path.
    /// It only runs the execution without generating a proof.
    pub fn execute(&self, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        self.prover.execute(stdin, None)
    }

    /// Get the execution statistics with the given standard input and debug information.
    pub fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        self.prover.stats(stdin, debug_info, minimal_memory, mpi_node)
    }

    /// Verify the constraints with the given standard input and debug information.
    pub fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.prover.verify_constraints_debug(stdin, debug_info)
    }

    /// Verify the constraints with the given standard input.
    pub fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult> {
        self.prover.verify_constraints(stdin)
    }

    pub fn vk(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        self.prover.vk(elf)
    }

    pub fn verify(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<()> {
        self.prover.verify(proof, publics, vk)
    }

    /// Generate a proof with the given standard input.
    /// Returns a `ProveBuilder` that allows setting per-proof options before running.
    ///
    /// # Example
    /// ```ignore
    /// let result = prover.prove(stdin).compressed().run()?;
    /// ```
    pub fn prove(&self, stdin: ZiskStdin) -> ProveBuilder<'_, C> {
        ProveBuilder::new(&self.prover, stdin)
    }

    pub fn prove_snark(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        self.prover.prove_snark(proof, publics, vk)
    }

    pub fn compress(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        self.prover.compress(proof, publics, vk)
    }

    pub fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        self.prover.prove_phase(phase_inputs, options, phase)
    }

    pub fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        self.prover.aggregate_proofs(agg_proofs, last_proof, final_proof, options)
    }

    /// Broadcast data to all MPI processes.
    pub fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        self.prover.mpi_broadcast(data)
    }
}

/// Builder for configuring and running a proof.
///
/// This struct provides a fluent API for setting per-proof options
/// before executing the proof generation.
///
/// # Example
/// ```ignore
/// let result = prover.prove(stdin).compressed().run()?;
/// ```
pub struct ProveBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    stdin: ZiskStdin,
    mode: ProofMode,
    proof_options: ProofOpts,
}

impl<'a, C: ZiskBackend> ProveBuilder<'a, C> {
    fn new(prover: &'a C::Prover, stdin: ZiskStdin) -> Self {
        Self { prover, stdin, mode: ProofMode::VadcopFinal, proof_options: ProofOpts::default() }
    }

    /// Enable compressed proof generation.
    pub fn compressed(mut self) -> Self {
        self.mode = ProofMode::VadcopFinalCompressed;
        self
    }

    pub fn plonk(mut self) -> Self {
        self.mode = ProofMode::Snark;
        self
    }

    pub fn with_proof_options(mut self, options: ProofOpts) -> Self {
        self.proof_options = options;
        self
    }

    /// Execute the proof generation with the configured options.
    pub fn run(self) -> Result<ZiskProveResult> {
        self.prover.prove(self.stdin, self.mode, self.proof_options)
    }

    pub fn run_debug(self) -> Result<ZiskProveResult> {
        self.prover.prove_debug(self.stdin, self.proof_options)
    }
}
