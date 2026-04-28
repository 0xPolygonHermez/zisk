use crate::{
    coordinator::exec_stats_from_job,
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator, PrecompileHintsRelay, WorkersPool,
};
use chrono::Utc;
use colored::Colorize;
use proofman::{ContributionsInfo, WitnessInfo};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{error, info, warn};
use zisk_cluster_common::{
    ChallengesDto, ContributionParamsDto, ContributionsResult, CoordinatorMessageDto,
    ExecuteTaskRequestDto, ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto,
    ExecuteTaskResponseResultDataDto, ExecutionResult, HintsModeDto, HintsSourceDto,
    InputSourceDto, InputStreamDataDto, InputsModeDto, Job, JobId, JobPhase, JobResult,
    JobResultData, JobState, StreamMessageKind, WorkerId, WorkerState, ZiskExecutorTimeDto,
};
use zisk_common::io::{StreamRead, StreamSource, ZiskStream};
use zisk_common::AsmExecutionInfo;
use zisk_common::ZiskExecutorTime;
use zisk_common::ZISK_PUBLICS;

impl Coordinator {
    /// Dispatches Phase 1 (Contributions) tasks to all selected workers.
    ///
    /// Orchestrates the distribution of initial computation tasks across the selected
    /// worker set. Each worker receives a customized task containing their specific
    /// work partition and coordination parameters.
    ///
    /// # Parameters
    ///
    /// * `job` - Job containing partition assignments and configuration
    /// * `active_workers` - List of workers that should receive tasks
    pub(super) async fn dispatch_contributions_messages(
        &self,
        job: &Job,
        active_workers: &[WorkerId],
    ) -> CoordinatorResult<()> {
        let input_source = match job.inputs_mode {
            InputsModeDto::InputsPath(ref inputs_path) => {
                InputSourceDto::InputPath(inputs_path.clone())
            }
            InputsModeDto::InputsData(ref inputs_hex) => {
                let inputs = hex::decode(inputs_hex).map_err(|e| {
                    CoordinatorError::Internal(format!(
                        "Failed to decode inline input data for job {}: {}",
                        job.job_id, e
                    ))
                })?;
                InputSourceDto::InputData(inputs)
            }
            InputsModeDto::InputsStream(_) => {
                // Coordinator will relay streamed inputs to workers via InputStreamData.
                // Workers receive InputNull and start execution; data arrives incrementally
                // through append_raw_input (same mechanism as PushJobInput).
                InputSourceDto::InputNull
            }
            InputsModeDto::InputsNone => InputSourceDto::InputNull,
        };

        let hints_source = match &job.hints_mode {
            HintsModeDto::HintsPath(ref hints_uri) => HintsSourceDto::HintsPath(hints_uri.clone()),
            HintsModeDto::HintsData(ref hints_hex) => {
                let hints = hex::decode(hints_hex).map_err(|e| {
                    CoordinatorError::Internal(format!(
                        "Failed to decode inline hints data for job {}: {}",
                        job.job_id, e
                    ))
                })?;
                HintsSourceDto::HintsData(hints)
            }
            HintsModeDto::HintsStream(hints_uri) => {
                // Hints will be streamed separately
                HintsSourceDto::HintsStream(hints_uri.clone())
            }
            HintsModeDto::HintsNone => HintsSourceDto::HintsNull,
        };

        // Use Arc to avoid expensive clones
        let active_workers = active_workers.to_vec();
        let total_workers = active_workers.len() as u32;

        let cloned_active_workers = active_workers.clone();
        let execution_only = job.execution_only;
        let tasks = active_workers.into_iter().enumerate().map(|(rank_id, worker_id)| {
            let job_id = job.job_id.clone();
            let data_id = job.data_id.clone();
            let input_source = input_source.clone();
            let hints_source = hints_source.clone();
            let worker_allocation = job.partitions[rank_id].clone();
            let job_compute_capacity = job.compute_capacity;
            let workers_pool = &self.workers_pool;

            async move {
                let contribution_params = ContributionParamsDto {
                    data_id,
                    input_source,
                    hints_source,
                    rank_id: rank_id as u32,
                    total_workers,
                    worker_allocation,
                    job_compute_units: job_compute_capacity,
                };

                let params = if execution_only {
                    ExecuteTaskRequestTypeDto::ExecutionParams(contribution_params)
                } else {
                    ExecuteTaskRequestTypeDto::ContributionParams(contribution_params)
                };

                let req = ExecuteTaskRequestDto {
                    worker_id: worker_id.clone(),
                    job_id: job_id.clone(),
                    params,
                };
                let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

                let send_result = workers_pool.send_message(&worker_id, req).await;
                let state_result = workers_pool
                    .mark_worker_with_state(
                        &worker_id,
                        WorkerState::Computing((job_id.clone(), JobPhase::Contributions)),
                    )
                    .await;

                (worker_id, send_result, state_result)
            }
        });

        // Process tasks with a concurrency limit
        use futures::stream::StreamExt;

        let results: Vec<_> = futures::stream::iter(tasks).buffer_unordered(16).collect().await;

        // Check for any errors
        for (worker_id, send_result, state_result) in results {
            send_result.map_err(|e| {
                CoordinatorError::Internal(format!(
                    "Failed to send message to worker {}: {}",
                    worker_id, e
                ))
            })?;

            state_result.map_err(|e| {
                CoordinatorError::Internal(format!(
                    "Failed to update state for worker {}: {}",
                    worker_id, e
                ))
            })?;
        }

        if matches!(hints_source, HintsSourceDto::HintsStream(_)) {
            self.initialize_stream(job, cloned_active_workers.clone()).await?;
        }

        if matches!(job.inputs_mode, InputsModeDto::InputsStream(ref uri) if !uri.starts_with("grpc://"))
        {
            self.initialize_input_relay(job, cloned_active_workers).await?;
        }

        Ok(())
    }

