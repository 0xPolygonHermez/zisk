use crate::create_debug_info;
use crate::ZiskProofWithPublicValues;
use crate::ZiskPublics;
use crate::{
    get_program_vk_with_proving_key, verify_zisk_proof_with_proving_key,
    verify_zisk_snark_proof_with_proving_key,
};
use crate::{ProofMode, ProofOpts};
use crate::{
    ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProgramPK, ZiskProgramVK,
    ZiskProof, ZiskProveResult, ZiskVerifyConstraintsResult,
};
use anyhow::Result;
use colored::Colorize;
use executor::ZiskExecutor;
use fields::Goldilocks;
use proofman::{
    AggProofs, AggProofsRegister, ExecutionInfo, ProofMan, ProvePhase, ProvePhaseInputs,
    ProvePhaseResult, SnarkProtocol, SnarkWrapper,
};
use proofman_common::{ProofCtx, ProofOptions, RowInfo};
use proofman_util::VadcopFinalProof;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use zisk_common::stats_mark;
use zisk_common::{io::ZiskStdin, ElfBinaryLike, ExecutorStatsHandle, ZiskExecutionResult};

pub(crate) struct ProverBackend {
    proofman: Option<ProofMan<Goldilocks>>,
    snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
    executor: Option<Arc<ZiskExecutor<Goldilocks>>>,
    proving_key_path: PathBuf,
    proving_key_snark_path: Option<PathBuf>,
}

impl ProverBackend {
    pub fn new(
        proofman: ProofMan<Goldilocks>,
        snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
        executor: Arc<ZiskExecutor<Goldilocks>>,
        proving_key_path: PathBuf,
        proving_key_snark_path: Option<PathBuf>,
    ) -> Self {
        Self {
            proofman: Some(proofman),
            snark_wrapper,
            executor: Some(executor),
            proving_key_path,
            proving_key_snark_path,
        }
    }

    pub fn new_verifier(
        proving_key_path: PathBuf,
        proving_key_snark_path: Option<PathBuf>,
    ) -> Self {
        Self {
            proofman: None,
            snark_wrapper: None,
            executor: None,
            proving_key_path,
            proving_key_snark_path,
        }
    }

