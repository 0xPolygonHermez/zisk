//! Chunk collector component.
//!
//! ## Overview
//!
//! During witness computation, secondary state machines need data from
//! specific execution chunks. This component:
//!
//! 1. Determines which chunks each instance needs (via checkpoints)
//! 2. Orders chunks for optimal parallel processing
//! 3. Executes chunks and routes data to the appropriate collectors
//!
//! ## Chunk Ordering Strategy
//!
//! Uses a greedy algorithm that prioritizes completing instances that need
//! fewer remaining chunks, minimizing time-to-first-completion.

use crossbeam::atomic::AtomicCell;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Instant,
};
use tracing::error;
use zisk_common::{
    CheckPoint, ChunkId, EmuTrace, ExecutorStatsHandle, Instance, PayloadType, Stats,
};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};
use crate::{state::ChunkCollector, ExecutionState, StaticDataBusCollect, StaticSMBundle};
use asm_runner::AsmRunnerRH;

/// Per-instance chunk-collector slot map. Same shape as
/// [`crate::ChunkCollectorStore::inner`].
type CollectorSlots = Arc<RwLock<HashMap<usize, Vec<Option<ChunkCollector>>>>>;

/// Borrowed context handed to each rayon worker. Bundles the 14
/// references the chunk-processing loop needs so signatures stay
/// readable. Constructed once per `collect()` call.
struct WorkerCtx<'a, F: PrimeField64> {
    // ── Work feed ──
    next_chunk: &'a AtomicUsize,
    ordered_chunks: &'a [usize],
    chunks_to_execute: &'a [Vec<usize>],
    data_buses: &'a [Mutex<Option<StaticDataBusCollect<PayloadType, F>>>],

    // ── Inputs ──
    zisk_rom: &'a ZiskRom,
    min_traces: &'a [EmuTrace],
    pctx: &'a ProofCtx<F>,

    // ── Output sinks ──
    collectors_by_instance: &'a CollectorSlots,
    n_chunks_left: &'a [AtomicUsize],
    collect_start_times: &'a [AtomicCell<Option<Instant>>],
    stats: &'a ExecutorStatsHandle,

    // ── Indexing ──
    global_ids_map: &'a HashMap<usize, usize>,
    global_id_chunks: &'a HashMap<usize, Vec<usize>>,

    // ── Error sink ──
    errors: &'a Mutex<Vec<String>>,
}

/// Push a pre-formatted error message into the shared error sink.
/// Silently drops the error if the mutex is poisoned — used in worker
/// code where panicking further would just compound failure.
#[inline]
fn push_error(errors: &Mutex<Vec<String>>, message: String) {
    if let Ok(mut errs) = errors.lock() {
        errs.push(message);
    }
}

pub struct ChunkDataCollector<F: PrimeField64> {
    /// State machine bundle for building data buses.
    sm_bundle: Arc<StaticSMBundle<F>>,
}

impl<F: PrimeField64> ChunkDataCollector<F> {
    /// Creates a new `ChunkDataCollector`.
    ///
    /// # Arguments
    /// * `sm_bundle` - State machine bundle.
    pub fn new(sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self { sm_bundle }
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        self.sm_bundle.set_rom(zisk_rom)
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) -> ExecutorResult<()> {
        self.sm_bundle.set_rh_data(rh_data)
    }

    /// Computes which chunks need to be executed for each instance.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces from execution.
    /// * `secn_instances` - Map of global ID to secondary instances.
    ///
    /// # Returns
    /// Tuple of `(chunks_to_execute, global_id_chunks)` where:
    /// - `chunks_to_execute[chunk_id]` = list of global_ids that need this chunk
    /// - `global_id_chunks[global_id]` = list of chunk_ids this instance needs
    pub fn compute_chunks_to_execute(
        &self,
        min_traces: &[EmuTrace],
        secn_instances: &HashMap<usize, &dyn Instance<F>>,
    ) -> (Vec<Vec<usize>>, HashMap<usize, Vec<usize>>) {
        let mut chunks_to_execute = vec![Vec::new(); min_traces.len()];
        let mut global_id_chunks: HashMap<usize, Vec<usize>> = HashMap::new();

        secn_instances.iter().for_each(|(global_idx, secn_instance)| {
            match secn_instance.check_point() {
                CheckPoint::None => {}
                CheckPoint::Single(chunk_id) => {
                    chunks_to_execute[chunk_id.as_usize()].push(*global_idx);
                    global_id_chunks.entry(*global_idx).or_default().push(chunk_id.as_usize());
                }
                CheckPoint::Multiple(chunk_ids) => {
                    chunk_ids.iter().for_each(|&chunk_id| {
                        chunks_to_execute[chunk_id.as_usize()].push(*global_idx);
                        global_id_chunks.entry(*global_idx).or_default().push(chunk_id.as_usize());
                    });
                }
            }
        });

        for chunk_ids in global_id_chunks.values_mut() {
            chunk_ids.sort();
        }

        (chunks_to_execute, global_id_chunks)
    }