    async fn initialize_stream(
        &self,
        job: &Job,
        cloned_active_workers: Vec<WorkerId>,
    ) -> Result<(), CoordinatorError> {
        let hints_uri = match &job.hints_mode {
            HintsModeDto::HintsStream(uri) => uri,
            _ => unreachable!(),
        };
        let job_id_clone = job.job_id.clone();
        let workers_clone = Arc::new(cloned_active_workers.clone());
        let workers_pool = Arc::clone(&self.workers_pool);

        // Async dispatcher — no blocking, pure async flow for maximum performance.
        let dispatcher =
            move |sequence_number: u32, stream_type: StreamMessageKind, payload: Vec<u8>| {
                use futures::future::join_all;
                use zisk_cluster_common::{StreamDataDto, StreamPayloadDto};

                let job_id = job_id_clone.clone();
                let workers = Arc::clone(&workers_clone);
                let pool = Arc::clone(&workers_pool);

                Box::pin(async move {
                    let sends = workers.iter().map(|worker_id| {
                        let job_id = job_id.clone();
                        let worker_id = worker_id.clone();
                        let payload = payload.clone();
                        let pool = Arc::clone(&pool);
                        let stream_type = stream_type.clone();

                        async move {
                            let msg = CoordinatorMessageDto::StreamData(StreamDataDto {
                                job_id: job_id.clone(),
                                stream_type,
                                stream_payload: Some(StreamPayloadDto { sequence_number, payload }),
                            });

                            if let Err(e) = pool.send_message(&worker_id, msg).await {
                                error!(
                                    "Failed to send hints to worker {} for job {}: {}",
                                    worker_id, job_id, e
                                );
                            }
                        }
                    });

                    join_all(sends).await;
                })
            };
        let hints_relay = PrecompileHintsRelay::new(dispatcher);
        let mut stream = ZiskStream::new(hints_relay);

        // For gRPC push, use a channel-backed reader and store the sender so
        // that `push_hints_grpc_data` can feed chunks into the relay.
        let stream_reader = if hints_uri.starts_with("grpc://") {
            let (reader, tx) = StreamSource::channel();
            self.grpc_hints_senders.write().await.insert(job.job_id.clone(), tx);
            reader
        } else {
            StreamSource::from_uri(hints_uri).map_err(|e| {
                CoordinatorError::Internal(format!(
                    "Failed to create hints stream reader for job {}: {}",
                    job.job_id, e
                ))
            })?
        };

        stream.set_stream_src(stream_reader).map_err(|e| {
            CoordinatorError::Internal(format!(
                "Failed to set hints stream for job {}: {}",
                job.job_id, e
            ))
        })?;
        stream.start_stream().map_err(|e| {
            CoordinatorError::Internal(format!(
                "Failed to start hints stream for job {}: {}",
                job.job_id, e
            ))
        })?;
        Ok(())
    }

    /// Push a raw hints chunk from the gRPC `PushJobHintsInput` path into the
    /// per-job relay.  Returns an error if the job has no active gRPC hints
    /// relay (i.e. it was not submitted with a `"grpc://"` hints URI).
    pub async fn push_hints_grpc_data(
        &self,
        job_id: &JobId,
        data: Vec<u8>,
    ) -> CoordinatorResult<()> {
        let map = self.grpc_hints_senders.read().await;
        let tx = map.get(job_id).ok_or_else(|| {
            CoordinatorError::Internal(format!(
                "no gRPC hints relay for job {} (job not found or not using grpc hints)",
                job_id
            ))
        })?;
        tx.send(Some(data)).map_err(|_| {
            CoordinatorError::Internal(format!(
                "gRPC hints relay channel closed for job {}",
                job_id
            ))
        })
    }

