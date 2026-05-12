use crate::coordinator_errors::{CoordinatorError, CoordinatorResult};
use chrono::Utc;
use std::time::Duration;
use tracing::{error, info, warn};
use zisk_cluster_common::{
    AggProofData, ChallengesDto, CoordinatorMessageDto, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, Job,
    JobId, JobPhase, JobResult, JobResultData, JobState, PendingAggTask, PhaseTimings,
    ProveParamsDto, WorkerId, WorkerState,
};

use crate::Coordinator;

impl Coordinator {
    /// Initiates Phase 2 (Prove) execution across all selected workers.
    ///
    /// Orchestrates the distribution of proof generation tasks using the challenges
    /// generated in Phase 1. This method ensures all workers receive the complete
    /// challenge set and transition properly to the proof generation phase.
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the job transitioning to Phase 2
    /// * `active_workers` - List of workers that should participate in Phase 2
    /// * `challenges` - Challenges generated from Phase 1 contributions
    pub(super) async fn start_prove(
        &self,
        job_id: &JobId,
        active_workers: &[WorkerId],
        challenges: Vec<ChallengesDto>,
    ) -> CoordinatorResult<()> {
        // Send messages to active workers
        for worker_id in active_workers {
            if let Some(worker_state) = self.workers_pool.worker_state(worker_id).await {
                // Validate worker is in the expected Phase 1 computing state
                // This ensures proper phase sequencing and prevents race conditions
                if !matches!(worker_state, WorkerState::Computing((_, JobPhase::Contributions))) {
                    let reason =
                        format!("Worker {worker_id} is not in computing state for {}", job_id);
                    return Err(CoordinatorError::InvalidRequest(reason));
                }

                // Transition worker to Phase 2 computing state
                // This atomic update ensures consistent state tracking across the system
                self.workers_pool
                    .mark_worker_with_state(
                        worker_id,
                        WorkerState::Computing((job_id.clone(), JobPhase::Prove)),
                    )
                    .await?;

                // Create Phase 2 task with complete challenge set
                // All workers receive the full challenge data regardless of their individual contributions
                let req = ExecuteTaskRequestDto {
                    worker_id: worker_id.clone(),
                    job_id: job_id.clone(),
                    params: ExecuteTaskRequestTypeDto::ProveParams(ProveParamsDto {
                        challenges: challenges.clone(), // Complete challenge set from Phase 1 aggregation
                    }),
                };
                let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

                // Send start prove message to worker
                // Network failures here will cause the method to fail and require retry logic
                self.workers_pool.send_message(worker_id, req).await?;
            } else {
                // Worker disappeared between Phase 1 completion and Phase 2 start
                // This can happen due to disconnections or system state changes
                warn!("Worker {} not found when starting Phase2", worker_id);
                return Err(CoordinatorError::NotFoundOrInaccessible);
            }
        }

        Ok(())
    }

    /// Processes Phase 2 (Proofs) completion and orchestrates transition to Phase 3.
    ///
    /// Handles the coordination required when workers complete their proof generation tasks.
    ///
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing proof results from a worker
    pub(super) async fn handle_proofs_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id.clone();
        let worker_id = execute_task_response.worker_id.clone();

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let mut job = job_entry.write().await;

        // If in simulation mode, complete the job
        if job.execution_mode.is_simulating() {
            return self.complete_simulated_job(&mut job, &worker_id).await;
        }

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool
                .mark_worker_with_state(&execute_task_response.worker_id, WorkerState::Ready)
                .await?;
            return Ok(());
        }

        // Store Proof response
        self.store_proof_response(&mut job, execute_task_response).await?;

        // Assign aggregator worker if not already assigned
        let agg_worker_id = self.resolve_aggregator_assignment(&mut job, &worker_id).await?;

        let all_done = self.check_phase2_completion(&job, &worker_id).await?;

        if all_done {
            job.phase_timings.insert(
                JobPhase::Aggregate,
                PhaseTimings { start_time: Utc::now(), end_time: None },
            );
        }

        let proofs = self.collect_worker_proofs(&job, &agg_worker_id, &worker_id)?;
        let task = PendingAggTask { proofs, all_done, proof_type: job.proof_type };

        if job.agg_task_inflight.is_none() {
            // Nothing in-flight — store a copy and dispatch the original immediately.
            job.agg_task_inflight = Some(task.clone());
            drop(job);
            self.send_aggregation_task(
                &job_id,
                &agg_worker_id,
                task.proofs,
                task.all_done,
                task.proof_type,
            )
            .await?;
        } else {
            // Task in-flight — queue this one; it will be sent after the ack.
            job.agg_task_queue.push_back(task);
        }

        Ok(())
    }

    /// Stores a single worker's Contribution response in the job state.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to update
    /// * `execute_task_response` - The response from the worker containing proof data
    async fn store_proof_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id;
        let worker_id = execute_task_response.worker_id;

        let phase2_results = job.results.entry(JobPhase::Prove).or_default();

        // Check for duplicate results
        if phase2_results.contains_key(&worker_id) {
            let msg =
                format!("Received duplicate Proof result from worker {} for {}", worker_id, job_id);
            warn!(msg);
            return Err(CoordinatorError::InvalidRequest(msg));
        }

        // Extract and validate proofs data from Phase2 response
        let data = match execute_task_response.result_data {
            Some(ExecuteTaskResponseResultDataDto::Proofs(proof_list)) => {
                let agg_proofs: Vec<AggProofData> = proof_list
                    .into_iter()
                    .map(|proof| AggProofData {
                        airgroup_id: proof.airgroup_id,
                        values: proof.values,
                        worker_idx: proof.worker_idx,
                    })
                    .collect();
                JobResultData::AggProofs(agg_proofs)
            }
            _ => {
                return Err(CoordinatorError::InvalidRequest(
                    "Expected Proofs result data for Phase2".to_string(),
                ));
            }
        };

        phase2_results.insert(
            worker_id.clone(),
            JobResult { success: execute_task_response.success, data, end_time: Utc::now() },
        );

        Ok(())
    }

    /// Completes a simulated job by marking it as completed and freeing resources.
    ///
    /// # Parameters
    ///
    /// * `job` - Mutable reference to job for state updates
    async fn complete_simulated_job(
        &self,
        job: &mut Job,
        worker_id: &WorkerId,
    ) -> CoordinatorResult<()> {
        job.change_state(JobState::Completed);

        let assigned_workers = job.workers.clone();

        // Reset worker statuses back to Idle
        self.workers_pool.mark_workers_with_state(&assigned_workers, WorkerState::Ready).await?;

        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(
            job.phase_start_time(&JobPhase::Prove).unwrap_or_else(|| {
                error!("Missing start time for Phase2 in job {}", job.job_id);
                end_time
            }),
        );

        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Provide operational visibility into Phase 2 progress
        // This logging helps with monitoring long-running proof generation jobs
        info!(
            "[Phase2 progress] Worker {} done. (duration: {:.3}s)",
            worker_id,
            duration_ms.as_secs_f32()
        );

        let duration_simulation = Duration::from_millis(job.duration_ms.unwrap_or(0));

        info!(
            "[Simulated Job Finished] {} (duration: {:.3}s)",
            job.job_id,
            duration_simulation.as_secs_f32()
        );

        Ok(())
    }
}
