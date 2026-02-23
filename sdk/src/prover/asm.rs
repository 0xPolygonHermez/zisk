use crate::get_asm_paths;
use crate::{
    check_paths_exist, ensure_custom_commits,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProgramPK, ZiskProgramVK,
    ZiskProof, ZiskProofWithPublicValues, ZiskProveResult, ZiskPublics,
    ZiskVerifyConstraintsResult,
};
use crate::{ProofMode, ProofOpts};
use asm_runner::{AsmRunnerOptions, AsmServices};
use executor::{get_packed_info, init_executor_asm, AsmResources};
use proofman::{AggProofs, ExecutionInfo, ProofMan, ProvePhase, ProvePhaseInputs, SnarkWrapper};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions, RankInfo, RowInfo, VerboseMode};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rom_setup::{generate_assembly, get_output_path, DEFAULT_CACHE_PATH};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use zisk_common::io::ZiskStdin;
use zisk_common::ElfBinaryLike;
use zisk_common::ExecutorStatsHandle;
use zisk_core::Riscv2zisk;
use zisk_distributed_common::LoggingConfig;

use anyhow::Result;

pub struct Asm;

impl ZiskBackend for Asm {
    type Prover = AsmProver;
}

pub struct AsmProver {
    pub(crate) core_prover: AsmCoreProver,
    pub n_setups: AtomicU64,
}

impl AsmProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        verify_constraints: bool,
        aggregation: bool,
        snark_wrapper: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
        verbose: u8,
        shared_tables: bool,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        no_auto_setup: bool,
        gpu_params: ParamsGPU,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let core_prover = AsmCoreProver::new(
            verify_constraints,
            aggregation,
            snark_wrapper,
            proving_key,
            proving_key_snark,
            verbose,
            shared_tables,
            base_port,
            unlock_mapped_memory,
            no_auto_setup,
            gpu_params,
            logging_config,
        )?;

        Ok(Self { core_prover, n_setups: AtomicU64::new(0) })
    }

    pub fn new_verifier(proving_key: PathBuf, proving_key_snark: PathBuf) -> Result<Self> {
        let core_prover = AsmCoreProver::new_verifier(proving_key, proving_key_snark)?;

        Ok(Self { core_prover, n_setups: AtomicU64::new(0) })
    }
}

impl ProverEngine for AsmProver {
    fn world_rank(&self) -> i32 {
        self.core_prover.rank_info.world_rank
    }

