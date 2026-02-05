use crate::create_debug_info;
use crate::ZiskPublics;
use crate::{ProofMode, ProofOpts};
use crate::{
    ZiskAggPhaseResult, ZiskExecuteResult, ZiskPhaseResult, ZiskProgramVK, ZiskProof,
    ZiskProveResult, ZiskVerifyConstraintsResult,
};
use anyhow::Result;
use colored::Colorize;
use fields::Goldilocks;
use proofman::{
    get_vadcop_final_proof_vkey, verify_snark_proof, AggProofs, ProofInfo, ProofMan, ProvePhase,
    ProvePhaseInputs, ProvePhaseResult, SnarkProof, SnarkProtocol, SnarkWrapper,
};
use proofman_common::{ProofCtx, ProofOptions};
use proofman_util::VadcopFinalProof;
use rom_setup::rom_merkle_setup_verkey;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    ElfBinaryLike, ExecutorStatsHandle, ZiskExecutionResult,
};
use zisk_verifier::verify_zisk_proof;
use zisk_witness::WitnessLib;

pub(crate) struct ProverBackend {
    proofman: Option<ProofMan<Goldilocks>>,
    snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
    witness_lib: OnceLock<WitnessLib<Goldilocks>>,
    proving_key_path: PathBuf,
    proving_key_snark_path: Option<PathBuf>,
}

