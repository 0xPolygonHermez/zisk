use crate::{ensure_custom_commits, Proof, ProverEngine, RankInfo, ZiskBackend, ZiskLibLoader};
use fields::{ExtensionField, Goldilocks, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo, ParamsGPU};
use std::{collections::HashMap, path::PathBuf};
use zisk_common::{ExecutorStats, ZiskExecutionResult, ZiskLib};

use anyhow::Result;

pub struct Emu;

impl ZiskBackend for Emu {
    type Prover = EmuProver;
}

pub struct EmuProver {
    pub(crate) core_prover: EmuCoreProver,
}

impl EmuProver {
    pub fn new(
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
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
        )?;
        Ok(Self { core_prover })
    }
}

impl ProverEngine for EmuProver {
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

pub struct EmuCoreProver {
    rank_info: RankInfo,
    witness_lib: Box<dyn ZiskLib<Goldilocks>>,
    proving_key: PathBuf,
    proofman: ProofMan<Goldilocks>,
    verify_constraints: bool,
}

impl EmuCoreProver {
    pub fn new(
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        witness_lib: PathBuf,
        proving_key: PathBuf,
        elf: PathBuf,
        verbose: u8,
        shared_tables: bool,
    ) -> Result<Self> {
        let rom_bin_path = ensure_custom_commits(&proving_key, &elf)?;
        let custom_commits_map = HashMap::from([("rom".to_string(), rom_bin_path)]);

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

        Ok(Self {
            rank_info: RankInfo { world_rank, local_rank },
            proving_key,
            witness_lib,
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
