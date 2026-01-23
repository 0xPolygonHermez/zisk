use crate::{
    check_paths_exist, create_debug_info, get_custom_commits_map,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    RankInfo, ZiskAggPhaseResult, ZiskExecuteResult, ZiskLibLoader, ZiskPhaseResult,
    ZiskProveResult, ZiskVerifyConstraintsResult,
};
use proofman::{AggProofs, ProofMan, ProvePhase, ProvePhaseInputs};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions};
use std::path::PathBuf;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::ExecutorStats;
use zisk_distributed_common::LoggingConfig;

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
        rma: bool,
        compressed: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        proving_key_snark: Option<PathBuf>,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let core_prover = EmuCoreProver::new(
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

    fn set_hints_stream(&self, _: StreamSource) -> Result<()> {
        unreachable!("EMU prover does not support precompile hints");
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
        if hints_stream.is_some() {
            return Err(anyhow::anyhow!("EMU prover does not support precompile hints"));
        }
        self.core_prover.backend.execute(stdin, None, output_path)
    }

    fn stats(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        debug_info: Option<Option<String>>,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
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

pub struct EmuCoreProver {
    backend: ProverBackend,
    rank_info: RankInfo,
}

impl EmuCoreProver {
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
        gpu_params: ParamsGPU,
        verify_proofs: bool,
        minimal_memory: bool,
        save_proofs: bool,
        output_dir: Option<PathBuf>,
        logging_config: Option<LoggingConfig>,
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

        Ok(Self { backend: core, rank_info: RankInfo { world_rank, local_rank } })
    }
}
