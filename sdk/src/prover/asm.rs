use crate::get_asm_paths;
use crate::{
    check_paths_exist, ensure_custom_commits,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    ZiskAggPhaseResult, ZiskExecuteResult, ZiskLibLoader, ZiskPhaseResult, ZiskProgramVK,
    ZiskProof, ZiskProofWithPublicValues, ZiskProveResult, ZiskPublics,
    ZiskVerifyConstraintsResult,
};
use crate::{ProofMode, ProofOpts};
use asm_runner::{AsmRunnerOptions, AsmServices};
use proofman::{AggProofs, ExecutionInfo, ProofMan, ProvePhase, ProvePhaseInputs, SnarkWrapper};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions, RankInfo, RowInfo, VerboseMode};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rom_setup::DEFAULT_CACHE_PATH;
use std::sync::OnceLock;
use std::{collections::HashMap, path::PathBuf};
use tracing::info;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::ElfBinaryLike;
use zisk_common::ExecutorStatsHandle;
use zisk_distributed_common::LoggingConfig;
use zisk_witness::get_packed_info;

use anyhow::Result;

pub struct Asm;

impl ZiskBackend for Asm {
    type Prover = AsmProver;
}

pub struct AsmProver {
    pub(crate) core_prover: AsmCoreProver,
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
            gpu_params,
            logging_config,
        )?;

        Ok(Self { core_prover })
    }

    pub fn new_verifier(proving_key: PathBuf, proving_key_snark: PathBuf) -> Result<Self> {
        let core_prover = AsmCoreProver::new_verifier(proving_key, proving_key_snark)?;

        Ok(Self { core_prover })
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

    fn set_hints_stream(&self, hints_stream: StreamSource) -> Result<()> {
        self.core_prover.backend.set_hints_stream(hints_stream)
    }

    fn executed_steps(&self) -> u64 {
        self.core_prover
            .backend
            .execution_result()
            .map(|(exec_result, _)| exec_result.steps)
            .unwrap_or(0)
    }

    fn setup(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        let pctx = self.core_prover.backend.get_pctx()?;
        let (rom_bin_path, vk) = ensure_custom_commits(&pctx, elf)?;
        let custom_commits_map = HashMap::from([("rom".to_string(), rom_bin_path)]);

        let default_cache_path = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| anyhow::anyhow!("Failed to read HOME environment variable: {e}"))?
            .join(DEFAULT_CACHE_PATH);

        let (asm_mt_filename, asm_rh_filename) = get_asm_paths(elf)?;

        let asm_mt_path = default_cache_path.join(asm_mt_filename);
        let asm_rh_path = default_cache_path.join(asm_rh_filename);

        check_paths_exist(&asm_mt_path)?;
        check_paths_exist(&asm_rh_path)?;

        timer_start_info!(STARTING_ASM_MICROSERVICES);
        let world_rank = self.core_prover.rank_info.world_rank;
        let local_rank = self.core_prover.rank_info.local_rank;
        let asm_services = AsmServices::new(world_rank, local_rank, self.core_prover.base_port);

        let asm_runner_options = AsmRunnerOptions::new()
            .with_base_port(self.core_prover.base_port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_verbose(self.core_prover.verbose == VerboseMode::Debug)
            .with_metrics(self.core_prover.verbose == VerboseMode::Debug)
            .with_unlock_mapped_memory(self.core_prover.unlock_mapped_memory);

        asm_services.start_asm_services(&asm_mt_path, asm_runner_options)?;
        timer_stop_and_log_info!(STARTING_ASM_MICROSERVICES);

        let witness_lib = ZiskLibLoader::load_asm(
            self.core_prover.verbose,
            self.core_prover.shared_tables,
            asm_mt_path.clone(),
            asm_rh_path,
            self.core_prover.base_port,
            self.core_prover.unlock_mapped_memory,
            elf.with_hints(),
        )?;

        self.core_prover
            .asm_services
            .set(asm_services)
            .map_err(|_| anyhow::anyhow!("ASM services have already been initialized."))?;

        self.core_prover.backend.register_witness_lib(
            elf.elf(),
            witness_lib,
            custom_commits_map,
        )?;
        Ok(ZiskProgramVK { vk })
    }

    fn get_execution_info(&self) -> Result<ExecutionInfo> {
        self.core_prover.backend.get_execution_info()
    }

    fn execute(&self, stdin: ZiskStdin, output_path: Option<PathBuf>) -> Result<ZiskExecuteResult> {
        self.core_prover.backend.execute(stdin, output_path)
    }

    fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
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

    fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.core_prover.backend.verify_constraints_debug(stdin, debug_info)
    }

    fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult> {
        self.core_prover.backend.verify_constraints(stdin)
    }

    fn vk(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        self.core_prover.backend.vk(elf)
    }

    fn verify(&self, proof: &ZiskProof, publics: &ZiskPublics, vk: &ZiskProgramVK) -> Result<()> {
        self.core_prover.backend.verify(proof, publics, vk)
    }

    fn prove_debug(&self, stdin: ZiskStdin, proof_options: ProofOpts) -> Result<ZiskProveResult> {
        self.core_prover.backend.prove_debug(stdin, proof_options)
    }

    fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        self.core_prover.backend.prove(stdin, mode, proof_options)
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
    asm_services: OnceLock<AsmServices>,
    rank_info: RankInfo,
    verbose: VerboseMode,
    shared_tables: bool,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
}

impl Drop for AsmCoreProver {
    fn drop(&mut self) {
        // Shut down ASM microservices
        info!(">>> [{}] Stopping ASM microservices.", self.rank_info.world_rank);
        if let Some(asm_services) = &self.asm_services.get() {
            if let Err(e) = asm_services.stop_asm_services() {
                tracing::error!(
                    ">>> [{}] Failed to stop ASM microservices: {}",
                    self.rank_info.world_rank,
                    e
                );
            }
        }
    }
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
            snark_wrapper = Some(SnarkWrapper::new(&proving_key_snark, verbose.into())?);
        }

        let core =
            ProverBackend::new(proofman, snark_wrapper, proving_key, Some(proving_key_snark));

        Ok(Self {
            backend: core,
            asm_services: OnceLock::new(),
            rank_info,
            verbose: verbose.into(),
            shared_tables,
            base_port,
            unlock_mapped_memory,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_verifier(proving_key: PathBuf, proving_key_snark: PathBuf) -> Result<Self> {
        let core_prover = ProverBackend::new_verifier(proving_key, Some(proving_key_snark));

        Ok(Self {
            backend: core_prover,
            asm_services: OnceLock::new(),
            rank_info: RankInfo { world_rank: 0, local_rank: 0, n_processes: 1 },
            verbose: VerboseMode::Info,
            shared_tables: false,
            base_port: None,
            unlock_mapped_memory: false,
        })
    }
}
