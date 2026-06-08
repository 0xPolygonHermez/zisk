use crate::{
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator, CoordinatorError, CoordinatorResult,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use zisk_cluster_common::{
    ComputeCapacity, CoordinatorMessageDto, DataId, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto,
    HintsModeDto, InputsModeDto, Job, JobExecutionMode, JobId, JobPhase, JobState,
    LaunchProofResponseDto, LaunchWrapRequestDto, WorkerState, WrapParamsDto,
};
use zisk_common::{Proof, ProofKind};

impl Coordinator {
    /// Launch a wrap job: compress/reduce an existing vadcop proof to minimal or SNARK format.
    ///
    /// Selects any single idle worker, sends a WRAP task to it, and returns the job ID.
    /// The proof data must be a bincode-encoded `Proof`.
    pub async fn launch_wrap(
        &self,
        request: LaunchWrapRequestDto,
    ) -> CoordinatorResult<LaunchProofResponseDto> {
        // Atomic single-worker reservation under one pool write lock —
        // prevents two concurrent launch_wrap calls from double-booking.
        let job_id = JobId::new();
        let worker_id =
            self.workers_pool.try_reserve_single_ready_for(&job_id, JobPhase::Aggregate).await?;

        let mut job = Job::new(
            job_id.clone(),
            DataId::new(),
            String::new(),
            InputsModeDto::InputsNone,
            HintsModeDto::HintsNone,
            ComputeCapacity::from(1),
            ComputeCapacity::from(1),
            vec![worker_id.clone()],
            vec![],
            JobExecutionMode::Standard,
            BTreeMap::new(),
            false,
            ProofKind::VadcopFinal,
        );
        job.change_state(JobState::Running(JobPhase::Aggregate)); // reuse Aggregate phase as wrap phase
        let program = self.program_alias_for_hash(&job.hash_id);

        let job_arc = Arc::new(RwLock::new(job));
        self.jobs.write().await.insert(job_id.clone(), job_arc);
        self.alloc_job_events(&job_id).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Queued).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;

        crate::metrics::record_job_started(crate::metrics::KIND_PROVE, &program);

        let req = ExecuteTaskRequestDto {
            worker_id: worker_id.clone(),
            job_id: job_id.clone(),
            params: ExecuteTaskRequestTypeDto::WrapParams(WrapParamsDto {
                proof_data: request.proof_data,
                proof_dest: request.proof_dest,
            }),
        };
        let message = CoordinatorMessageDto::ExecuteTaskRequest(req);
        if let Err(e) = self.workers_pool.send_message(&worker_id, message).await {
            // Canonical failure path releases the worker reservation.
            let reason = format!("Failed to dispatch wrap task: {}", e);
            let _ = self.fail_job(&job_id, &reason).await;
            return Err(e);
        }

        tracing::info!("[Wrap] Job {} started on worker {}", job_id, worker_id);

        Ok(LaunchProofResponseDto { job_id })
    }

    /// Handle completion of a wrap task from a worker.
    pub(super) async fn handle_wrap_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = &execute_task_response.job_id;
        let worker_id = &execute_task_response.worker_id;

        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?
        };
        let mut job = job_entry.write().await;

        if job.state().is_resolved() {
            return Ok(());
        }

        self.workers_pool.mark_worker_with_state(worker_id, WorkerState::Ready).await?;

        if !execute_task_response.success {
            // fail_job (takes its own job lock) records failure metrics
            // and runs post_launch_proof; inline change_state would skip both.
            drop(job);
            let reason = execute_task_response.error_message.unwrap_or_default();
            let _ = self.fail_job(job_id, &reason).await;
            return Err(CoordinatorError::Internal(format!("Wrap task failed: {}", reason)));
        }

        let Some(ExecuteTaskResponseResultDataDto::WrapResult(wrap_result)) =
            execute_task_response.result_data
        else {
            return Err(CoordinatorError::Internal(
                "Expected WrapResult in wrap completion".to_string(),
            ));
        };

        let zisk_proof = bincode::serde::decode_from_slice::<Proof, _>(
            &wrap_result.proof_data,
            bincode::config::standard(),
        )
        .map(|(v, _)| v)
        .map_err(|e| {
            CoordinatorError::Internal(format!("Failed to deserialize wrap proof: {}", e))
        })?;
        job.proof = Some(zisk_proof);
        job.change_state(JobState::Completed);

        // Pairs with launch_wrap's record_job_started.
        let program = self.program_alias_for_hash(&job.hash_id);
        crate::metrics::record_job_terminal(
            crate::metrics::KIND_PROVE,
            crate::metrics::OUTCOME_SUCCESS,
            &program,
            &job.workers,
            job.phase_start_time(&JobPhase::Aggregate),
            job.executed_steps,
        );

        tracing::info!("[Wrap] Job {} completed successfully", job_id);

        drop(job);

        self.fire_job_event(
            job_id,
            CoordinatorJobEvent::Completed(CoordinatorJobResult::Wrap {
                proof_bytes: wrap_result.proof_data,
            }),
        )
        .await;

        Ok(())
    }
}
