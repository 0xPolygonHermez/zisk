mod asm;
mod backend;
mod emu;
use crate::guest::GuestProgram;
pub use asm::*;
use backend::*;
pub use emu::*;
use precompiles_hints::HintsProcessor;
use proofman::{
    AggProofs, AggProofsRegister, ProvePhase, ProvePhaseInputs, ProvePhaseResult, WitnessInfo,
};
use proofman_common::{ProofOptions, RowInfo};

use anyhow::Result;
use asm_runner::HintsShmem;
use executor::AsmResources;
use proofman::PlanningInfo;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use zisk_common::{
    io::StreamSource, io::ZiskStdin, ExecutorStatsHandle, ProofMode, StatsCostPerType,
    ZiskExecutorSummary, ZiskExecutorTime, ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues,
    ZiskPublics, ZiskVK, ZiskVerifyBuilder,
};
use zisk_core::ZiskRom;
use zisk_distributed_common::StreamMessage;

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

#[derive(Debug, Clone)]
pub struct ZiskProgramPK {
    zisk_rom: Arc<ZiskRom>,
    rom_bin_path: PathBuf,
    asm_resources: Option<AsmResources>,
}

impl ZiskProgramPK {
    pub fn new_emu(zisk_rom: Arc<ZiskRom>, rom_bin_path: PathBuf) -> Self {
        Self { zisk_rom, rom_bin_path, asm_resources: None }
    }

    pub fn new_asm(
        zisk_rom: Arc<ZiskRom>,
        rom_bin_path: PathBuf,
        asm_resources: AsmResources,
    ) -> Self {
        Self { zisk_rom, rom_bin_path, asm_resources: Some(asm_resources) }
    }

    pub fn get_rom_path(&self) -> &Path {
        &self.rom_bin_path
    }

    pub fn get_zisk_rom(&self) -> Arc<ZiskRom> {
        self.zisk_rom.clone()
    }

    pub fn set_active_services(&self, is_first_partition: bool) {
        self.asm_resources.as_ref().map(|r| r.set_active_services(is_first_partition));
    }

    pub fn register_hints_stream(&self, stream: StreamSource) -> Result<()> {
        if let Some(asm_resources) = &self.asm_resources {
            asm_resources
                .set_hints_stream_src(stream)
                .map_err(|e| anyhow::anyhow!("Failed to set hints stream source: {}", e))?;
        } else {
            return Err(anyhow::anyhow!(
                "ASM resources not initialized, cannot register hints stream"
            ));
        }
        Ok(())
    }

