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
    HintsModeDto, InputsModeDto, Job, JobExecutionMode, JobPhase, JobState, LaunchProofResponseDto,
    LaunchWrapRequestDto, WorkerId, WorkerState, WrapParamsDto,
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
        // Select any single idle worker
        let worker_id = {
            let ids = self.workers_pool.connected_worker_ids().await;
            let mut found: Option<WorkerId> = None;
            for id in ids {
                if matches!(self.workers_pool.worker_state(&id).await, Some(WorkerState::Ready)) {
                    found = Some(id);
                    break;
                }
            }
            found.ok_or(CoordinatorError::InsufficientCapacity)?
        };

        // Create a minimal job entry (no partitions / inputs needed for wrap)
        let job = Job::new(
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

        let job_id = job.job_id.clone();

        let mut job = job;
        job.change_state(JobState::Running(JobPhase::Aggregate)); // reuse Aggregate phase as wrap phase

        let job_arc = Arc::new(RwLock::new(job));
        self.jobs.write().await.insert(job_id.clone(), job_arc);
        self.alloc_job_events(&job_id).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Queued).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;

        // Mark worker as computing and send the task
        self.workers_pool
            .mark_worker_with_state(
                &worker_id,
                WorkerState::Computing((job_id.clone(), zisk_cluster_common::JobPhase::Aggregate)),
            )
            .await?;

        let req = ExecuteTaskRequestDto {
            worker_id: worker_id.clone(),
            job_id: job_id.clone(),
            params: ExecuteTaskRequestTypeDto::WrapParams(WrapParamsDto {
                proof_data: request.proof_data,
                proof_dest: request.proof_dest,
            }),
        };

        let message = CoordinatorMessageDto::ExecuteTaskRequest(req);
        self.workers_pool.send_message(&worker_id, message).await?;

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

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let mut job = job_entry.write().await;

        self.workers_pool.mark_worker_with_state(worker_id, WorkerState::Ready).await?;

        if !execute_task_response.success {
            job.change_state(JobState::Failed);
            let reason = execute_task_response.error_message.unwrap_or_default();
            drop(job);
            self.fire_job_event(job_id, CoordinatorJobEvent::Failed(reason.clone())).await;
            return Err(CoordinatorError::Internal(format!("Wrap task failed: {}", reason)));
        }

        let ExecuteTaskResponseResultDataDto::WrapResult(wrap_result) =
            execute_task_response.result_data
        else {
            return Err(CoordinatorError::Internal(
                "Expected WrapResult in wrap completion".to_string(),
            ));
        };

        let zisk_proof = bincode::deserialize::<Proof>(&wrap_result.proof_data).map_err(|e| {
            CoordinatorError::Internal(format!("Failed to deserialize wrap proof: {}", e))
        })?;
        job.proof = Some(zisk_proof);
        job.change_state(JobState::Completed);

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
