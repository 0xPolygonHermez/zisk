use crate::ProofMode;
use crate::{
    Proof, RankInfo, ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProgramVK,
    ZiskProveResult, ZiskVerifyConstraintsResult,
};
use anyhow::Result;
use colored::Colorize;
use fields::Goldilocks;
use proofman::{
    AggProofs, ProofInfo, ProofMan, ProvePhase, ProvePhaseInputs, ProvePhaseResult, SnarkWrapper,
};
use proofman_common::{DebugInfo, ProofOptions};
use rom_setup::rom_vkey;
use std::path::PathBuf;
use zisk_common::{io::ZiskStdin, ExecutorStats, ProofLog, ZiskExecutionResult, ZiskLib};
use zisk_verifier::verify_zisk_proof;

pub(crate) struct ProverBackend {
    pub verify_constraints: bool,
    pub aggregation: bool,
    pub rma: bool,
    pub witness_lib: Box<dyn ZiskLib<Goldilocks>>,
    pub proving_key: PathBuf,
    pub verify_proofs: bool,
    pub minimal_memory: bool,
    pub save_proofs: bool,
    pub output_dir: Option<PathBuf>,
    pub proofman: ProofMan<Goldilocks>,
    pub snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
    #[allow(unused)]
    pub rank_info: RankInfo,
}

impl ProverBackend {
    pub(crate) fn execute(
        &self,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        self.witness_lib.set_stdin(stdin);

        let start = std::time::Instant::now();

        self.proofman
            .execute_from_lib(output_path)
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let elapsed = start.elapsed();

        let (result, _) = self.witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        Ok(ZiskExecuteResult { execution: result, duration: elapsed })
    }

    pub(crate) fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: DebugInfo,
        _mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
        self.witness_lib.set_stdin(stdin);

        let world_rank = self.proofman.get_world_rank();
        let local_rank = self.proofman.get_local_rank();
        let n_processes = self.proofman.get_n_processes();

        let mut is_active = true;

        if let Some(mpi_node) = _mpi_node {
            if local_rank != mpi_node as i32 {
                is_active = false;
            }
        }

        self.proofman.split_active_processes(is_active);

        if !is_active {
            println!(
                "{}: {}",
                format!("Rank {local_rank}").bright_yellow().bold(),
                "Inactive rank, skipping computation.".bright_yellow()
            );

            return Ok((world_rank, n_processes, None));
        }

        self.proofman
            .compute_witness_from_lib(
                &debug_info,
                ProofOptions::new(
                    false,
                    false,
                    false,
                    false,
                    false,
                    self.minimal_memory,
                    false,
                    PathBuf::new(),
                ),
            )
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let (_, stats): (ZiskExecutionResult, ExecutorStats) =
            self.witness_lib.execution_result().ok_or_else(|| {
                anyhow::anyhow!("Failed to get execution result from emulator prover")
            })?;

