use anyhow::Result;
use asm_runner::AsmServices;
use consensus_core::JobId;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman_common::ProofOptions;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};
use witness::WitnessLibrary;

use zisk_common::{MpiContext, ZiskLibInitFn};

use crate::prover_grpc_endpoint::ComputationResult;
use crate::prover_service::{JobContext, ProverServiceConfig};

pub struct ProofGenerator {
    // It is important to keep the witness_lib declaration before the proofman declaration
    // to ensure that the witness library is dropped before the proofman.
    witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
    proofman: Arc<ProofMan<Goldilocks>>,
    mpi_context: MpiContext,
    asm_services: Option<AsmServices>,
}

impl ProofGenerator {
    pub fn new(config: &ProverServiceConfig, mpi_context: MpiContext) -> Result<Self> {
        info!("Starting asm microservices...");

        let world_rank = config.asm_runner_options.world_rank;
        let local_rank = config.asm_runner_options.local_rank;
        let base_port = config.asm_runner_options.base_port;
        let unlock_mapped_memory = config.asm_runner_options.unlock_mapped_memory;

        let asm_services = if config.emulator {
            None
        } else {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services.start_asm_services(
                config.asm.as_ref().unwrap(),
                config.asm_runner_options.clone(),
            )?;
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
        )
        .expect("Failed to initialize witness library");

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

        proofman.register_witness(witness_lib.as_mut(), library);

        let witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync> = Arc::from(witness_lib);

        Ok(Self { witness_lib, proofman: Arc::new(proofman), mpi_context, asm_services })
    }

    pub async fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        proofman.set_mpi_ctx2(1, 0);

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
        let phase_inputs = proofman::ProvePhaseInputs::Contributions(Some(
            job.lock().await.block.input_path.clone(),
        ));

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
    ) -> Result<Vec<Vec<u64>>> {
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
                proof.unwrap()
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
