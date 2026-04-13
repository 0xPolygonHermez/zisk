mod asm;
mod backend;
mod emu;
use crate::guest::{GuestProgram, ProgramId};
pub use asm::*;
use backend::*;
pub use emu::*;
use executor::get_packed_info;
use proofman::{
    AggProofs, AggProofsRegister, ProvePhase, ProvePhaseInputs, ProvePhaseResult, WitnessInfo,
};
use proofman_common::{ProofOptions, ProofmanOptions, RowInfo};

use anyhow::{anyhow, Result};
use asm_runner::HintsShmem;
use precompiles_hints::HintsProcessor;
use proofman::PlanningInfo;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    ExecutorStatsHandle, ProofMode, StatsCostPerType, ZiskExecutorSummary, ZiskExecutorTime,
    ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues, ZiskPublics, ZiskVK, ZiskVerifyBuilder,
};
use zisk_core::ZiskRom;

pub struct ZiskExecuteResult {
    pub total_duration: Duration,
    pub executor_summary: ZiskExecutorSummary,
    pub planning_info: PlanningInfo,
    pub publics: ZiskPublics,
}

impl ZiskExecuteResult {
    pub fn new(
        total_duration: Duration,
        executor_summary: ZiskExecutorSummary,
        planning_info: PlanningInfo,
        publics: &[u8],
    ) -> Self {
        Self { total_duration, executor_summary, planning_info, publics: ZiskPublics::new(publics) }
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.publics
    }

    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.publics.read()
    }

    pub fn get_public_values_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        self.publics.read_abi()
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.executor_summary.steps
    }

    pub fn get_execution_total_cost(&self) -> u64 {
        self.executor_summary.cost_per_type.total_cost()
    }

    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        &self.executor_summary.cost_per_type
    }

    pub fn get_duration(&self) -> Duration {
        self.total_duration
    }

    /// Construct a result from a remote gateway response.
    pub fn from_remote(
        steps: u64,
        duration: Duration,
        cost_per_type: StatsCostPerType,
        publics: &[u8],
    ) -> Self {
        let executor_summary = ZiskExecutorSummary {
            steps,
            executor_time: ZiskExecutorTime { total_duration: duration, ..Default::default() },
            cost_per_type,
        };
        Self {
            total_duration: duration,
            executor_summary,
            planning_info: PlanningInfo { planning_info: vec![], num_instances: 0 },
            publics: ZiskPublics::new(publics),
        }
    }
}

pub struct ZiskVerifyConstraintsResult {
    pub executor_summary: ZiskExecutorSummary,
    pub duration: Duration,
    pub stats: ExecutorStatsHandle,
    pub publics: ZiskPublics,
}

impl ZiskVerifyConstraintsResult {
    pub fn new(
        execution: ZiskExecutorSummary,
        duration: Duration,
        stats: ExecutorStatsHandle,
        publics: &[u8],
    ) -> Self {
        Self { executor_summary: execution, duration, stats, publics: ZiskPublics::new(publics) }
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.publics
    }

    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.publics.read()
    }

    pub fn get_public_values_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        self.publics.read_abi()
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.executor_summary.steps
    }

    pub fn get_execution_total_cost(&self) -> u64 {
        self.executor_summary.cost_per_type.total_cost()
    }

    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        &self.executor_summary.cost_per_type
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
    }
}

/// ASM-specific configuration options
#[derive(Clone, Default)]
pub struct AsmOptions {
    pub asm_path: Option<PathBuf>,
    pub base_port: Option<u16>,
    pub no_auto_setup: bool,
    pub unlock_mapped_memory: bool,
    pub asm_out_file: bool,
    pub is_distributed: bool,
}

impl AsmOptions {
    pub fn asm_path(mut self, path: PathBuf) -> Self {
        self.asm_path = Some(path);
        self
    }

    pub fn base_port(mut self, port: u16) -> Self {
        self.base_port = Some(port);
        self
    }

    pub fn no_auto_setup(mut self) -> Self {
        self.no_auto_setup = true;
        self
    }

    pub fn unlock_mapped_memory(mut self) -> Self {
        self.unlock_mapped_memory = true;
        self
    }

    pub fn asm_out_file(mut self) -> Self {
        self.asm_out_file = true;
        self
    }

    pub fn is_distributed(mut self) -> Self {
        self.is_distributed = true;
        self
    }
}