    /// Orders chunks for optimal processing.
    ///
    /// Uses a greedy algorithm to minimize the time until any instance
    /// has all its chunks collected.
    ///
    /// # Arguments
    /// * `chunks_to_execute` - Which instances need each chunk.
    /// * `global_id_chunks` - Which chunks each instance needs.
    ///
    /// # Returns
    /// Ordered list of chunk IDs to process.
    pub fn order_chunks(
        &self,
        chunks_to_execute: &[Vec<usize>],
        global_id_chunks: &HashMap<usize, Vec<usize>>,
    ) -> Vec<usize> {
        let mut ordered_chunks = Vec::new();
        let mut already_selected_chunks = vec![false; chunks_to_execute.len()];

        let mut n_global_ids_incompleted = global_id_chunks.len();
        let mut n_chunks_by_global_id: HashMap<usize, usize> =
            global_id_chunks.iter().map(|(global_id, chunks)| (*global_id, chunks.len())).collect();

        while n_global_ids_incompleted > 0 {
            let selected_global_id = n_chunks_by_global_id
                .iter()
                .filter(|(_, &count)| count > 0)
                .min_by_key(|(_, &count)| count)
                .map(|(&global_id, _)| global_id);

            if let Some(global_id) = selected_global_id {
                for chunk_id in global_id_chunks[&global_id].iter() {
                    if already_selected_chunks[*chunk_id] {
                        continue;
                    }
                    ordered_chunks.push(*chunk_id);
                    already_selected_chunks[*chunk_id] = true;
                    for global_idx in chunks_to_execute[*chunk_id].iter() {
                        if let Some(count) = n_chunks_by_global_id.get_mut(global_idx) {
                            *count -= 1;
                            if *count == 0 {
                                n_chunks_by_global_id.remove(global_idx);
                                n_global_ids_incompleted -= 1;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        ordered_chunks
    }

    /// Collects chunk data for a single secondary instance.
    ///
    /// Convenience method that wraps `collect()` for single-instance collection.
    /// Avoids the caller needing to create a HashMap for one instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `state` - Execution state for storing collectors.
    /// * `global_id` - Global ID of the instance.
    /// * `instance` - The secondary instance to collect for.
    pub fn collect_single(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        global_id: usize,
        instance: &dyn Instance<F>,
    ) -> ExecutorResult<()> {
        let mut map = HashMap::with_capacity(1);
        map.insert(global_id, instance);
        self.collect(pctx, state, map)?;
        Ok(())
    }

    /// Collects chunk data for the given secondary instances.
    ///
    /// Processes chunks in parallel, collecting data into the execution state's
    /// collectors_by_instance map.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `state` - Execution state for storing collectors.
    /// * `secn_instances` - Map of global ID to secondary instances.
    pub fn collect(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        secn_instances: HashMap<usize, &dyn Instance<F>>,
    ) -> ExecutorResult<()> {
        let min_traces_guard = state.min_traces.read_or_poison("min_traces")?;
        let min_traces = min_traces_guard.as_ref().ok_or(ExecutorError::MinTracesNotSet)?;

        // Compute chunks to execute
        let (chunks_to_execute, global_id_chunks) =
            self.compute_chunks_to_execute(min_traces, &secn_instances);

        let ordered_chunks = self.order_chunks(&chunks_to_execute, &global_id_chunks);
        let global_ids: Vec<usize> = secn_instances.keys().copied().collect();

        let collect_start_times: Vec<AtomicCell<Option<Instant>>> =
            global_ids.iter().map(|_| AtomicCell::new(None)).collect();

        let global_ids_map: HashMap<usize, usize> =
            global_ids.iter().enumerate().map(|(idx, &id)| (id, idx)).collect();

        // Build one data bus per chunk in parallel. Empty chunks
        // (no instances need them) get `None` directly without
        // running the per-chunk bundle scan.
        let data_buses: Vec<Option<_>> = chunks_to_execute
            .par_iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                if global_idxs.is_empty() {
                    Ok(None)
                } else {
                    crate::StaticDataBusCollect::for_chunk(
                        &self.sm_bundle,
                        pctx,
                        &secn_instances,
                        ChunkId(chunk_id),
                        global_idxs,
                    )
                    .map(Some)
                }
            })
            .collect::<ExecutorResult<_>>()?;

        // Wrap each so chunk-player threads can write to them concurrently.
        let data_buses: Vec<_> = data_buses.into_iter().map(Mutex::new).collect();

        let n_chunks_left: Vec<AtomicUsize> = global_ids
            .iter()
            .map(|global_id| {
                global_id_chunks.get(global_id).map(|chunks| AtomicUsize::new(chunks.len())).ok_or(
                    ExecutorError::MissingIndexEntry {
                        global_id: *global_id,
                        index: "global_id_chunks",
                    },
                )
            })
            .collect::<ExecutorResult<Vec<_>>>()?;

        // Initialize collectors and stats
        for global_id in global_ids.iter() {
            let (airgroup_id, air_id) = pctx.dctx_get_instance_info(*global_id)?;
            let n_chunks = global_id_chunks
                .get(global_id)
                .ok_or(ExecutorError::MissingIndexEntry {
                    global_id: *global_id,
                    index: "global_id_chunks",
                })?
                .len();
            let stats = Stats::new_pending_collection(airgroup_id, air_id, n_chunks);

            state
                .collector_store
                .inner
                .write_or_poison("collector_store")?
                .insert(*global_id, (0..n_chunks).map(|_| None).collect());
            state.stats.insert_witness_stats(*global_id, stats);
        }

        let next_chunk = AtomicUsize::new(0);
        let zisk_rom = state.get_rom()?;
        let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

        let ctx = WorkerCtx {
            next_chunk: &next_chunk,
            ordered_chunks: &ordered_chunks,
            chunks_to_execute: &chunks_to_execute,
            data_buses: &data_buses,
            zisk_rom: &zisk_rom,
            min_traces,
            pctx,
            collectors_by_instance: &state.collector_store.inner,
            n_chunks_left: &n_chunks_left,
            collect_start_times: &collect_start_times,
            stats: &state.stats,
            global_ids_map: &global_ids_map,
            global_id_chunks: &global_id_chunks,
            errors: &errors,
        };

        rayon::in_place_scope(|scope| {
            for _ in 0..rayon::current_num_threads() {
                let ctx = &ctx;
                scope.spawn(move |_| Self::worker_loop(ctx));
            }
        });

        // Collect any errors from parallel execution.
        // Use unwrap_or_else to handle poisoned mutex (e.g., if a worker thread panicked).
        // We extract the data even if poisoned, then report the poisoning as an additional error.
        let err_vec = errors.lock().unwrap_or_else(|poisoned| {
            error!("errors mutex was poisoned during parallel chunk execution");
            poisoned.into_inner()
        });

        if !err_vec.is_empty() {
            let message = err_vec
                .iter()
                .enumerate()
                .map(|(i, e)| format!("[Error {}] {e}", i + 1))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(ExecutorError::MtChunkProcessing { count: err_vec.len(), message });
        }

        Ok(())
    }

    /// Rayon-spawned worker loop. Steals chunk IDs off the shared
    /// counter; takes each chunk's data bus exactly once (subsequent
    /// stealers see `None` and skip); processes via
    /// [`Self::process_one_chunk`].
    fn worker_loop(ctx: &WorkerCtx<'_, F>) {
        loop {
            let next_chunk_id = ctx.next_chunk.fetch_add(1, Ordering::Relaxed);
            if next_chunk_id >= ctx.ordered_chunks.len() {
                break;
            }
            let chunk_id = ctx.ordered_chunks[next_chunk_id];

            let data_bus = match ctx.data_buses[chunk_id].lock() {
                Ok(mut lock) => match lock.take() {
                    Some(bus) => bus,
                    // Another worker already processed this chunk —
                    // shouldn't happen given `fetch_add` partitioning,
                    // but defensive.
                    None => continue,
                },
                Err(_) => {
                    push_error(
                        ctx.errors,
                        format!("data_buses lock poisoned for chunk {chunk_id}"),
                    );
                    continue;
                }
            };

            Self::process_one_chunk(chunk_id, data_bus, ctx);
        }
    }

    /// Process a single chunk: replay its emu traces through the data
    /// bus, drain device entries into per-instance collector slots,
    /// advance witness-ready counters. On an instance's final chunk,
    /// records witness stats via [`Self::record_completion_stats`].
    fn process_one_chunk(
        chunk_id: usize,
        mut data_bus: StaticDataBusCollect<PayloadType, F>,
        ctx: &WorkerCtx<'_, F>,
    ) {
        // Mark collection start time for each affected instance, and
        // remember which globals this chunk feeds so we can advance
        // their `n_chunks_left` counters once the bus output is stored.
        let mut affected_globals: Vec<(usize, usize)> = Vec::new();
        for global_id in ctx.chunks_to_execute[chunk_id].iter() {
            match ctx.global_ids_map.get(global_id) {
                Some(&global_id_idx) => {
                    let start_time_cell = &ctx.collect_start_times[global_id_idx];
                    if start_time_cell.load().is_none() {
                        start_time_cell.store(Some(Instant::now()));
                    }
                    affected_globals.push((*global_id, global_id_idx));
                }
                None => {
                    push_error(ctx.errors, format!("global_id {global_id} not in global_ids_map"));
                }
            }
        }

        // Run the emulator over this chunk's traces.
        ZiskEmulator::process_emu_traces::<F, _, _>(
            ctx.zisk_rom,
            ctx.min_traces,
            chunk_id,
            &mut data_bus,
        );

        // Drain device collectors and build per-instance entries.
        let devices = data_bus.into_devices(false);
        let mut entries: Vec<(usize, usize, Option<ChunkCollector>)> = Vec::new();
        for (global_id, col) in devices {
            match ctx.global_id_chunks.get(&global_id) {
                Some(chunk_order) => {
                    if let Some(position) = chunk_order.iter().position(|&id| id == chunk_id) {
                        entries.push((global_id, position, Some((chunk_id, col))));
                    } else {
                        push_error(
                            ctx.errors,
                            format!(
                                "chunk_id {chunk_id} not in chunk_order for global_id {global_id}"
                            ),
                        );
                    }
                }
                None => {
                    push_error(
                        ctx.errors,
                        format!("global_id {global_id} not found in global_id_chunks"),
                    );
                }
            }
        }

        // Store collected entries in the per-instance slot map.
        match ctx.collectors_by_instance.write() {
            Ok(mut guard) => {
                for (global_id, position, entry) in entries {
                    if let Some(vec) = guard.get_mut(&global_id) {
                        vec[position] = entry;
                    } else {
                        push_error(
                            ctx.errors,
                            format!("global_id {global_id} not in collectors_by_instance"),
                        );
                    }
                }
            }
            Err(_) => {
                push_error(ctx.errors, "collectors_by_instance lock poisoned".to_string());
            }
        }

        // Advance counters; on the last chunk for an instance, flip its
        // witness-ready flag and record completion stats.
        for (global_id, global_id_idx) in affected_globals {
            if ctx.n_chunks_left[global_id_idx].fetch_sub(1, Ordering::SeqCst) == 1 {
                ctx.pctx.set_witness_ready(global_id, true);
                Self::record_completion_stats(global_id, global_id_idx, ctx);
            }
        }
    }

    /// Called when an instance's *final* chunk has been collected:
    /// reads the recorded start time, computes elapsed duration, and
    /// publishes per-witness stats. Each error path delegates to
    /// [`push_error`] so a failure here cannot abort other workers.
    fn record_completion_stats(global_id: usize, global_id_idx: usize, ctx: &WorkerCtx<'_, F>) {
        let Some(collect_start_time) = ctx.collect_start_times[global_id_idx].load() else {
            push_error(ctx.errors, format!("collect_start_time not set for global_id {global_id}"));
            return;
        };

        let collect_duration = collect_start_time.elapsed().as_millis() as u64;

        match (ctx.pctx.dctx_get_instance_info(global_id), ctx.global_id_chunks.get(&global_id)) {
            (Ok((airgroup_id, air_id)), Some(chunks)) => {
                let new_stats = Stats::new_with_collection(
                    airgroup_id,
                    air_id,
                    chunks.len(),
                    collect_start_time,
                    collect_duration,
                );
                ctx.stats.insert_witness_stats(global_id, new_stats);
            }
            (Err(e), _) => {
                push_error(
                    ctx.errors,
                    format!("Failed to get instance info for global_id {global_id}: {e}"),
                );
            }
            (Ok(_), None) => {
                push_error(ctx.errors, format!("global_id {global_id} not in global_id_chunks"));
            }
        }
    }
}
