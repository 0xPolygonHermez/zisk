use anyhow::Result;
use asm_runner::AsmRunnerOptions;
use asm_runner::AsmServices;
use consensus_common::JobId;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::AggProofs;
use proofman::{ProofInfo, ProofMan};
use proofman_common::ProofOptions;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};
use witness::WitnessLibrary;

use zisk_common::ZiskLibInitFn;

use crate::prover_service::{ComputationResult, JobContext, ProverServiceConfig};

pub struct ProofGenerator {
    // It is important to keep the witness_lib declaration before the proofman declaration
    // to ensure that the witness library is dropped before the proofman.
    witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
    proofman: Arc<ProofMan<Goldilocks>>,
    asm_services: Option<AsmServices>,
}

impl ProofGenerator {
    pub fn new(config: &ProverServiceConfig) -> Result<Self> {
        info!("Starting asm microservices...");

        let proofman = ProofMan::<Goldilocks>::new(
            config.proving_key.clone(),
            config.custom_commits_map.clone(),
            config.verify_constraints,
            config.aggregation,
            config.final_snark,
            config.gpu_params.clone(),
            config.verbose.into(),
        )
        .expect("Failed to initialize proofman");

        let mpi_ctx = proofman.get_mpi_ctx();

        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(config.verbose > 0)
            .with_base_port(config.asm_port)
            .with_world_rank(mpi_ctx.rank)
            .with_local_rank(mpi_ctx.node_rank)
            .with_unlock_mapped_memory(config.unlock_mapped_memory);

        let world_rank = mpi_ctx.rank;
        let local_rank = mpi_ctx.node_rank;
        let base_port = config.asm_port;
        let unlock_mapped_memory = config.unlock_mapped_memory;

        let asm_services = if config.emulator {
            None
        } else {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services
                .start_asm_services(config.asm.as_ref().unwrap(), asm_runner_options.clone())?;
            Some(asm_services)
        };

        let library =
            unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };

        let mut witness_lib = witness_lib_constructor(
            config.verbose.into(),
            config.elf.clone(),
            config.asm.clone(),
            config.asm_rom.clone(),
            config.chunk_size_bits,
            Some(world_rank),
            Some(local_rank),
            base_port,
            unlock_mapped_memory,
            config.shared_tables,
        )
        .expect("Failed to initialize witness library");

        proofman.register_witness(witness_lib.as_mut(), library);

        let witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync> = Arc::from(witness_lib);

        Ok(Self { witness_lib, proofman: Arc::new(proofman), asm_services })
    }

    pub async fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        let job_id = job.lock().await.job_id.clone();

        tokio::spawn(async move {
            let result = Self::compute_phase1_task(job_id.clone(), job, proofman).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Phase1 {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Phase 1 computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Phase1 {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn compute_phase1_task(
        job_id: JobId,
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
    ) -> Result<Vec<u64>> {
        info!("Computing Phase 1 for job {}", job_id);

        // Prepare parameters
        let job = job.lock().await;
        let proof_info = ProofInfo::new(
            Some(job.block.input_path.clone()),
            job.total_compute_units as usize,
            job.allocation.clone(),
        );
        let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

        let options = ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            minimal_memory: true,
        };
        let phase = proofman::ProvePhase::Contributions;

        // Handle the result immediately without holding it across await
        let challenge = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Contributions(challenge)) => {
                info!("Phase 1 computation successful for job {}", job_id);
                challenge
            }
            Ok(_) => {
                error!("Error during Phase 1 computation for job {}", job_id);
                return Err(anyhow::anyhow!("Unexpected result type during Phase 1 computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for job {}: {:?}", job_id, err);
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        println!("Phase 1 challenge: {:?}", challenge);

        Ok(challenge.to_vec())
    }

    pub async fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<Vec<u64>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        // TODO!!!!!!! Challenges must arrive in Vec<[u64;10]>
        let challenges: Vec<[u64; 10]> = challenges
            .into_iter()
            .map(|v| v.try_into().expect("Each challenge must have exactly 10 elements"))
            .collect();

        let job_id = job.lock().await.job_id.clone();

        tokio::spawn(async move {
            let result = Self::execute_phase2(job, proofman, challenges).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Phase2 {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Phase 2 computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Phase2 {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn execute_phase2(
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
        challenges: Vec<[u64; 10]>,
    ) -> Result<Vec<AggProofs>> {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        info!("Computing Phase 2 for job {}", job_id);

        // Prepare parameters
        let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

        let options = ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            minimal_memory: true,
        };
        let phase = proofman::ProvePhase::Internal;

        // Handle the result immediately without holding it across await
        let proof = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Internal(proof)) => {
                info!("Phase 1 computation successful for job {}", job_id);
                proof
            }
            Ok(_) => {
                error!("Error during Phase 1 computation for job {}", job_id);
                return Err(anyhow::anyhow!("Unexpected result type during Phase 1 computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for job {}: {:?}", job_id, err);
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        info!("Phase 2 computation completed for job {}", job.job_id);

        Ok(proof)
    }
}
