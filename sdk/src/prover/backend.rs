use crate::create_debug_info;
use crate::{
    Proof, ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProgramVK, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};
use crate::{ProofMode, ProofOpts};
use anyhow::Result;
use colored::Colorize;
use fields::Goldilocks;
use proofman::{
    get_vadcop_final_proof_vkey, verify_snark_proof, AggProofs, ProofInfo, ProofMan, ProvePhase,
    ProvePhaseInputs, ProvePhaseResult, SnarkWrapper,
};
use proofman_common::ProofOptions;
use rom_setup::{rom_vkey, verify_program_vk_publics};
use std::path::PathBuf;
use zisk_common::{io::ZiskStdin, ExecutorStats, ProofLog, ZiskExecutionResult, ZiskLib};
use zisk_verifier::verify_zisk_proof;

pub(crate) struct ProverBackend {
    pub proofman: ProofMan<Goldilocks>,
    pub snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
    pub witness_lib: Box<dyn ZiskLib<Goldilocks>>,
    pub proving_key_path: PathBuf,
    pub proving_key_snark_path: Option<PathBuf>,
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
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        _mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

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
                    minimal_memory,
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
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let start = std::time::Instant::now();

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

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
        self.verify_constraints_debug(stdin, None)
    }

    pub(crate) fn vk(&self) -> Result<ZiskProgramVK> {
        let elf_path = self.witness_lib.get_elf_path();
        let proving_key_path = self.proving_key_path.clone();
        let (vk, publics_pos) = rom_vkey(&elf_path, &None, &proving_key_path)?;

        Ok(ZiskProgramVK { vk, starting_pos_publics_program_vk: publics_pos })
    }

    pub(crate) fn prove_debug(
        &self,
        stdin: ZiskStdin,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        let start = std::time::Instant::now();

        self.witness_lib.set_stdin(stdin);

        self.proofman.set_barrier();
        self.proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(None, 1, vec![0], 0)),
                ProofOptions::new(
                    false,
                    false,
                    false,
                    false,
                    proof_options.verify_proofs,
                    proof_options.minimal_memory,
                    proof_options.save_proofs,
                    proof_options
                        .output_dir_path
                        .clone()
                        .expect("output_dir must be set, unreachable"),
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let elapsed = start.elapsed();

        let (execution_result, stats) = self.witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.lock().unwrap().get_id();
            _stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.lock().unwrap().store_stats();
        }

        self.proofman.set_barrier();

        Ok(ZiskProveResult {
            execution: execution_result,
            duration: elapsed,
            stats,
            proof_id: None,
            proof: Proof::Null(),
        })
    }

    pub(crate) fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        let start = std::time::Instant::now();

        self.witness_lib.set_stdin(stdin);

        let compressed = matches!(mode, ProofMode::VadcopFinalCompressed);

        self.proofman.set_barrier();
        let proof = self
            .proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(None, 1, vec![0], 0)),
                ProofOptions::new(
                    false,
                    false,
                    proof_options.rma,
                    compressed,
                    proof_options.verify_proofs,
                    proof_options.minimal_memory,
                    proof_options.save_proofs,
                    proof_options
                        .output_dir_path
                        .clone()
                        .expect("output_dir must be set, unreachable"),
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

        let output_dir = proof_options.output_dir_path.as_ref().unwrap();

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
                let plonk_proof = self
                    .snark_wrapper
                    .as_ref()
                    .unwrap()
                    .generate_final_snark_proof(&vadcop_proof, output_dir)?;

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

    pub(crate) fn verify(&self, proof: &ZiskProveResult, program_vk: &ZiskProgramVK) -> Result<()> {
        match &proof.proof {
            Proof::Null() => Err(anyhow::anyhow!("No proof found to verify.")),
            Proof::Plonk(proof) => {
                let public_values = proof.get_public_bytes();
                verify_program_vk_publics(
                    &program_vk.vk,
                    program_vk.starting_pos_publics_program_vk,
                    public_values,
                )?;

                if self.proving_key_snark_path.is_none() {
                    return Err(anyhow::anyhow!(
                        "Proving key snark path is not set, cannot verify Plonk proof."
                    ));
                }

                let verkey_path = PathBuf::from(format!(
                    "{}/{}/{}.verkey.json",
                    self.proving_key_snark_path.as_ref().unwrap().display(),
                    "final",
                    "final"
                ));
                Ok(verify_snark_proof(proof, &verkey_path)?)
            }
            Proof::VadcopFinal(proof) => {
                let public_values = proof.get_public_bytes();

                verify_program_vk_publics(
                    &program_vk.vk,
                    program_vk.starting_pos_publics_program_vk,
                    &public_values,
                )?;

                let vk = get_vadcop_final_proof_vkey(&self.proving_key_path, proof.compressed)?;
                verify_zisk_proof(proof, &vk)
            }
        }
    }
}
