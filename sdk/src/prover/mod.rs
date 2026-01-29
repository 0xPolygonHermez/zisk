mod asm;
mod backend;
mod emu;
pub use asm::*;
use backend::*;
pub use emu::*;
use proofman::{
    AggProofs, ProvePhase, ProvePhaseInputs, ProvePhaseResult, SnarkProof, SnarkProtocol,
};
use proofman_common::ProofOptions;
use proofman_util::VadcopFinalProof;
use sha2::{Digest, Sha256};

use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use zisk_common::{io::ZiskStdin, ExecutorStats, ZiskExecutionResult};

pub struct ZiskExecuteResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
}

pub struct ZiskVerifyConstraintsResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub stats: ExecutorStats,
}

pub struct ZiskProgramVK {
    pub vk: Vec<u8>,
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

#[derive(Debug, PartialEq, Eq)]
pub enum Proof {
    Null(),
    VadcopFinal(Vec<u8>),
    VadcopFinalCompressed(Vec<u8>),
    Plonk(Vec<u8>),
    Fflonk(Vec<u8>),
}

pub struct ZiskPublics {
    rom_root: [u64; 4],
    publics: [u64; 64],
}

impl ZiskPublics {
    pub fn get_num_publics(&self) -> usize {
        68
    }

    pub fn new(publics_bytes: Vec<u8>) -> Self {
        assert!(publics_bytes.len() == 544, "Not enough bytes to fill ZiskPublics");

        let mut rom_root = [0u64; 4];
        for (i, r) in rom_root.iter_mut().enumerate() {
            let start = i * 8;
            let end = start + 8;
            *r = u64::from_le_bytes(publics_bytes[start..end].try_into().unwrap());
        }

        let mut publics = [0u64; 64];
        for (i, p) in publics.iter_mut().enumerate() {
            let start = 32 + i * 8;
            let end = start + 8;
            *p = u64::from_le_bytes(publics_bytes[start..end].try_into().unwrap());
        }

        Self { rom_root, publics }
    }

    pub fn new_empty() -> Self {
        Self { rom_root: [0; 4], publics: [0; 64] }
    }

    pub fn get_publics(&self) -> &[u64; 64] {
        &self.publics
    }

    pub fn get_publics_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(512);

        for &v in &self.publics {
            bytes.extend_from_slice(&v.to_le_bytes());
        }

        bytes
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(544);

        for &v in &self.rom_root {
            bytes.extend_from_slice(&v.to_le_bytes());
        }

        for &v in &self.publics {
            bytes.extend_from_slice(&v.to_le_bytes());
        }

        bytes
    }

    pub fn bytes_solidity(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(288);

        for &v in &self.rom_root {
            bytes.extend_from_slice(&v.to_be_bytes());
        }

        for &v in &self.publics {
            let v32 = v as u32;
            bytes.extend_from_slice(&v32.to_be_bytes());
        }

        bytes
    }

    pub fn hash_solidity(&self) -> Vec<u8> {
        let bytes = self.bytes_solidity();

        // SHA-256
        let hash = Sha256::digest(&bytes);

        hash.to_vec()
    }
}

pub struct ZiskProveResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub stats: ExecutorStats,
    pub proof_id: Option<String>,
    pub proof: Proof,
    pub publics: ZiskPublics,
}

impl ZiskProveResult {
    pub fn new(
        execution: ZiskExecutionResult,
        duration: Duration,
        stats: ExecutorStats,
        proof_id: Option<String>,
        proof: Proof,
        publics: ZiskPublics,
    ) -> Self {
        Self { execution, duration, stats, proof_id, proof, publics }
    }