/// Comprehensive prover configuration containing all settings
#[derive(Clone)]
pub struct BackendProverOpts {
    // Proof settings
    pub aggregation: bool,
    pub verify_proofs: bool,
    pub minimal_memory: bool,
    pub output_dir_path: Option<PathBuf>,
    pub verbose: u8,

    // Proving keys
    pub proving_key: Option<PathBuf>,
    pub proving_key_snark: Option<PathBuf>,

    pub plonk: bool,         // Whether to generate PLONK/SNARK proofs are allowed
    pub preload_plonk: bool, // Whether to preload PLONK/SNARK proving keys

    // MPI settings
    pub shared_tables: bool,
    pub rma: bool,

    // ProofmanOptions fields (flattened)
    pub preallocate_fixed_gpu: bool,
    pub gpu: bool,
    pub packed: bool,
    pub max_witness_stored: Option<usize>,
    pub number_threads_witness: Option<usize>,
    pub max_streams: Option<usize>,

    // ASM-specific options
    pub asm_options: AsmOptions,
}

impl Default for BackendProverOpts {
    fn default() -> Self {
        Self {
            aggregation: true,
            verify_proofs: false,
            rma: false,
            minimal_memory: false,
            output_dir_path: None,
            verbose: 0,
            proving_key: None,
            proving_key_snark: None,
            preload_plonk: false,
            plonk: false,
            shared_tables: true,
            preallocate_fixed_gpu: false,
            gpu: false,
            packed: false,
            max_witness_stored: None,
            number_threads_witness: None,
            max_streams: None,
            asm_options: AsmOptions::default(),
        }
    }
}

impl BackendProverOpts {
    /// Build ProofmanOptions from the configuration fields
    pub fn build_proofman_options(&self) -> ProofmanOptions {
        let mut options = ProofmanOptions::new();

        if let Some(max_witness_stored) = self.max_witness_stored {
            options.with_max_witness_stored(max_witness_stored);
        }
        if let Some(number_threads_witness) = self.number_threads_witness {
            options.with_number_threads_pools_witness(number_threads_witness);
        }
        if let Some(max_streams) = self.max_streams {
            options.with_max_number_streams(max_streams);
        }

        if self.gpu {
            options.gpu();
        }

        if self.packed {
            options.packed();
        }

        // Only call packed_info when packed or gpu is enabled
        if self.packed || self.gpu {
            options.packed_info(get_packed_info());
        }

        options
    }

    // Builder methods for all configuration
    pub fn aggregation(mut self, value: bool) -> Self {
        self.aggregation = value;
        self
    }

    pub fn no_aggregation(mut self) -> Self {
        self.aggregation = false;
        self
    }

    pub fn verify_proofs(mut self) -> Self {
        self.verify_proofs = true;
        self
    }

    pub fn rma(mut self, value: bool) -> Self {
        self.rma = value;
        self
    }

    pub fn minimal_memory(mut self) -> Self {
        self.minimal_memory = true;
        self
    }

    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir_path = Some(path);
        self
    }

    pub fn verbose(mut self, level: u8) -> Self {
        self.verbose = level;
        self
    }

    pub fn proving_key(mut self, path: PathBuf) -> Self {
        self.proving_key = Some(path);
        self
    }

    pub fn proving_key_plonk(mut self, path: PathBuf) -> Self {
        self.proving_key_snark = Some(path);
        self
    }

    pub fn plonk(mut self, preload: bool) -> Self {
        self.plonk = true;
        if preload {
            self.preload_plonk = true;
        }
        self
    }

    pub fn shared_tables(mut self, value: bool) -> Self {
        self.shared_tables = value;
        self
    }

    pub fn preallocate_fixed_gpu(mut self) -> Self {
        self.preallocate_fixed_gpu = true;
        self
    }

    pub fn gpu(mut self) -> Self {
        self.gpu = true;
        self
    }

    pub fn packed(mut self) -> Self {
        self.packed = true;
        self
    }

    pub fn max_witness_stored(mut self, max: usize) -> Self {
        self.max_witness_stored = Some(max);
        self
    }

    pub fn number_threads_witness(mut self, threads: usize) -> Self {
        self.number_threads_witness = Some(threads);
        self
    }

    pub fn max_streams(mut self, max: usize) -> Self {
        self.max_streams = Some(max);
        self
    }

    pub fn with_asm_options(mut self, options: AsmOptions) -> Self {
        self.asm_options = options;
        self
    }
}

