use crate::{ensure_custom_commits, Proof, ProverEngine, RankInfo, ZiskBackend, ZiskLibLoader};
use asm_runner::{AsmRunnerOptions, AsmServices};
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_common::{initialize_logger, DebugInfo, ParamsGPU};
use rom_setup::DEFAULT_CACHE_PATH;
use std::{collections::HashMap, path::PathBuf};
use tracing::info;
use zisk_common::{ExecutorStats, ZiskExecutionResult, ZiskLib};

use anyhow::Result;

pub struct Asm<F: PrimeField64>(std::marker::PhantomData<F>);

impl<F> ZiskBackend for Asm<F>
where
    F: PrimeField64,
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    type Prover = AsmProver<F>;
}

pub struct AsmProver<F>
where
    F: PrimeField64,
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    pub(crate) core_prover: AsmCoreProver<F>,
}

impl<F: PrimeField64> AsmProver<F>
where
    F: PrimeField64,
    GoldilocksQuinticExtension: ExtensionField<F>,
{
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
        )?;
        Ok(Self { core_prover })
    }
}

impl<F> ProverEngine for AsmProver<F>
where
    F: PrimeField64,
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    fn verify_constraints(&self, input: Option<PathBuf>) -> Result<()> {
        self.core_prover.verify_constraints(input)
    }

    fn generate_proof(&self, input: Option<PathBuf>) -> Result<Proof> {
        // Perform proof generation logic here
        Ok(Proof)
    }

    fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)> {
        self.core_prover.execution_result()
    }
}

pub struct AsmCoreProver<F>
where
    F: PrimeField64,
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    rank_info: RankInfo,
    witness_lib: Box<dyn ZiskLib<F>>,
    proofman: ProofMan<F>,
    verify_constraints: bool,
}

impl<F> AsmCoreProver<F>
where
    F: PrimeField64,
    GoldilocksQuinticExtension: ExtensionField<F>,
{
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
    ) -> Result<Self> {
        let rom_bin_path = ensure_custom_commits(&proving_key, &elf)?;
        let custom_commits_map = HashMap::from([("rom".to_string(), rom_bin_path)]);

        let default_cache_path =
            std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

        let asm_mt_path = default_cache_path.join(asm_mt_filename);
        let asm_rh_path = default_cache_path.join(asm_rh_filename);

        // TODO! Check if paths exist

        let proofman = ProofMan::<F>::new(
            proving_key,
            custom_commits_map,
            verify_constraints,
            aggregation,
            final_snark,
            ParamsGPU::default(),
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

        Ok(Self {
            rank_info: RankInfo { world_rank, local_rank },
            witness_lib,
            proofman,
            verify_constraints,
        })
    }

    fn verify_constraints(&self, input: Option<PathBuf>) -> Result<()> {
        if !self.verify_constraints {
            return Err(anyhow::anyhow!("Constraint verification is disabled for this prover."));
        }

        self.proofman
            .verify_proof_constraints_from_lib(input, &DebugInfo::default(), false)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        Ok(())
    }

    fn generate_proof(&self) -> Result<Proof> {
        // Perform proof generation logic here
        Ok(Proof)
    }

    fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)> {
        self.witness_lib.get_execution_result()
    }
}