    /// Signal EOF on the gRPC hints relay for a job.  Called when the client
    /// closes the `PushJobHintsInput` stream.
    pub async fn finish_hints_grpc_stream(&self, job_id: &JobId) {
        if let Some(tx) = self.grpc_hints_senders.write().await.remove(job_id) {
            let _ = tx.send(None);
        }
    }

    /// Spawn a background thread that reads input chunks from a stream URI and
    /// relays them to all workers as `InputStreamData` messages.
    ///
    /// Workers start with `InputNull` and receive data incrementally via
    /// `append_raw_input` (same mechanism as `PushJobInput`).
    ///
    /// On stream errors, the job is marked as failed so workers are not left
    /// waiting for data indefinitely.
    async fn initialize_input_relay(
        &self,
        job: &Job,
        active_workers: Vec<WorkerId>,
    ) -> Result<(), CoordinatorError> {
        let inputs_uri = match &job.inputs_mode {
            InputsModeDto::InputsStream(uri) => uri.clone(),
            _ => unreachable!(),
        };

        let job_id = job.job_id.clone();
        let workers_pool = Arc::clone(&self.workers_pool);

        // Grab the job Arc so the relay thread can mark it failed on error.
        let job_arc = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .expect("job must be in jobs map before relay is spawned");

        // Spawn a background thread: opens the stream reader, reads chunks,
        // and relays each chunk to all workers via InputStreamData.
        std::thread::spawn(move || {
            // Create a dedicated multi-threaded tokio runtime so the QUIC
            // reader's `block_on_dedicated` / `block_in_place` works correctly.
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to create input relay runtime");

            rt.block_on(async move {
                let result =
                    Self::run_input_relay(&inputs_uri, &job_id, &active_workers, &workers_pool)
                        .await;

                if let Err(e) = result {
                    error!("Input relay failed for job {}: {}", job_id, e);
                    // Mark the job as failed so workers are not left waiting.
                    let mut job = job_arc.write().await;
                    if !job.state().is_resolved() {
                        job.change_state(JobState::Failed);
                    }
                }
            });
        });

        Ok(())
    }