    fn local_rank(&self) -> i32 {
        self.core_prover.rank_info.local_rank
    }

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.core_prover.backend.set_stdin(stdin)
    }

    fn register_program(&self, pk: &ZiskProgramPK) -> Result<()> {
        self.core_prover.backend.register_program(pk)
    }

    fn executed_steps(&self) -> u64 {
        self.core_prover
            .backend
            .execution_result()
            .map(|(exec_result, _)| exec_result.steps)
            .unwrap_or(0)
    }

    fn setup(&self, elf: &impl ElfBinaryLike) -> Result<(ZiskProgramPK, ZiskProgramVK)> {
        let pctx = self.core_prover.backend.get_pctx()?;
        let (rom_bin_path, vk) = ensure_custom_commits(&pctx, elf)?;

        let world_rank = self.core_prover.rank_info.world_rank;
        let local_rank = self.core_prover.rank_info.local_rank;
        let n_processes = self.core_prover.rank_info.n_processes;
        let unlock_mapped_memory = self.core_prover.unlock_mapped_memory;
        let verbose_mode = self.core_prover.verbose;
        let rank_info = self.core_prover.rank_info.clone();
        let base_port = Some(AsmServices::port_base_offset(
            self.core_prover.base_port,
            n_processes,
            self.n_setups.load(Ordering::SeqCst),
        ));

        let rv2zk = Riscv2zisk::new(elf.elf());

        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
        let zisk_rom = Arc::new(zisk_rom);

        let default_cache_path = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| anyhow::anyhow!("Failed to read HOME environment variable: {e}"))?
            .join(DEFAULT_CACHE_PATH);

        let (asm_mt_filename, asm_rh_filename) = get_asm_paths(elf)?;

        let asm_mt_path = default_cache_path.join(asm_mt_filename);
        let asm_rh_path = default_cache_path.join(asm_rh_filename);

        if check_paths_exist(&asm_mt_path).is_err() || check_paths_exist(&asm_rh_path).is_err() {
            if self.core_prover.no_auto_setup {
                return Err(anyhow::anyhow!(
                    "Assembly files not found for ELF {}. Force ROM setup is enabled, but assembly files are still missing. Please ensure that the assembly generation process has been completed successfully.",
                    elf.name()
                ));
            }

            tracing::info!(
                ">>> ROM SETUP (one time only) - Generating assembly files for ELF: {}",
                elf.name()
            );
            timer_start_info!(ROM_SETUP);
            let output_path = get_output_path(&None)?;
            generate_assembly(
                elf.elf(),
                elf.name(),
                &output_path,
                elf.with_hints(),
                self.core_prover.verbose != VerboseMode::Info,
            )?;
            timer_stop_and_log_info!(ROM_SETUP);
            tracing::info!("<<< ROM SETUP complete - Assembly files cached for future use");
        }

        timer_start_info!(STARTING_ASM_MICROSERVICES);
        let asm_services = AsmServices::new(world_rank, local_rank, base_port);

        let asm_runner_options = AsmRunnerOptions::new()
            .with_base_port(base_port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_verbose(verbose_mode == VerboseMode::Debug)
            .with_metrics(verbose_mode == VerboseMode::Debug)
            .with_unlock_mapped_memory(unlock_mapped_memory);

        asm_services.start_asm_services(&asm_mt_path, asm_runner_options)?;
        timer_stop_and_log_info!(STARTING_ASM_MICROSERVICES);

        let asm_resources = AsmResources::new(
            local_rank,
            base_port,
            unlock_mapped_memory,
            verbose_mode,
            elf.with_hints(),
        );

        self.n_setups.fetch_add(1, Ordering::SeqCst);

        Ok((
            ZiskProgramPK {
                zisk_rom,
                elf_bin_path: rom_bin_path,
                asm_services: Some(asm_services),
                asm_resources: Some(asm_resources),
                rank_info,
                use_hints: elf.with_hints(),
            },
            ZiskProgramVK { vk },
        ))
    }

    fn get_execution_info(&self) -> Result<ExecutionInfo> {
        self.core_prover.backend.get_execution_info()
    }

    fn execute(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        self.core_prover.backend.execute(pk, stdin, output_path)
    }

    fn stats(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        self.core_prover.backend.stats(pk, stdin, debug_info, minimal_memory, mpi_node)
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

    fn verify_constraints_debug(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.core_prover.backend.verify_constraints_debug(pk, stdin, debug_info)
    }

    fn verify_constraints(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.core_prover.backend.verify_constraints(pk, stdin)
    }

    fn vk(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        self.core_prover.backend.vk(elf)
    }

    fn verify(&self, proof: &ZiskProof, publics: &ZiskPublics, vk: &ZiskProgramVK) -> Result<()> {
        self.core_prover.backend.verify(proof, publics, vk)
    }

    fn prove(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        self.core_prover.backend.prove(pk, stdin, mode, proof_options)
    }

    fn prove_snark(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        self.core_prover.backend.prove_snark(proof, publics, vk)
    }

    fn compress(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        self.core_prover.backend.compress(proof, publics, vk)
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

    fn is_first_partition(&self) -> Result<bool> {
        self.core_prover.backend.is_first_partition()
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
}

pub struct AsmCoreProver {
    backend: ProverBackend,
    rank_info: RankInfo,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    verbose: VerboseMode,
    no_auto_setup: bool,
}

impl AsmCoreProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        verify_constraints: bool,
        aggregation: bool,
        use_snark_wrapper: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
        verbose: u8,
        shared_tables: bool,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        no_auto_setup: bool,
        gpu_params: ParamsGPU,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        check_paths_exist(&proving_key)?;
        let proofman = ProofMan::new(
            proving_key.clone(),
            verify_constraints,
            aggregation,
            gpu_params,
            verbose.into(),
            get_packed_info(),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let rank_info = proofman.get_rank_info();

        if logging_config.is_some() {
            zisk_distributed_common::init(logging_config.as_ref(), Some(&rank_info))?;
        } else {
            initialize_logger(verbose.into(), Some(&rank_info));
        }

        proofman.set_barrier();

        let mut snark_wrapper = None;
        if use_snark_wrapper {
            check_paths_exist(&proving_key_snark)?;
            let (aux_trace, d_buffers, reload_fixed_pols_gpu) = proofman.get_preallocated_buffers();
            snark_wrapper = Some(SnarkWrapper::new_with_preallocated_buffers(
                &proving_key_snark,
                verbose.into(),
                Some(aux_trace),
                Some(d_buffers),
                Some(reload_fixed_pols_gpu),
            )?);
        }

        let executor = init_executor_asm(
            verbose.into(),
            shared_tables,
            unlock_mapped_memory,
            &proofman.get_wcm(),
        )?;

        let core = ProverBackend::new(
            proofman,
            snark_wrapper,
            executor,
            proving_key,
            Some(proving_key_snark),
        );

        Ok(Self {
            backend: core,
            rank_info,
            base_port,
            unlock_mapped_memory,
            verbose: verbose.into(),
            no_auto_setup,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_verifier(proving_key: PathBuf, proving_key_snark: PathBuf) -> Result<Self> {
        let core_prover = ProverBackend::new_verifier(proving_key, Some(proving_key_snark));

        Ok(Self {
            backend: core_prover,
            rank_info: RankInfo { world_rank: 0, local_rank: 0, n_processes: 1 },
            base_port: None,
            unlock_mapped_memory: false,
            verbose: VerboseMode::Info,
            no_auto_setup: false,
        })
    }
}
