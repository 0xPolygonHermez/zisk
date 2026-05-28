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
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

// ZDIAG: hang-instrumentation - remove after diagnosis
static ZDIAG_BACKEND_SEQ: AtomicU64 = AtomicU64::new(0);
static ZDIAG_BACKEND_SET_BARRIER_SEQ: AtomicU64 = AtomicU64::new(0);
use zisk_cluster_common::StreamMessage;
use zisk_common::io::StreamSource;
use zisk_common::stats_mark;
use zisk_common::ZiskExecutorTime;
use zisk_common::{io::ZiskStdin, ExecutorStatsHandle, ZiskExecutorSummary};
use zisk_common::{PlonkVkBlob, PlonkVkey, ProgramVK, Proof, ProofBody, ProofKind, PublicValues};

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

    pub(crate) fn set_asm_resources(&self, resources: Arc<AsmResources>) -> Result<()> {
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

    pub(crate) fn append_raw_input(&self, bytes: &[u8]) -> Result<()> {
        self.asm_emulator()
            .ok_or_else(|| {
                anyhow::anyhow!("ASM resources not initialized, cannot append raw input")
            })?
            .append_raw_input(bytes)
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
            Some(a) => a.get_hints_processor(),
            None => {
                Err(anyhow::anyhow!("ASM resources not initialized, cannot get hints processor"))
            }
        }
    }

    pub(crate) fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.set_active_services(is_first_partition)?;
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
        self.executor.set_stdin(stdin)
    }

    pub fn execution_result(&self) -> Result<(ZiskExecutorSummary, ExecutorStatsHandle)> {
        Ok(self.executor.get_execution_result())
    }

    pub(crate) fn execute(&self, stdin: ZiskStdin) -> Result<ExecuteOutput> {
        let _zd_seq = ZDIAG_BACKEND_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        let _zd_start = std::time::Instant::now();
        eprintln!(
            "[ZDIAG BACKEND-EXECUTE-ENTER] seq={} pid={} tid={:?}",
            _zd_seq, std::process::id(), std::thread::current().id()
        );
        self.executor.set_stdin(stdin)?;

        let start = std::time::Instant::now();

        let _zd_pm_start = std::time::Instant::now();
        let pm_result = self.proofman.execute_from_lib(None);
        eprintln!(
            "[ZDIAG BACKEND-EXECUTE-PROOFMAN-DONE] seq={} pid={} tid={:?} elapsed_ms={} ok={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            _zd_pm_start.elapsed().as_millis(), pm_result.is_ok()
        );
        pm_result.map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

        let elapsed = start.elapsed();

        let (result, _) = self.executor.get_execution_result();

        let publics = self.proofman.get_publics();

        eprintln!(
            "[ZDIAG BACKEND-EXECUTE-EXIT] seq={} pid={} tid={:?} total_ms={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            _zd_start.elapsed().as_millis()
        );
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

        // ZDIAG: split_active_processes is a collective call; inactive ranks skip downstream collectives
        let _zd_seq = ZDIAG_BACKEND_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        eprintln!(
            "[ZDIAG BACKEND-STATS-SPLIT] seq={} pid={} tid={:?} world_rank={} local_rank={} is_active={} mpi_node_param={:?}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            rank_info.world_rank, rank_info.local_rank, is_active, _mpi_node
        );
        self.proofman.split_active_processes(is_active);

        if !is_active {
            eprintln!(
                "[ZDIAG BACKEND-STATS-INACTIVE-EARLY-RETURN] seq={} pid={} tid={:?} world_rank={}",
                _zd_seq, std::process::id(), std::thread::current().id(), rank_info.world_rank
            );
            println!(
                "{}: {}",
                format!("Rank {}", rank_info.local_rank).bright_yellow().bold(),
                "Inactive rank, skipping computation.".bright_yellow()
            );

            return Ok((rank_info.world_rank, rank_info.n_processes, None));
        }

        let _zd_t1 = std::time::Instant::now();
        let cw_result = self.proofman.compute_witness_from_lib(
            &debug_info,
            ProofOptions::new(false, false, false, false, false, minimal_memory),
        );
        eprintln!(
            "[ZDIAG BACKEND-STATS-CW-DONE] seq={} pid={} tid={:?} elapsed_ms={} ok={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            _zd_t1.elapsed().as_millis(), cw_result.is_ok()
        );
        cw_result.map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;

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
        let _zd_seq = ZDIAG_BACKEND_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        let _zd_outer_start = std::time::Instant::now();
        eprintln!(
            "[ZDIAG BACKEND-VERIFY-ENTER] seq={} pid={} tid={:?}",
            _zd_seq, std::process::id(), std::thread::current().id()
        );
        let start = std::time::Instant::now();

        let debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        self.executor.set_stdin(stdin)?;

        let _zd_pm_start = std::time::Instant::now();
        let pm_result = self.proofman.verify_proof_constraints_from_lib(&debug_info);
        eprintln!(
            "[ZDIAG BACKEND-VERIFY-PROOFMAN-DONE] seq={} pid={} tid={:?} elapsed_ms={} ok={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            _zd_pm_start.elapsed().as_millis(), pm_result.is_ok()
        );
        pm_result.map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
        let elapsed = start.elapsed();
        eprintln!(
            "[ZDIAG BACKEND-VERIFY-EXIT] seq={} pid={} tid={:?} total_ms={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            _zd_outer_start.elapsed().as_millis()
        );

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

        // ZDIAG: barrier 1 of 2 in prove() — collective
        let _zd_bseq = ZDIAG_BACKEND_SET_BARRIER_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        let _zd_bstart = std::time::Instant::now();
        eprintln!(
            "[ZDIAG BACKEND-PROVE-BARRIER1-ENTER] bseq={} pid={} tid={:?}",
            _zd_bseq, std::process::id(), std::thread::current().id()
        );
        self.proofman.set_barrier();
        eprintln!(
            "[ZDIAG BACKEND-PROVE-BARRIER1-EXIT] bseq={} pid={} tid={:?} elapsed_ms={}",
            _zd_bseq, std::process::id(), std::thread::current().id(),
            _zd_bstart.elapsed().as_millis()
        );

        let _zd_pseq = ZDIAG_BACKEND_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        let _zd_pstart = std::time::Instant::now();
        eprintln!(
            "[ZDIAG BACKEND-PROVE-PROOFMAN-ENTER] seq={} pid={} tid={:?}",
            _zd_pseq, std::process::id(), std::thread::current().id()
        );
        let proof_result = self.proofman.generate_proof_from_lib(
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
        );
        eprintln!(
            "[ZDIAG BACKEND-PROVE-PROOFMAN-EXIT] seq={} pid={} tid={:?} elapsed_ms={} ok={}",
            _zd_pseq, std::process::id(), std::thread::current().id(),
            _zd_pstart.elapsed().as_millis(), proof_result.is_ok()
        );
        let proof = proof_result.map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

        let proof = match proof {
            ProvePhaseResult::Full(_, proof) => proof,
            _ => None,
        };

        let (execution_result, _stats) = self.executor.get_execution_result();

        // Store the stats in stats.json
        stats_mark!(_stats, 0, "END", 0);

        #[cfg(feature = "stats")]
        _stats.store_stats();

        // ZDIAG: barrier 2 of 2 in prove() — collective
        let _zd_bseq2 = ZDIAG_BACKEND_SET_BARRIER_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        let _zd_bstart2 = std::time::Instant::now();
        eprintln!(
            "[ZDIAG BACKEND-PROVE-BARRIER2-ENTER] bseq={} pid={} tid={:?}",
            _zd_bseq2, std::process::id(), std::thread::current().id()
        );
        self.proofman.set_barrier();
        eprintln!(
            "[ZDIAG BACKEND-PROVE-BARRIER2-EXIT] bseq={} pid={} tid={:?} elapsed_ms={}",
            _zd_bseq2, std::process::id(), std::thread::current().id(),
            _zd_bstart2.elapsed().as_millis()
        );

        let vadcop_vk_u64 = self.get_vadcop_vk(minimal)?;

        match (proof_kind, proof) {
            (ProofKind::Plonk, Some(vadcop_proof)) => {
                let snark_proof = self
                    .snark_wrapper
                    .as_ref()
                    .unwrap()
                    .generate_final_snark_proof(&vadcop_proof)?;

                let publics = PublicValues::new_from_u64(&vadcop_proof.public_values);
                let program_vk = ProgramVK::new_from_publics(&vadcop_proof.public_values);
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
                            },
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
            (_, Some(p)) => Ok(ProveOutput::new(
                execution_result,
                start.elapsed(),
                Proof {
                    body: ProofBody::Vadcop { proof: p.proof, zisk_vk: vadcop_vk_u64, minimal },
                    publics: PublicValues::new_from_u64(&p.public_values),
                    program_vk: ProgramVK::new_from_publics(&p.public_values),
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

        let mut pubs_u64 = program_vk.vk.clone();
        pubs_u64.extend(publics.public_u64());
        let vadcop_final_proof = VadcopFinalProof::new(proof.to_vec(), pubs_u64, false);

        let minimal_proof = self
            .proofman
            .generate_vadcop_final_proof_compressed(&vadcop_final_proof)
            .map_err(|e| anyhow::anyhow!("Error generating minimal proof: {}", e))?;

        let time = start.elapsed();

        let proof = Proof {
            body: ProofBody::Vadcop {
                proof: minimal_proof.proof.clone(),
                zisk_vk: self.get_vadcop_vk(true)?,
                minimal: true,
            },
            publics: PublicValues::new_from_u64(&minimal_proof.public_values),
            program_vk: ProgramVK::new_from_publics(&minimal_proof.public_values),
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
        let vadcop_final_proof = VadcopFinalProof::new(proof.to_vec(), pubs_u64, false);

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
            },
            publics: PublicValues::new_from_u64(&vadcop_final_proof.public_values),
            program_vk: ProgramVK::new_from_publics(&vadcop_final_proof.public_values),
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

    pub(crate) fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u64>> {
        Ok(get_vadcop_final_proof_vkey(&self.proving_key_path, minimal)?)
    }

    pub(crate) fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        // ZDIAG: per-call timing of the actual MPI call (P2P fan-out underneath)
        let _zd_start = std::time::Instant::now();
        let _zd_in_size = data.len();
        self.proofman.mpi_broadcast(data);
        let _zd_ms = _zd_start.elapsed().as_millis();
        if _zd_ms > 500 {
            eprintln!(
                "[ZDIAG BACKEND-MPI-BCAST-SLOW] pid={} tid={:?} in_size={} out_size={} elapsed_ms={}",
                std::process::id(), std::thread::current().id(),
                _zd_in_size, data.len(), _zd_ms
            );
        }
        Ok(())
    }

    pub(crate) fn notify_cluster_cancellation(&self) {
        eprintln!(
            "[ZDIAG BACKEND-NOTIFY-CANCEL] pid={} tid={:?}",
            std::process::id(), std::thread::current().id()
        );
        self.proofman.notify_cancellation();
    }

    pub(crate) fn cluster_barrier(&self) {
        let _zd_start = std::time::Instant::now();
        eprintln!(
            "[ZDIAG BACKEND-CLUSTER-BARRIER-ENTER] pid={} tid={:?}",
            std::process::id(), std::thread::current().id()
        );
        self.proofman.set_barrier();
        eprintln!(
            "[ZDIAG BACKEND-CLUSTER-BARRIER-EXIT] pid={} tid={:?} elapsed_ms={}",
            std::process::id(), std::thread::current().id(),
            _zd_start.elapsed().as_millis()
        );
    }
}
