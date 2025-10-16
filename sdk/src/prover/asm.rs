use crate::{
    check_paths_exist, create_debug_info, ensure_custom_commits,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    Proof, RankInfo, ZiskLibLoader,
};
use asm_runner::{AsmRunnerOptions, AsmServices};
use proofman::ProofMan;
use proofman_common::{initialize_logger, ParamsGPU};
use rom_setup::DEFAULT_CACHE_PATH;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tracing::info;
use zisk_common::{ExecutorStats, ZiskExecutionResult};

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
        final_snark: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        asm_mt_filename: String,
        asm_rh_filename: String,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let core_prover = AsmCoreProver::new(
            verify_constraints,
            aggregation,
            final_snark,
            witness_lib,
            proving_key,
            elf,
            verbose,
            shared_tables,
            asm_mt_filename,
            asm_rh_filename,
            base_port,
            unlock_mapped_memory,
            gpu_params,
            verify_proofs,
            minimal_memory,
            save_proofs,
            output_dir,
        )?;

        Ok(Self { core_prover })
    }
}

impl ProverEngine for AsmProver {
    fn debug_verify_constraints(
        &self,
        input: Option<PathBuf>,
        debug_info: Option<Option<String>>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        let debug_info =
            create_debug_info(debug_info, self.core_prover.backend.proving_key.clone());

        self.core_prover.backend.debug_verify_constraints(input, debug_info)
    }

    fn verify_constraints(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.core_prover.backend.verify_constraints(input)
    }

    fn prove(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)> {
        self.core_prover.backend.prove(input)
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
        final_snark: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        asm_mt_filename: String,
        asm_rh_filename: String,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
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

        let proofman = ProofMan::new(
            proving_key.clone(),
            custom_commits_map,
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
            verbose.into(),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mpi_ctx = proofman.get_mpi_ctx();

        let world_rank = mpi_ctx.rank;
        let local_rank = mpi_ctx.node_rank;

        initialize_logger(verbose.into(), Some(world_rank));

        info!(">>> [{}] Starting ASM microservices.", world_rank);

        let asm_services = AsmServices::new(world_rank, local_rank, base_port);

        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(verbose > 0)
            .with_base_port(base_port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_unlock_mapped_memory(unlock_mapped_memory);

        asm_services.start_asm_services(&asm_mt_path, asm_runner_options)?;

        let (library, mut witness_lib) = ZiskLibLoader::load_asm(
            witness_lib,
            elf,
            world_rank,
            local_rank,
            verbose.into(),
            shared_tables,
            asm_mt_path,
            asm_rh_path,
            base_port,
            unlock_mapped_memory,
        )?;

        proofman.register_witness(&mut *witness_lib, library);

        let core = ProverBackend {
            verify_constraints,
            aggregation,
            final_snark,
            witness_lib,
            proving_key: proving_key.clone(),
            verify_proofs,
            minimal_memory,
            save_proofs,
            output_dir,
            proofman,
        };

        Ok(Self { backend: core, asm_services, rank_info: RankInfo { world_rank, local_rank } })
    }
}
