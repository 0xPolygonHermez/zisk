use crate::{
    coordinator::exec_stats_from_job,
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
};
use chrono::Utc;
use colored::Colorize;
use std::collections::{BTreeMap, BTreeSet};
use std::{sync::atomic::Ordering, time::Duration};
use tracing::{error, info, warn};
use zisk_cluster_common::{
    AggDispatch, AggNode, AggParamsDto, AggProofData, AggSet, AggTaskKind, CoordinatorMessageDto,
    ExecuteTaskRequestDto, ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto,
    ExecuteTaskResponseResultDataDto, Job, JobId, JobPhase, JobResultData, JobState, ProofStarkDto,
    WorkerId,
};
use zisk_common::Proof;

use crate::Coordinator;

impl Coordinator {
    /// Handles aggregation completion, finalizes the job if all steps are done.
    ///    
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing final proof or failure details
    pub(super) async fn handle_recurser_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = &execute_task_response.job_id;

        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?
        };

        // Hold the write lock through all state mutations so a racing
        // fail_job cannot park the worker SettingUp between is_resolved
        // and our worker-state writes. Drop it before sibling calls that
        // take their own.
        let mut job = job_entry.write().await;

        if job.state().is_resolved() {
            return Ok(());
        }

        if !execute_task_response.success {
            drop(job);
            let reason = format!("Aggregation failed in job {}", job_id);
            self.fail_job(job_id, &reason).await?;

            return Err(CoordinatorError::Internal(reason));
        }

        // Bind the payload to the phase mutated below (see
        // `validate_response_phase`). After the failure branch: a failed
        // response carries no `result_data` to bind.
        Self::validate_response_phase(&job, &execute_task_response)?;

        // Check against the task the worker was given. Neither the payload shape nor
        // current scheduler state identifies which task a response answers: an
        // absorb's ack can arrive after its node has closed.
        let worker_id = execute_task_response.worker_id.clone();

        let Some(scheduler) = job.agg.as_ref() else {
            return Err(CoordinatorError::InvalidRequest(format!(
                "Worker {worker_id} sent an aggregation result for {job_id} before aggregation \
                 started"
            )));
        };
        // Nothing outstanding: either a duplicate, or a worker submitting a result
        // for a task it was never given. Both are rejected -- acting on one would
        // put two tasks on a worker, and the other completes the job from nothing.
        let Some(expected) = scheduler.outstanding(&worker_id) else {
            return Err(CoordinatorError::InvalidRequest(format!(
                "Worker {worker_id} sent an aggregation result for {job_id} with no task \
                 outstanding"
            )));
        };

        match (expected, execute_task_response.result_data) {
            // Absorb-only: acks with no proof, and that ack releases the next task.
            (AggTaskKind::Absorb, Some(ExecuteTaskResponseResultDataDto::FinalProof(ack)))
                if ack.proof_data.is_empty() =>
            {
                let scheduler = job.agg.as_mut().expect("checked above");
                let next = scheduler.on_ack(&worker_id);
                let freed = scheduler.take_released();
                drop(job);

                self.release_donors(job_id, &freed).await;
                if let Some(next) = next {
                    self.dispatch_agg(job_id, next).await?;
                }
                Ok(())
            }

            // An intermediate node's folded subtree.
            (
                AggTaskKind::Drain,
                Some(ExecuteTaskResponseResultDataDto::PartialAggProofs(proofs)),
            ) => {
                let node =
                    job.agg.as_ref().and_then(|s| s.live_node(&worker_id)).ok_or_else(|| {
                        CoordinatorError::Internal(format!(
                            "Node {worker_id} owes {job_id} a subtree but is not live"
                        ))
                    })?;
                // Coverage is per airgroup, not per node: a leaf only has a
                // contribution entry for the airgroups it had instances in, so
                // stamping the node's whole leaf set onto every exported proof makes
                // the next fold cancel with "missing contribution".
                let per_airgroup = Self::airgroup_coverage(node);
                drop(job);
                self.absorb_node_export(job_id, &worker_id, proofs, per_airgroup).await
            }

            // The job's final proof. An empty payload would complete the job from
            // nothing, so it falls through to the rejection below.
            (
                AggTaskKind::DrainFinal,
                Some(ExecuteTaskResponseResultDataDto::FinalProof(proof_data)),
            ) if !proof_data.proof_data.is_empty() => {
                let agg_worker_id = worker_id;

                // Decode before touching anything: failing after would leave the job
                // in Recurse with no node, nothing in flight, and its aggregator
                // handed back to the pool.
                let zisk_proof = bincode::serde::decode_from_slice::<Proof, _>(
                    &proof_data.proof_data,
                    bincode::config::standard(),
                )
                .map(|(v, _)| v)
                .map_err(|e| {
                    CoordinatorError::Internal(format!("Failed to deserialize proof: {}", e))
                })?;

                let scheduler = job.agg.as_mut().expect("checked above");
                scheduler.take_live(&agg_worker_id);
                if let Some(stray) = scheduler.on_ack(&agg_worker_id) {
                    warn!(
                        "Discarding aggregation task queued for {} after {} completed",
                        stray.worker, job_id
                    );
                }

                self.workers_pool.release_computing_to_ready(&agg_worker_id, job_id).await;
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

                let phase1_time =
                    job.phase_start_time(&JobPhase::Contributions).unwrap_or_else(|| {
                        error!("Missing start time for Phase1 in job {}", job.job_id);
                        end_time
                    });
                let phase2_time = job.phase_start_time(&JobPhase::Prove).unwrap_or_else(|| {
                    error!("Missing start time for Phase2 in job {}", job.job_id);
                    end_time
                });
                let phase3_time = job.phase_start_time(&JobPhase::Recurse).unwrap_or_else(|| {
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

                let metadata_str = Self::format_job_metadata(job.metadata.as_ref());

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
                                        let duration =
                                            result.end_time.signed_duration_since(start_time);
                                        (worker_id.clone(), duration.num_milliseconds())
                                    })
                                    .collect();

                                if durations_ms.len() > 1 {
                                    durations_ms.sort_by_key(|(_, duration)| *duration);

                                    let (best_worker, best_duration) = &durations_ms[0];
                                    let (worst_worker, worst_duration) =
                                        durations_ms.last().unwrap();

                                    let avg_duration =
                                        durations_ms.iter().map(|(_, d)| d).sum::<i64>() as f64
                                            / durations_ms.len() as f64;

                                    let diff_percentage = if *best_duration > 0 {
                                        ((*worst_duration - *best_duration) as f64
                                            / *best_duration as f64)
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
                                            if let JobResultData::Challenges(contrib) = &result.data
                                            {
                                                contrib.task_received_time.map(|task_received| {
                                                    let delay = task_received
                                                        .signed_duration_since(start_time);
                                                    (
                                                        worker_id.clone(),
                                                        delay.num_milliseconds().max(0),
                                                    )
                                                })
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();

                                    if !delays_ms.is_empty() {
                                        delays_ms.sort_by_key(|(_, delay)| *delay);
                                        let (best_delay_worker, best_delay) = &delays_ms[0];
                                        let (worst_delay_worker, worst_delay) =
                                            delays_ms.last().unwrap();
                                        let avg_delay =
                                            delays_ms.iter().map(|(_, d)| d).sum::<i64>() as f64
                                                / delays_ms.len() as f64;

                                        let delay_diff_percentage = if *best_delay > 0 {
                                            ((*worst_delay - *best_delay) as f64
                                                / *best_delay as f64)
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
                                            if let JobResultData::Challenges(contrib) = &result.data
                                            {
                                                Some((
                                                    worker_id.clone(),
                                                    contrib.witness_info.witness_time,
                                                ))
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
                                        let avg_witness = witness_times
                                            .iter()
                                            .map(|(_, t)| *t as f64)
                                            .sum::<f64>()
                                            / witness_times.len() as f64;

                                        let witness_diff_percentage = if *best_witness > 0.0 {
                                            ((*worst_witness - *best_witness) as f64
                                                / *best_witness as f64)
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
                                            if let JobResultData::Challenges(contrib) = &result.data
                                            {
                                                contrib
                                                    .zisk_executor_time
                                                    .asm_execution_duration
                                                    .as_ref()
                                                    .map(|asm| {
                                                        (worker_id.clone(), asm.time, asm.mhz)
                                                    })
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
                                        let avg_asm = asm_times
                                            .iter()
                                            .map(|(_, t, _)| *t as f64)
                                            .sum::<f64>()
                                            / asm_times.len() as f64;

                                        let asm_diff_percentage = if *best_asm > 0.0 {
                                            ((*worst_asm - *best_asm) as f64 / *best_asm as f64)
                                                * 100.0
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
                let uptime =
                    humantime::format_duration(Duration::from_secs(total_secs)).to_string();

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
                                warn!(
                                    "Failed to serialize proof for event on job {}: {}",
                                    job_id, e
                                );
                                vec![]
                            }),
                        None => vec![],
                    };
                    let stats = exec_stats_from_job(&job);
                    CoordinatorJobEvent::Completed(CoordinatorJobResult::Prove {
                        proof_bytes,
                        stats,
                    })
                };

                // Release job lock before calling post_launch_proof
                drop(job);

                self.fire_job_event(job_id, prove_event).await;

                self.post_launch_proof(job_id).await?;

                Ok(())
            }

            (expected, _) => Err(CoordinatorError::InvalidRequest(format!(
                "Worker {worker_id} sent a response for {job_id} that does not answer the \
                 {expected:?} task it was given"
            ))),
        }
    }

    /// The set a worker's phase-2 result represents: one proof per airgroup,
    /// covering only that worker, and resident on it.
    /// Takes the proofs out of the stored phase-2 result rather than cloning them:
    /// the node retains them for the rest of phase 3, and a second copy in
    /// `job.results` would roughly double peak proof residency. Nothing reads that
    /// payload again -- the phase-2 stats use only the result's timestamp.
    pub(super) fn worker_agg_set(job: &mut Job, worker_id: &WorkerId) -> CoordinatorResult<AggSet> {
        let worker_index = job.workers.iter().position(|w| w == worker_id).ok_or_else(|| {
            CoordinatorError::InvalidRequest(format!(
                "Worker {worker_id} is not assigned to {}",
                job.job_id
            ))
        })? as u32;

        let job_id = job.job_id.clone();
        let results = job.results.get_mut(&JobPhase::Prove).ok_or_else(|| {
            CoordinatorError::InvalidRequest(format!("No Phase2 results for {job_id}"))
        })?;
        let result = results.get_mut(worker_id).ok_or_else(|| {
            CoordinatorError::InvalidRequest(format!(
                "Worker {worker_id} has not completed Phase2 for {job_id}"
            ))
        })?;
        let JobResultData::AggProofs(proofs) = &mut result.data else {
            return Err(CoordinatorError::InvalidRequest(
                "Expected AggProofs data for Phase2".to_string(),
            ));
        };

        // Coverage tells the scheduler the job is done and comes from an untrusted
        // worker. A leaf covers its own index and nothing else.
        let covers: BTreeSet<u32> =
            proofs.iter().flat_map(|p| p.worker_indexes.iter().copied()).collect();
        if covers != BTreeSet::from([worker_index]) {
            return Err(CoordinatorError::InvalidRequest(format!(
                "Worker {worker_id} claims its Phase2 proof covers {covers:?}, expected \
                 only {worker_index}"
            )));
        }

        // Only now: taking first would destroy the leaf on a rejected payload, and
        // the duplicate guard blocks any resubmission.
        Ok(AggSet { covers, proofs: std::mem::take(proofs), location: worker_id.clone() })
    }

    /// Which leaf workers each airgroup's proofs actually cover, from the sets this
    /// node absorbed.
    fn airgroup_coverage(node: &AggNode) -> BTreeMap<u64, BTreeSet<u32>> {
        let mut per_airgroup: BTreeMap<u64, BTreeSet<u32>> = BTreeMap::new();
        for input in &node.inputs {
            for proof in &input.proofs {
                per_airgroup
                    .entry(proof.airgroup_id)
                    .or_default()
                    .extend(proof.worker_indexes.iter().copied());
            }
        }
        per_airgroup
    }

    /// Feed a node's folded subtree back into the scheduler as an available set. It
    /// stays resident on that node, so it only travels if the next fold lands
    /// elsewhere.
    async fn absorb_node_export(
        &self,
        job_id: &JobId,
        worker_id: &WorkerId,
        proofs: Vec<ProofStarkDto>,
        per_airgroup: BTreeMap<u64, BTreeSet<u32>>,
    ) -> CoordinatorResult<()> {
        // The airgroup ids come from an untrusted worker and end up indexing a raw
        // Vec in the receiving worker's proofman. Keep only the ones this node
        // actually held, and reject a repeat.
        let mut seen = BTreeSet::new();
        let mut folded = Vec::with_capacity(proofs.len());
        for proof in proofs {
            let Some(covers) = per_airgroup.get(&proof.airgroup_id) else {
                // proofman drains one proof per airgroup, including ones this node
                // never absorbed; those carry nothing to account for.
                continue;
            };
            if !seen.insert(proof.airgroup_id) {
                return Err(CoordinatorError::InvalidRequest(format!(
                    "Node {worker_id} returned airgroup {} twice for {job_id}",
                    proof.airgroup_id
                )));
            }
            folded.push(AggProofData {
                airgroup_id: proof.airgroup_id,
                values: proof.values,
                worker_indexes: covers.iter().copied().collect(),
            });
        }

        if folded.len() != per_airgroup.len() {
            return Err(CoordinatorError::InvalidRequest(format!(
                "Node {worker_id} returned {} of {} airgroups for {job_id}",
                folded.len(),
                per_airgroup.len()
            )));
        }
        let (dispatches, freed) = {
            let jobs_map = self.jobs.read().await;
            let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
            let mut job = job_entry.write().await;

            // The lock was released in between, so the job may have been failed.
            if job.state().is_resolved() {
                return Ok(());
            }

            let scheduler = job
                .agg
                .as_mut()
                .ok_or_else(|| CoordinatorError::Internal("No scheduler".into()))?;

            // A replay finds nothing to consume; folding it again would trip
            // proofman's duplicate-index check.
            if scheduler.take_live(worker_id).is_none() {
                return Ok(());
            }
            let released = scheduler.on_ack(worker_id);

            info!(
                "[Phase3] {job_id} {worker_id} returned a subtree covering {:?}",
                per_airgroup.values().flatten().copied().collect::<BTreeSet<_>>()
            );

            let set = AggSet {
                covers: per_airgroup.values().flatten().copied().collect(),
                proofs: folded,
                location: worker_id.clone(),
            };
            // One export can yield both: what the ack released, and the new fold.
            let dispatches: Vec<_> =
                released.into_iter().chain(scheduler.on_set_ready(set)).collect();
            (dispatches, scheduler.take_released())
        };

        self.release_donors(job_id, &freed).await;

        // Send them all before reporting: abandoning the second would leave it
        // marked in-flight forever with nothing to retry it.
        let mut first_error = None;
        for dispatch in dispatches {
            if let Err(e) = self.dispatch_agg(job_id, dispatch).await {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Whether losing this worker costs the job anything. One that has handed its
    /// set over holds nothing the coordinator cannot replace, so the job carries on
    /// without it. Anything else is fatal, as it was before distribution.
    pub(super) async fn agg_worker_is_dispensable(
        &self,
        job_id: &JobId,
        worker_id: &WorkerId,
    ) -> bool {
        let jobs_map = self.jobs.read().await;
        let Some(job_entry) = jobs_map.get(job_id) else {
            return false;
        };
        let job = job_entry.read().await;
        // No scheduler: a wrap or aggregate-proofs job, which also runs in Recurse
        // and depends on its worker as it always did.
        job.agg.as_ref().is_some_and(|s| !s.holds_work(worker_id))
    }

    /// Return donors to the pool. Guarded: one may have disconnected or been parked
    /// awaiting recovery, and an unconditional write would resurrect it.
    pub(super) async fn release_donors(&self, job_id: &JobId, donors: &[WorkerId]) {
        for donor in donors {
            self.workers_pool.release_computing_to_ready(donor, job_id).await;
        }
    }

    /// Send a scheduled fold.
    pub(super) async fn dispatch_agg(
        &self,
        job_id: &JobId,
        dispatch: AggDispatch,
    ) -> CoordinatorResult<()> {
        let step = match (dispatch.last_proof, dispatch.final_proof) {
            (false, _) => "absorb",
            (true, false) => "fold",
            (true, true) => "fold+final",
        };
        info!(
            "[Phase3] {job_id} {} {step}: {} proof(s) in, now covers {:?}",
            dispatch.worker,
            dispatch.proofs.len(),
            dispatch.covers
        );

        self.send_agg_task(job_id, &dispatch).await
    }

    /// Re-sends a node's retained inputs to a reconnecting worker. The inputs are
    /// kept until the node exports, so replay is idempotent.
    pub(super) async fn replay_inflight_agg_task_if_recurser(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> CoordinatorResult<()> {
        let dispatch = {
            let jobs_map = self.jobs.read().await;
            let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
            let mut job = job_entry.write().await;
            // Covers the open node too: without distribution the aggregator stays
            // open for the whole job and never becomes live, so keying replay on
            // `live` alone would miss the case this exists for.
            let Some(dispatch) = job.agg.as_mut().and_then(|s| s.replay_for(worker_id)) else {
                return Ok(());
            };
            dispatch
        };

        info!("Replaying aggregation inputs to reconnected node {worker_id}");
        self.send_agg_task(job_id, &dispatch).await
    }

    /// Sends an aggregation task to a node.
    async fn send_agg_task(&self, job_id: &JobId, dispatch: &AggDispatch) -> CoordinatorResult<()> {
        let proof_type = {
            let jobs_map = self.jobs.read().await;
            let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
            let proof_type = job_entry.read().await.proof_type;
            proof_type
        };

        let agg_proofs: Vec<ProofStarkDto> = dispatch
            .proofs
            .iter()
            .map(|p| ProofStarkDto {
                airgroup_id: p.airgroup_id,
                values: p.values.clone(),
                worker_indexes: p.worker_indexes.clone(),
            })
            .collect();

        let req = ExecuteTaskRequestDto {
            worker_id: dispatch.worker.clone(),
            job_id: job_id.clone(),
            params: ExecuteTaskRequestTypeDto::AggParams(AggParamsDto {
                agg_proofs,
                last_proof: dispatch.last_proof,
                final_proof: dispatch.final_proof,
                proof_type,
                keep_resident: dispatch.keep_resident,
                reset_state: dispatch.reset_state,
            }),
            metadata: None,
        };

        self.workers_pool
            .send_message(&dispatch.worker, CoordinatorMessageDto::ExecuteTaskRequest(req))
            .await
    }
}
