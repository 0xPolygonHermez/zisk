mod asm;
mod asm_exec;
mod backend;
mod emu;
mod emu_exec;
use crate::guest::{GuestProgram, ProgramId};
pub use asm::*;
pub use asm_exec::*;
use backend::*;
pub use emu::*;
pub use emu_exec::*;
use proofman::{
    AggProofs, AggProofsRegister, ProvePhase, ProvePhaseInputs, ProvePhaseResult, WitnessInfo,
};
use proofman_common::{ProofOptions, ProofmanOptions, RowInfo};
use proofman_verifier::VadcopFinalProof;
use zisk_pil::get_packed_info;

use anyhow::{anyhow, Result};
use asm_runner::HintsShmem;
use precompiles_hints::HintsProcessor;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    AirInstanceCount, ExecutorStatsHandle, ProgramVK, Proof, ProofBody, ProofKind, PublicValues,
    StatsCostPerType, ZiskExecutorTime,
};
use zisk_core::ZiskRom;

use crate::{ExecuteOutput, ProveOutput, VerifyConstraintsOutput};

/// ASM-specific configuration options
#[derive(Clone, Default)]
pub struct AsmOptions {
    pub asm_path: Option<PathBuf>,
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
    pub(crate) aggregation: bool,
    pub(crate) verify_constraints: bool,
    pub(crate) verify_proofs: bool,
    pub(crate) minimal_memory: bool,
    pub(crate) verbose: u8,

    // Proving keys
    pub(crate) proving_key: Option<PathBuf>,
    pub(crate) proving_key_snark: Option<PathBuf>,

    pub(crate) plonk: bool, // Whether to generate PLONK/SNARK proofs are allowed
    pub(crate) preload_plonk: bool, // Whether to preload PLONK/SNARK proving keys

    // ProofmanOptions fields (flattened)
    pub(crate) gpu: bool,
    pub(crate) cpu_mops: bool,
    pub(crate) packed: bool,
    pub(crate) max_witness_stored: Option<usize>,
    pub(crate) number_threads_witness: Option<usize>,
    pub(crate) max_streams: Option<usize>,

    // ASM-specific options
    pub(crate) asm_options: AsmOptions,
}