    /// Core loop for the input relay: connects to the stream, reads chunks,
    /// and broadcasts each to all workers.
    async fn run_input_relay(
        inputs_uri: &str,
        job_id: &JobId,
        workers: &[WorkerId],
        workers_pool: &WorkersPool,
    ) -> anyhow::Result<()> {
        let mut stream = StreamSource::from_uri(inputs_uri)?;

        // The SDK creates its listener after receiving the submit_job response, so
        // the socket may not exist yet when this relay thread starts. Retry with
        // backoff for up to 60 s before giving up.
        if !stream.is_active() {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            loop {
                match stream.open() {
                    Ok(_) => break,
                    Err(e) => {
                        if std::time::Instant::now() >= deadline {
                            return Err(e.context(format!(
                                "timed out waiting for input stream socket to become available: {}",
                                inputs_uri
                            )));
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }
            }
        }

        info!("Input relay started for job {} from {}", job_id, inputs_uri);

        loop {
            match stream.next() {
                Ok(Some(chunk)) => {
                    let sends = workers.iter().map(|worker_id| {
                        let job_id = job_id.clone();
                        let worker_id = worker_id.clone();
                        let payload = chunk.clone();

                        async move {
                            let msg = CoordinatorMessageDto::InputStreamData(InputStreamDataDto {
                                job_id,
                                payload,
                            });
                            if let Err(e) = workers_pool.send_message(&worker_id, msg).await {
                                error!("Failed to relay input to worker {}: {}", worker_id, e);
                            }
                        }
                    });

                    futures::future::join_all(sends).await;
                }
                Ok(None) => {
                    info!("Input relay finished for job {} (stream ended)", job_id);
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// Processes Phase 1 (Contributions) completion and orchestrates transition to Phase 2.
    ///
    /// Handles the coordination required when workers complete their initial
    /// contribution tasks.
    ///
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing contribution results from a worker
    pub(super) async fn handle_contributions_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id.clone();

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        let worker_id = execute_task_response.worker_id.clone();

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool.mark_worker_with_state(&worker_id, WorkerState::Ready).await?;
            return Ok(());
        }

        // Store Contributions response and extract instances
        let instances = self.store_contribution_response(&mut job, execute_task_response).await?;
        job.instances = Some(instances);

        // Check if all contributions are complete
        if !self.check_phase1_completion(&job, &worker_id) {
            return Ok(());
        }

        // Print execution summary from Phase 1 completion
        self.print_execution_summary(&job);

        // Validate and extract challenges in a single operation to minimize lock time
        let challenges = self.validate_and_extract_challenges(&job).await?;

        // Update job state to Phase2
        job.challenges = Some(challenges);
        job.change_state(JobState::Running(JobPhase::Prove));

        let challenges_dto = self.collect_challenges_dto(&job);

        let active_workers = self.select_workers_for_execution(&job)?;

        drop(job); // Release jobs lock early

        self.fire_job_event(&job_id, CoordinatorJobEvent::Progress(JobPhase::Prove)).await;

        // Start Phase2 for all workers
        self.start_prove(&job_id, &active_workers, challenges_dto).await?;

        info!("[Phase2] Started with {} workers for {}", active_workers.len(), job_id);

        Ok(())
    }

    pub(super) async fn handle_execution_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id.clone();

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        let worker_id = execute_task_response.worker_id.clone();

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool.mark_worker_with_state(&worker_id, WorkerState::Ready).await?;
            return Ok(());
        }

        // Store Execution response and extract instances and executed_steps
        let (instances, executed_steps) =
            self.store_execution_response(&mut job, execute_task_response).await?;
        job.instances = Some(instances);
        job.executed_steps = Some(executed_steps);

        // Check if all execution results are complete
        if !self.check_execution_completion(&job, &worker_id) {
            return Ok(());
        }

        // Print execution summary
        self.print_execution_summary(&job);

        // Mark job as completed (execution-only, no proof generation)
        job.change_state(JobState::Completed);

        // Calculate total execution time
        let end_time = Utc::now();
        let start_time = job.phase_start_time(&JobPhase::Execution).unwrap_or(end_time);
        let total_duration = end_time.signed_duration_since(start_time);
        let duration = Duration::from_millis(total_duration.num_milliseconds() as u64);

        let header = format!("[Execution] Job {} completed successfully ✔", job_id).green();
        let duration_str = format!("Duration: {:.3}s", duration.as_secs_f32()).bold();
        let steps_str = if let Some(executed_steps) = job.executed_steps {
            format!("Steps: {}", Self::format_number_with_dots(executed_steps)).bold()
        } else {
            "Steps: N/A".to_string().red().bold()
        };
        let instances_str = if let Some(instances) = job.instances {
            format!("Instances: {}", Self::format_number_with_dots(instances)).bold()
        } else {
            "Instances: N/A".to_string().red().bold()
        };

        let metadata_str = if job.metadata.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> =
                job.metadata.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
            format!(" {}", pairs.join(", "))
        };

        info!(
            "{} {} {} {} Capacity: {}{}",
            header, duration_str, steps_str, instances_str, job.compute_capacity, metadata_str
        );

        // Print ASM execution statistics if multiple workers
        let workers = job.workers.clone();
        if workers.len() > 1 {
            if let Some(results) = job.results.get(&JobPhase::Execution) {
                // Extract overall execution times (phase duration from task received to completion)
                let mut execution_durations: Vec<(WorkerId, i64)> = results
                    .iter()
                    .filter_map(|(worker_id, result)| {
                        if let JobResultData::Execution(exec_result) = &result.data {
                            exec_result.task_received_time.map(|task_received| {
                                let duration = result.end_time.signed_duration_since(task_received);
                                (worker_id.clone(), duration.num_milliseconds())
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                if execution_durations.len() > 1 {
                    execution_durations.sort_by_key(|(_, duration)| *duration);
                    let (best_worker, best_duration) = &execution_durations[0];
                    let (worst_worker, worst_duration) = execution_durations.last().unwrap();
                    let avg_duration = execution_durations.iter().map(|(_, d)| d).sum::<i64>()
                        as f64
                        / execution_durations.len() as f64;

                    let diff_percentage = if *best_duration > 0 {
                        ((*worst_duration - *best_duration) as f64 / *best_duration as f64) * 100.0
                    } else {
                        0.0
                    };

                    info!(
                        "[Execution] Performance for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                        job_id,
                        avg_duration / 1000.0,
                        best_worker,
                        *best_duration as f64 / 1000.0,
                        worst_worker,
                        *worst_duration as f64 / 1000.0,
                        diff_percentage
                    );
                }

                // Extract ASM execution times
                let mut asm_times: Vec<(WorkerId, f32, f32)> = results
                    .iter()
                    .filter_map(|(worker_id, result)| {
                        if let JobResultData::Execution(exec_result) = &result.data {
                            exec_result
                                .zisk_executor_time
                                .asm_execution_duration
                                .as_ref()
                                .map(|asm| (worker_id.clone(), asm.time, asm.mhz))
                        } else {
                            None
                        }
                    })
                    .collect();

                if !asm_times.is_empty() {
                    asm_times.sort_by(|(_, a, _), (_, b, _)| {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let (best_asm_worker, best_asm, best_mhz) = &asm_times[0];
                    let (worst_asm_worker, worst_asm, worst_mhz) = asm_times.last().unwrap();
                    let avg_asm = asm_times.iter().map(|(_, t, _)| *t as f64).sum::<f64>()
                        / asm_times.len() as f64;

                    let asm_diff_percentage = if *best_asm > 0.0 {
                        ((*worst_asm - *best_asm) as f64 / *best_asm as f64) * 100.0
                    } else {
                        0.0
                    };

                    info!(
                        "[Execution] ASM for {} - Avg: {:.3}s, Best: {} ({:.3}s @ {:.1}MHz), Worst: {} ({:.3}s @ {:.1}MHz), Diff: {:.1}%",
                        job_id,
                        avg_asm,
                        best_asm_worker,
                        *best_asm,
                        *best_mhz,
                        worst_asm_worker,
                        *worst_asm,
                        *worst_mhz,
                        asm_diff_percentage
                    );
                }
            }
        }

        // Mark all workers as idle
        for worker_id in &job.workers {
            self.workers_pool.mark_worker_with_state(worker_id, WorkerState::Ready).await?;
        }

        let exec_stats = exec_stats_from_job(&job);

        let public_outputs = job
            .results
            .get(&JobPhase::Execution)
            .and_then(|m| m.values().next())
            .and_then(|r| {
                if let JobResultData::Execution(ref e) = r.data {
                    Some(e.public_outputs.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Release job lock before cleanup
        drop(job);

        self.fire_job_event(
            &job_id,
            CoordinatorJobEvent::Completed(CoordinatorJobResult::Execute {
                stats: exec_stats,
                public_outputs,
            }),
        )
        .await;

        let mut job = job_entry.write().await;

        // Clean up process data for the job (no webhook for execution-only)
        job.cleanup();

        Ok(())
    }

    /// Stores a single worker's Contribution response in the job state.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to update
    /// * `execute_task_response` - The response from the worker containing contribution data
    async fn store_contribution_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<u64> {
        let contributions_results = job.results.entry(JobPhase::Contributions).or_default();

        let worker_id = execute_task_response.worker_id.clone();

        // Check for duplicate results
        if contributions_results.contains_key(&worker_id) {
            warn!(
                "Received duplicate Contribution result from worker {worker_id} for {}",
                job.job_id
            );
            return Err(CoordinatorError::InvalidRequest(format!(
                "Duplicate Contribution result from worker {worker_id} for {}",
                job.job_id
            )));
        }

        let data = self.extract_challenges_data(execute_task_response.result_data)?;
        let instances =
            if let JobResultData::Challenges(ref contrib) = data { contrib.instances } else { 0 };

        contributions_results.insert(
            worker_id.clone(),
            JobResult { success: execute_task_response.success, data, end_time: Utc::now() },
        );

        Ok(instances)
    }

    /// Stores a single worker's Execution-only response in the job state.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to update
    /// * `execute_task_response` - The response from the worker containing execution data
    async fn store_execution_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<(u64, u64)> {
        let execution_results = job.results.entry(JobPhase::Execution).or_default();

        let worker_id = execute_task_response.worker_id.clone();

        // Check for duplicate results
        if execution_results.contains_key(&worker_id) {
            warn!("Received duplicate Execution result from worker {worker_id} for {}", job.job_id);
            return Err(CoordinatorError::InvalidRequest(format!(
                "Duplicate Execution result from worker {worker_id} for {}",
                job.job_id
            )));
        }

        let data = self.extract_execution_data(execute_task_response.result_data)?;
        let (instances, executed_steps) = if let JobResultData::Execution(ref exec_result) = data {
            (exec_result.instances, exec_result.executed_steps)
        } else {
            (0, 0)
        };

        execution_results.insert(
            worker_id.clone(),
            JobResult { success: execute_task_response.success, data, end_time: Utc::now() },
        );

        Ok((instances, executed_steps))
    }

    /// Extracts challenge data from the worker's result response.
    ///
    /// # Parameters
    ///
    /// * `result_data` - The result data from the worker's response
    fn extract_challenges_data(
        &self,
        result_data: ExecuteTaskResponseResultDataDto,
    ) -> CoordinatorResult<JobResultData> {
        match result_data {
            ExecuteTaskResponseResultDataDto::Challenges(ch_list) => {
                if ch_list.challenges.is_empty() {
                    return Err(CoordinatorError::InvalidRequest(
                        "Received empty Challenges result data".to_string(),
                    ));
                }

                let contributions: Vec<ContributionsInfo> = ch_list
                    .challenges
                    .into_iter()
                    .map(|challenge| ContributionsInfo {
                        worker_index: challenge.worker_index,
                        airgroup_id: challenge.airgroup_id as usize,
                        challenge: challenge.challenge,
                        aggregated: false,
                    })
                    .collect();

                let witness_info = WitnessInfo {
                    summary_info: ch_list.witness_info.summary_info,
                    publics: ch_list.witness_info.publics,
                    proof_values: ch_list.witness_info.proof_values,
                    witness_time: ch_list.witness_info.witness_time,
                    total_instances: ch_list.witness_info.total_instances as usize,
                };

                let zisk_executor_time = Self::extract_execution_info(&ch_list.zisk_executor_time);

                Ok(JobResultData::Challenges(ContributionsResult {
                    witness_info,
                    challenges: contributions,
                    zisk_executor_time,
                    task_received_time: chrono::DateTime::<Utc>::from_timestamp(
                        (ch_list.zisk_executor_time.task_received_time / 1000.0) as i64,
                        ((ch_list.zisk_executor_time.task_received_time % 1000.0) * 1_000_000.0)
                            as u32,
                    ),
                    instances: ch_list.witness_info.total_instances,
                }))
            }
            _ => Err(CoordinatorError::InvalidRequest(
                "Expected Challenges result data for Phase1".to_string(),
            )),
        }
    }

    /// Extracts execution-only data from the worker's result response.
    ///
    /// # Parameters
    ///
    /// * `result_data` - The result data from the worker's response
    fn extract_execution_data(
        &self,
        result_data: ExecuteTaskResponseResultDataDto,
    ) -> CoordinatorResult<JobResultData> {
        match result_data {
            ExecuteTaskResponseResultDataDto::Execution(exec_data) => {
                let zisk_executor_time =
                    Self::extract_execution_info(&exec_data.zisk_executor_time);
                let instances = exec_data.instances;
                let executed_steps = exec_data.executed_steps;

                let task_received_time = chrono::DateTime::<Utc>::from_timestamp(
                    (exec_data.zisk_executor_time.task_received_time / 1000.0) as i64,
                    ((exec_data.zisk_executor_time.task_received_time % 1000.0) * 1_000_000.0)
                        as u32,
                );

                Ok(JobResultData::Execution(ExecutionResult {
                    instances,
                    executed_steps,
                    zisk_executor_time,
                    task_received_time,
                    public_outputs: Self::publics_u64_to_bytes(&exec_data.publics),
                }))
            }
            _ => {
                Err(CoordinatorError::InvalidRequest("Expected Execution result data".to_string()))
            }
        }
    }

    /// Extracts and converts execution timing information from DTO to internal representation.
    ///
    /// Serializes a `Vec<u64>` of public values into the byte format expected by
    /// [`zisk_common::PublicValues::new`]: a 32-byte zero header followed by each u64
    /// as 8 little-endian bytes (total = `ZISK_PUBLICS * 8 + 32` = 544 bytes).
    /// Returns an empty Vec if the slice doesn't have exactly `ZISK_PUBLICS` elements.
    fn publics_u64_to_bytes(publics: &[u64]) -> Vec<u8> {
        if publics.len() != ZISK_PUBLICS {
            return vec![];
        }
        let mut bytes = vec![0u8; 32]; // header
        for v in publics {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }

    /// # Parameters
    ///
    /// * `exec_time_dto` - The execution time DTO from the worker's response
    fn extract_execution_info(exec_time_dto: &ZiskExecutorTimeDto) -> ZiskExecutorTime {
        ZiskExecutorTime {
            total_duration: exec_time_dto.total_duration as u64,
            execution_duration: exec_time_dto.execution_duration as u64,
            count_and_plan_duration: exec_time_dto.count_and_plan_duration as u64,
            count_and_plan_mo_duration: exec_time_dto.count_and_plan_mo_duration as u64,
            asm_execution_duration: exec_time_dto
                .asm_execution_duration
                .as_ref()
                .map(|asm_info| AsmExecutionInfo { time: asm_info.time, mhz: asm_info.mhz }),
        }
    }

    /// Prints execution summary information from Phase 1 completion.
    ///
    /// Extracts and displays execution information from the first completed worker's
    /// contribution results, including timing, summary info, and key metrics.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job containing Phase 1 results
    fn print_execution_summary(&self, job: &Job) {
        // Find the first completed contribution result to extract WitnessInfo summary
        if let Some(contributions_results) = job.results.get(&JobPhase::Contributions) {
            if let Some((_worker_id, job_result)) = contributions_results.iter().next() {
                if let JobResultData::Challenges(contributions_result) = &job_result.data {
                    info!("Execution Summary: {}", contributions_result.witness_info.summary_info);
                }
            }
        }
    }

    /// Checks if all workers have completed Phase 1 contributions.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to check
    fn check_phase1_completion(&self, job: &Job, worker_id: &WorkerId) -> bool {
        let phase1_results_len =
            job.results.get(&JobPhase::Contributions).map(|r| r.len()).unwrap_or(0);

        let end_time = Utc::now();
        let phase_start_time =
            job.phase_start_time(&JobPhase::Contributions).unwrap_or_else(|| {
                error!("Missing start time for Phase1 in job {}", job.job_id);
                end_time
            });
        let duration = end_time.signed_duration_since(phase_start_time);
        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Get execution info from the worker's result
        let worker_result =
            job.results.get(&JobPhase::Contributions).and_then(|results| results.get(worker_id));

        let (asm_info_str, witness_time_str, delay_time_str) = if let Some(job_result) =
            worker_result
        {
            match &job_result.data {
                JobResultData::Challenges(contributions_result) => {
                    // Calculate delay: time from coordinator sending job to worker receiving task
                    let delay_duration = contributions_result
                        .task_received_time
                        .map(|task_received| task_received.signed_duration_since(phase_start_time))
                        .unwrap_or_else(chrono::Duration::zero);
                    let delay_ms = delay_duration.num_milliseconds().max(0) as f32;
                    let delay_str = format!(", Delay: {:.3}s", delay_ms / 1000.0);

                    let asm_str = contributions_result
                        .zisk_executor_time
                        .asm_execution_duration
                        .as_ref()
                        .map(|asm_info| {
                            format!(
                                ", Asm Execution: {:.3}s at {} MHz",
                                asm_info.time, asm_info.mhz
                            )
                        })
                        .unwrap_or_default();

                    let witness_str = format!(
                        ", Witness: {:.3}s",
                        contributions_result.witness_info.witness_time / 1000.0
                    );

                    (asm_str, witness_str, delay_str)
                }
                _ => (String::new(), String::new(), String::new()),
            }
        } else {
            (String::new(), String::new(), String::new())
        };

        info!(
            "[Phase1] {} finished phase 1 for {} ({}/{} workers done, Phase: {:.3}s{}{}{})",
            worker_id,
            job.job_id,
            phase1_results_len,
            job.workers.len(),
            duration_ms.as_secs_f32(),
            delay_time_str,
            witness_time_str,
            asm_info_str,
        );

        // Ensure we have results from all assigned workers before proceeding.
        // If not all workers have responded (and we're not in simulation mode),
        // return early and wait for more results.
        job.execution_mode.is_simulating() || phase1_results_len >= job.workers.len()
    }

    /// Checks if all workers have completed Execution phase (execution-only, no proofs).
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to check
    fn check_execution_completion(&self, job: &Job, worker_id: &WorkerId) -> bool {
        let execution_results_len =
            job.results.get(&JobPhase::Execution).map(|r| r.len()).unwrap_or(0);

        let end_time = Utc::now();
        let phase_start_time = job.phase_start_time(&JobPhase::Execution).unwrap_or_else(|| {
            error!("Missing start time for Execution phase in job {}", job.job_id);
            end_time
        });
        let duration = end_time.signed_duration_since(phase_start_time);
        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Get execution info from the worker's result
        let worker_result =
            job.results.get(&JobPhase::Execution).and_then(|results| results.get(worker_id));

        let (asm_info_str, delay_time_str) = if let Some(job_result) = worker_result {
            match &job_result.data {
                JobResultData::Execution(execution_result) => {
                    // Calculate delay: time from coordinator sending job to worker receiving task
                    let delay_duration = execution_result
                        .task_received_time
                        .map(|task_received| task_received.signed_duration_since(phase_start_time))
                        .unwrap_or_else(chrono::Duration::zero);
                    let delay_ms = delay_duration.num_milliseconds().max(0) as f32;
                    let delay_str = format!(", Delay: {:.3}s", delay_ms / 1000.0);

                    let asm_str = execution_result
                        .zisk_executor_time
                        .asm_execution_duration
                        .as_ref()
                        .map(|asm_info| {
                            format!(
                                ", Asm Execution: {:.3}s at {} MHz",
                                asm_info.time, asm_info.mhz
                            )
                        })
                        .unwrap_or_default();

                    (asm_str, delay_str)
                }
                _ => (String::new(), String::new()),
            }
        } else {
            (String::new(), String::new())
        };

        info!(
            "[Execution] {} finished execution for {} ({}/{} workers done, Phase: {:.3}s{}{})",
            worker_id,
            job.job_id,
            execution_results_len,
            job.workers.len(),
            duration_ms.as_secs_f32(),
            delay_time_str,
            asm_info_str,
        );

        // Ensure we have results from all assigned workers before proceeding.
        job.execution_mode.is_simulating() || execution_results_len >= job.workers.len()
    }

    /// Validates Phase 1 results and extracts challenge data with simulation mode handling.
    ///
    /// Performs comprehensive validation of all Phase 1 contribution results and extracts
    /// the cryptographic challenges needed for Phase 2 proof generation.
    ///
    /// # Parameters
    ///
    /// * `job` - Job containing all Phase 1 results to validate and process
    async fn validate_and_extract_challenges(
        &self,
        job: &Job,
    ) -> CoordinatorResult<Vec<ContributionsInfo>> {
        // Extract data we need while minimizing lock time
        let (simulating, phase1_results) = {
            let empty_results = HashMap::new();
            let phase1_results =
                job.results.get(&JobPhase::Contributions).unwrap_or(&empty_results).clone();
            let simulating = job.execution_mode.is_simulating();

            (simulating, phase1_results)
        };

        // Validate all results are successful
        // In simulation mode, we assume success since we're not running real distributed computation
        let all_successful =
            if simulating { true } else { phase1_results.values().all(|result| result.success) };

        if !all_successful {
            // Identify specific workers that failed for detailed error reporting
            let failed_workers: Vec<WorkerId> = phase1_results
                .iter()
                .filter_map(
                    |(worker_id, result)| {
                        if !result.success {
                            Some(worker_id.clone())
                        } else {
                            None
                        }
                    },
                )
                .collect();

            let reason =
                format!("Phase1 failed for workers: {failed_workers:?} in job {}", job.job_id);
            self.fail_job(&job.job_id, &reason).await?;

            return Err(CoordinatorError::WorkerError(reason));
        }

        // Extract and prepare challenges based on execution mode
        let challenges: Vec<ContributionsInfo> = if simulating {
            // Simulation mode: replicate single worker's challenges across all expected workers
            // This maintains algorithm correctness while using minimal computational resources
            let first_challenges = match phase1_results.values().next().unwrap().data {
                JobResultData::Challenges(ref values) => &values.challenges,
                _ => unreachable!("Expected Challenges data in Phase1 results"),
            };

            // Create challenge sets for each simulated worker using the same base challenges
            vec![first_challenges.clone(); phase1_results.len()].into_iter().flatten().collect()
        } else {
            // Standard mode: aggregate challenges from all participating workers
            // Each worker contributes their portion of the overall challenge space
            let (challenges, witness_info): (Vec<Vec<ContributionsInfo>>, Vec<WitnessInfo>) =
                phase1_results
                    .values()
                    .map(|results| match &results.data {
                        JobResultData::Challenges(values) => {
                            (values.challenges.clone(), values.witness_info.clone())
                        }
                        _ => unreachable!("Expected Challenges data in Phase1 results"),
                    })
                    .unzip();

            let first = witness_info.first().ok_or_else(|| {
                CoordinatorError::Internal(format!("No witness info found in job {}", job.job_id))
            })?;

            let mut mismatched_workers = Vec::new();

            for (worker_idx, info) in witness_info.iter().enumerate() {
                if info.publics != first.publics || info.proof_values != first.proof_values {
                    mismatched_workers.push((worker_idx, info));
                }
            }

            if !mismatched_workers.is_empty() {
                // Format detailed mismatch report
                let mismatch_report: Vec<String> = mismatched_workers
                    .iter()
                    .map(|(idx, info)| {
                        format!(
                            "Worker {} differs: publics={:?}, proof_values={:?}",
                            idx, info.publics, info.proof_values
                        )
                    })
                    .collect();

                return Err(CoordinatorError::Internal(format!(
                    "WitnessInfo mismatch in job {}:\n{}",
                    job.job_id,
                    mismatch_report.join("\n")
                )));
            }

            // Flatten all worker contributions into unified challenge vector
            // Maintains worker indexing and airgroup assignments for proper coordination
            challenges.into_iter().flatten().collect()
        };

        Ok(challenges)
    }

    fn collect_challenges_dto(&self, job: &Job) -> Vec<ChallengesDto> {
        let mut challenges_dto = Vec::new();

        for challenge in job.challenges.as_ref().unwrap() {
            challenges_dto.push(ChallengesDto {
                worker_index: challenge.worker_index,
                airgroup_id: challenge.airgroup_id as u32,
                challenge: challenge.challenge.to_vec(),
            })
        }

        challenges_dto
    }
}
