use crate::{
    check_paths_exist, create_debug_info, get_custom_commits_map,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    Proof, RankInfo, ZiskLibLoader,
};
use proofman::ProofMan;
use proofman_common::{initialize_logger, ParamsGPU};
use std::{path::PathBuf, time::Duration};
use zisk_common::{ExecutorStats, ZiskExecutionResult};

use anyhow::Result;

pub struct Emu;

impl ZiskBackend for Emu {
    type Prover = EmuProver;
}

pub struct EmuProver {
    pub(crate) core_prover: EmuCoreProver,
}

impl EmuProver {
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
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let core_prover = EmuCoreProver::new(
            verify_constraints,
            aggregation,
            final_snark,
            witness_lib,
            proving_key,
            elf,
            verbose,
            shared_tables,
            gpu_params,
            verify_proofs,
            minimal_memory,
            save_proofs,
            output_dir,
        )?;

        Ok(Self { core_prover })
    }
}

impl ProverEngine for EmuProver {
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

pub struct EmuCoreProver {
    backend: ProverBackend,
    _rank_info: RankInfo,
}

impl EmuCoreProver {
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
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let custom_commits_map = get_custom_commits_map(&proving_key, &elf)?;

        check_paths_exist(&witness_lib)?;
        check_paths_exist(&proving_key)?;
        check_paths_exist(&elf)?;

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

        // Build emulator library
        let (library, mut witness_lib) = ZiskLibLoader::load_emu(
            witness_lib,
            elf,
            world_rank,
            local_rank,
            verbose.into(),
            shared_tables,
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

        Ok(Self { backend: core, _rank_info: RankInfo { world_rank, local_rank } })
    }
}
