use crate::create_debug_info;
use crate::BackendProverOpts;
use crate::{
    ExecuteOutput, ProveOutput, VerifyConstraintsOutput, ZiskAggPhaseResult, ZiskPhaseResult,
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
use proofman_verifier::VadcopFinalProof;
use recurser::prove::{
    prove_recurser_aggregator, register_recurser_setup, ProveRecurserAggregatorOptions,
    RegisteredRecurser,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use zisk_cluster_common::StreamMessage;
use zisk_common::io::StreamSource;
use zisk_common::stats_mark;
use zisk_common::ZiskExecutorTime;
use zisk_common::{io::ZiskStdin, ExecutorStatsHandle, ZiskExecutorSummary};
use zisk_common::{
    HashMode, PlonkVkBlob, PlonkVkey, ProgramVK, Proof, ProofBody, ProofKind, PublicValues,
};

pub(crate) struct ProverBackend {
    proofman: ProofMan<Goldilocks>,
    snark_wrapper: Option<SnarkWrapper<Goldilocks>>,
    executor: Arc<ZiskExecutor<Goldilocks>>,
    proving_key_path: PathBuf,
    proving_key_snark_path: Option<PathBuf>,
    /// Recurser setups registered with `proofman`, keyed by `recurser_id`.
    /// A recurser must be registered (via [`register_recurser`]) before it can
    /// prove — the same register-then-prove lifecycle as a regular program.
    registered_recursers: std::sync::Mutex<HashMap<String, RegisteredRecurser>>,
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
            proofman,
            snark_wrapper,
            executor,
            proving_key_path,
            proving_key_snark_path,
            registered_recursers: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        self.executor.asm_emulator()
    }

    pub(crate) fn set_asm_resources(&self, resources: Arc<AsmResources>) -> Result<()> {
        self.executor.set_asm_resources(resources).map_err(Into::into)
    }

    pub(crate) fn clear_asm_resources(&self) -> Result<()> {
        self.executor.clear_asm_resources().map_err(Into::into)
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
            .map_err(Into::into)
    }

    pub(crate) fn append_raw_input(&self, bytes: &[u8]) -> Result<()> {
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot append raw input")
            })?
            .append_raw_input(bytes)
            .map_err(Into::into)
    }

    pub(crate) fn register_hints_stream(&self, stream: StreamSource) -> Result<()> {
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot register hints stream")
            })?
            .set_hints_stream_src(stream)
            .map_err(|e| anyhow::anyhow!("Failed to set hints stream source: {}", e))
    }

    pub(crate) fn register_inputs_stream(&self, stream: StreamSource) -> Result<()> {
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot register inputs stream")
            })?
            .set_inputs_stream_src(stream)
            .map_err(|e| anyhow::anyhow!("Failed to set inputs stream source: {}", e))
    }

    pub(crate) fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        match self.asm_emulator() {
            Some(a) => a.get_hints_processor().map_err(Into::into),
            None => {
                Err(anyhow::anyhow!("ASM resources not initialized, cannot get hints processor"))
            }
        }
    }

    pub(crate) fn set_active_services(&self, is_first_process: bool) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.set_active_services(is_first_process)?;
        }
        Ok(())
    }

    pub(crate) fn reset(&self) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.reset()?;
        }
        Ok(())
    }

    pub(crate) fn cancel(&self) {
        self.proofman.cancel();
    }

    pub(crate) fn signal_cancellation(&self) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.signal_cancellation()?;
        }
        Ok(())
    }

    pub(crate) fn wait_until_proofman_ready(&self) {
        self.proofman.wait_until_proofman_ready();
    }

    pub fn get_pctx(&self) -> Result<Arc<ProofCtx<Goldilocks>>> {
        Ok(self.proofman.get_wcm().get_pctx())
    }

    /// Hash family the loaded proving key was generated with (e.g. "Poseidon1" / "Poseidon2").
    pub fn hash(&self) -> Result<String> {
        Ok(self.get_pctx()?.global_info.hash.clone())
    }

    /// Loaded proving key's hash family as a [`HashMode`]. Errors if the key's
    /// recorded hash is not a recognized mode.
    pub fn hash_mode(&self) -> Result<HashMode> {
        let hash = self.hash()?;
        hash.parse::<HashMode>().map_err(|e| {
            anyhow::anyhow!("proving key hash {hash:?} is not a recognized HashMode: {e}")
        })
    }

    pub fn register_program(
        &self,
        zisk_rom: Arc<zisk_core::ZiskRom>,
        rom_bin_path: &std::path::Path,
        with_hints: bool,
    ) -> Result<()> {
        self.executor.set_rom(zisk_rom, with_hints)?;

        let custom_commits_map = HashMap::from([("rom".to_string(), rom_bin_path.to_path_buf())]);
        self.proofman
            .register_custom_commits(custom_commits_map)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.executor.set_stdin(stdin).map_err(Into::into)
    }

    pub fn execution_result(&self) -> Result<(ZiskExecutorSummary, ExecutorStatsHandle)> {
        Ok(self.executor.get_execution_result())
    }

    pub(crate) fn execute(&self, stdin: ZiskStdin) -> Result<ExecuteOutput> {
        self.executor.set_stdin(stdin)?;

        let start = std::time::Instant::now();

        self.proofman
            .execute_from_lib(None)
            .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let elapsed = start.elapsed();

        let (result, _) = self.executor.get_execution_result();

        let publics = self.proofman.get_publics();

        Ok(ExecuteOutput::new(elapsed, result, &publics))
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
                ProofOptions::new(false, false, false, false, false, minimal_memory),
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
    ) -> Result<VerifyConstraintsOutput> {
        let start = std::time::Instant::now();

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        self.executor.set_stdin(stdin)?;

        self.proofman
            .verify_proof_constraints_from_lib(&debug_info)
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
        let elapsed = start.elapsed();

        let (result, _stats) = self.executor.get_execution_result();

        stats_mark!(_stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        _stats.store_stats();

        let publics = self.proofman.get_publics();

        Ok(VerifyConstraintsOutput::new(result, elapsed.as_millis() as u64, &publics))
    }

    pub(crate) fn prove(
        &self,
        stdin: ZiskStdin,
        proof_kind: ProofKind,
        prover_options: BackendProverOpts,
    ) -> Result<ProveOutput> {
        if proof_kind == ProofKind::Plonk && self.snark_wrapper.is_none() {
            return Err(anyhow::anyhow!(
                "Snark wrapper is not initialized. Cannot generate snark proof."
            ));
        }

        let start = std::time::Instant::now();

        self.executor.set_stdin(stdin)?;

        let minimal = matches!(proof_kind, ProofKind::VadcopFinalMinimal);

        self.proofman.set_partition(1, vec![0], 0)?;

        self.proofman.set_barrier();
        let proof = self
            .proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(),
                ProofOptions::new(
                    false,
                    prover_options.aggregation,
                    true,
                    minimal,
                    prover_options.verify_proofs,
                    prover_options.minimal_memory,
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let proof = match proof {
            ProvePhaseResult::Full(_, proof) => proof,
            _ => None,
        };

        let (execution_result, _stats) = self.executor.get_execution_result();

        // Store the stats in stats.json
        stats_mark!(_stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        _stats.store_stats();

        self.proofman.set_barrier();

        let vadcop_vk_u64 = self.get_vadcop_vk(minimal)?;

        match (proof_kind, proof) {
            (ProofKind::Plonk, Some(vadcop_proof)) => {
                let snark_proof = self
                    .snark_wrapper
                    .as_ref()
                    .unwrap()
                    .generate_final_snark_proof(&vadcop_proof)?;

                let publics = PublicValues::new_from_u64(&vadcop_proof.public_values);
                let program_vk = ProgramVK::new_from_publics_with_mode(
                    &vadcop_proof.public_values,
                    self.hash_mode()?,
                );
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
                    Ok(ProveOutput::new(
                        execution_result,
                        start.elapsed(),
                        Proof {
                            body: ProofBody::Plonk {
                                proof_bytes: snark_proof.proof_bytes,
                                plonk_vk: Box::new(PlonkVkBlob {
                                    vadcop_vk: vadcop_vk_u64,
                                    plonk_vkey,
                                }),
                                publics,
                            },
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
            (_, Some(p)) => Ok(ProveOutput::new(
                execution_result,
                start.elapsed(),
                Proof {
                    program_vk: ProgramVK::new_from_publics_with_mode(
                        &p.public_values,
                        self.hash_mode()?,
                    ),
                    body: ProofBody::Vadcop {
                        proof: p.proof,
                        zisk_vk: vadcop_vk_u64,
                        minimal,
                        hash: self.hash()?,
                        publics_full: p.public_values,
                    },
                },
            )),
            (_, None) => Ok(ProveOutput::new_null(execution_result, start.elapsed())),
        }
    }

    pub(crate) fn minimal(
        &self,
        proof: &[u64],
        publics: &PublicValues,
        program_vk: &ProgramVK,
    ) -> Result<ProveOutput> {
        let start = std::time::Instant::now();

        let hash = self.hash()?;
        let mut pubs_u64 = program_vk.vk.clone();
        pubs_u64.extend(publics.public_u64());
        let vadcop_final_proof =
            VadcopFinalProof::new(proof.to_vec(), pubs_u64, false, hash.clone());

        let minimal_proof = self
            .proofman
            .generate_vadcop_final_proof_compressed(&vadcop_final_proof)
            .map_err(|e| anyhow::anyhow!("Error generating minimal proof: {}", e))?;

        let time = start.elapsed();

        let proof = Proof {
            program_vk: ProgramVK::new_from_publics_with_mode(
                &minimal_proof.public_values,
                self.hash_mode()?,
            ),
            body: ProofBody::Vadcop {
                proof: minimal_proof.proof.clone(),
                zisk_vk: self.get_vadcop_vk(true)?,
                minimal: true,
                hash,
                publics_full: minimal_proof.public_values,
            },
        };

        Ok(ProveOutput::new(ZiskExecutorSummary::default(), time, proof))
    }

    pub(crate) fn plonk(
        &self,
        proof: &[u64],
        publics: &PublicValues,
        program_vk: &ProgramVK,
    ) -> Result<ProveOutput> {
        if self.snark_wrapper.is_none() {
            return Err(anyhow::anyhow!(
                "Snark wrapper is not initialized. Cannot generate snark proof."
            ));
        }

        let start = std::time::Instant::now();

        let mut pubs_u64 = program_vk.vk.clone();
        pubs_u64.extend(publics.public_u64());
        let vadcop_final_proof =
            VadcopFinalProof::new(proof.to_vec(), pubs_u64, false, self.hash()?);

        let snark_proof =
            self.snark_wrapper.as_ref().unwrap().generate_final_snark_proof(&vadcop_final_proof)?;

        let time = start.elapsed();

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

        let proof = Proof {
            body: ProofBody::Plonk {
                proof_bytes: snark_proof.proof_bytes.clone(),
                plonk_vk: Box::new(PlonkVkBlob {
                    vadcop_vk: self.get_vadcop_vk(false)?,
                    plonk_vkey,
                }),
                publics: PublicValues::new_from_u64(&vadcop_final_proof.public_values),
            },
            program_vk: ProgramVK::new_from_publics_with_mode(
                &vadcop_final_proof.public_values,
                self.hash_mode()?,
            ),
        };

        Ok(ProveOutput::new(ZiskExecutorSummary::default(), time, proof))
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

    pub(crate) fn register_worker_proofs(&self, agg_proofs: Vec<AggProofsRegister>) -> Result<()> {
        self.proofman
            .register_aggregated_proofs(agg_proofs)
            .map_err(|e| anyhow::anyhow!("Error registering aggregate proof: {}", e))
    }

    pub(crate) fn join_worker_proofs(
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

    /// Register a recurser setup with proofman so it can later prove. The same
    /// lifecycle as registering a program: do this once, then prove many times.
    /// Idempotent — re-registering an already-registered `recurser_id` is cheap.
    pub(crate) fn register_recurser(&self, output_dir: &str, recurser_id: &str) -> Result<()> {
        if self.registered_recursers.lock().unwrap().contains_key(recurser_id) {
            return Ok(());
        }
        let registered = register_recurser_setup(&self.proofman, output_dir, recurser_id)
            .map_err(|e| anyhow::anyhow!("recurser registration failed: {e:#}"))?;
        self.registered_recursers.lock().unwrap().insert(recurser_id.to_string(), registered);
        Ok(())
    }

    /// Fold two proofs through a previously-registered recurser. Errors if the
    /// recurser was not registered first via [`register_recurser`].
    pub(crate) fn prove_recurser(
        &self,
        recurser_id: &str,
        proof_a: &VadcopFinalProof,
        proof_b: &VadcopFinalProof,
        free_inputs_a: &[u64],
        free_inputs_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
    ) -> Result<VadcopFinalProof> {
        let registered =
            self.registered_recursers.lock().unwrap().get(recurser_id).cloned().ok_or_else(
                || {
                    anyhow::anyhow!(
                        "recurser '{recurser_id}' is not registered; call register_recurser first"
                    )
                },
            )?;

        let opts = ProveRecurserAggregatorOptions {
            registered: &registered,
            proof_a,
            proof_b,
            free_inputs_a,
            free_inputs_b,
            root_c_recurser_agg,
        };
        prove_recurser_aggregator(&self.proofman, &opts)
            .map_err(|e| anyhow::anyhow!("recurser proof generation failed: {e:#}"))
    }

    pub(crate) fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u64>> {
        Ok(get_vadcop_final_proof_vkey(&self.proving_key_path, minimal)?)
    }

    pub(crate) fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        self.proofman.mpi_broadcast(data);
        Ok(())
    }

    pub(crate) fn notify_cluster_cancellation(&self) {
        self.proofman.notify_cancellation();
    }

    pub(crate) fn cluster_barrier(&self) {
        self.proofman.set_barrier();
    }
}
