use crate::{
    Proof, RankInfo, ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};
use anyhow::Result;
use bytemuck::cast_slice;
use colored::Colorize;
use fields::Goldilocks;
use proofman::{AggProofs, ProofInfo, ProofMan, ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::{DebugInfo, ProofOptions};
use std::{fs::File, io::Write, path::PathBuf};
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    ExecutorStats, ProofLog, ZiskExecutionResult, ZiskLib,
};

pub(crate) struct ProverBackend {
    pub verify_constraints: bool,
    pub aggregation: bool,
    pub rma: bool,
    pub compressed: bool,
    pub witness_lib: Box<dyn ZiskLib<Goldilocks>>,
    pub proving_key: PathBuf,
    pub verify_proofs: bool,
    pub minimal_memory: bool,
    pub save_proofs: bool,
    pub output_dir: Option<PathBuf>,
    pub proofman: ProofMan<Goldilocks>,
    pub rank_info: RankInfo,
}

impl ProverBackend {
    pub(crate) fn execute(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        self.witness_lib.set_stdin(stdin);
        if let Some(stream) = hints_stream {
            self.witness_lib
                .set_hints_stream(stream)
                .map_err(|e| anyhow::anyhow!("Error setting hints stream: {}", e))?;
        }

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
        hints_stream: Option<StreamSource>,
        debug_info: DebugInfo,
        _mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
        self.witness_lib.set_stdin(stdin);
        if let Some(stream) = hints_stream {
            self.witness_lib
                .set_hints_stream(stream)
                .map_err(|e| anyhow::anyhow!("Error setting hints stream: {}", e))?;
        }

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
        hints_stream: Option<StreamSource>,
        debug_info: DebugInfo,
    ) -> Result<ZiskVerifyConstraintsResult> {
        if !self.verify_constraints {
            return Err(anyhow::anyhow!("Constraint verification is disabled for this prover."));
        }

        let start = std::time::Instant::now();

        self.witness_lib.set_stdin(stdin);
        if let Some(stream) = hints_stream {
            self.witness_lib
                .set_hints_stream(stream)
                .map_err(|e| anyhow::anyhow!("Error setting hints stream: {}", e))?;
        }

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
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.verify_constraints_debug(stdin, hints_stream, DebugInfo::default())
    }

    pub(crate) fn prove(
        &self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskProveResult> {
        if self.verify_constraints {
            return Err(anyhow::anyhow!(
                "Prover initialized with constraint verification enabled. Use `prove` instead."
            ));
        }

        let start = std::time::Instant::now();

        self.witness_lib.set_stdin(stdin);
        if let Some(stream) = hints_stream {
            self.witness_lib
                .set_hints_stream(stream)
                .map_err(|e| anyhow::anyhow!("Error setting hints stream: {}", e))?;
        }

        self.proofman.set_barrier();
        let proof = self
            .proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(None, 1, vec![0], 0)),
                ProofOptions::new(
                    self.verify_constraints,
                    self.aggregation,
                    self.rma,
                    self.compressed,
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

        let proof = Proof { id: proof_id, proof };

        if let Some(proof_id) = proof.id.clone() {
            let output_dir = self.output_dir.as_ref().unwrap();

            if self.rank_info.local_rank == 0 && !output_dir.exists() {
                std::fs::create_dir_all(output_dir)?;
            }

            let logs = ProofLog::new(execution_result.steps, proof_id, elapsed.as_secs_f64());
            let log_path = output_dir.join("result.json");
            ProofLog::write_json_log(&log_path, &logs)
                .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))?;

            // Save the uncompressed vadcop final proof
            let output_file_path = output_dir.join("vadcop_final_proof.bin");
            let vadcop_proof = proof.proof.clone().unwrap();
            let mut file = File::create(output_file_path)?;
            file.write_all(cast_slice(&vadcop_proof))?;
        }

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.lock().unwrap().get_id();
            _stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.lock().unwrap().store_stats();
        }

        self.proofman.set_barrier();

        Ok(ZiskProveResult { execution: execution_result, duration: elapsed, stats, proof })
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
}