        Ok((world_rank, n_processes, Some(stats)))
    }

    pub(crate) fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: DebugInfo,
    ) -> Result<ZiskVerifyConstraintsResult> {
        if !self.verify_constraints {
            return Err(anyhow::anyhow!("Constraint verification is disabled for this prover."));
        }

        let start = std::time::Instant::now();

        self.witness_lib.set_stdin(stdin);

        self.proofman
            .verify_proof_constraints_from_lib(&debug_info, false)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
        let elapsed = start.elapsed();

        let (result, stats) = self.witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.lock().unwrap().get_id();
            _stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.lock().unwrap().store_stats();
        }

        Ok(ZiskVerifyConstraintsResult { execution: result, duration: elapsed, stats })
    }

    pub(crate) fn verify_constraints(
        &self,
        stdin: ZiskStdin,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.verify_constraints_debug(stdin, DebugInfo::default())
    }

    pub(crate) fn vk(&self) -> Result<ZiskProgramVK> {
        let elf_path = self.witness_lib.get_elf_path();
        let proving_key_path = self.proofman.get_proving_key_path();
        let program_vk = rom_vkey(&elf_path, &None, &proving_key_path)?;

        let (vadcop_proof_vk, vadcop_proof_compressed_vk) = match self.aggregation {
            true => self.proofman.get_aggregated_vadcop_proof_vkey(),
            false => (Vec::new(), Vec::new()),
        };

        Ok(ZiskProgramVK { program_vk, vadcop_proof_vk, vadcop_proof_compressed_vk })
    }

    pub(crate) fn prove(&self, stdin: ZiskStdin, mode: ProofMode) -> Result<ZiskProveResult> {
        if self.verify_constraints {
            return Err(anyhow::anyhow!(
                "Prover initialized with constraint verification enabled. Use `prove` instead."
            ));
        }

        let start = std::time::Instant::now();

        self.witness_lib.set_stdin(stdin);

        let compressed = matches!(mode, ProofMode::VadcopFinalCompressed);

        self.proofman.set_barrier();
        let proof = self
            .proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(None, 1, vec![0], 0)),
                ProofOptions::new(
                    self.verify_constraints,
                    self.aggregation,
                    self.rma,
                    compressed,
                    self.verify_proofs,
                    self.minimal_memory,
                    self.save_proofs,
                    self.output_dir.clone().expect("output_dir must be set, unreachable"),
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let elapsed = start.elapsed();

        let (proof_id, proof) = match proof {
            ProvePhaseResult::Full(proof_id, proof) => (proof_id, proof),
            _ => (None, None),
        };

        let (execution_result, stats) = self.witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        let output_dir = self.output_dir.as_ref().unwrap();

        if let Some(proof_id) = proof_id.clone() {
            let logs =
                ProofLog::new(execution_result.executed_steps, proof_id, elapsed.as_secs_f64());
            let log_path = output_dir.join("result.json");
            ProofLog::write_json_log(&log_path, &logs)
                .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))?;
        }

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.lock().unwrap().get_id();
            _stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.lock().unwrap().store_stats();
        }

        self.proofman.set_barrier();

        match (mode, proof) {
            (ProofMode::Plonk, Some(vadcop_proof)) => {
                let plonk_proof = self.snark_wrapper.as_ref().unwrap().generate_final_snark_proof(
                    &vadcop_proof,
                    output_dir,
                    self.save_proofs,
                )?;

                Ok(ZiskProveResult {
                    execution: execution_result,
                    duration: elapsed,
                    stats,
                    proof_id,
                    proof: Proof::Plonk(plonk_proof),
                })
            }
            (_, Some(p)) => Ok(ZiskProveResult {
                execution: execution_result,
                duration: elapsed,
                stats,
                proof_id,
                proof: Proof::VadcopFinal(p),
            }),
            (_, None) => Ok(ZiskProveResult {
                execution: execution_result,
                duration: elapsed,
                stats,
                proof_id,
                proof: Proof::Null(),
            }),
        }
    }

    pub(crate) fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        self.proofman
            .generate_proof_from_lib(phase_inputs, options, phase.clone())
            .map_err(|e| anyhow::anyhow!("Error generating proof in phase {:?}: {}", phase, e))
    }

    pub(crate) fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        let result = self
            .proofman
            .receive_aggregated_proofs(agg_proofs, last_proof, final_proof, options)
            .map_err(|e| anyhow::anyhow!("Error aggregating proofs: {}", e))?;

        Ok(result.map(|agg| ZiskAggPhaseResult { agg_proofs: agg }))
    }

    pub(crate) fn mpi_broadcast(&self, data: &mut Vec<u8>) {
        self.proofman.mpi_broadcast(data);
    }

    pub(crate) fn verify(&self, proof: &ZiskProveResult, vk: &ZiskProgramVK) -> Result<()> {
        match &proof.proof {
            Proof::Null() => Err(anyhow::anyhow!("No proof found to verify.")),
            Proof::Plonk(_) => {
                Err(anyhow::anyhow!("Plonk proofs are not supported for verification."))
            }
            Proof::VadcopFinal(proof) => {
                let vk = if proof.compressed {
                    &vk.vadcop_proof_compressed_vk
                } else {
                    &vk.vadcop_proof_vk
                };
                verify_zisk_proof(proof, vk)
            }
        }
    }
}
