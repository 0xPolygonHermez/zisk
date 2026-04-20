//! Stub for the ASM backend on non-x86_64 Linux platforms.
//!
//! Provides the same public types (`Asm`, `AsmProver`, `AsmSetupBuilder`, etc.)
//! so that downstream code compiles everywhere, but every method panics with a
//! clear "unsupported platform" message at runtime.

use crate::guest::ProgramId;
use crate::prover::{ProverEngine, ZiskBackend};
use crate::{
    BackendProverOpts, ExecuteOutput, GuestProgram, ProveOutput, ZiskAggPhaseResult,
    ZiskPhaseResult, ZiskVerifyConstraintsResult,
};
use anyhow::Result;
use proofman::{AggProofs, AggProofsRegister, ProvePhase, ProvePhaseInputs, WitnessInfo};
use proofman_common::{ProofOptions, ProofmanOptions, RowInfo};
use std::path::PathBuf;
use zisk_cluster_common::LoggingConfig;
use zisk_common::io::ZiskStdin;
use zisk_common::{
    ExecutorStatsHandle, ProgramVK, ProofKind, PublicValues, ZiskExecutorTime, ZiskVK,
};

const UNSUPPORTED: &str = "ASM backend is only supported on Linux x86_64";

pub struct Asm;

impl ZiskBackend for Asm {
    type Prover = AsmProver;
}

pub struct AsmSetupBuilder<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> AsmSetupBuilder<'a> {
    pub fn with_hints(self) -> Self {
        self
    }

    pub fn run(self) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }
}

pub struct AsmProver {
    _private: (),
}

impl AsmProver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _snark_wrapper: bool,
        _preload_snark: bool,
        _proving_key: PathBuf,
        _proving_key_snark: PathBuf,
        _shared_tables: bool,
        _base_port: Option<u16>,
        _unlock_mapped_memory: bool,
        _asm_out_file: bool,
        _no_auto_setup: bool,
        _options: ProofmanOptions,
        _is_distributed: bool,
        _logging_config: Option<LoggingConfig>,
    ) -> Result<Self> {
        anyhow::bail!(UNSUPPORTED)
    }
}

impl ProverEngine for AsmProver {
    type Builder<'a> = AsmSetupBuilder<'a>;

    fn setup<'a>(&'a self, _elf: &'a GuestProgram) -> Self::Builder<'a> {
        AsmSetupBuilder { _marker: std::marker::PhantomData }
    }

    fn setup_internal(&self, _elf: &GuestProgram, _with_hints: bool) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn world_rank(&self) -> i32 {
        0
    }

    fn local_rank(&self) -> i32 {
        0
    }

    fn set_stdin(&self, _stdin: ZiskStdin) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn register_program(&self, _program_id: &ProgramId) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn executed_steps(&self) -> u64 {
        0
    }

    fn get_execution_info(&self) -> Result<(WitnessInfo, ZiskExecutorTime)> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn get_instance_trace(
        &self,
        _instance_id: usize,
        _first_row: usize,
        _num_rows: usize,
        _offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn get_instance_air_values(&self, _instance_id: usize) -> Result<Vec<u64>> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn get_instance_fixed(
        &self,
        _instance_id: usize,
        _first_row: usize,
        _num_rows: usize,
        _offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn execute(&self, _program: &GuestProgram, _stdin: ZiskStdin) -> Result<ExecuteOutput> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn stats(
        &self,
        _program: &GuestProgram,
        _stdin: ZiskStdin,
        _debug_info: Option<Option<String>>,
        _minimal_memory: bool,
        _mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn verify_constraints(
        &self,
        _program: &GuestProgram,
        _stdin: ZiskStdin,
        _debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn prove(
        &self,
        _program: &GuestProgram,
        _stdin: ZiskStdin,
        _proof_kind: ProofKind,
        _prover_options: BackendProverOpts,
    ) -> Result<ProveOutput> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn wrap_proof(
        &self,
        _proof_bytes: &[u8],
        _publics: &PublicValues,
        _vk: &ProgramVK,
        _proof_kind: ProofKind,
    ) -> Result<ProveOutput> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn prove_phase(
        &self,
        _phase_inputs: ProvePhaseInputs,
        _options: ProofOptions,
        _phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn set_partition(
        &self,
        _total_compute_units: usize,
        _allocation: Vec<u32>,
        _rank_id: usize,
    ) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn register_aggregated_proofs(&self, _agg_proofs: Vec<AggProofsRegister>) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn aggregate_proofs(
        &self,
        _agg_proofs: Vec<AggProofs>,
        _last_proof: bool,
        _final_proof: bool,
        _options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn get_vadcop_vk(&self, _minimal: bool) -> Result<ZiskVK> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn mpi_broadcast(&self, _data: &mut Vec<u8>) -> Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }

    fn cancel(&self) {}
}

pub struct AsmInfo {
    _private: (),
}

pub struct AsmCoreProver {
    _private: (),
}
