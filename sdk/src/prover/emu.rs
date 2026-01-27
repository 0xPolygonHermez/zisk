use crate::{
    check_paths_exist, get_custom_commits_map,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    RankInfo, ZiskAggPhaseResult, ZiskExecuteResult, ZiskLibLoader, ZiskPhaseResult, ZiskProgramVK,
    ZiskProveResult, ZiskVerifyConstraintsResult,
};
use proofman::{AggProofs, ProofMan, ProvePhase, ProvePhaseInputs, SnarkWrapper};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions};
use std::path::PathBuf;
use zisk_common::io::ZiskStdin;
use zisk_common::ExecutorStats;
use zisk_distributed_common::LoggingConfig;
use zisk_witness::WitnessLibrary;

use crate::{ProofMode, ProofOpts};

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
        snark_wrapper: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        gpu_params: ParamsGPU,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let core_prover = EmuCoreProver::new(
            verify_constraints,
            aggregation,
            snark_wrapper,
            proving_key,
            proving_key_snark,
            elf,
            verbose,
            shared_tables,
            gpu_params,
            logging_config,
        )?;

        Ok(Self { core_prover })
    }

    pub fn new_verifier(proving_key: PathBuf, proving_key_snark: PathBuf) -> Result<Self> {
        let core_prover = EmuCoreProver::new_verifier(proving_key, proving_key_snark)?;

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
        self.core_prover.backend.witness_lib.as_ref().unwrap().set_stdin(stdin);
    }

    fn executed_steps(&self) -> u64 {
        self.core_prover
            .backend
            .witness_lib
            .as_ref()
            .unwrap()
            .execution_result()
            .map(|(exec_result, _)| exec_result.executed_steps)
            .unwrap_or(0)
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
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
        self.core_prover.backend.stats(stdin, debug_info, minimal_memory, mpi_node)
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

    fn vk(&self, elf_path: PathBuf) -> Result<ZiskProgramVK> {
        self.core_prover.backend.vk(elf_path)
    }

    fn verify(&self, proof: &ZiskProveResult, vk: &ZiskProgramVK) -> Result<()> {
        self.core_prover.backend.verify(proof, vk)
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

pub struct EmuCoreProver {
    backend: ProverBackend,
    rank_info: RankInfo,
}

impl EmuCoreProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        verify_constraints: bool,
        aggregation: bool,
        use_snark_wrapper: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
        gpu_params: ParamsGPU,
        logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        let custom_commits_map = get_custom_commits_map(&proving_key, &elf)?;

        check_paths_exist(&proving_key)?;
        check_paths_exist(&elf)?;

        // Build emulator library
        let mut witness_lib = ZiskLibLoader::load_emu(elf, verbose.into(), shared_tables)?;

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

        witness_lib.register_witness(&proofman.get_wcm())?;

        proofman.set_barrier();

        let mut snark_wrapper = None;
        if use_snark_wrapper {
            check_paths_exist(&proving_key_snark)?;
            snark_wrapper = Some(SnarkWrapper::new(&proving_key_snark, verbose.into())?);
        }

        let core = ProverBackend {
            witness_lib: Some(witness_lib),
            proofman: Some(proofman),
            snark_wrapper,
            proving_key_path: proving_key,
            proving_key_snark_path: Some(proving_key_snark),
            verifier_only: false,
        };

        Ok(Self { backend: core, rank_info: RankInfo { world_rank, local_rank } })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_verifier(proving_key: PathBuf, proving_key_snark: PathBuf) -> Result<Self> {
        let core_prover = ProverBackend {
            witness_lib: None,
            proofman: None,
            snark_wrapper: None,
            proving_key_path: proving_key,
            proving_key_snark_path: Some(proving_key_snark),
            verifier_only: true,
        };

        Ok(Self { backend: core_prover, rank_info: RankInfo { world_rank: 0, local_rank: 0 } })
    }
}
