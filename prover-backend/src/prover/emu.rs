use crate::execute_client::ExecuteClient;
use crate::guest::ProgramId;
use crate::GuestProgram;
use crate::{
    check_paths_exist,
    prover::{ProverBackend, ProverEngine, ZiskBackend, ZiskProver},
    ExecuteOutput, ProveOutput, VerifyConstraintsOutput, ZiskAggPhaseResult, ZiskPhaseResult,
};
use crate::{ensure_program_vk, get_rom_bin_path, BackendProverOpts};
use asm_runner::HintsShmem;
use executor::ZiskExecutor;
use precompiles_hints::HintsProcessor;
use proofman::{
    AggProofs, AggProofsRegister, ProofMan, ProvePhase, ProvePhaseInputs, SnarkWrapper, WitnessInfo,
};
use proofman_common::{initialize_logger, ProofOptions, ProofmanOptions, RankInfo, RowInfo};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use zisk_cluster_common::LoggingConfig;
use zisk_common::io::StreamSource;
use zisk_common::{
    io::ZiskStdin, ExecutorStatsHandle, ProgramVK, ProofKind, PublicValues, ZiskExecutorTime,
};
use zisk_core::{Riscv2zisk, ZiskRom};

use anyhow::Result;

pub struct Emu;

impl ZiskBackend for Emu {
    type Prover = EmuProver;
}

/// Builder for EMU backend setup (hints not supported).
pub struct EmuSetupBuilder<'a> {
    prover: &'a EmuProver,
    elf: &'a GuestProgram,
}

impl<'a> EmuSetupBuilder<'a> {
    fn new(prover: &'a EmuProver, elf: &'a GuestProgram) -> Self {
        Self { prover, elf }
    }

    /// Execute the setup and return the program proving and verification keys.
    pub fn run(self) -> Result<ProgramVK> {
        self.prover.setup_internal(self.elf, false, false)
    }
}

pub struct EmuProver {
    pub(crate) core_prover: EmuCoreProver,
    program_cache: Arc<RwLock<HashMap<ProgramId, Arc<ZiskRom>>>>,
}

impl EmuProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        snark_wrapper: bool,
        preload_snark: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
        shared_tables: bool,
        options: ProofmanOptions,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let core_prover = EmuCoreProver::new(
            snark_wrapper,
            preload_snark,
            proving_key,
            proving_key_snark,
            shared_tables,
            options,
            logging_config,
        )?;

        let program_cache = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self { core_prover, program_cache })
    }
}

impl ProverEngine for EmuProver {
    type Builder<'a> = EmuSetupBuilder<'a>;

    fn setup<'a>(&'a self, elf: &'a GuestProgram) -> Self::Builder<'a> {
        EmuSetupBuilder::new(self, elf)
    }

    fn setup_internal(
        &self,
        elf: &GuestProgram,
        _with_hints: bool,
        _emulator_only: bool,
    ) -> Result<ProgramVK> {
        let pctx = self.core_prover.backend.get_pctx()?;

        let program_vk = ensure_program_vk(&pctx, elf)?;

        let rv2zk = Riscv2zisk::new(elf.elf());

        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
        let zisk_rom = Arc::new(zisk_rom);

        self.program_cache.write().unwrap().insert(elf.program_id.clone(), zisk_rom);
        Ok(program_vk)
    }

    fn world_rank(&self) -> i32 {
        self.core_prover.rank_info.world_rank
    }

