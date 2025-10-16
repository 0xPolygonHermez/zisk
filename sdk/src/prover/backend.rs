use crate::Proof;
use anyhow::Result;
use bytemuck::cast_slice;
use fields::Goldilocks;
use proofman::{ProofInfo, ProofMan, ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::{DebugInfo, ProofOptions};
use std::{fs::File, io::Write, path::PathBuf, time::Duration};
use zisk_common::{ExecutorStats, ProofLog, ZiskExecutionResult, ZiskLib};
use zstd::Encoder;

pub struct ProverBackend {
    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,
    pub witness_lib: Box<dyn ZiskLib<Goldilocks>>,
    pub proving_key: PathBuf,
    pub verify_proofs: bool,
    pub minimal_memory: bool,
    pub save_proofs: bool,
    pub output_dir: Option<PathBuf>,
    pub proofman: ProofMan<Goldilocks>,
}

impl ProverBackend {
    pub fn debug_verify_constraints(
        &self,
        input: Option<PathBuf>,
        debug_info: DebugInfo,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        if !self.verify_constraints {
            return Err(anyhow::anyhow!("Constraint verification is disabled for this prover."));
        }

        let start = std::time::Instant::now();
        self.proofman
            .verify_proof_constraints_from_lib(input, &debug_info, false)
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

        Ok((result, elapsed, stats))
    }

    pub fn verify_constraints(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.debug_verify_constraints(input, DebugInfo::default())
    }

    pub fn prove(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)> {
        if self.verify_constraints {
            return Err(anyhow::anyhow!(
                "Prover initialized with constraint verification enabled. Use `prove` instead."
            ));
        }

        let start = std::time::Instant::now();

        self.proofman.set_barrier();
        let proof = self
            .proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(input, 1, vec![0], 0)),
                ProofOptions::new(
                    self.verify_constraints,
                    self.aggregation,
                    self.final_snark,
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

            if !output_dir.exists() {
                std::fs::create_dir_all(output_dir)?;
            }

            let logs =
                ProofLog::new(execution_result.executed_steps, proof_id, elapsed.as_secs_f64());
            let log_path = output_dir.join("result.json");
            ProofLog::write_json_log(&log_path, &logs)
                .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))?;

            // Save the uncompressed vadcop final proof
            let output_file_path = output_dir.join("vadcop_final_proof.bin");
            let vadcop_proof = proof.proof.clone().unwrap();
            let mut file = File::create(output_file_path)?;
            file.write_all(cast_slice(&vadcop_proof))?;

            // Save the compressed vadcop final proof using zstd (fastest compression level)
            let compressed_output_path = output_dir.join("vadcop_final_proof.compressed.bin");
            let compressed_file = File::create(&compressed_output_path)?;
            let mut encoder = Encoder::new(compressed_file, 1)?;
            encoder.write_all(cast_slice(&vadcop_proof))?;
            encoder.finish()?;

            let original_size = vadcop_proof.len() * 8;
            let compressed_size = std::fs::metadata(&compressed_output_path)?.len();
            let compression_ratio = compressed_size as f64 / original_size as f64;

            println!("Vadcop final proof saved:");
            println!("  Original: {} bytes", original_size);
            println!("  Compressed: {} bytes (ratio: {:.2}x)", compressed_size, compression_ratio);
        }

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.lock().unwrap().get_id();
            _stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.lock().unwrap().store_stats();
        }

        self.proofman.set_barrier();

        Ok((execution_result, elapsed, stats, proof))
    }
}
