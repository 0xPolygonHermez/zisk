use crate::coordinator_errors::{CoordinatorError, CoordinatorResult};
use crate::job_events::CoordinatorJobEvent;
use chrono::Utc;
use std::time::Duration;
use tracing::{error, info, warn};
use zisk_cluster_common::{
    AggProofData, AggScheduler, ChallengesDto, CoordinatorMessageDto, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, Job,
    JobId, JobPhase, JobResult, JobResultData, JobState, PhaseTimings, ProveParamsDto, WorkerId,
    WorkerState,
};

use crate::{coordinator::Phase2Outcome, Coordinator};

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
        for worker_id in active_workers {
            // Atomic transition; rejects if a concurrent fail_job parked
            // the worker SettingUp between Phase 1 completion and here.
            self.workers_pool
                .try_transition_computing_phase(
                    worker_id,
                    job_id,
                    JobPhase::Contributions,
                    JobPhase::Prove,
                )
                .await?;

            let req = ExecuteTaskRequestDto {
                worker_id: worker_id.clone(),
                job_id: job_id.clone(),
                params: ExecuteTaskRequestTypeDto::ProveParams(ProveParamsDto {
                    challenges: challenges.clone(),
                }),
                metadata: None,
            };
            let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

            self.workers_pool.send_message(worker_id, req).await?;
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

        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(&job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?
        };

        let mut job = job_entry.write().await;

        // If in simulation mode, complete the job
        if job.execution_mode.is_simulating() {
            return self.complete_simulated_job(&mut job, &worker_id).await;
        }

        if job.state().is_resolved() {
            return Ok(());
        }

        // Bind the payload to the phase mutated below (see
        // `validate_response_phase`). After `is_resolved`, so a late response for
        // a resolved job stays a silent no-op rather than an error; and after the
        // simulation short-circuit above, which bypasses the phase machinery
        // wholesale (one physical worker stands in for N, and
        // completion/challenge checks are likewise skipped).
        Self::validate_response_phase(&job, &execute_task_response)?;

        // Store Proof response
        self.store_proof_response(&mut job, execute_task_response).await?;

        // Everything fallible first: a rejected payload after the transition and the
        // state change would leave the job a leaf short with nothing to fail it.
        let set = Self::worker_agg_set(&mut job, &worker_id)?;

        // Its phase-2 leaf stays resident here, so it is the only worker that can fold
        // it. Held Computing until the scheduler seeds a node on it or ships its set.
        self.workers_pool
            .try_transition_computing_phase(&worker_id, &job_id, JobPhase::Prove, JobPhase::Recurse)
            .await?;

        let starting = job.agg.is_none();
        if starting {
            // Before the state change: failing after it would leave the job in
            // Recurse with no scheduler and nothing to fail it.
            let arity = self.agg_arity().await?;
            job.change_state(JobState::Running(JobPhase::Recurse));
            job.agg = Some(AggScheduler::new(
                arity,
                job.workers.len(),
                self.config.coordinator.distributed_aggregation,
            ));
            info!("[Phase3] Aggregation started for {job_id} (arity {arity})");
        }

        // Still timed from the last phase-2 completion, to stay comparable.
        match self.check_phase2_completion(&job, &worker_id).await? {
            Phase2Outcome::AllDone => {
                job.phase_timings.insert(
                    JobPhase::Recurse,
                    PhaseTimings { start_time: Utc::now(), end_time: None },
                );
            }
            Phase2Outcome::Waiting => {}
            Phase2Outcome::Failed(reason) => {
                // `fail_job` re-takes this job's write lock.
                drop(job);
                self.fail_job(&job_id, &reason).await?;
                return Err(CoordinatorError::Internal(reason));
            }
        }

        let scheduler = job.agg.as_mut().expect("just created");
        let dispatch = scheduler.on_set_ready(set);
        // Drained here too: if this step yields the final dispatch there is no later
        // scheduling point, and the donor would stay Computing after a successful job.
        let freed = scheduler.take_released();
        drop(job);

        self.release_donors(&job_id, &freed).await;

        if starting {
            self.fire_job_event(&job_id, CoordinatorJobEvent::Progress(JobPhase::Recurse)).await;
        }

        if let Some(dispatch) = dispatch {
            self.dispatch_agg(&job_id, dispatch).await?;
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
                        worker_indexes: proof.worker_indexes,
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
