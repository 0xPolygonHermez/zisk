use crate::{
    coordinator::exec_stats_from_job,
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
};
use chrono::Utc;
use colored::Colorize;
use std::{sync::atomic::Ordering, time::Duration};
use tracing::{error, info, warn};
use zisk_cluster_common::{
    AggParamsDto, AggProofData, CoordinatorMessageDto, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, Job,
    JobId, JobPhase, JobResultData, JobState, ProofStarkDto, WorkerId, WorkerState,
};
use zisk_common::{Proof, ProofKind};

use crate::Coordinator;

impl Coordinator {
    /// Handles aggregation completion, finalizes the job if all steps are done.
    ///    
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing final proof or failure details
    pub(super) async fn handle_aggregation_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = &execute_task_response.job_id;

        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?
        };

        // If job has Failed, mark worker as Idle and return early
        if matches!(job_entry.read().await.state(), JobState::Failed) {
            self.workers_pool
                .mark_worker_with_state(&execute_task_response.worker_id, WorkerState::Ready)
                .await?;
            return Ok(());
        }

        // An aggregation request has failed, fail the job
        if !execute_task_response.success {
            let reason = format!("Aggregation failed in job {}", job_id);
            self.fail_job(job_id, &reason).await?;

            return Err(CoordinatorError::Internal(reason));
        }

        // Extract the proof data
        let proof_data = match execute_task_response.result_data {
            Some(ExecuteTaskResponseResultDataDto::FinalProof(final_proof)) => final_proof,
            _ => {
                return Err(CoordinatorError::InvalidRequest(
                    "Expected FinalProof result data for Aggregation".to_string(),
                ));
            }
        };

        // Empty proof_data means this was an intermediate aggregation step.
        // Clear the in-flight slot and dispatch the next queued task, if any.
        if proof_data.proof_data.is_empty() {
            self.dispatch_next_agg_task(job_id).await?;
            return Ok(());
        }

        let mut job = job_entry.write().await;

        let agg_worker_id = &job.agg_worker_id.as_ref().unwrap().clone();

        // Mark the aggregation worker as Idle
        self.workers_pool.mark_worker_with_state(agg_worker_id, WorkerState::Ready).await?;

        // Finalize completed job
        let zisk_proof = bincode::serde::decode_from_slice::<Proof, _>(
            &proof_data.proof_data,
            bincode::config::standard(),
        )
        .map(|(v, _)| v)
        .map_err(|e| CoordinatorError::Internal(format!("Failed to deserialize proof: {}", e)))?;
        job.proof = Some(zisk_proof);
        job.executed_steps = Some(proof_data.executed_steps);
        job.instances = Some(proof_data.instances);

        job.change_state(JobState::Completed);

        crate::metrics::record_job_terminal(
            crate::metrics::OUTCOME_SUCCESS,
            &job.workers,
            job.phase_start_time(&JobPhase::Contributions),
        );

        let end_time = Utc::now();

        let phase1_time = job.phase_start_time(&JobPhase::Contributions).unwrap_or_else(|| {
            error!("Missing start time for Phase1 in job {}", job.job_id);
            end_time
        });
        let phase2_time = job.phase_start_time(&JobPhase::Prove).unwrap_or_else(|| {
            error!("Missing start time for Phase2 in job {}", job.job_id);
            end_time
        });
        let phase3_time = job.phase_start_time(&JobPhase::Aggregate).unwrap_or_else(|| {
            error!("Missing start time for Phase3 in job {}", job.job_id);
            end_time
        });

        let phase1_duration = phase2_time.signed_duration_since(phase1_time);
        let phase2_duration = phase3_time.signed_duration_since(phase2_time);
        let phase3_duration = end_time.signed_duration_since(phase3_time);

        info!(
            "[Phase3] WorkerId {} done, phase 3 completed for {} ({:.3}s)",
            agg_worker_id,
            job_id,
            phase3_duration.as_seconds_f32()
        );

        let duration = Duration::from_millis(job.duration_ms.unwrap_or(0));

        let header = format!("[Job] Finished {} successfully ✔", job_id).green();
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
            "{} {} ({:.3}s+{:.3}s+{:.3}s) {} {} Capacity: {}{}",
            header,
            duration_str,
            phase1_duration.as_seconds_f32(),
            phase2_duration.as_seconds_f32(),
            phase3_duration.as_seconds_f32(),
            steps_str,
            instances_str,
            job.compute_capacity,
            metadata_str,
        );

        let workers = job.workers.clone();

        if workers.len() > 1 {
            for phase in [JobPhase::Contributions, JobPhase::Prove] {
                if let Some(results) = job.results.get(&phase) {
                    if let Some(start_time) = job.phase_start_time(&phase) {
                        let mut durations_ms: Vec<(WorkerId, i64)> = results
                            .iter()
                            .map(|(worker_id, result)| {
                                let duration = result.end_time.signed_duration_since(start_time);
                                (worker_id.clone(), duration.num_milliseconds())
                            })
                            .collect();

                        if durations_ms.len() > 1 {
                            durations_ms.sort_by_key(|(_, duration)| *duration);

                            let (best_worker, best_duration) = &durations_ms[0];
                            let (worst_worker, worst_duration) = durations_ms.last().unwrap();

                            let avg_duration = durations_ms.iter().map(|(_, d)| d).sum::<i64>()
                                as f64
                                / durations_ms.len() as f64;

                            let diff_percentage = if *best_duration > 0 {
                                ((*worst_duration - *best_duration) as f64 / *best_duration as f64)
                                    * 100.0
                            } else {
                                0.0
                            };

                            info!(
                                "[Job] {:?} Performance for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                                phase,
                                job_id,
                                avg_duration / 1000.0,
                                best_worker,
                                *best_duration as f64 / 1000.0,
                                worst_worker,
                                *worst_duration as f64 / 1000.0,
                                diff_percentage
                            );
                        }

                        // For Phase 1, also show delay, witness, and ASM execution statistics
                        if phase == JobPhase::Contributions && durations_ms.len() > 1 {
                            // Extract delay times (coordinator send to worker start)
                            let mut delays_ms: Vec<(WorkerId, i64)> = results
                                .iter()
                                .filter_map(|(worker_id, result)| {
                                    if let JobResultData::Challenges(contrib) = &result.data {
                                        contrib.task_received_time.map(|task_received| {
                                            let delay =
                                                task_received.signed_duration_since(start_time);
                                            (worker_id.clone(), delay.num_milliseconds().max(0))
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !delays_ms.is_empty() {
                                delays_ms.sort_by_key(|(_, delay)| *delay);
                                let (best_delay_worker, best_delay) = &delays_ms[0];
                                let (worst_delay_worker, worst_delay) = delays_ms.last().unwrap();
                                let avg_delay = delays_ms.iter().map(|(_, d)| d).sum::<i64>()
                                    as f64
                                    / delays_ms.len() as f64;

                                let delay_diff_percentage = if *best_delay > 0 {
                                    ((*worst_delay - *best_delay) as f64 / *best_delay as f64)
                                        * 100.0
                                } else {
                                    0.0
                                };

                                info!(
                                    "[Job] Contributions Delay for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                                    job_id,
                                    avg_delay / 1000.0,
                                    best_delay_worker,
                                    *best_delay as f64 / 1000.0,
                                    worst_delay_worker,
                                    *worst_delay as f64 / 1000.0,
                                    delay_diff_percentage
                                );
                            }

                            // Extract witness times
                            let mut witness_times: Vec<(WorkerId, f32)> = results
                                .iter()
                                .filter_map(|(worker_id, result)| {
                                    if let JobResultData::Challenges(contrib) = &result.data {
                                        Some((worker_id.clone(), contrib.witness_info.witness_time))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !witness_times.is_empty() {
                                witness_times.sort_by(|(_, a), (_, b)| {
                                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                let (best_witness_worker, best_witness) = &witness_times[0];
                                let (worst_witness_worker, worst_witness) =
                                    witness_times.last().unwrap();
                                let avg_witness =
                                    witness_times.iter().map(|(_, t)| *t as f64).sum::<f64>()
                                        / witness_times.len() as f64;

                                let witness_diff_percentage = if *best_witness > 0.0 {
                                    ((*worst_witness - *best_witness) as f64 / *best_witness as f64)
                                        * 100.0
                                } else {
                                    0.0
                                };

                                info!(
                                    "[Job] Contributions Witness for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                                    job_id,
                                    avg_witness / 1000.0,
                                    best_witness_worker,
                                    *best_witness as f64 / 1000.0,
                                    worst_witness_worker,
                                    *worst_witness as f64 / 1000.0,
                                    witness_diff_percentage
                                );
                            }

                            // Extract ASM execution times
                            let mut asm_times: Vec<(WorkerId, f32, f32)> = results
                                .iter()
                                .filter_map(|(worker_id, result)| {
                                    if let JobResultData::Challenges(contrib) = &result.data {
                                        contrib
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
                                let (worst_asm_worker, worst_asm, worst_mhz) =
                                    asm_times.last().unwrap();
                                let avg_asm =
                                    asm_times.iter().map(|(_, t, _)| *t as f64).sum::<f64>()
                                        / asm_times.len() as f64;

                                let asm_diff_percentage = if *best_asm > 0.0 {
                                    ((*worst_asm - *best_asm) as f64 / *best_asm as f64) * 100.0
                                } else {
                                    0.0
                                };

                                info!(
                                    "[Job] Contributions ASM for {} - Avg: {:.3}s, Best: {} ({:.3}s @ {:.1}MHz), Worst: {} ({:.3}s @ {:.1}MHz), Diff: {:.1}%",
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
                }
            }
        }

        let duration = Utc::now().signed_duration_since(self.start_time_utc);
        let total_secs = duration.num_seconds().max(0) as u64; // avoid negative durations
        let uptime = humantime::format_duration(Duration::from_secs(total_secs)).to_string();

        info!(
            "[Coordinator] Started at {} UTC — Uptime: {}",
            self.start_time_utc.format("%Y-%m-%d %H:%M:%S"),
            uptime
        );

        info!(
            "[Coordinator] Registrations: {} Reconnections: {}",
            self.registrations.load(Ordering::Relaxed),
            self.reconnections.load(Ordering::Relaxed)
        );

        // Build proof bytes and stats for the event before releasing the lock
        let prove_event = {
            let proof_bytes = match job.proof.as_ref() {
                Some(p) => bincode::serde::encode_to_vec(p, bincode::config::standard())
                    .unwrap_or_else(|e| {
                        warn!("Failed to serialize proof for event on job {}: {}", job_id, e);
                        vec![]
                    }),
                None => vec![],
            };
            let stats = exec_stats_from_job(&job);
            CoordinatorJobEvent::Completed(CoordinatorJobResult::Prove { proof_bytes, stats })
        };

        // Release job lock before calling post_launch_proof
        drop(job);

        self.fire_job_event(job_id, prove_event).await;

        self.post_launch_proof(job_id).await?;

        Ok(())
    }

    /// Collects the proofs stored from a worker for aggregation.
    ///     
    /// # Parameters
    ///
    /// * `job` - Reference to the job containing proof results
    /// * `agg_worker_id` - Worker ID assigned as the aggregator
    /// * `worker_id` - Worker ID whose proofs are being collected
    pub(super) fn collect_worker_proofs(
        &self,
        job: &Job,
        agg_worker_id: &WorkerId,
        worker_id: &WorkerId,
    ) -> CoordinatorResult<Vec<AggProofData>> {
        Ok(if worker_id == agg_worker_id {
            vec![]
        } else {
            let job_results = job.results.get(&JobPhase::Prove).unwrap();

            let job_result = job_results.get(worker_id).ok_or(CoordinatorError::InvalidRequest(
                format!("Worker {worker_id} has not completed Phase2 for {}", job.job_id),
            ))?;

            match &job_result.data {
                JobResultData::AggProofs(values) => values.clone(),
                _ => {
                    return Err(CoordinatorError::InvalidRequest(
                        "Expected AggProofs data for Phase2".to_string(),
                    ));
                }
            }
        })
    }

    /// Re-sends the in-flight aggregation task to a reconnecting aggregator.
    /// No-op if the worker is not the aggregator for this job, or if no task is in-flight.
    pub(super) async fn replay_inflight_agg_task_if_aggregator(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> CoordinatorResult<()> {
        let inflight = {
            let jobs_map = self.jobs.read().await;
            let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
            let job = job_entry.read().await;

            // Only replay for the designated aggregator
            if job.agg_worker_id.as_ref() != Some(worker_id) {
                return Ok(());
            }

            job.agg_task_inflight.clone()
        };

        if let Some(task) = inflight {
            info!("Replaying in-flight agg task to reconnected aggregator {worker_id}");
            self.send_aggregation_task(
                job_id,
                worker_id,
                task.proofs,
                task.all_done,
                task.proof_type,
            )
            .await?;
        }

        Ok(())
    }

    /// Clears the in-flight slot and dispatches the next queued aggregation task, if any.
    /// Called after the aggregator acknowledges an intermediate step.
    async fn dispatch_next_agg_task(&self, job_id: &JobId) -> CoordinatorResult<()> {
        let (task, agg_worker_id) = {
            let jobs_map = self.jobs.read().await;
            let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
            let mut job = job_entry.write().await;

            job.agg_task_inflight = None;

            let Some(task) = job.agg_task_queue.pop_front() else {
                return Ok(());
            };
            job.agg_task_inflight = Some(task.clone());
            let agg_worker_id = job
                .agg_worker_id
                .clone()
                .ok_or_else(|| CoordinatorError::Internal("No aggregator assigned".into()))?;
            (task, agg_worker_id)
        };

        self.send_aggregation_task(
            job_id,
            &agg_worker_id,
            task.proofs,
            task.all_done,
            task.proof_type,
        )
        .await
    }

    /// Sends an aggregation task to the designated aggregator worker.
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the job being processed
    /// * `agg_worker_id` - Worker ID assigned as the aggregator
    /// * `proofs` - List of proofs to aggregate
    /// * `all_done` - Indicates if this is the final aggregation step
    pub(super) async fn send_aggregation_task(
        &self,
        job_id: &JobId,
        agg_worker_id: &WorkerId,
        proofs: Vec<AggProofData>,
        all_done: bool,
        proof_type: ProofKind,
    ) -> CoordinatorResult<()> {
        let proofs: Vec<ProofStarkDto> = proofs
            .into_iter()
            .map(|p| ProofStarkDto {
                airgroup_id: p.airgroup_id,
                values: p.values,
                worker_idx: p.worker_idx,
            })
            .collect();

        let req = ExecuteTaskRequestDto {
            worker_id: agg_worker_id.clone(),
            job_id: job_id.clone(),
            params: ExecuteTaskRequestTypeDto::AggParams(AggParamsDto {
                agg_proofs: proofs,
                last_proof: all_done,
                final_proof: all_done,
                proof_type,
            }),
        };

        let message = CoordinatorMessageDto::ExecuteTaskRequest(req);

        self.workers_pool.send_message(agg_worker_id, message).await?;

        Ok(())
    }
}