pub struct ZiskProveResult {
    pub executor_summary: ZiskExecutorSummary,
    pub duration: Duration,
    stats: ExecutorStatsHandle,
    proof_id: Option<String>,
    proof_with_publics: ZiskProofWithPublicValues,
}

impl ZiskProveResult {
    pub fn new(
        execution: ZiskExecutorSummary,
        duration: Duration,
        stats: ExecutorStatsHandle,
        proof_id: Option<String>,
        proof_with_publics: ZiskProofWithPublicValues,
    ) -> Self {
        Self { executor_summary: execution, duration, stats, proof_id, proof_with_publics }
    }

    pub fn new_null(
        execution: ZiskExecutorSummary,
        duration: Duration,
        stats: ExecutorStatsHandle,
    ) -> Self {
        Self {
            executor_summary: execution,
            duration,
            stats,
            proof_id: None,
            proof_with_publics: ZiskProofWithPublicValues {
                proof: ZiskProof::Null(),
                publics: ZiskPublics::new_empty(),
                program_vk: ZiskProgramVK::new_empty(),
                zisk_vk: ZiskVK { vk: Vec::new() },
                plonk_vkey: None,
            },
        }
    }

    /// Construct a result from a remote gateway response (no ExecutorStatsHandle).
    pub fn from_remote(
        proof_with_publics: ZiskProofWithPublicValues,
        steps: u64,
        duration: Duration,
        cost_per_type: StatsCostPerType,
    ) -> Self {
        let executor_summary = ZiskExecutorSummary {
            steps,
            executor_time: ZiskExecutorTime { total_duration: duration, ..Default::default() },
            cost_per_type,
        };
        Self {
            executor_summary,
            duration,
            stats: ExecutorStatsHandle::default(),
            proof_id: None,
            proof_with_publics,
        }
    }

    pub fn get_stats(&self) -> &ExecutorStatsHandle {
        &self.stats
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.executor_summary.steps
    }

    pub fn get_execution_total_cost(&self) -> u64 {
        self.executor_summary.cost_per_type.total_cost()
    }

    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        &self.executor_summary.cost_per_type
    }

    pub fn get_proof_id(&self) -> Option<&String> {
        self.proof_id.as_ref()
    }

    pub fn get_proof(&self) -> &ZiskProofWithPublicValues {
        &self.proof_with_publics
    }

    pub fn get_proof_bytes(&self) -> Vec<u8> {
        self.proof_with_publics.get_proof_bytes()
    }

    pub fn get_publics(&self) -> &ZiskPublics {
        &self.proof_with_publics.publics
    }

    pub fn get_program_vk(&self) -> &ZiskProgramVK {
        &self.proof_with_publics.program_vk
    }

    pub fn save_proof(&self, path: impl AsRef<Path>) -> Result<()> {
        self.proof_with_publics.save(path)
    }

    /// Deserialize a value from public outputs.
    /// The value must have been previously written with bincode serialization using `commit()`.
    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.proof_with_publics.publics.read()
    }

    /// Decode an ABI-encoded value from public outputs.
    /// The value must have been previously written with ABI encoding using `write_abi()`.
    pub fn get_public_values_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        self.proof_with_publics.publics.read_abi()
    }

    pub fn verify(&self) -> Result<()> {
        self.proof_with_publics.verify()
    }

    pub fn with_publics<'a>(&'a self, publics: &'a ZiskPublics) -> ZiskVerifyBuilder<'a> {
        self.proof_with_publics.with_publics(publics)
    }

    pub fn with_program_vk<'a>(&'a self, program_vk: &'a ZiskProgramVK) -> ZiskVerifyBuilder<'a> {
        self.proof_with_publics.with_program_vk(program_vk)
    }
}

pub type ZiskPhaseResult = ProvePhaseResult;

pub struct ZiskAggPhaseResult {
    pub agg_proofs: Vec<AggProofs>,
}

pub trait ProverEngine {
    /// Builder type returned by setup()
    type Builder<'a>
    where
        Self: 'a;

    /// Internal setup implementation (called by builder's run())
    fn setup_internal(&self, elf: &GuestProgram, with_hints: bool) -> Result<()>;

