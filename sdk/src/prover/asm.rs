use crate::{
    check_paths_exist, create_debug_info, ensure_custom_commits,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    RankInfo, ZiskAggPhaseResult, ZiskExecuteResult, ZiskLibLoader, ZiskPhaseResult,
    ZiskProveResult, ZiskVerifyConstraintsResult,
};
use asm_runner::{AsmRunnerOptions, AsmServices};
use proofman::{AggProofs, ProofMan, ProvePhase, ProvePhaseInputs};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rom_setup::DEFAULT_CACHE_PATH;
use std::{collections::HashMap, path::PathBuf};
use tracing::info;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::ExecutorStatsHandle;
use zisk_distributed_common::LoggingConfig;

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
        rma: bool,
        compressed: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        proving_key_snark: Option<PathBuf>,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        asm_mt_filename: String,
        asm_rh_filename: String,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        with_hints: bool,
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let core_prover = AsmCoreProver::new(
            verify_constraints,
            aggregation,
            rma,
            compressed,
            witness_lib,
            proving_key,
            proving_key_snark,
            elf,
            verbose,
            shared_tables,
            asm_mt_filename,
            asm_rh_filename,
            base_port,
            unlock_mapped_memory,
            with_hints,
            gpu_params,
            verify_proofs,
            minimal_memory,
            save_proofs,
            output_dir,
            logging_config,
        )?;

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

    fn set_stdin(&self, stdin: ZiskStdin) {
        self.core_prover.backend.witness_lib.set_stdin(stdin);
    }

    fn set_hints_stream(&self, hints_stream: StreamSource) -> anyhow::Result<()> {
        self.core_prover.backend.witness_lib.set_hints_stream(hints_stream)
    }

    fn executed_steps(&self) -> u64 {
        self.core_prover
            .backend
            .witness_lib
            .execution_result()
            .map(|(exec_result, _)| exec_result.steps)
            .unwrap_or(0)
    }

    fn execute(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        self.core_prover.backend.execute(stdin, hints_stream, output_path)
    }

    fn stats(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        debug_info: Option<Option<String>>,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        let debug_info =
            create_debug_info(debug_info, self.core_prover.backend.proving_key.clone())?;

        self.core_prover.backend.stats(stdin, hints_stream, debug_info, mpi_node)
    }

    fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let debug_info =
            create_debug_info(debug_info, self.core_prover.backend.proving_key.clone())?;

        self.core_prover.backend.verify_constraints_debug(stdin, hints_stream, debug_info)
    }

    fn verify_constraints(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.core_prover.backend.verify_constraints(stdin, hints_stream)
    }

    fn prove(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskProveResult> {
        self.core_prover.backend.prove(stdin, hints_stream)
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

    fn mpi_broadcast(&self, data: &mut Vec<u8>) {
        self.core_prover.backend.mpi_broadcast(data);
    }
}

pub struct AsmCoreProver {
    backend: ProverBackend,
    asm_services: AsmServices,
    rank_info: RankInfo,
}

impl Drop for AsmCoreProver {
    fn drop(&mut self) {
        // Shut down ASM microservices
        info!(">>> [{}] Stopping ASM microservices.", self.rank_info.world_rank);
        if let Err(e) = self.asm_services.stop_asm_services() {
            tracing::error!(
                ">>> [{}] Failed to stop ASM microservices: {}",
                self.rank_info.world_rank,
                e
            );
        }
    }
}

impl AsmCoreProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        verify_constraints: bool,
        aggregation: bool,
        rma: bool,
        compressed: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        _proving_key_snark: Option<PathBuf>,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        asm_mt_filename: String,
        asm_rh_filename: String,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        with_hints: bool,
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let rom_bin_path = ensure_custom_commits(&proving_key, &elf)?;
        let custom_commits_map = HashMap::from([("rom".to_string(), rom_bin_path)]);

        let default_cache_path = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| anyhow::anyhow!("Failed to read HOME environment variable: {e}"))?
            .join(DEFAULT_CACHE_PATH);

        let asm_mt_path = default_cache_path.join(asm_mt_filename);
        let asm_rh_path = default_cache_path.join(asm_rh_filename);

        check_paths_exist(&witness_lib)?;
        check_paths_exist(&proving_key)?;
        check_paths_exist(&elf)?;
        check_paths_exist(&asm_mt_path)?;
        check_paths_exist(&asm_rh_path)?;

        let (library, mut witness_lib) = ZiskLibLoader::load_asm(
            witness_lib,
            elf,
            verbose.into(),
            shared_tables,
            asm_mt_path.clone(),
            asm_rh_path,
            base_port,
            unlock_mapped_memory,
            with_hints,
        )?;

        let proofman = ProofMan::new(
            proving_key.clone(),
            custom_commits_map,
            verify_constraints,
            aggregation,
            gpu_params,
            verbose.into(),
            witness_lib.get_packed_info(),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let world_rank = proofman.get_world_rank();
        let local_rank = proofman.get_local_rank();

        if logging_config.is_some() {
            zisk_distributed_common::init(logging_config.as_ref(), Some(world_rank))?;
        } else {
            initialize_logger(verbose.into(), Some(world_rank));
        }

        timer_start_info!(STARTING_ASM_MICROSERVICES);
        let asm_services = AsmServices::new(world_rank, local_rank, base_port);

        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(verbose > 0)
            .with_base_port(base_port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_unlock_mapped_memory(unlock_mapped_memory);

        asm_services.start_asm_services(&asm_mt_path, asm_runner_options)?;
        timer_stop_and_log_info!(STARTING_ASM_MICROSERVICES);

        proofman.register_witness(&mut *witness_lib, library)?;

        proofman.set_barrier();

        let core = ProverBackend {
            verify_constraints,
            aggregation,
            rma,
            compressed,
            witness_lib,
            proving_key: proving_key.clone(),
            verify_proofs,
            minimal_memory,
            save_proofs,
            output_dir,
            proofman,
            rank_info: RankInfo { world_rank, local_rank },
        };

        Ok(Self { backend: core, asm_services, rank_info: RankInfo { world_rank, local_rank } })
    }
}
