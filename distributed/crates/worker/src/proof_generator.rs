use anyhow::Result;
use asm_runner::AsmRunnerOptions;
use asm_runner::AsmServices;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::AggProofs;
use proofman::ContributionsInfo;
use proofman::ProvePhaseInputs;
use proofman::{ProofInfo, ProofMan};
use proofman_common::ProofOptions;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};
use witness::WitnessLibrary;
use zisk_distributed_common::AggregationParams;
use zisk_distributed_common::JobId;
use zisk_distributed_common::JobPhase;

use zisk_common::ZiskLibInitFn;

use crate::worker_service::{ComputationResult, JobContext, ProverServiceConfig};

pub struct ProofGenerator {
    // It is important to keep the witness_lib declaration before the proofman declaration
    // to ensure that the witness library is dropped before the proofman.
    _witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
    _asm_services: Option<AsmServices>,

    proofman: Arc<ProofMan<Goldilocks>>,
    local_rank: i32,
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

        Ok(Self {
            _witness_lib: witness_lib,
            proofman: Arc::new(proofman),
            local_rank,
            _asm_services: asm_services,
        })
    }

    pub fn local_rank(&self) -> i32 {
        self.local_rank
    }

    fn get_proof_options_partial_contribution() -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            minimal_memory: false, // TODO: OPTION
        }
    }

    fn get_proof_options_prove() -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: true,
            final_snark: false,
            verify_proofs: false,
            save_proofs: false,
            test_mode: false,
            output_dir_path: PathBuf::default(),
            minimal_memory: false,
        }
    }

    fn get_proof_options_aggregation(agg_params: &AggregationParams) -> ProofOptions {
        ProofOptions {
            verify_constraints: agg_params.verify_constraints,
            aggregation: agg_params.aggregation,
            final_snark: agg_params.final_snark,
            verify_proofs: agg_params.verify_proofs,
            save_proofs: agg_params.save_proofs,
            test_mode: agg_params.test_mode,
            output_dir_path: agg_params.output_dir_path.clone(),
            minimal_memory: agg_params.minimal_memory,
        }
    }

    pub async fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

            info!("Computing Contribution for {job_id}");

            let proof_info = ProofInfo::new(
                Some(job.block.input_path.clone()),
                job.total_compute_units as usize,
                job.allocation.clone(),
                job.rank_id as usize,
            );
            let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

            let options = Self::get_proof_options_partial_contribution();

            let result =
                Self::execute_contribution_task(job_id.clone(), proofman, phase_inputs, options)
                    .await;

            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Challenge {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Contribution computation failed for {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Challenge {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn execute_contribution_task(
        job_id: JobId,
        proofman: Arc<ProofMan<Goldilocks>>,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
    ) -> Result<Vec<ContributionsInfo>> {
        let phase = proofman::ProvePhase::Contributions;

        // Handle the result immediately without holding it across await
        let challenge = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Contributions(challenge)) => {
                info!("Contribution computation successful for {job_id}");
                challenge
            }
            Ok(_) => {
                error!("Error during Contribution computation for {job_id}");
                return Err(anyhow::anyhow!(
                    "Unexpected result type during Contribution computation"
                ));
            }
            Err(err) => {
                error!("Failed to generate proof for {job_id}: {:?}", err);
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

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

            info!("Computing Prove for {job_id}");

            let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

            let options = Self::get_proof_options_prove();

            let result =
                Self::execute_prove_task(job_id.clone(), proofman, phase_inputs, options).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Proofs {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Prove computation failed for {}: {}", job_id, error);
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
        job_id: JobId,
        proofman: Arc<ProofMan<Goldilocks>>,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
    ) -> Result<Vec<AggProofs>> {
        let world_rank = proofman.get_mpi_ctx().rank;

        let proof = match proofman.generate_proof_from_lib(
            phase_inputs,
            options,
            proofman::ProvePhase::Internal,
        ) {
            Ok(proofman::ProvePhaseResult::Internal(proof)) => {
                if world_rank == 0 {
                    info!("Prove computation successful for {job_id}",);
                }
                proof
            }
            Ok(_) => {
                error!("Error during Prove computation for {job_id}");
                return Err(anyhow::anyhow!("Unexpected result type during Prove computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for {job_id}: {err}");
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

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

            info!("Starting aggregation step for {job_id}");

            let agg_proofs: Vec<AggProofs> = agg_params
                .agg_proofs
                .iter()
                .map(|v| AggProofs {
                    airgroup_id: v.airgroup_id,
                    proof: v.values.clone(),
                    worker_indexes: vec![v.worker_idx as usize],
                })
                .collect();

            let options = Self::get_proof_options_aggregation(&agg_params);

            let result = proofman
                .receive_aggregated_proofs(
                    agg_proofs,
                    agg_params.last_proof,
                    agg_params.final_proof,
                    &options,
                )
                .map(|proof| proof.into_iter().map(|p| p.proof).collect())
                .unwrap_or_default();

            let _ = tx.send(ComputationResult::AggProof {
                job_id,
                success: true,
                result: Ok(Some(result)),
            });
        })
    }

    pub async fn partial_contribution_broadcast(&self, job: Arc<Mutex<JobContext>>) {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        let proof_info = ProofInfo::new(
            Some(job.block.input_path.clone()),
            job.total_compute_units as usize,
            job.allocation.clone(),
            job.rank_id as usize,
        );
        let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

        let options = Self::get_proof_options_partial_contribution();

        let mut serialized =
            borsh::to_vec(&(JobPhase::Contributions, job_id, phase_inputs, options)).unwrap();

        self.proofman.mpi_broadcast(&mut serialized);
    }

    pub async fn prove_broadcast(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
    ) {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

        let options = Self::get_proof_options_prove();

        let mut serialized =
            borsh::to_vec(&(JobPhase::Prove, job_id, phase_inputs, options)).unwrap();

        self.proofman.mpi_broadcast(&mut serialized);
    }

    pub async fn receive_mpi_request(&self) -> Result<()> {
        let mut bytes: Vec<u8> = Vec::new();

        self.proofman.mpi_broadcast(&mut bytes);

        // extract byte 0 to decide the option
        let phase = borsh::from_slice(&bytes[0..1]).unwrap();

        match phase {
            JobPhase::Contributions => {
                let (job_id, phase_inputs, options): (JobId, ProvePhaseInputs, ProofOptions) =
                    borsh::from_slice(&bytes[1..]).unwrap();

                Self::execute_contribution_task(
                    job_id,
                    self.proofman.clone(),
                    phase_inputs,
                    options,
                )
                .await?;
            }
            JobPhase::Prove => {
                let (job_id, phase_inputs, options): (JobId, ProvePhaseInputs, ProofOptions) =
                    borsh::from_slice(&bytes[1..]).unwrap();

                Self::execute_prove_task(job_id, self.proofman.clone(), phase_inputs, options)
                    .await?;
            }
            JobPhase::Aggregate => {
                unreachable!("Aggregate phase is not supported in MPI broadcast");
            }
        }
        Ok(())
    }
}