    /// Create a setup builder for the given ELF program.
    ///
    /// Returns a builder that allows optional configuration (like `.with_hints()` for ASM)
    /// before executing with `.run()`.
    fn setup<'a>(&'a self, elf: &'a GuestProgram) -> Self::Builder<'a>;

    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()>;

    fn register_program(&self, program_id: &ProgramId) -> Result<()>;

    fn executed_steps(&self) -> u64;

    fn get_execution_info(&self) -> Result<(WitnessInfo, ZiskExecutorTime)>;

    fn get_instance_trace(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>>;

    fn get_instance_air_values(&self, instance_id: usize) -> Result<Vec<u64>>;

    fn get_instance_fixed(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>>;

    fn execute(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult>;

    fn stats(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)>;

    fn verify_constraints(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult>;

    fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK>;

    fn prove(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        mode: ProofMode,
        prover_options: BackendProverOpts,
    ) -> Result<ZiskProveResult>;

    fn wrap_proof(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
        mode: ProofMode,
    ) -> Result<ZiskProofWithPublicValues>;

    fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult>;

    fn set_partition(
        &self,
        total_compute_units: usize,
        allocation: Vec<u32>,
        rank_id: usize,
    ) -> Result<()>;

    fn register_aggregated_proofs(&self, agg_proofs: Vec<AggProofsRegister>) -> Result<()>;

    fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>>;

    fn get_vadcop_vk(&self, minimal: bool) -> Result<ZiskVK>;

    fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()>;

    // --- ASM-only operations ---
    // These are meaningful only for the ASM backend (distributed worker path).
    // Data operations (submit_hint, submit_input, register_hints_stream) return Err by
    // default because calling them on a non-ASM backend is always a caller bug.
    // State operations (set_active_services, reset_resources) default to no-ops because
    // they are safe to skip when there are no ASM resources to manage.

    fn submit_hint(&self, _bytes: &[u8]) -> Result<()> {
        Err(anyhow::anyhow!("submit_hint not supported by this backend"))
    }

    fn submit_input(&self, _bytes: &[u8]) -> Result<()> {
        Err(anyhow::anyhow!("submit_input not supported by this backend"))
    }

    fn register_hints_stream(&self, _stream: StreamSource) -> Result<()> {
        Err(anyhow::anyhow!("register_hints_stream not supported by this backend"))
    }

    fn get_hints_processor(&self) -> Result<Option<Arc<HintsProcessor<HintsShmem>>>> {
        Ok(None)
    }

    fn set_active_services(&self, _is_first_partition: bool) -> Result<()> {
        Ok(())
    }

    fn reset_resources(&self) -> Result<()> {
        Ok(())
    }

    fn cancel(&self);
}

pub trait ZiskBackend: Send + Sync {
    type Prover: ProverEngine + Send + Sync;
}

pub struct ZiskProver<C: ZiskBackend> {
    pub prover: C::Prover,
    program_cache: RwLock<HashMap<ProgramId, Arc<ZiskRom>>>,
    prover_options: BackendProverOpts,
}

impl<C: ZiskBackend> ZiskProver<C> {
    /// Create a new ZiskProver with the given prover engine and prover options.
    pub fn new(prover: C::Prover, prover_options: BackendProverOpts) -> Self {
        Self { prover, program_cache: RwLock::new(HashMap::new()), prover_options }
    }

    pub fn get_cached_program(&self, program_id: &ProgramId) -> Result<Arc<ZiskRom>> {
        let cache =
            self.program_cache.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;

        cache.get(program_id).cloned().ok_or_else(|| anyhow!("Program not found: {:?}", program_id))
    }

    /// Set the standard input for the current proof.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.prover.set_stdin(stdin)
    }

    pub fn register_program(&self, program_id: &ProgramId) -> Result<()> {
        self.prover.register_program(program_id)
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

    pub fn get_execution_info(&self) -> Result<(WitnessInfo, ZiskExecutorTime)> {
        self.prover.get_execution_info()
    }

    /// Execute the prover with the given standard input and output path.
    /// It only runs the execution without generating a proof.
    /// The program must have been setup previously using `.setup()`.
    pub fn execute(&self, program: &GuestProgram, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        self.prover.execute(program, stdin, None)
    }

    /// Get the execution statistics with the given standard input and debug information.
    /// The program must have been setup previously using `.setup()`.
    pub fn stats(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        self.prover.stats(program, stdin, debug_info, minimal_memory, mpi_node)
    }

    /// Get the instance trace for a given instance ID and row range.
    pub fn get_instance_trace(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        self.prover.get_instance_trace(instance_id, first_row, num_rows, offset)
    }

    /// Get the instance AIR values for a given instance ID.
    pub fn get_instance_air_values(&self, instance_id: usize) -> Result<Vec<u64>> {
        self.prover.get_instance_air_values(instance_id)
    }

    /// Get the instance fixed for a given instance ID and row range.
    pub fn get_instance_fixed(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        self.prover.get_instance_fixed(instance_id, first_row, num_rows, offset)
    }

    /// Verify the constraints with the given standard input and debug information.
    /// Verify the constraints with the given standard input and optional debug information.
    /// The program must have been setup previously using `.setup()`.
    pub fn verify_constraints(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.prover.verify_constraints(program, stdin, debug_info)
    }

    pub fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK> {
        self.prover.vk(elf)
    }

    /// Generate a proof with the given standard input.
    /// Returns a `ProveBuilder` that allows setting per-proof options before running.
    /// The program must have been setup previously using `.setup()`.
    ///
    /// # Example
    /// ```ignore
    /// let result = prover.prove(&program, stdin)?.minimal().run()?;
    /// ```
    pub fn prove<'a>(&'a self, program: &'a GuestProgram, stdin: ZiskStdin) -> ProveBuilder<'a, C> {
        ProveBuilder::new(&self.prover, self, program, stdin)
    }

    /// Wrap an existing proof to a different format (minimal or PLONK/SNARK).
    /// Returns a `WrapBuilder` that allows overriding publics or program_vk before wrapping.
    ///
    /// # Example
    /// ```ignore
    /// // Wrap to minimal
    /// let minimal = prover.wrap_proof(&proof_with_publics, ProofMode::VadcopFinalMinimal).run()?;
    ///
    /// // Wrap to SNARK with custom verification key
    /// let snark = prover.wrap_proof(&proof_with_publics, ProofMode::Plonk)
    ///     .with_program_vk(&custom_vk)
    ///     .run()?;
    /// ```
    pub fn wrap_proof<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
        mode: ProofMode,
    ) -> WrapBuilder<'a, C> {
        WrapBuilder::new(&self.prover, proof_with_publics, mode)
    }

    pub fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        self.prover.prove_phase(phase_inputs, options, phase)
    }

    pub fn set_partition(
        &self,
        total_compute_units: usize,
        allocation: Vec<u32>,
        rank_id: usize,
    ) -> Result<()> {
        self.prover.set_partition(total_compute_units, allocation, rank_id)
    }

    pub fn register_aggregated_proofs(&self, agg_proofs: Vec<AggProofsRegister>) -> Result<()> {
        self.prover.register_aggregated_proofs(agg_proofs)
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

    /// Get the Vadcop verification key.
    ///
    /// # Parameters
    ///
    /// * `minimal` - If true, returns the minimal verification key.
    ///   If false, returns the full verification key.
    pub fn get_vadcop_vk(&self, minimal: bool) -> Result<ZiskVK> {
        self.prover.get_vadcop_vk(minimal)
    }

    pub fn submit_hint(&self, bytes: &[u8]) -> Result<()> {
        self.prover.submit_hint(bytes)
    }

    pub fn submit_input(&self, bytes: &[u8]) -> Result<()> {
        self.prover.submit_input(bytes)
    }

    pub fn register_hints_stream(&self, stream: StreamSource) -> Result<()> {
        self.prover.register_hints_stream(stream)
    }

    pub fn get_hints_processor(&self) -> Result<Option<Arc<HintsProcessor<HintsShmem>>>> {
        self.prover.get_hints_processor()
    }

    pub fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        self.prover.set_active_services(is_first_partition)
    }

    pub fn reset_resources(&self) -> Result<()> {
        self.prover.reset_resources()
    }

    pub fn cancel(&self) {
        self.prover.cancel()
    }
}

// ASM-specific setup implementation
impl ZiskProver<Asm> {
    /// Setup a guest program and return a builder for optional configuration.
    ///
    /// Returns a builder that allows optional configuration (like `.with_hints()`)
    /// before executing with `.run()`. The ZiskRom is automatically cached.
    ///
    /// # Example
    /// ```ignore
    /// // ASM backend with hints
    /// prover.setup(&program).with_hints().run()?;
    ///
    /// // ASM backend without hints
    /// prover.setup(&program).run()?;
    /// ```
    pub fn setup<'a>(&'a self, elf: &'a GuestProgram) -> AsmSetupBuilder<'a> {
        self.prover.setup(elf)
    }