    pub fn get_pctx(&self) -> Result<Arc<ProofCtx<Goldilocks>>> {
        let proofman = self.proofman.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Proofman is not initialized. Please initialize it before use.")
        })?;
        Ok(proofman.get_wcm().get_pctx())
    }

    pub fn register_program(&self, program_pk: &ZiskProgramPK) -> Result<()> {
        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;

        let proofman = self.proofman.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Proofman is not initialized. Please initialize it before use.")
        })?;

        if let Some(asm_resources) = &program_pk.asm_resources {
            executor.set_asm_resources(asm_resources.clone());
        }

        executor.set_rom(program_pk.zisk_rom.clone(), program_pk.use_hints);

        let custom_commits_map =
            HashMap::from([("rom".to_string(), program_pk.elf_bin_path.clone())]);
        proofman
            .register_custom_commits(custom_commits_map)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;
        executor.set_stdin(stdin);
        Ok(())
    }

    pub fn execution_result(&self) -> Result<(ZiskExecutionResult, ExecutorStatsHandle)> {
        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;

        let (result, stats) = executor.get_execution_result();

        Ok((result, stats))
    }

    pub(crate) fn execute(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot execute in verifier mode"))?;

        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;

        self.register_program(pk)?;

        executor.set_stdin(stdin);

        let start = std::time::Instant::now();

        let planning_info = proofman
            .execute_from_lib(output_path)
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let elapsed = start.elapsed();

        let (result, _) = executor.get_execution_result();

        let publics = proofman.get_publics();

        Ok(ZiskExecuteResult::new(result, planning_info, elapsed, &publics))
    }

    pub(crate) fn stats(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        _mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot compute stats in verifier mode"))?;

        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        self.register_program(pk)?;

        executor.set_stdin(stdin);

        let rank_info = proofman.get_rank_info();

        let mut is_active = true;

        if let Some(mpi_node) = _mpi_node {
            if rank_info.local_rank != mpi_node as i32 {
                is_active = false;
            }
        }

        proofman.split_active_processes(is_active);

        if !is_active {
            println!(
                "{}: {}",
                format!("Rank {}", rank_info.local_rank).bright_yellow().bold(),
                "Inactive rank, skipping computation.".bright_yellow()
            );

            return Ok((rank_info.world_rank, rank_info.n_processes, None));
        }

        proofman
            .compute_witness_from_lib(
                &debug_info,
                ProofOptions::new(false, false, false, false, false, minimal_memory, false, None),
            )
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let (_, stats): (ZiskExecutionResult, ExecutorStatsHandle) =
            executor.get_execution_result();

        Ok((rank_info.world_rank, rank_info.n_processes, Some(stats)))
    }

    pub(crate) fn get_instance_trace(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot get instance trace in verifier mode"))?;

        proofman
            .get_instance_trace(instance_id, first_row, num_rows, offset)
            .map_err(|e| anyhow::anyhow!("Error getting instance trace: {}", e))
    }

    pub(crate) fn get_instance_air_values(&self, instance_id: usize) -> Result<Vec<u64>> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot get instance AIR values in verifier mode"))?;

        proofman
            .get_instance_air_values(instance_id)
            .map_err(|e| anyhow::anyhow!("Error getting instance AIR values: {}", e))
    }

    pub(crate) fn get_instance_fixed(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot get instance fixed in verifier mode"))?;

        proofman
            .get_instance_fixed(instance_id, first_row, num_rows, offset)
            .map_err(|e| anyhow::anyhow!("Error getting instance fixed: {}", e))
    }

    pub(crate) fn verify_constraints_debug(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot verify constraints in verifier mode"))?;

        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;

        let start = std::time::Instant::now();

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        self.register_program(pk)?;

        executor.set_stdin(stdin);

        proofman
            .verify_proof_constraints_from_lib(&debug_info, false)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
        let elapsed = start.elapsed();

        let (result, stats) = executor.get_execution_result();

        stats_mark!(stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        stats.store_stats();

        let publics = proofman.get_publics();

        Ok(ZiskVerifyConstraintsResult::new(result, elapsed, stats, &publics))
    }

    pub(crate) fn verify_constraints(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.verify_constraints_debug(pk, stdin, None)
    }

    pub(crate) fn vk(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        get_program_vk_with_proving_key(elf, self.proving_key_path.clone())
    }

    pub(crate) fn prove(
        &self,
        pk: &ZiskProgramPK,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot prove in verifier mode"))?;

        let executor = self.executor.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Executor is not initialized. Please initialize it before use.")
        })?;

        self.register_program(pk)?;

        if mode == ProofMode::Snark && self.snark_wrapper.is_none() {
            return Err(anyhow::anyhow!(
                "Snark wrapper is not initialized. Cannot generate snark proof."
            ));
        }

        let start = std::time::Instant::now();

        executor.set_stdin(stdin);

        let compressed = matches!(mode, ProofMode::VadcopFinalCompressed);

        proofman.set_partition(1, vec![0], 0)?;

        proofman.set_barrier();
        let proof = proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(),
                ProofOptions::new(
                    false,
                    proof_options.aggregation,
                    proof_options.rma,
                    compressed,
                    proof_options.verify_proofs,
                    proof_options.minimal_memory,
                    proof_options.save_proofs,
                    proof_options.output_dir_path.clone(),
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let (proof_id, proof) = match proof {
            ProvePhaseResult::Full(proof_id, proof) => (proof_id, proof),
            _ => (None, None),
        };

        let (execution_result, stats) = executor.get_execution_result();

        // Store the stats in stats.json
        stats_mark!(stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        stats.store_stats();

        proofman.set_barrier();

        match (mode, proof) {
            (ProofMode::Snark, Some(vadcop_proof)) => {
                let snark_proof = self.snark_wrapper.as_ref().unwrap().generate_final_snark_proof(
                    &vadcop_proof,
                    proof_options.output_dir_path.clone(),
                )?;

                let publics = ZiskPublics::new(&vadcop_proof.public_values);
                let program_vk = ZiskProgramVK::new_from_publics(&vadcop_proof.public_values);
                if snark_proof.protocol_id == SnarkProtocol::Plonk.protocol_id() {
                    Ok(ZiskProveResult::new(
                        execution_result,
                        start.elapsed(),
                        stats,
                        proof_id,
                        ZiskProofWithPublicValues {
                            proof: ZiskProof::Plonk(snark_proof.proof_bytes),
                            publics,
                            program_vk,
                        },
                    ))
                } else if snark_proof.protocol_id == SnarkProtocol::Fflonk.protocol_id() {
                    Ok(ZiskProveResult::new(
                        execution_result,
                        start.elapsed(),
                        stats,
                        proof_id,
                        ZiskProofWithPublicValues {
                            proof: ZiskProof::Fflonk(snark_proof.proof_bytes),
                            publics,
                            program_vk,
                        },
                    ))
                } else {
                    Err(anyhow::anyhow!(
                        "Unsupported snark protocol id: {}",
                        snark_proof.protocol_id
                    ))
                }
            }
            (_, Some(p)) => {
                let proof = if compressed {
                    ZiskProof::VadcopFinalCompressed(p.proof)
                } else {
                    ZiskProof::VadcopFinal(p.proof)
                };
                Ok(ZiskProveResult::new(
                    execution_result,
                    start.elapsed(),
                    stats,
                    proof_id,
                    ZiskProofWithPublicValues {
                        proof,
                        publics: ZiskPublics::new(&p.public_values),
                        program_vk: ZiskProgramVK::new_from_publics(&p.public_values),
                    },
                ))
            }
            (_, None) => Ok(ZiskProveResult::new_null(execution_result, start.elapsed(), stats)),
        }
    }

    pub(crate) fn compress(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        program_vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot compress in verifier mode"))?;

        let proof_bytes = match proof {
            ZiskProof::VadcopFinal(bytes) => bytes.clone(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Cannot generate SNARK proof. Only VadcopFinal proofs can be converted to SNARK proofs.",
                ));
            }
        };

        let mut pubs = program_vk.vk.clone();
        pubs.extend(publics.public_bytes());
        let vadcop_final_proof = VadcopFinalProof::new(proof_bytes, pubs, false);

        let compressed_proof = proofman
            .generate_vadcop_final_proof_compressed(&vadcop_final_proof, None, false)
            .map_err(|e| anyhow::anyhow!("Error generating compressed proof: {}", e))?;

        Ok(ZiskProofWithPublicValues {
            proof: ZiskProof::VadcopFinalCompressed(compressed_proof.proof),
            publics: ZiskPublics::new(&compressed_proof.public_values),
            program_vk: ZiskProgramVK::new_from_publics(&compressed_proof.public_values),
        })
    }

    pub(crate) fn prove_snark(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        program_vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
        if self.snark_wrapper.is_none() {
            return Err(anyhow::anyhow!(
                "Snark wrapper is not initialized. Cannot generate snark proof."
            ));
        }

        let proof_bytes = match proof {
            ZiskProof::VadcopFinal(bytes) => bytes.clone(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Cannot generate SNARK proof. Only VadcopFinal proofs can be converted to SNARK proofs.",
                ));
            }
        };

        let mut pubs = program_vk.vk.clone();
        pubs.extend(publics.public_bytes());
        let vadcop_final_proof = VadcopFinalProof::new(proof_bytes, pubs, false);

        let snark_proof = self
            .snark_wrapper
            .as_ref()
            .unwrap()
            .generate_final_snark_proof(&vadcop_final_proof, None)?;

        if snark_proof.protocol_id == SnarkProtocol::Plonk.protocol_id() {
            Ok(ZiskProofWithPublicValues {
                proof: ZiskProof::Plonk(snark_proof.proof_bytes),
                publics: publics.clone(),
                program_vk: program_vk.clone(),
            })
        } else if snark_proof.protocol_id == SnarkProtocol::Fflonk.protocol_id() {
            Ok(ZiskProofWithPublicValues {
                proof: ZiskProof::Fflonk(snark_proof.proof_bytes),
                publics: publics.clone(),
                program_vk: program_vk.clone(),
            })
        } else {
            Err(anyhow::anyhow!("Unsupported snark protocol id: {}", snark_proof.protocol_id))
        }
    }

    pub(crate) fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot prove in verifier mode"))?;

        proofman
            .generate_proof_from_lib(phase_inputs, options, phase.clone())
            .map_err(|e| anyhow::anyhow!("Error generating proof in phase {:?}: {}", phase, e))
    }

    pub(crate) fn set_partition(
        &self,
        total_compute_units: usize,
        allocation: Vec<u32>,
        rank_id: usize,
    ) -> Result<()> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot set partition in verifier mode"))?;

        Ok(proofman.set_partition(total_compute_units, allocation, rank_id)?)
    }

    pub(crate) fn is_first_partition(&self) -> Result<bool> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot set partition in verifier mode"))?;

        Ok(proofman.is_first_partition())
    }

    pub(crate) fn get_execution_info(&self) -> Result<ExecutionInfo> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot get execution info in verifier mode"))?;
        Ok(proofman.get_execution_info())
    }

    pub(crate) fn register_aggregated_proofs(
        &self,
        agg_proofs: Vec<AggProofsRegister>,
    ) -> Result<()> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot aggregate proofs in verifier mode"))?;

        proofman
            .register_aggregated_proofs(agg_proofs)
            .map_err(|e| anyhow::anyhow!("Error registering aggregate proof: {}", e))
    }

    pub(crate) fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot aggregate proofs in verifier mode"))?;

        let result = proofman
            .receive_aggregated_proofs(agg_proofs, last_proof, final_proof, options)
            .map_err(|e| anyhow::anyhow!("Error aggregating proofs: {}", e))?;

        Ok(result.map(|agg| ZiskAggPhaseResult { agg_proofs: agg }))
    }

    pub(crate) fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot broadcast in verifier mode"))?;

        proofman.mpi_broadcast(data);
        Ok(())
    }

    pub(crate) fn verify(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        program_vk: &ZiskProgramVK,
    ) -> Result<()> {
        match &proof {
            ZiskProof::Null() => Err(anyhow::anyhow!("No proof found to verify.")),
            ZiskProof::Plonk(_) | ZiskProof::Fflonk(_) => verify_zisk_snark_proof_with_proving_key(
                proof,
                publics,
                program_vk,
                self.proving_key_path.clone(),
                self.proving_key_snark_path
                    .clone()
                    .expect("Proving key snark path is required for snark proofs"),
            ),
            ZiskProof::VadcopFinal(_) | ZiskProof::VadcopFinalCompressed(_) => {
                verify_zisk_proof_with_proving_key(
                    proof,
                    publics,
                    program_vk,
                    self.proving_key_path.clone(),
                )
            }
        }
    }
}
