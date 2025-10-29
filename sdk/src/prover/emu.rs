use crate::{
    check_paths_exist, create_debug_info, get_custom_commits_map,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    Proof, RankInfo, ZiskLibLoader,
};
use proofman::{AggProofs, ProofMan, ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions};
use std::{path::PathBuf, time::Duration};
use zisk_common::{io::ZiskStdin, ExecutorStats, ZiskExecutionResult};

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
    fn world_rank(&self) -> i32 {
        self.core_prover.rank_info.world_rank
    }

    fn local_rank(&self) -> i32 {
        self.core_prover.rank_info.local_rank
    }

    fn set_stdin(&self, stdin: ZiskStdin) {
        self.core_prover.backend.witness_lib.set_stdin(stdin);
    }

    fn executed_steps(&self) -> u64 {
        self.core_prover
            .backend
            .witness_lib
            .execution_result()
            .map(|(exec_result, _)| exec_result.executed_steps)
            .unwrap_or(0)
    }

    fn execute(
        &self,
        stdin: ZiskStdin,
        output_path: PathBuf,
    ) -> Result<(ZiskExecutionResult, Duration)> {
        self.core_prover.backend.execute(stdin, output_path)
    }

    fn debug_verify_constraints(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        let debug_info =
            create_debug_info(debug_info, self.core_prover.backend.proving_key.clone());

        self.core_prover.backend.debug_verify_constraints(stdin, debug_info)
    }

    fn verify_constraints(
        &self,
        stdin: ZiskStdin,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.core_prover.backend.verify_constraints(stdin)
    }

    fn prove(
        &self,
        stdin: ZiskStdin,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)> {
        self.core_prover.backend.prove(stdin)
    }

    fn generate_proof_from_lib(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ProvePhaseResult, Box<dyn std::error::Error>> {
        self.core_prover.backend.generate_proof_from_lib(phase_inputs, options, phase)
    }

    fn receive_aggregated_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Option<Vec<AggProofs>> {
        self.core_prover.backend.receive_aggregated_proofs(
            agg_proofs,
            last_proof,
            final_proof,
            options,
        )
    }

    fn mpi_broadcast(&self, data: &mut Vec<u8>) {
        self.core_prover.backend.mpi_broadcast(data);
    }
}

pub struct EmuCoreProver {
    backend: ProverBackend,
    rank_info: RankInfo,
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

        // Build emulator library
        let (library, mut witness_lib) =
            ZiskLibLoader::load_emu(witness_lib, elf, verbose.into(), shared_tables)?;

        let proofman = ProofMan::new(
            proving_key.clone(),
            custom_commits_map,
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
            verbose.into(),
            witness_lib.get_packed_info(),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let world_rank = proofman.get_world_rank();
        let local_rank = proofman.get_local_rank();

        initialize_logger(verbose.into(), Some(world_rank));

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

        Ok(Self { backend: core, rank_info: RankInfo { world_rank, local_rank } })
    }
}
