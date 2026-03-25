use crate::GuestProgram;
use crate::{
    check_paths_exist,
    prover::{ProverBackend, ProverEngine, ZiskBackend},
    ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProgramPK, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};
use crate::{ensure_custom_commits, ProofOpts};
use executor::{get_packed_info, initialize_executor};
use proofman::{
    AggProofs, AggProofsRegister, ProofMan, ProvePhase, ProvePhaseInputs, SnarkWrapper, WitnessInfo,
};
use proofman_common::{initialize_logger, ParamsGPU, ProofOptions, RankInfo, RowInfo};
use std::path::PathBuf;
use std::sync::Arc;
use zisk_common::{
    io::ZiskStdin, ExecutorStatsHandle, ProofMode, ZiskExecutorTime, ZiskProgramVK, ZiskProof,
    ZiskProofWithPublicValues, ZiskPublics,
};
use zisk_core::Riscv2zisk;
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
        snark_wrapper: bool,
        proving_key: PathBuf,
        proving_key_snark: PathBuf,
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
            verbose,
            shared_tables,
            gpu_params,
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

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.core_prover.backend.set_stdin(stdin)
    }

    fn register_program(&self, pk: &ZiskProgramPK) -> Result<()> {
        self.core_prover.backend.register_program(pk)
    }

    fn setup(&self, elf: &GuestProgram, _with_hints: bool) -> Result<(ZiskProgramPK, ZiskProgramVK)> {
        let pctx = self.core_prover.backend.get_pctx()?;

        let (rom_bin_path, vk) = ensure_custom_commits(&pctx, elf)?;

        let rv2zk = Riscv2zisk::new(elf.elf());

        let zisk_rom = rv2zk.run().unwrap_or_else(|e| panic!("Application error: {e}"));
        let zisk_rom = Arc::new(zisk_rom);

        Ok((ZiskProgramPK::new(zisk_rom, rom_bin_path), ZiskProgramVK { vk }))
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

    fn verify_constraints(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.core_prover.backend.verify_constraints(pk, stdin, debug_info)
    }

    fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK> {
        self.core_prover.backend.vk(elf)
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

    fn plonk(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        self.core_prover.backend.plonk(proof, publics, vk)
    }

    fn reduce(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        self.core_prover.backend.reduce(proof, publics, vk)
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
        verbose: u8,
        shared_tables: bool,
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

        let executor =
            initialize_executor(verbose.into(), shared_tables, false, &proofman.get_wcm())?;

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