    /// Returns `true` if the last `setup()` call used `.with_hints()`.
    pub fn was_setup_with_hints(&self) -> Result<bool> {
        Ok(self.get_hints_processor()?.is_some())
    }
}

// EMU-specific setup implementation
impl ZiskProver<Emu> {
    /// Setup a guest program and return a builder.
    ///
    /// Returns a builder that must be executed with `.run()`.
    /// The ZiskRom is automatically cached.
    ///
    /// # Example
    /// ```ignore
    /// prover.setup(&program).run()?;
    /// ```
    pub fn setup<'a>(&'a self, elf: &'a GuestProgram) -> EmuSetupBuilder<'a> {
        self.prover.setup(elf)
    }
}

/// Builder for configuring and running a proof.
///
/// This struct provides a fluent API for setting the proof mode
/// before executing the proof generation.
///
/// # Example
/// ```ignore
/// let result = prover.prove(stdin).minimal().run()?;
/// ```
pub struct ProveBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    zisk_prover: &'a ZiskProver<C>,
    guest_program: &'a GuestProgram,
    stdin: ZiskStdin,
    mode: ProofMode,
}

impl<'a, C: ZiskBackend> ProveBuilder<'a, C> {
    fn new(
        prover: &'a C::Prover,
        zisk_prover: &'a ZiskProver<C>,
        guest_program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> Self {
        Self { prover, zisk_prover, guest_program, stdin, mode: ProofMode::VadcopFinal }
    }

    /// Enable minimal proof generation.
    pub fn wrap_proof(mut self, proof_mode: ProofMode) -> Self {
        assert!(
            matches!(proof_mode, ProofMode::VadcopFinalMinimal | ProofMode::Plonk),
            "Invalid proof mode for ProveBuilder: {:?}",
            proof_mode
        );
        self.mode = proof_mode;
        self
    }

    /// Execute the proof generation with the configured options.
    pub fn run(self) -> Result<ZiskProveResult> {
        self.prover.prove(
            self.guest_program,
            self.stdin,
            self.mode,
            self.zisk_prover.prover_options.clone(),
        )
    }
}

/// Builder for wrapping/converting proofs to different formats.
///
/// This builder allows optionally overriding the publics or program verification key
/// from the original proof before wrapping it to the target format (minimal or SNARK).
///
/// # Example
/// ```ignore
/// // Wrap to minimal
/// let minimal = prover.wrap(&proof_with_publics, ProofMode::VadcopFinalMinimal).run()?;
///
/// // Wrap to SNARK with custom publics
/// let snark = prover.wrap(&proof_with_publics, ProofMode::Plonk)
///     .with_publics(&custom_publics)
///     .run()?;
/// ```
pub struct WrapBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    proof_with_publics: &'a ZiskProofWithPublicValues,
    mode: ProofMode,
    override_publics: Option<&'a ZiskPublics>,
    override_program_vk: Option<&'a ZiskProgramVK>,
}

impl<'a, C: ZiskBackend> WrapBuilder<'a, C> {
    fn new(
        prover: &'a C::Prover,
        proof_with_publics: &'a ZiskProofWithPublicValues,
        mode: ProofMode,
    ) -> Self {
        Self { prover, proof_with_publics, mode, override_publics: None, override_program_vk: None }
    }

    /// Override the publics from the original proof.
    pub fn with_publics(mut self, publics: &'a ZiskPublics) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key from the original proof.
    pub fn with_program_vk(mut self, program_vk: &'a ZiskProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Execute the proof wrapping with the configured options.
    pub fn run(self) -> Result<ZiskProofWithPublicValues> {
        let publics = self.override_publics.unwrap_or(&self.proof_with_publics.publics);
        let program_vk = self.override_program_vk.unwrap_or(&self.proof_with_publics.program_vk);
        self.prover.wrap_proof(&self.proof_with_publics.proof, publics, program_vk, self.mode)
    }
}
