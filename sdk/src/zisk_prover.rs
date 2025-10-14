use anyhow::Result;
use asm_runner::AsmServices;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_common::DebugInfo;
use std::path::PathBuf;
use zisk_common::{ExecutorStats, ZiskExecutionResult, ZiskLib};

pub struct ZiskProver<F: PrimeField64> {
    witness_lib: Box<dyn ZiskLib<F>>,
    world_rank: i32,
    proofman: ProofMan<F>,
    asm_services: Option<AsmServices>,
    debug_info: DebugInfo,
}

impl<F: PrimeField64> ZiskProver<F>
where
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    pub fn new(
        witness_lib: Box<dyn ZiskLib<F>>,
        world_rank: i32,

        proofman: ProofMan<F>,
        asm_services: Option<AsmServices>,
        debug_info: DebugInfo,
    ) -> Result<Self> {
        Ok(Self { witness_lib, world_rank, proofman, asm_services, debug_info })
    }

    pub fn verify_constraints(&self, input: Option<PathBuf>) -> Result<()> {
        self.proofman
            .verify_proof_constraints_from_lib(input.clone(), &self.debug_info, false)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        Ok(())
    }

    pub fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)> {
        self.witness_lib.get_execution_result()
    }

    pub fn finalize(&self) -> Result<()> {
        if let Some(asm_services) = &self.asm_services {
            // Shut down ASM microservices
            tracing::info!("<<< [{}] Shutting down ASM microservices.", self.world_rank);
            asm_services.stop_asm_services()?;
        }
        Ok(())
    }
}
