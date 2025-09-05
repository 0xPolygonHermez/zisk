use anyhow::Result;
use asm_runner::AsmRunnerOptions;
use asm_runner::AsmServices;
use consensus_common::AggregationParams;
use consensus_common::JobId;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::AggProofs;
use proofman::ContributionsInfo;
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
            let result = Self::compute_contribution_task(job_id.clone(), job, proofman).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Challenge {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Contribution computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Challenge {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn compute_contribution_task(
        job_id: JobId,
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
    ) -> Result<Vec<ContributionsInfo>> {
        info!("Computing Contribution for job {}", job_id);

        // Prepare parameters
        let job = job.lock().await;
        let proof_info = ProofInfo::new(
            Some(job.block.input_path.clone()),
            job.total_compute_units as usize,
            job.allocation.clone(),
            job.rank_id as usize,
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
                info!("Contribution computation successful for job {}", job_id);
                challenge
            }
            Ok(_) => {
                error!("Error during Contribution computation for job {}", job_id);
                return Err(anyhow::anyhow!(
                    "Unexpected result type during Contribution computation"
                ));
            }
            Err(err) => {
                error!("Failed to generate proof for job {}: {:?}", job_id, err);
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        Ok(challenge)
    }

    pub async fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        // TODO! Check that each Vec<u64> has exactly 10 elements
        // Chunk into slices of 10
        // Flatten all the Vec<u64> into a single iterator of u64, then chunk into arrays of 10
        // let challenges: Vec<[u64; 10]> = challenges
        //     .into_iter()
        //     .flatten() // This flattens Vec<Vec<u64>> into an iterator of u64
        //     .collect::<Vec<u64>>() // Collect into a single Vec<u64>
        //     .chunks_exact(10) // Now we can chunk the flattened data
        //     .map(|chunk| chunk.try_into().expect("Chunk must have length 10"))
        //     .collect();

        let job_id = job.lock().await.job_id.clone();

        tokio::spawn(async move {
            let result = Self::execute_prove_task(job, proofman, challenges).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Proofs {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Prove computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Proofs {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn execute_prove_task(
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
        challenges: Vec<ContributionsInfo>,
    ) -> Result<Vec<AggProofs>> {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        info!("Computing Prove for job {}", job_id);

        // Prepare parameters

        //TODO! Fix airgroup_id, now is harcoded
        // let contributions_info = challenges
        //     .into_iter()
        //     .map(|challenge| ContributionsInfo { challenge, airgroup_id: 0, worker_index: job.rank_id })
        //     .collect::<Vec<ContributionsInfo>>();

        let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

        let options = ProofOptions {
            verify_constraints: false,
            aggregation: true,
            final_snark: false,
            verify_proofs: false,
            save_proofs: false,
            test_mode: false,
            output_dir_path: PathBuf::default(),
            minimal_memory: false,
        };
        let phase = proofman::ProvePhase::Internal;

        // Handle the result immediately without holding it across await
        let proof = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Internal(proof)) => {
                info!("Prove computation successful for job {}", job_id);
                proof
            }
            Ok(_) => {
                error!("Error during Prove computation for job {}", job_id);
                return Err(anyhow::anyhow!("Unexpected result type during Prove computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for job {}: {:?}", job_id, err);
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        Ok(proof)
    }

    pub async fn aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        let job_id = job.lock().await.job_id.clone();

        tokio::spawn(async move {
            let result = Self::execute_aggregation_task(job, proofman, agg_params).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::AggProof {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Prove computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::AggProof {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn execute_aggregation_task(
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
        agg_params: AggregationParams,
    ) -> Result<Option<Vec<u64>>> {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        info!("Computing Aggregation for job {}", job_id);

        let agg_proofs: Vec<AggProofs> = agg_params
            .agg_proofs
            .iter()
            .map(|v| AggProofs {
                airgroup_id: v.airgroup_id,
                proof: v.values.clone(),
                worker_indexes: vec![v.worker_idx as usize],
            })
            .collect();

        let options = ProofOptions {
            verify_constraints: agg_params.verify_constraints,
            aggregation: agg_params.aggregation,
            final_snark: agg_params.final_snark,
            verify_proofs: agg_params.verify_proofs,
            save_proofs: agg_params.save_proofs,
            test_mode: agg_params.test_mode,
            output_dir_path: agg_params.output_dir_path,
            minimal_memory: agg_params.minimal_memory,
        };

        // Handle the result immediately without holding it across await
        let proof = proofman.receive_aggregated_proofs(
            agg_proofs,
            agg_params.last_proof,
            agg_params.final_proof,
            &options,
        );

        info!("Aggregation computation successful for job {}", job_id);

        Ok(Some(proof.unwrap()[0].proof.clone()))
    }
}
