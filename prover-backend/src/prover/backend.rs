use crate::create_debug_info;
use crate::GuestProgram;
use crate::ProofOpts;
use crate::{
    ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};
use anyhow::Result;
use asm_runner::HintsShmem;
use colored::Colorize;
use executor::{AsmResources, EmulatorAsm, ZiskExecutor};
use fields::Goldilocks;
use precompiles_hints::HintsProcessor;
use proofman::get_vadcop_final_proof_vkey;
use proofman::{
    AggProofs, AggProofsRegister, ProofMan, ProvePhase, ProvePhaseInputs, ProvePhaseResult,
    SnarkProtocol, SnarkWrapper, WitnessInfo,
};
use proofman_common::{ProofCtx, ProofOptions, RowInfo};
use proofman_util::VadcopFinalProof;
use rom_setup::rom_merkle_setup_verkey;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use zisk_common::io::StreamSource;
use zisk_common::stats_mark;
use zisk_common::ZiskExecutorTime;
use zisk_common::{io::ZiskStdin, ExecutorStatsHandle, ZiskExecutorSummary};
use zisk_common::{
    PlonkVkey, ProofMode, ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues, ZiskPublics, ZiskVK,
};
use zisk_distributed_common::StreamMessage;

pub(crate) struct ProverBackend {
    proofman: ProofMan<Goldilocks>,
    snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
    executor: Arc<ZiskExecutor<Goldilocks>>,
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
        Self { proofman, snark_wrapper, executor, proving_key_path, proving_key_snark_path }
    }

    fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        self.executor.asm_emulator()
    }

    pub(crate) fn set_asm_resources(&self, resources: AsmResources) -> Result<()> {
        self.executor.set_asm_resources(resources)
    }

    pub(crate) fn submit_hint(&self, bytes: &[u8]) -> Result<()> {
        let message: StreamMessage = borsh::from_slice(&bytes[1..])
            .map_err(|e| anyhow::anyhow!("Failed to deserialize hint StreamMessage: {}", e))?;
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot submit hint data")
            })?
            .submit_hint_direct(&message.data)
            .map_err(|e| anyhow::anyhow!("Failed to submit hint data: {}", e))
    }

    pub(crate) fn submit_input(&self, bytes: &[u8]) -> Result<()> {
        let message: StreamMessage = borsh::from_slice(&bytes[1..])
            .map_err(|e| anyhow::anyhow!("Failed to deserialize input StreamMessage: {}", e))?;
        // SAFETY: Vec<u64> is heap-allocated with 8-byte alignment. Viewing its bytes as
        // &[u8] is valid because u8 has no alignment requirement, the pointer arithmetic
        // is in-bounds, and the lifetime is tied to `message` which outlives this scope.
        let reinterpreted_data = unsafe {
            std::slice::from_raw_parts(
                message.data.as_ptr() as *const u8,
                message.data.len() * std::mem::size_of::<u64>(),
            )
        };
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot append input data")
            })?
            .append_raw_input(reinterpreted_data)
    }

    pub(crate) fn register_hints_stream(&self, stream: StreamSource) -> Result<()> {
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot register hints stream")
            })?
            .set_hints_stream_src(stream)
            .map_err(|e| anyhow::anyhow!("Failed to set hints stream source: {}", e))
    }

    pub(crate) fn get_hints_processor(&self) -> Result<Option<Arc<HintsProcessor<HintsShmem>>>> {
        match self.asm_emulator() {
            Some(a) => a.get_hints_processor(),
            None => Ok(None),
        }
    }

    pub(crate) fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.set_active_services(is_first_partition)?;
        }
        Ok(())
    }

    pub(crate) fn reset_resources(&self) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.reset()?;
        }
        Ok(())
    }

    pub(crate) fn cancel(&self) {
        self.proofman.cancel();
    }

    pub fn get_pctx(&self) -> Result<Arc<ProofCtx<Goldilocks>>> {
        Ok(self.proofman.get_wcm().get_pctx())
    }

    pub fn register_program(
        &self,
        zisk_rom: Arc<zisk_core::ZiskRom>,
        rom_bin_path: &std::path::Path,
    ) -> Result<()> {
        let use_hints =
            self.executor.asm_emulator().map(|a| a.use_hints()).transpose()?.unwrap_or(false);

        self.executor.set_rom(zisk_rom, use_hints)?;

        let custom_commits_map = HashMap::from([("rom".to_string(), rom_bin_path.to_path_buf())]);
        self.proofman
            .register_custom_commits(custom_commits_map)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.executor.set_stdin(stdin)
    }

    pub fn execution_result(&self) -> Result<(ZiskExecutorSummary, ExecutorStatsHandle)> {
        Ok(self.executor.get_execution_result())
    }

    pub(crate) fn execute(
        &self,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        self.executor.set_stdin(stdin)?;

        let start = std::time::Instant::now();

        let planning_info = self
            .proofman
            .execute_from_lib(output_path)
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let elapsed = start.elapsed();

        let (result, _) = self.executor.get_execution_result();

        let publics = self.proofman.get_publics();

        Ok(ZiskExecuteResult::new(elapsed, result, planning_info, &publics))
    }

    pub(crate) fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        _mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        self.executor.set_stdin(stdin)?;

        self.executor.set_packed(self.proofman.get_options().packed);

        let rank_info = self.proofman.get_rank_info();

        let mut is_active = true;

        if let Some(mpi_node) = _mpi_node {
            if rank_info.local_rank != mpi_node as i32 {
                is_active = false;
            }
        }

        self.proofman.split_active_processes(is_active);

        if !is_active {
            println!(
                "{}: {}",
                format!("Rank {}", rank_info.local_rank).bright_yellow().bold(),
                "Inactive rank, skipping computation.".bright_yellow()
            );

            return Ok((rank_info.world_rank, rank_info.n_processes, None));
        }

        self.proofman
            .compute_witness_from_lib(
                &debug_info,
                ProofOptions::new(false, false, false, false, false, minimal_memory, None),
            )
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let (_, stats): (ZiskExecutorSummary, ExecutorStatsHandle) =
            self.executor.get_execution_result();

        Ok((rank_info.world_rank, rank_info.n_processes, Some(stats)))
    }

    pub(crate) fn get_instance_trace(
        &self,
        instance_id: usize,
        first_row: usize,
        num_rows: usize,
        offset: Option<usize>,
    ) -> Result<Vec<RowInfo>> {
        self.proofman
            .get_instance_trace(instance_id, first_row, num_rows, offset)
            .map_err(|e| anyhow::anyhow!("Error getting instance trace: {}", e))
    }

    pub(crate) fn get_instance_air_values(&self, instance_id: usize) -> Result<Vec<u64>> {
        self.proofman
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
        self.proofman
            .get_instance_fixed(instance_id, first_row, num_rows, offset)
            .map_err(|e| anyhow::anyhow!("Error getting instance fixed: {}", e))
    }

    pub(crate) fn verify_constraints(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let start = std::time::Instant::now();

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        self.executor.set_stdin(stdin)?;

        self.executor.set_packed(self.proofman.get_options().packed);

        self.proofman
            .verify_proof_constraints_from_lib(&debug_info)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
        let elapsed = start.elapsed();

        let (result, stats) = self.executor.get_execution_result();

        stats_mark!(stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        stats.store_stats();

        let publics = self.proofman.get_publics();

        Ok(ZiskVerifyConstraintsResult::new(result, elapsed, stats, &publics))
    }

    pub(crate) fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK> {
        let vk = rom_merkle_setup_verkey(
            elf.elf(),
            &None,
            &self.proving_key_path,
            self.proofman.get_options().gpu,
        )?;
        Ok(ZiskProgramVK { vk })
    }

    pub(crate) fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        if mode == ProofMode::Snark && self.snark_wrapper.is_none() {
            return Err(anyhow::anyhow!(
                "Snark wrapper is not initialized. Cannot generate snark proof."
            ));
        }

        let start = std::time::Instant::now();

        self.executor.set_stdin(stdin)?;

        self.executor.set_packed(self.proofman.get_options().packed);

        let minimal = matches!(mode, ProofMode::VadcopFinalMinimal);

        self.proofman.set_partition(1, vec![0], 0)?;

        self.proofman.set_barrier();
        let proof = self
            .proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(),
                ProofOptions::new(
                    false,
                    proof_options.aggregation,
                    proof_options.rma,
                    minimal,
                    proof_options.verify_proofs,
                    proof_options.minimal_memory,
                    proof_options.output_dir_path.clone(),
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let (proof_id, proof) = match proof {
            ProvePhaseResult::Full(proof_id, proof) => (proof_id, proof),
            _ => (None, None),
        };

        let (execution_result, stats) = self.executor.get_execution_result();

        // Store the stats in stats.json
        stats_mark!(stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        stats.store_stats();

        self.proofman.set_barrier();

        let zisk_vk = ZiskVK { vk: get_vadcop_final_proof_vkey(&self.proving_key_path, minimal)? };

        match (mode, proof) {
            (ProofMode::Snark, Some(vadcop_proof)) => {
                let snark_proof = self.snark_wrapper.as_ref().unwrap().generate_final_snark_proof(
                    &vadcop_proof,
                    proof_options.output_dir_path.clone(),
                )?;

                let publics = ZiskPublics::new(&vadcop_proof.public_values);
                let program_vk = ZiskProgramVK::new_from_publics(&vadcop_proof.public_values);
                if snark_proof.protocol_id == SnarkProtocol::Plonk.protocol_id() {
                    let proving_key_snark =
                        self.proving_key_snark_path.as_ref().ok_or_else(|| {
                            anyhow::anyhow!("Proving key snark path is required for Plonk proofs")
                        })?;
                    let verkey_path = PathBuf::from(format!(
                        "{}/{}/{}.verkey.json",
                        proving_key_snark.display(),
                        "final",
                        "final"
                    ));
                    let plonk_vkey = PlonkVkey::load(&verkey_path)?;
                    Ok(ZiskProveResult::new(
                        execution_result,
                        start.elapsed(),
                        stats,
                        proof_id,
                        ZiskProofWithPublicValues {
                            proof: ZiskProof::Plonk(snark_proof.proof_bytes),
                            publics,
                            program_vk,
                            zisk_vk,
                            plonk_vkey: Some(plonk_vkey),
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
                let proof = if minimal {
                    ZiskProof::VadcopFinalMinimal(p.proof)
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
                        zisk_vk,
                        plonk_vkey: None,
                    },
                ))
            }
            (_, None) => Ok(ZiskProveResult::new_null(execution_result, start.elapsed(), stats)),
        }
    }

    pub(crate) fn minimal(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        program_vk: &ZiskProgramVK,
    ) -> Result<ZiskProofWithPublicValues> {
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

        let minimal_proof = self
            .proofman
            .generate_vadcop_final_proof_compressed(&vadcop_final_proof, None)
            .map_err(|e| anyhow::anyhow!("Error generating minimal proof: {}", e))?;

        Ok(ZiskProofWithPublicValues {
            proof: ZiskProof::VadcopFinalMinimal(minimal_proof.proof),
            publics: ZiskPublics::new(&minimal_proof.public_values),
            program_vk: ZiskProgramVK::new_from_publics(&minimal_proof.public_values),
            zisk_vk: ZiskVK { vk: get_vadcop_final_proof_vkey(&self.proving_key_path, true)? },
            plonk_vkey: None,
        })
    }

    pub(crate) fn plonk(
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

        let proving_key_snark = self.proving_key_snark_path.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Proving key snark path is required for Plonk proofs")
        })?;
        let verkey_path = PathBuf::from(format!(
            "{}/{}/{}.verkey.json",
            proving_key_snark.display(),
            "final",
            "final"
        ));
        let plonk_vkey = PlonkVkey::load(&verkey_path)?;

        let zisk_vk = ZiskVK { vk: get_vadcop_final_proof_vkey(&self.proving_key_path, false)? };

        if snark_proof.protocol_id == SnarkProtocol::Plonk.protocol_id() {
            Ok(ZiskProofWithPublicValues {
                proof: ZiskProof::Plonk(snark_proof.proof_bytes),
                publics: publics.clone(),
                program_vk: program_vk.clone(),
                zisk_vk,
                plonk_vkey: Some(plonk_vkey),
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
        self.proofman
            .generate_proof_from_lib(phase_inputs, options, phase.clone())
            .map_err(|e| anyhow::anyhow!("Error generating proof in phase {:?}: {}", phase, e))
    }

    pub(crate) fn set_partition(
        &self,
        total_compute_units: usize,
        allocation: Vec<u32>,
        rank_id: usize,
    ) -> Result<()> {
        Ok(self.proofman.set_partition(total_compute_units, allocation, rank_id)?)
    }

    pub(crate) fn get_execution_info(&self) -> Result<(WitnessInfo, ZiskExecutorTime)> {
        let witness_info = self.proofman.get_witness_info();
        let (execution_result, _) = self.executor.get_execution_result();
        Ok((witness_info, execution_result.executor_time))
    }

    pub(crate) fn register_aggregated_proofs(
        &self,
        agg_proofs: Vec<AggProofsRegister>,
    ) -> Result<()> {
        self.proofman
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
        let result = self
            .proofman
            .receive_aggregated_proofs(agg_proofs, last_proof, final_proof, options)
            .map_err(|e| anyhow::anyhow!("Error aggregating proofs: {}", e))?;

        Ok(result.map(|agg| ZiskAggPhaseResult { agg_proofs: agg }))
    }

    pub(crate) fn get_vadcop_vk(&self, minimal: bool) -> Result<ZiskVK> {
        let vk = get_vadcop_final_proof_vkey(&self.proving_key_path, minimal)?;
        Ok(ZiskVK { vk })
    }

    pub(crate) fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        self.proofman.mpi_broadcast(data);
        Ok(())
    }
}