    pub fn submit_input(&self, bytes: &[u8]) -> Result<()> {
        if let Some(asm_resources) = &self.asm_resources {
            let message: StreamMessage = borsh::from_slice(&bytes[1..]).unwrap();
            let reinterpreted_data = unsafe {
                std::slice::from_raw_parts(
                    message.data.as_ptr() as *const u8,
                    message.data.len() * std::mem::size_of::<u64>(),
                )
            };
            asm_resources.inputs_shmem_writer.append_input(reinterpreted_data)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("ASM resources not initialized, cannot append input data"))
        }
    }

    pub fn submit_hint(&self, bytes: &[u8]) -> Result<()> {
        if let Some(asm_resources) = &self.asm_resources {
            let message: StreamMessage = borsh::from_slice(&bytes[1..]).unwrap();
            asm_resources
                .submit_hint_direct(&message.data)
                .map_err(|e| anyhow::anyhow!("Failed to submit hint data: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("ASM resources not initialized, cannot submit hint data"))
        }
    }
    pub fn get_hints_processor(&self) -> Option<Arc<HintsProcessor<HintsShmem>>> {
        self.asm_resources.as_ref().and_then(|r| r.get_hints_processor())
    }

    pub fn reset(&self) {
        if let Some(asm_resources) = &self.asm_resources {
            asm_resources.reset();
        }
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

    pub fn verify(&self) -> Result<()> {
        self.proof_with_publics.verify()
    }

    pub fn publics<'a>(&'a self, publics: &'a ZiskPublics) -> ZiskVerifyBuilder<'a> {
        self.proof_with_publics.publics(publics)
    }

    pub fn program_vk<'a>(&'a self, program_vk: &'a ZiskProgramVK) -> ZiskVerifyBuilder<'a> {
        self.proof_with_publics.program_vk(program_vk)
    }
}

pub type ZiskPhaseResult = ProvePhaseResult;

pub struct ZiskAggPhaseResult {
    pub agg_proofs: Vec<AggProofs>,
}

pub trait ProverEngine {
    fn setup(&self, elf: &GuestProgram) -> Result<(ZiskProgramPK, ZiskProgramVK)>;

    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()>;

    fn register_program(&self, pk: &ZiskProgramPK) -> Result<()>;

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
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult>;

    fn stats(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)>;

    fn verify_constraints(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult>;

    fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK>;

    fn prove(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult>;

    fn plonk(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues>;

    fn reduce(
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

    pub fn setup(&self, elf: &GuestProgram) -> Result<(ZiskProgramPK, ZiskProgramVK)> {
        self.prover.setup(elf)
    }

    /// Set the standard input for the current proof.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.prover.set_stdin(stdin)
    }

    pub fn register_program(&self, pk: &ZiskProgramPK) -> Result<()> {
        self.prover.register_program(pk)
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
    pub fn execute(&self, pk: &ZiskProgramPK, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        self.prover.execute(pk, stdin, None)
    }

    /// Get the execution statistics with the given standard input and debug information.
    pub fn stats(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        self.prover.stats(pk, stdin, debug_info, minimal_memory, mpi_node)
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
    pub fn verify_constraints(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.prover.verify_constraints(pk, stdin, debug_info)
    }

    pub fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK> {
        self.prover.vk(elf)
    }

    /// Generate a proof with the given standard input.
    /// Returns a `ProveBuilder` that allows setting per-proof options before running.
    ///
    /// # Example
    /// ```ignore
    /// let result = prover.prove(&pk, stdin).compressed().run()?;
    /// ```
    pub fn prove<'a>(&'a self, pk: &'a ZiskProgramPK, stdin: ZiskStdin) -> ProveBuilder<'a, C> {
        ProveBuilder::new(&self.prover, pk, stdin)
    }

    /// Generate a PLONK/SNARK proof from an existing proof.
    /// Returns a `PlonkBuilder` that allows overriding publics or program_vk.
    ///
    /// # Example
    /// ```ignore
    /// let snark = prover.plonk(&proof_with_publics).run()?;
    /// let snark = prover.plonk(&proof_with_publics).program_vk(&custom_vk).run()?;
    /// ```
    pub fn plonk<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> PlonkBuilder<'a, C> {
        PlonkBuilder::new(&self.prover, proof_with_publics)
    }

    /// Reduce/compress a proof to a smaller representation.
    /// Returns a `ReduceBuilder` that allows overriding publics or program_vk.
    ///
    /// # Example
    /// ```ignore
    /// let reduced = prover.reduce(&proof_with_publics).run()?;
    /// let reduced = prover.reduce(&proof_with_publics).publics(&custom_publics).run()?;
    /// ```
    pub fn reduce<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> ReduceBuilder<'a, C> {
        ReduceBuilder::new(&self.prover, proof_with_publics)
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
    pk: &'a ZiskProgramPK,
    stdin: ZiskStdin,
    mode: ProofMode,
    proof_options: ProofOpts,
}

impl<'a, C: ZiskBackend> ProveBuilder<'a, C> {
    fn new(prover: &'a C::Prover, pk: &'a ZiskProgramPK, stdin: ZiskStdin) -> Self {
        Self {
            prover,
            pk,
            stdin,
            mode: ProofMode::VadcopFinal,
            proof_options: ProofOpts::default(),
        }
    }

    /// Enable compressed proof generation.
    pub fn reduced(mut self) -> Self {
        self.mode = ProofMode::VadcopFinalReduced;
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
        self.prover.prove(self.pk, self.stdin, self.mode, self.proof_options)
    }
}

/// Builder for reducing/compressing a proof.
///
/// This builder allows optionally overriding the publics or program verification key
/// from the original proof before reducing it.
///
/// # Example
/// ```ignore
/// let reduced = prover.reduce(&proof_with_publics).run()?;
/// // Or override publics:
/// let reduced = prover.reduce(&proof_with_publics).publics(&custom_publics).run()?;
/// ```
pub struct ReduceBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    proof_with_publics: &'a ZiskProofWithPublicValues,
    override_publics: Option<&'a ZiskPublics>,
    override_program_vk: Option<&'a ZiskProgramVK>,
}

impl<'a, C: ZiskBackend> ReduceBuilder<'a, C> {
    fn new(prover: &'a C::Prover, proof_with_publics: &'a ZiskProofWithPublicValues) -> Self {
        Self { prover, proof_with_publics, override_publics: None, override_program_vk: None }
    }

    /// Override the publics from the original proof.
    pub fn publics(mut self, publics: &'a ZiskPublics) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key from the original proof.
    pub fn program_vk(mut self, program_vk: &'a ZiskProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Execute the proof reduction with the configured options.
    pub fn run(self) -> Result<ZiskProofWithPublicValues> {
        let publics = self.override_publics.unwrap_or(&self.proof_with_publics.publics);
        let program_vk = self.override_program_vk.unwrap_or(&self.proof_with_publics.program_vk);
        self.prover.reduce(&self.proof_with_publics.proof, publics, program_vk)
    }
}

/// Builder for generating a PLONK/SNARK proof from an existing proof.
///
/// This builder allows optionally overriding the publics or program verification key
/// from the original proof before generating the SNARK.
///
/// # Example
/// ```ignore
/// let snark = prover.plonk(&proof_with_publics).run()?;
/// // Or override verification key:
/// let snark = prover.plonk(&proof_with_publics).program_vk(&custom_vk).run()?;
/// ```
pub struct PlonkBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    proof_with_publics: &'a ZiskProofWithPublicValues,
    override_publics: Option<&'a ZiskPublics>,
    override_program_vk: Option<&'a ZiskProgramVK>,
}

impl<'a, C: ZiskBackend> PlonkBuilder<'a, C> {
    fn new(prover: &'a C::Prover, proof_with_publics: &'a ZiskProofWithPublicValues) -> Self {
        Self { prover, proof_with_publics, override_publics: None, override_program_vk: None }
    }

    /// Override the publics from the original proof.
    pub fn publics(mut self, publics: &'a ZiskPublics) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key from the original proof.
    pub fn program_vk(mut self, program_vk: &'a ZiskProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Execute the SNARK proof generation with the configured options.
    pub fn run(self) -> Result<ZiskProofWithPublicValues> {
        let publics = self.override_publics.unwrap_or(&self.proof_with_publics.publics);
        let program_vk = self.override_program_vk.unwrap_or(&self.proof_with_publics.program_vk);
        self.prover.plonk(&self.proof_with_publics.proof, publics, program_vk)
    }
}
