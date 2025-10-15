use crate::{ensure_custom_commits, Proof, ProverEngine, RankInfo, ZiskBackend, ZiskLibLoader};
use asm_runner::{AsmRunnerOptions, AsmServices};
use fields::{ExtensionField, Goldilocks, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo, ParamsGPU};
use rom_setup::DEFAULT_CACHE_PATH;
use std::{collections::HashMap, path::PathBuf};
use tracing::info;
use zisk_common::{ExecutorStats, ZiskExecutionResult, ZiskLib};

use anyhow::Result;

pub struct Asm;

impl ZiskBackend for Asm {
    type Prover = AsmProver;
}

pub struct AsmProver {
    pub(crate) core_prover: AsmCoreProver,
}

impl AsmProver {
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

impl ProverEngine for AsmProver {
    fn debug_verify_constraints(
        &self,
        input: Option<PathBuf>,
        debug_info: Option<Option<String>>,
    ) -> Result<()> {
        let debug_info = match &debug_info {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => json_to_debug_instances_map(
                self.core_prover.proving_key.clone(),
                debug_value.clone(),
            ),
        };

        self.core_prover.debug_verify_constraints(input, debug_info)
    }

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

pub struct AsmCoreProver {
    rank_info: RankInfo,
    witness_lib: Box<dyn ZiskLib<Goldilocks>>,
    proving_key: PathBuf,
    proofman: ProofMan<Goldilocks>,
    verify_constraints: bool,
}

impl AsmCoreProver {
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

        let proofman = ProofMan::new(
            proving_key.clone(),
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
            proving_key,
            proofman,
            verify_constraints,
        })
    }

    fn debug_verify_constraints(
        &self,
        input: Option<PathBuf>,
        debug_info: DebugInfo,
    ) -> Result<()> {
        if !self.verify_constraints {
            return Err(anyhow::anyhow!("Constraint verification is disabled for this prover."));
        }

        self.proofman
            .verify_proof_constraints_from_lib(input, &debug_info, false)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        Ok(())
    }

    fn verify_constraints(&self, input: Option<PathBuf>) -> Result<()> {
        self.debug_verify_constraints(input, DebugInfo::default())
    }

    fn generate_proof(&self) -> Result<Proof> {
        // Perform proof generation logic here
        Ok(Proof)
    }

    fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)> {
        self.witness_lib.get_execution_result()
    }
}