impl ProverBackend {
    pub fn new(
        proofman: ProofMan<Goldilocks>,
        snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
        proving_key_path: PathBuf,
        proving_key_snark_path: Option<PathBuf>,
    ) -> Self {
        Self {
            proofman: Some(proofman),
            snark_wrapper,
            witness_lib: OnceLock::new(),
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
            witness_lib: OnceLock::new(),
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

    pub fn register_witness_lib(
        &self,
        elf: &[u8],
        mut witness_lib: WitnessLib<Goldilocks>,
        custom_commits_map: HashMap<String, PathBuf>,
    ) -> Result<()> {
        let proofman = self.proofman.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Proofman is not initialized. Please initialize it before use.")
        })?;

        witness_lib.register_witness(elf, &proofman.get_wcm())?;

        if self.witness_lib.set(witness_lib).is_err() {
            return Err(anyhow::anyhow!("Witness library has already been registered."));
        }

        proofman
            .register_custom_commits(custom_commits_map)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("Witness_lib is not initialized. Please initialize it before use.")
        })?;
        witness_lib.set_stdin(stdin);
        Ok(())
    }

    pub fn set_hints_stream(&self, hints_stream: StreamSource) -> Result<()> {
        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("Witness_lib is not initialized. Please initialize it before use.")
        })?;
        witness_lib.set_hints_stream(hints_stream)
    }

    pub fn execution_result(&self) -> Result<(ZiskExecutionResult, ExecutorStatsHandle)> {
        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("Witness_lib is not initialized. Please initialize it before use.")
        })?;

        let (result, stats) = witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        Ok((result, stats))
    }

    pub(crate) fn execute(
        &self,
        stdin: ZiskStdin,
        output_path: Option<PathBuf>,
    ) -> Result<ZiskExecuteResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot execute in verifier mode"))?;

        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("witness_lib is not initialized. Please initialize it before use.")
        })?;

        witness_lib.set_stdin(stdin);

        let start = std::time::Instant::now();

        proofman
            .execute_from_lib(output_path)
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let elapsed = start.elapsed();

        let (result, _) = witness_lib.execution_result().ok_or_else(|| {
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
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot compute stats in verifier mode"))?;

        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("witness_lib is not initialized. Please initialize it before use.")
        })?;

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        witness_lib.set_stdin(stdin);

        let world_rank = proofman.get_world_rank();
        let local_rank = proofman.get_local_rank();
        let n_processes = proofman.get_n_processes();

        let mut is_active = true;

        if let Some(mpi_node) = _mpi_node {
            if local_rank != mpi_node as i32 {
                is_active = false;
            }
        }

        proofman.split_active_processes(is_active);

        if !is_active {
            println!(
                "{}: {}",
                format!("Rank {local_rank}").bright_yellow().bold(),
                "Inactive rank, skipping computation.".bright_yellow()
            );

            return Ok((world_rank, n_processes, None));
        }

        proofman
            .compute_witness_from_lib(
                &debug_info,
                ProofOptions::new(false, false, false, false, false, minimal_memory, false, None),
            )
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let (_, stats): (ZiskExecutionResult, ExecutorStatsHandle) =
            witness_lib.execution_result().ok_or_else(|| {
                anyhow::anyhow!("Failed to get execution result from emulator prover")
            })?;

        Ok((world_rank, n_processes, Some(stats)))
    }

    pub(crate) fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot verify constraints in verifier mode"))?;

        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("witness_lib is not initialized. Please initialize it before use.")
        })?;

        let start = std::time::Instant::now();

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        witness_lib.set_stdin(stdin);

        proofman
            .verify_proof_constraints_from_lib(&debug_info, false)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
        let elapsed = start.elapsed();

        let (result, stats) = witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = stats.get_inner().lock().unwrap().next_id();
            stats.get_inner().lock().unwrap().add_stat(
                0,
                stats_id,
                "END",
                0,
                ExecutorStatsEvent::Mark,
            );
            stats.get_inner().lock().unwrap().store_stats();
        }

        Ok(ZiskVerifyConstraintsResult { execution: result, duration: elapsed, stats })
    }

    pub(crate) fn verify_constraints(
        &self,
        stdin: ZiskStdin,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.verify_constraints_debug(stdin, None)
    }

    pub(crate) fn vk(&self, elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
        let proving_key_path = self.proving_key_path.clone();

        let vk = rom_merkle_setup_verkey(elf, &None, proving_key_path.as_path())?;

        Ok(ZiskProgramVK { vk })
    }

    pub(crate) fn prove_debug(
        &self,
        stdin: ZiskStdin,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot prove in verifier mode"))?;

        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("witness_lib is not initialized. Please initialize it before use.")
        })?;

        let start = std::time::Instant::now();

        witness_lib.set_stdin(stdin);

        proofman.set_barrier();
        proofman
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
                    proof_options.output_dir_path.clone(),
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let elapsed = start.elapsed();

        let (execution_result, stats) = witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.lock().unwrap().get_id();
            _stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.lock().unwrap().store_stats();
        }

        proofman.set_barrier();

        Ok(ZiskProveResult::new_null(execution_result, elapsed, stats))
    }

    pub(crate) fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult> {
        let proofman = self
            .proofman
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot prove in verifier mode"))?;

        let witness_lib = self.witness_lib.get().ok_or_else(|| {
            anyhow::anyhow!("witness_lib is not initialized. Please initialize it before use.")
        })?;

        if mode == ProofMode::Snark && self.snark_wrapper.is_none() {
            return Err(anyhow::anyhow!(
                "Snark wrapper is not initialized. Cannot generate snark proof."
            ));
        }

        let start = std::time::Instant::now();

        witness_lib.set_stdin(stdin);

        let compressed = matches!(mode, ProofMode::VadcopFinalCompressed);

        proofman.set_barrier();
        let proof = proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(None, 1, vec![0], 0)),
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

        let elapsed = start.elapsed();

        let (proof_id, proof) = match proof {
            ProvePhaseResult::Full(proof_id, proof) => (proof_id, proof),
            _ => (None, None),
        };

        let (execution_result, stats) = witness_lib.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = stats.get_inner().lock().unwrap().next_id();
            stats.get_inner().lock().unwrap().add_stat(
                0,
                stats_id,
                "END",
                0,
                ExecutorStatsEvent::Mark,
            );
            stats.get_inner().lock().unwrap().store_stats();
        }

        proofman.set_barrier();

        match (mode, proof) {
            (ProofMode::Snark, Some(vadcop_proof)) => {
                let snark_proof = self.snark_wrapper.as_ref().unwrap().generate_final_snark_proof(
                    &vadcop_proof,
                    proof_options.output_dir_path.clone(),
                )?;

                if snark_proof.protocol_id == SnarkProtocol::Plonk.protocol_id() {
                    let publics = ZiskPublics::new(vadcop_proof.public_values);
                    Ok(ZiskProveResult::new(
                        execution_result,
                        elapsed,
                        stats,
                        proof_id,
                        ZiskProof::Plonk(snark_proof.proof_bytes),
                        publics,
                    ))
                } else if snark_proof.protocol_id == SnarkProtocol::Fflonk.protocol_id() {
                    let publics = ZiskPublics::new(vadcop_proof.public_values);
                    Ok(ZiskProveResult::new(
                        execution_result,
                        elapsed,
                        stats,
                        proof_id,
                        ZiskProof::Fflonk(snark_proof.proof_bytes),
                        publics,
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
                    elapsed,
                    stats,
                    proof_id,
                    proof,
                    ZiskPublics::new(p.public_values),
                ))
            }
            (_, None) => Ok(ZiskProveResult::new_null(execution_result, elapsed, stats)),
        }
    }

    pub(crate) fn prove_snark(
        &self,
        proof: &ZiskProof,
        publics: &ZiskPublics,
        program_vk: &ZiskProgramVK,
    ) -> Result<ZiskProof> {
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
            Ok(ZiskProof::Plonk(snark_proof.proof_bytes))
        } else if snark_proof.protocol_id == SnarkProtocol::Fflonk.protocol_id() {
            Ok(ZiskProof::Fflonk(snark_proof.proof_bytes))
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
            ZiskProof::Plonk(proof_bytes) | ZiskProof::Fflonk(proof_bytes) => {
                let protocol_id = if let ZiskProof::Plonk(_) = &proof {
                    SnarkProtocol::Plonk.protocol_id()
                } else {
                    SnarkProtocol::Fflonk.protocol_id()
                };

                let verkey = get_vadcop_final_proof_vkey(&self.proving_key_path, false)?;

                let pubs = publics.bytes_solidity(program_vk, &verkey);
                let hash = Sha256::digest(&pubs).to_vec();

                let snark_proof = SnarkProof {
                    proof_bytes: proof_bytes.clone(),
                    public_bytes: pubs,
                    public_snark_bytes: hash,
                    protocol_id,
                };

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
                Ok(verify_snark_proof(&snark_proof, &verkey_path)?)
            }
            ZiskProof::VadcopFinal(proof_bytes) | ZiskProof::VadcopFinalCompressed(proof_bytes) => {
                let compressed = matches!(proof, ZiskProof::VadcopFinalCompressed(_));
                let mut pubs = program_vk.vk.clone();
                pubs.extend(publics.public_bytes());
                let vadcop_final_proof =
                    VadcopFinalProof::new(proof_bytes.clone(), pubs, compressed);

                let vk = get_vadcop_final_proof_vkey(&self.proving_key_path, compressed)?;
                verify_zisk_proof(&vadcop_final_proof, &vk)
            }
        }
    }
}