    pub fn new_null(
        execution: ZiskExecutionResult,
        duration: Duration,
        stats: ExecutorStats,
    ) -> Self {
        Self {
            execution,
            duration,
            stats,
            proof_id: None,
            proof: Proof::Null(),
            publics: ZiskPublics::new_empty(),
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        match &self.proof {
            Proof::Null() => Err(anyhow::anyhow!("No proof to save")),
            Proof::VadcopFinal(proof) => {
                let vadcop_final_proof =
                    VadcopFinalProof::new(proof.clone(), self.publics.bytes(), false);
                vadcop_final_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
            Proof::VadcopFinalCompressed(proof) => {
                let vadcop_final_proof =
                    VadcopFinalProof::new(proof.clone(), self.publics.bytes(), true);
                vadcop_final_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
            Proof::Plonk(snark_proof) => {
                let plonk_proof = SnarkProof {
                    proof_bytes: snark_proof.clone(),
                    public_bytes: self.publics.bytes_solidity(),
                    public_snark_bytes: self.publics.hash_solidity(),
                    protocol_id: SnarkProtocol::Plonk.protocol_id(),
                };
                plonk_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
            Proof::Fflonk(snark_proof) => {
                let plonk_proof = SnarkProof {
                    proof_bytes: snark_proof.clone(),
                    public_bytes: self.publics.bytes_solidity(),
                    public_snark_bytes: self.publics.hash_solidity(),
                    protocol_id: SnarkProtocol::Fflonk.protocol_id(),
                };
                plonk_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        if let Ok(vadcop_proof) = VadcopFinalProof::load(path.as_ref()) {
            let publics = ZiskPublics::new(vadcop_proof.public_values);
            let proof = if vadcop_proof.compressed {
                Proof::VadcopFinalCompressed(vadcop_proof.proof)
            } else {
                Proof::VadcopFinal(vadcop_proof.proof)
            };

            return Ok(Self {
                execution: ZiskExecutionResult::default(),
                duration: Duration::default(),
                stats: ExecutorStats::default(),
                proof_id: None,
                proof,
                publics,
            });
        }

        if let Ok(snark_proof) = SnarkProof::load(path.as_ref()) {
            let publics_bytes = snark_proof.public_bytes;

            let mut rom_root = [0u64; 4];
            for (i, item) in rom_root.iter_mut().enumerate() {
                let start = i * 8;
                let bytes: [u8; 8] = publics_bytes[start..start + 8]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid public bytes length"))?;
                *item = u64::from_be_bytes(bytes);
            }

            let mut publics = [0u64; 64];
            for (i, item) in publics.iter_mut().enumerate() {
                let start = 32 + i * 4;
                let bytes: [u8; 4] = publics_bytes[start..start + 4]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid public bytes length"))?;
                *item = u32::from_be_bytes(bytes) as u64;
            }

            let zisk_publics = ZiskPublics { rom_root, publics };

            let proof = match SnarkProtocol::from_protocol_id(snark_proof.protocol_id)? {
                SnarkProtocol::Plonk => Proof::Plonk(snark_proof.proof_bytes),
                SnarkProtocol::Fflonk => Proof::Fflonk(snark_proof.proof_bytes),
            };

            return Ok(Self {
                execution: ZiskExecutionResult::default(),
                duration: Duration::default(),
                stats: ExecutorStats::default(),
                proof_id: None,
                proof,
                publics: zisk_publics,
            });
        }

        Err(anyhow::anyhow!("Failed to load proof: unsupported format or corrupted file"))
    }

    pub fn get_publics(&self) -> &[u64; 64] {
        &self.publics.publics
    }

    pub fn hash_solidity(&self) -> Vec<u8> {
        self.publics.hash_solidity()
    }
}

pub type ZiskPhaseResult = ProvePhaseResult;

pub struct ZiskAggPhaseResult {
    pub agg_proofs: Vec<AggProofs>,
}

pub trait ProverEngine {
    fn setup(&self, elf_path: &str) -> Result<ZiskProgramVK>;

    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()>;

    fn executed_steps(&self) -> u64;

    fn execute(&self, stdin: ZiskStdin, output_path: Option<PathBuf>) -> Result<ZiskExecuteResult>;

    fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)>;

    fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult>;

    fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult>;

    fn vk(&self, elf_path: PathBuf) -> Result<ZiskProgramVK>;

    fn verify(&self, proof: &ZiskProveResult, vk: &ZiskProgramVK) -> Result<()>;

    fn prove_debug(&self, stdin: ZiskStdin, proof_options: ProofOpts) -> Result<ZiskProveResult>;

    fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult>;

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

    pub fn setup(&self, elf_path: &str) -> Result<ZiskProgramVK> {
        self.prover.setup(elf_path)
    }

    /// Set the standard input for the current proof.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.prover.set_stdin(stdin)
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
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
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

    pub fn vk(&self, elf_path: PathBuf) -> Result<ZiskProgramVK> {
        self.prover.vk(elf_path)
    }

    pub fn verify(&self, proof: &ZiskProveResult, vk: &ZiskProgramVK) -> Result<()> {
        self.prover.verify(proof, vk)
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