impl Default for BackendProverOpts {
    fn default() -> Self {
        Self {
            aggregation: true,
            verify_constraints: false,
            verify_proofs: false,

            minimal_memory: false,
            verbose: 0,
            proving_key: None,
            proving_key_snark: None,
            preload_plonk: false,
            plonk: false,
            gpu: false,
            cpu_mops: false,
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

        options.verbose_mode(self.verbose.into());

        if !self.aggregation || self.verify_constraints {
            options.no_aggregation();
        }

        if self.verify_constraints {
            options.verify_constraints();
        }

        options
    }

    pub fn asm_options(&self) -> &AsmOptions {
        &self.asm_options
    }

    pub fn asm_options_mut(&mut self) -> &mut AsmOptions {
        &mut self.asm_options
    }

    pub fn get_proving_key(&self) -> Option<&PathBuf> {
        self.proving_key.as_ref()
    }

    pub fn get_proving_key_snark(&self) -> Option<&PathBuf> {
        self.proving_key_snark.as_ref()
    }

    pub fn preload_plonk(&self) -> bool {
        self.preload_plonk
    }

    pub fn cpu_mops_enabled(&self) -> bool {
        self.cpu_mops
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

    pub fn verify_constraints(mut self) -> Self {
        self.verify_constraints = true;
        self.aggregation = false;
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

    pub fn gpu(mut self) -> Self {
        self.gpu = true;
        self
    }

    pub fn cpu_mops(mut self) -> Self {
        self.cpu_mops = true;
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
    fn setup_internal(
        &self,
        elf: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
    ) -> Result<ProgramVK>;

    /// Create a setup builder for the given ELF program.
    ///
    /// Returns a builder that allows optional configuration (like `.with_hints()` for ASM)
    /// before executing with `.run()`.
    fn setup<'a>(&'a self, elf: &'a GuestProgram) -> Self::Builder<'a>;

    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()>;

    fn register_program(&self, program_id: &ProgramId, with_hints: bool) -> Result<()>;

    fn executed_steps(&self) -> u64;

    /// Per-type execution cost from the last execution or proof run.
    fn execution_cost_per_type(&self) -> StatsCostPerType;

    /// Per-AIR instance plan from the last execution or proof run.
    fn execution_plan(&self) -> Vec<AirInstanceCount>;

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

    fn execute(&self, program: &GuestProgram, stdin: ZiskStdin) -> Result<ExecuteOutput>;

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
    ) -> Result<VerifyConstraintsOutput>;

    fn prove(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        proof_kind: ProofKind,
        prover_options: BackendProverOpts,
    ) -> Result<ProveOutput>;

    fn wrap_proof(
        &self,
        proof: &[u64],
        publics: &PublicValues,
        vk: &ProgramVK,
        proof_kind: ProofKind,
    ) -> Result<ProveOutput>;

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

    /// Register partial proofs received from other workers of the same job,
    /// ahead of this worker joining them in [`Self::join_worker_proofs`].
    fn register_worker_proofs(&self, agg_proofs: Vec<AggProofsRegister>) -> Result<()>;

    /// Join the workers' partial proofs into the job's aggregated proof (the
    /// in-job Aggregate phase). Unrelated to the recurser's `aggregate_proofs`.
    fn join_worker_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>>;

    fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u64>>;

    fn register_recurser(&self, output_dir: &str, recurser_id: &str) -> Result<()>;

    fn prove_recurser(
        &self,
        recurser_id: &str,
        proof_a: &VadcopFinalProof,
        proof_b: &VadcopFinalProof,
        free_inputs_a: &[u64],
        free_inputs_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
    ) -> Result<VadcopFinalProof>;

    /// Hash family the loaded proving key was generated with (e.g. "Poseidon1" / "Poseidon2").
    fn hash(&self) -> Result<String>;

    fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()>;

    // --- ASM-only operations ---
    // These are meaningful only for the ASM backend (distributed worker path).
    // Data operations (submit_hint, submit_input, register_hints_stream) return Err by
    // default because calling them on a non-ASM backend is always a caller bug.
    // State operations (set_active_services, reset) default to no-ops because
    // they are safe to skip when there are no ASM resources to manage.

    fn submit_hint(&self, _bytes: &[u8]) -> Result<()> {
        Err(anyhow::anyhow!("submit_hint not supported by this backend"))
    }

    fn submit_input(&self, _bytes: &[u8]) -> Result<()> {
        Err(anyhow::anyhow!("submit_input not supported by this backend"))
    }

    fn append_raw_input(&self, _bytes: &[u8]) -> Result<()> {
        Err(anyhow::anyhow!("append_raw_input not supported by this backend"))
    }

    fn register_hints_stream(&self, _stream: StreamSource) -> Result<()> {
        Err(anyhow::anyhow!("register_hints_stream not supported by this backend"))
    }

    fn register_inputs_stream(&self, _stream: StreamSource) -> Result<()> {
        Err(anyhow::anyhow!("register_inputs_stream not supported by this backend"))
    }

    fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>>;

    fn set_active_services(&self, _is_first_process: bool) -> Result<()> {
        Ok(())
    }

    fn reset(&self) -> Result<()> {
        Ok(())
    }

    fn notify_cluster_cancellation(&self) {}

    /// Collective MPI barrier across all ranks. All ranks must call this for
    /// the cluster to make progress. Used to synchronize end-of-task between
    /// rank 0 and peer ranks where there is no implicit aggregation sync
    /// (e.g. execute-only) and at the end of recovery.
    fn cluster_barrier(&self) {}

    /// Cancel the in-flight job.
    ///
    /// Backends with out-of-process workers (ASM C children) must wake those
    /// children here — otherwise their `executor::execute` won't return and
    /// the recovery handshake will deadlock. The Rust-side proofman cancel
    /// flag should be set first so that when the executor unwinds, proofman
    /// already knows to bail.
    fn cancel(&self) -> Result<()>;

    /// Block until any in-flight proofman entry point has returned. Called
    /// in recovery before advertising `Ready`. Default no-op.
    fn wait_until_proofman_ready(&self) {}
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

    pub fn register_program(&self, program_id: &ProgramId, with_hints: bool) -> Result<()> {
        self.prover.register_program(program_id, with_hints)
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

    /// Get the per-type execution cost from the last execution or proof run.
    pub fn execution_cost_per_type(&self) -> StatsCostPerType {
        self.prover.execution_cost_per_type()
    }

    /// Get the per-AIR instance plan from the last execution or proof run.
    pub fn execution_plan(&self) -> Vec<AirInstanceCount> {
        self.prover.execution_plan()
    }

    /// Get the executor time from the last execution or proof run.
    pub fn get_executor_time(&self) -> Result<ZiskExecutorTime> {
        Ok(self.prover.get_execution_info()?.1)
    }

    pub fn get_execution_info(&self) -> Result<(WitnessInfo, ZiskExecutorTime)> {
        self.prover.get_execution_info()
    }

    /// Execute the prover with the given standard input and output path.
    /// It only runs the execution without generating a proof.
    /// The program must have been setup previously using `.setup()`.
    pub fn execute(&self, program: &GuestProgram, stdin: ZiskStdin) -> Result<ExecuteOutput> {
        self.prover.execute(program, stdin)
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
    ) -> Result<VerifyConstraintsOutput> {
        self.prover.verify_constraints(program, stdin, debug_info)
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
    /// let minimal = prover.wrap_proof(&proof, ProofMode::VadcopFinalMinimal).run()?;
    ///
    /// // Wrap to SNARK with custom verification key
    /// let snark = prover.wrap_proof(&proof, ProofMode::Plonk)
    ///     .with_program_vk(&custom_vk)
    ///     .run()?;
    /// ```
    pub fn wrap_proof<'a>(&'a self, proof: &'a Proof, proof_kind: ProofKind) -> WrapBuilder<'a, C> {
        WrapBuilder::new(&self.prover, proof, proof_kind)
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

    pub fn register_worker_proofs(&self, agg_proofs: Vec<AggProofsRegister>) -> Result<()> {
        self.prover.register_worker_proofs(agg_proofs)
    }

    pub fn join_worker_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        self.prover.join_worker_proofs(agg_proofs, last_proof, final_proof, options)
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
    pub fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u64>> {
        self.prover.get_vadcop_vk(minimal)
    }

    /// Hash family the loaded proving key was generated with (e.g. "Poseidon1" / "Poseidon2").
    pub fn hash(&self) -> Result<String> {
        self.prover.hash()
    }

    /// Register a recurser setup so it can prove. One-time, like registering a
    /// program; must precede [`prove_recurser`](Self::prove_recurser).
    pub fn register_recurser(&self, output_dir: &str, recurser_id: &str) -> Result<()> {
        self.prover.register_recurser(output_dir, recurser_id)
    }

    /// Fold two recurser proofs through an already-registered recurser, reusing
    /// this prover's already-initialized proofman (one MPI init per process).
    pub fn prove_recurser(
        &self,
        recurser_id: &str,
        proof_a: &VadcopFinalProof,
        proof_b: &VadcopFinalProof,
        free_inputs_a: &[u64],
        free_inputs_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
    ) -> Result<VadcopFinalProof> {
        self.prover.prove_recurser(
            recurser_id,
            proof_a,
            proof_b,
            free_inputs_a,
            free_inputs_b,
            root_c_recurser_agg,
        )
    }

    pub fn submit_hint(&self, bytes: &[u8]) -> Result<()> {
        self.prover.submit_hint(bytes)
    }

    pub fn submit_input(&self, bytes: &[u8]) -> Result<()> {
        self.prover.submit_input(bytes)
    }

    pub fn append_raw_input(&self, bytes: &[u8]) -> Result<()> {
        self.prover.append_raw_input(bytes)
    }

    pub fn register_hints_stream(&self, stream: StreamSource) -> Result<()> {
        self.prover.register_hints_stream(stream)
    }

    pub fn register_inputs_stream(&self, stream: StreamSource) -> Result<()> {
        self.prover.register_inputs_stream(stream)
    }

    pub fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        self.prover.get_hints_processor()
    }

    pub fn set_active_services(&self, is_first_process: bool) -> Result<()> {
        self.prover.set_active_services(is_first_process)
    }

    pub fn reset(&self) -> Result<()> {
        self.prover.reset()
    }

    pub fn notify_cluster_cancellation(&self) {
        self.prover.notify_cluster_cancellation()
    }

    pub fn cluster_barrier(&self) {
        self.prover.cluster_barrier()
    }

    pub fn cancel(&self) -> Result<()> {
        self.prover.cancel()
    }

    pub fn wait_until_proofman_ready(&self) {
        self.prover.wait_until_proofman_ready()
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
    pub fn was_setup_with_hints(&self) -> bool {
        self.get_hints_processor().is_ok()
    }

    /// Execute via the Rust emulator path, regardless of how the program was set up.
    /// Lets callers run the AsmProver in emulator mode without spinning up ASM children.
    pub fn execute_emulator(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
    ) -> Result<ExecuteOutput> {
        self.prover.execute_emulator(program, stdin)
    }

    /// Generate a proof via the Rust emulator path, regardless of how the program was set up.
    /// Bypasses the `emulator_only` guard on the regular `prove()` method — this is the
    /// supported way to do emulator-mode proving with the ASM backend.
    pub fn prove_emulator(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        proof_kind: ProofKind,
    ) -> Result<ProveOutput> {
        self.prover.prove_emulator(program, stdin, proof_kind, self.prover_options.clone())
    }

    /// Verify constraints via the Rust emulator path, regardless of how the program was set up.
    pub fn verify_constraints_emulator(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<VerifyConstraintsOutput> {
        self.prover.verify_constraints_emulator(program, stdin, debug_info)
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
    proof_kind: ProofKind,
}

impl<'a, C: ZiskBackend> ProveBuilder<'a, C> {
    fn new(
        prover: &'a C::Prover,
        zisk_prover: &'a ZiskProver<C>,
        guest_program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> Self {
        Self { prover, zisk_prover, guest_program, stdin, proof_kind: ProofKind::VadcopFinal }
    }

    /// Enable minimal/compressed/SNARK proof generation.
    pub fn wrap_proof(mut self, proof_kind: ProofKind) -> Self {
        assert!(
            matches!(proof_kind, ProofKind::VadcopFinalMinimal | ProofKind::Plonk),
            "Invalid proof mode for ProveBuilder: {:?}",
            proof_kind
        );
        self.proof_kind = proof_kind;
        self
    }

    /// Execute the proof generation with the configured options.
    pub fn run(self) -> Result<ProveOutput> {
        self.prover.prove(
            self.guest_program,
            self.stdin,
            self.proof_kind,
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
/// let minimal = prover.wrap(&proof, ProofMode::VadcopFinalMinimal).run()?;
///
/// // Wrap to SNARK with custom publics
/// let snark = prover.wrap(&proof, ProofMode::Plonk)
///     .with_publics(&custom_publics)
///     .run()?;
/// ```
pub struct WrapBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    proof: &'a Proof,
    proof_kind: ProofKind,
    override_publics: Option<&'a PublicValues>,
    override_program_vk: Option<&'a ProgramVK>,
}

impl<'a, C: ZiskBackend> WrapBuilder<'a, C> {
    fn new(prover: &'a C::Prover, proof: &'a Proof, proof_kind: ProofKind) -> Self {
        Self { prover, proof, proof_kind, override_publics: None, override_program_vk: None }
    }

    /// Override the publics from the original proof.
    pub fn with_publics(mut self, publics: &'a PublicValues) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key from the original proof.
    pub fn with_program_vk(mut self, program_vk: &'a ProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Execute the proof wrapping with the configured options.
    pub fn run(self) -> Result<ProveOutput> {
        let derived_publics = self.proof.publics();
        let publics = self.override_publics.unwrap_or(&derived_publics);
        let program_vk = self.override_program_vk.unwrap_or(&self.proof.program_vk);
        let proof = match &self.proof.body {
            ProofBody::Vadcop { proof, .. } => proof.as_slice(),
            ProofBody::Plonk { .. } => {
                return Err(anyhow::anyhow!("Cannot wrap a Plonk proof"));
            }
        };
        self.prover.wrap_proof(proof, publics, program_vk, self.proof_kind)
    }
}