    fn local_rank(&self) -> i32 {
        self.core_prover.rank_info.local_rank
    }

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.core_prover.backend.set_stdin(stdin)
    }

    fn register_program(&self, program_id: &ProgramId, _with_hints: bool) -> Result<()> {
        let rom = self
            .program_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(program_id).cloned())
            .ok_or_else(|| {
            anyhow::anyhow!("Program '{}' not found in cache. Call setup() first.", program_id.name)
        })?;
        let pctx = self.core_prover.backend.get_pctx()?;
        let rom_bin_path = get_rom_bin_path(&pctx, program_id)?;
        self.core_prover.backend.register_program(rom, &rom_bin_path, false)
    }

    fn executed_steps(&self) -> u64 {
        self.core_prover
            .backend
            .execution_result()
            .map(|(exec_result, _)| exec_result.steps)
            .unwrap_or(0)
    }

    fn get_execution_info(&self) -> Result<(WitnessInfo, ZiskExecutorTime)> {
        self.core_prover.backend.get_execution_info()
    }

    fn execute(&self, program: &GuestProgram, stdin: ZiskStdin) -> Result<ExecuteOutput> {
        self.register_program(&program.program_id, false)?;
        self.core_prover.backend.execute(stdin)
    }

    fn stats(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        self.register_program(&program.program_id, false)?;
        self.core_prover.backend.stats(stdin, debug_info, minimal_memory, mpi_node)
    }

    fn get_instance_trace(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        self.core_prover.backend.get_instance_trace(instance_id, first_row, num_rows, offset)
    }

    fn get_instance_air_values(&self, instance_id: usize) -> Result<Vec<u64>> {
        self.core_prover.backend.get_instance_air_values(instance_id)
    }

    fn get_instance_fixed(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        self.core_prover.backend.get_instance_fixed(instance_id, first_row, num_rows, offset)
    }

    fn verify_constraints(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<VerifyConstraintsOutput> {
        self.register_program(&program.program_id, false)?;
        self.core_prover.backend.verify_constraints(stdin, debug_info)
    }

    fn prove(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        proof_kind: ProofKind,
        prover_options: BackendProverOpts,
    ) -> Result<ProveOutput> {
        self.register_program(&program.program_id, false)?;
        self.core_prover.backend.prove(stdin, proof_kind, prover_options)
    }

    fn wrap_proof(
        &self,
        proof: &[u64],
        publics: &PublicValues,
        vk: &ProgramVK,
        proof_kind: ProofKind,
    ) -> Result<ProveOutput> {
        match proof_kind {
            ProofKind::VadcopFinalMinimal => self.core_prover.backend.minimal(proof, publics, vk),
            ProofKind::Plonk => self.core_prover.backend.plonk(proof, publics, vk),
            _ => Err(anyhow::anyhow!("Unsupported proof mode for wrap: {:?}", proof_kind)),
        }
    }

    fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        self.core_prover.backend.prove_phase(phase_inputs, options, phase)
    }

    fn set_partition(
        &self,
        total_compute_units: usize,
        allocation: Vec<u32>,
        rank_id: usize,
    ) -> Result<()> {
        self.core_prover.backend.set_partition(total_compute_units, allocation, rank_id)
    }

    fn register_aggregated_proofs(&self, agg_proofs: Vec<AggProofsRegister>) -> Result<()> {
        self.core_prover.backend.register_aggregated_proofs(agg_proofs)
    }

    fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        self.core_prover.backend.aggregate_proofs(agg_proofs, last_proof, final_proof, options)
    }

    fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        self.core_prover.backend.mpi_broadcast(data)
    }

    fn notify_cluster_cancellation(&self) {
        self.core_prover.backend.notify_cluster_cancellation();
    }

    fn cluster_barrier(&self) {
        self.core_prover.backend.cluster_barrier();
    }

    fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u64>> {
        self.core_prover.backend.get_vadcop_vk(minimal)
    }

    fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        Err(anyhow::anyhow!("EmuProver does not support hints"))
    }

    fn cancel(&self) -> Result<()> {
        self.core_prover.backend.cancel();
        Ok(())
    }

    fn wait_until_proofman_ready(&self) {
        self.core_prover.backend.wait_until_proofman_ready();
    }
}

pub struct EmuCoreProver {
    backend: ProverBackend,
    rank_info: RankInfo,
}

impl EmuCoreProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        use_snark_wrapper: bool,
        preload_snark: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
        shared_tables: bool,
        options: ProofmanOptions,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        check_paths_exist(&proving_key)?;

        let proofman = ProofMan::new(proving_key.clone(), options.clone())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let rank_info = proofman.get_rank_info();

        if logging_config.is_some() {
            zisk_cluster_common::init(logging_config.as_ref(), Some(&rank_info))?;
        } else {
            initialize_logger(options.verbose_mode, Some(&rank_info));
        }

        proofman.set_barrier();

        let mut snark_wrapper = None;
        if use_snark_wrapper {
            check_paths_exist(&proving_key_snark)?;
            let (aux_trace, d_buffers, reload_fixed_pols_gpu) = proofman.get_preallocated_buffers();
            snark_wrapper = Some(SnarkWrapper::new_with_preallocated_buffers(
                &proving_key_snark,
                options.verbose_mode,
                Some(aux_trace),
                Some(d_buffers),
                Some(reload_fixed_pols_gpu),
                preload_snark,
                options.gpu,
            )?);
        }

        let executor = ZiskExecutor::new(
            &proofman.get_wcm(),
            options.verbose_mode,
            shared_tables,
            false,
            options.packed,
        )?;

        let core = ProverBackend::new(
            proofman,
            snark_wrapper,
            executor,
            proving_key,
            Some(proving_key_snark),
        );

        Ok(Self { backend: core, rank_info })
    }
}

impl ExecuteClient for ZiskProver<Emu> {
    fn setup(&self, program: &GuestProgram, _with_hints: bool) -> Result<()> {
        ZiskProver::<Emu>::setup(self, program).run().map(|_| ())
    }

    fn execute(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        _hints: Option<StreamSource>,
    ) -> Result<ExecuteOutput> {
        ZiskProver::<Emu>::execute(self, program, stdin)
    }
}
