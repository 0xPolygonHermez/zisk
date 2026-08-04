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

use crossbeam::atomic::AtomicCell;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
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

/// Per-`collect()` timing accumulators.
///
/// `pre_calculate` is now known to carry ~91% of the slow-mode witness excess, and its cost
/// is essentially this one call. The chunk set is byte-identical every job, yet the call
/// spans 185-1423 ms — so the question is whether one chunk stalls (a lock or a cold
/// buffer) or every chunk is uniformly slower (CPU starvation). These separate the two.
///
/// The decisive pair is `wall_ns` vs `cpu_ns`: a chunk that is on-CPU the whole time has
/// cpu/wall near 1 and is simply running slowly (starvation, frequency, sibling
/// contention); a chunk with cpu/wall well below 1 is blocked on something. And comparing
/// the summed chunk wall time against `workers x scope wall` shows how much of the pool
/// was idle or spinning rather than processing chunks.
/// Instance-count buckets for the per-chunk cost model. Index = instances attached to
/// the chunk, clamped to the last bucket.
const COST_BUCKETS: usize = 17;

struct CollectStats {
    chunks: AtomicUsize,
    wall_ns: AtomicU64,
    cpu_ns: AtomicU64,
    max_wall_ns: AtomicU64,
    max_chunk: AtomicUsize,
    /// Per-chunk wall time grouped by how many instances that chunk fed.
    ///
    /// This is the cost model for switching the replay to the instance axis (one trace
    /// live at a time, each chunk replayed once per instance that needs it). Regressing
    /// mean wall against instance count splits the per-chunk cost in two: the intercept is
    /// the instance-independent part (emulating the chunk), which an instance-axis replay
    /// pays once *per instance*; the slope is the per-instance bus fan-out, which it pays
    /// exactly as often as today. So intercept/slope is the exchange rate that decides
    /// whether trading memory for re-replay is worth it.
    bucket_count: [AtomicUsize; COST_BUCKETS],
    bucket_ns: [AtomicU64; COST_BUCKETS],
    /// Inside a chunk: the emulator pass (emulation + bus dispatch + collector pushes,
    /// interleaved and not separable without touching the emulator) versus draining the
    /// collectors into the store afterwards.
    emu_ns: AtomicU64,
    drain_ns: AtomicU64,
}

impl Default for CollectStats {
    fn default() -> Self {
        Self {
            chunks: AtomicUsize::new(0),
            wall_ns: AtomicU64::new(0),
            cpu_ns: AtomicU64::new(0),
            max_wall_ns: AtomicU64::new(0),
            max_chunk: AtomicUsize::new(0),
            bucket_count: std::array::from_fn(|_| AtomicUsize::new(0)),
            bucket_ns: std::array::from_fn(|_| AtomicU64::new(0)),
            emu_ns: AtomicU64::new(0),
            drain_ns: AtomicU64::new(0),
        }
    }
}

impl CollectStats {
    fn record(
        &self,
        chunk_id: usize,
        n_inst: usize,
        wall: std::time::Duration,
        cpu: Option<std::time::Duration>,
    ) {
        let ns = wall.as_nanos() as u64;
        let bucket = n_inst.min(COST_BUCKETS - 1);
        self.bucket_count[bucket].fetch_add(1, Ordering::Relaxed);
        self.bucket_ns[bucket].fetch_add(ns, Ordering::Relaxed);
        self.chunks.fetch_add(1, Ordering::Relaxed);
        self.wall_ns.fetch_add(ns, Ordering::Relaxed);
        if let Some(cpu) = cpu {
            self.cpu_ns.fetch_add(cpu.as_nanos() as u64, Ordering::Relaxed);
        }
        // Racy against a concurrent max update, so the reported id can lag the reported
        // value by one chunk. Fine for pointing at a straggler; not worth a lock on the
        // hot path.
        if ns > self.max_wall_ns.fetch_max(ns, Ordering::Relaxed) {
            self.max_chunk.store(chunk_id, Ordering::Relaxed);
        }
    }
}

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

    // ── Timing ──
    collect_stats: &'a CollectStats,
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
    /// Uses an algorithm to minimize the time until any instance has all its chunks collected.
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
        let t_guard = Instant::now();
        let min_traces_guard = state.min_traces.read_or_poison("min_traces")?;
        let min_traces = min_traces_guard.as_ref().ok_or(ExecutorError::MinTracesNotSet)?;
        let guard_wait = t_guard.elapsed();

        // Compute chunks to execute
        let t_plan = Instant::now();
        let (chunks_to_execute, global_id_chunks) =
            self.compute_chunks_to_execute(min_traces, &secn_instances);

        let ordered_chunks = self.order_chunks(&chunks_to_execute, &global_id_chunks);
        let plan_time = t_plan.elapsed();
        let global_ids: Vec<usize> = secn_instances.keys().copied().collect();

        let collect_start_times: Vec<AtomicCell<Option<Instant>>> =
            global_ids.iter().map(|_| AtomicCell::new(None)).collect();

        let global_ids_map: HashMap<usize, usize> =
            global_ids.iter().enumerate().map(|(idx, &id)| (id, idx)).collect();

        let zisk_rom = state.get_rom()?;

        // Build one data bus per chunk in parallel. Empty chunks
        // (no instances need them) get `None` directly without
        // running the per-chunk bundle scan.
        let t_bus = Instant::now();
        let data_buses: Vec<Option<_>> = chunks_to_execute
            .par_iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                if global_idxs.is_empty() {
                    Ok(None)
                } else {
                    crate::StaticDataBusCollect::for_chunk(
                        pctx,
                        &secn_instances,
                        ChunkId(chunk_id),
                        global_idxs,
                        zisk_rom.as_ref(),
                    )
                    .map(Some)
                }
            })
            .collect::<ExecutorResult<_>>()?;
        let bus_build = t_bus.elapsed();

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

        let collect_stats = CollectStats::default();
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
            collect_stats: &collect_stats,
        };

        // Page faults and allocator state across the replay. The replay allocates one
        // Vec::with_capacity per (chunk, instance) and frees it after; if the allocator
        // returned those pages to the kernel, every first touch re-faults, and a fault
        // storm reads as an IPC collapse on byte-identical work.
        let faults_before = proofman::page_faults();
        let alloc_before = proofman::allocator_stats();
        let t_scope = Instant::now();
        let n_workers = rayon::current_num_threads();
        rayon::in_place_scope(|scope| {
            for _ in 0..n_workers {
                let ctx = &ctx;
                scope.spawn(move |_| Self::worker_loop(ctx));
            }
        });
        let replay = t_scope.elapsed();
        let (minflt, majflt) = faults_before
            .zip(proofman::page_faults())
            .map(|((a0, b0), (a1, b1))| (a1.saturating_sub(a0), b1.saturating_sub(b0)))
            .unwrap_or((0, 0));
        // Signed: the allocator can also *release* during the window.
        let (d_mmap, d_inuse, d_free) = alloc_before
            .zip(proofman::allocator_stats())
            .map(|((m0, u0, f0), (m1, u1, f1))| {
                (m1 as i64 - m0 as i64, u1 as i64 - u0 as i64, f1 as i64 - f0 as i64)
            })
            .unwrap_or((0, 0, 0));

        let n_chunks = chunks_to_execute.len();
        let n_empty = chunks_to_execute.iter().filter(|c| c.is_empty()).count();
        // One intermediate-input Vec is allocated per (chunk, instance) pair, so this is
        // the per-collect allocation count — i.e. the size of the prize if collectors wrote
        // trace rows directly instead of materialising inputs. `max_inst` is also the
        // re-execution multiplier a scheme that replayed per-instance would pay, since one
        // replay currently fans out to every instance needing that chunk.
        let collector_pairs: usize = chunks_to_execute.iter().map(|c| c.len()).sum();
        let max_inst = chunks_to_execute.iter().map(|c| c.len()).max().unwrap_or(0);
        let non_empty = n_chunks - n_empty;

        // Peak concurrent trace residency for a scheme that writes trace rows directly
        // during this replay instead of materialising intermediate inputs.
        //
        // Such a scheme does NOT need every instance's trace at once: a trace only has to
        // be live from the first chunk that feeds it to the last. Sweeping `ordered_chunks`
        // and counting instances that have started but not finished gives the actual peak,
        // which is what the chunk ordering controls — and it is far below the
        // all-instances-resident ceiling. Reported with the airs at the peak so their trace
        // sizes can be summed for a byte figure.
        let mut first_at: HashMap<usize, usize> = HashMap::new();
        let mut last_at: HashMap<usize, usize> = HashMap::new();
        for (pos, &chunk_id) in ordered_chunks.iter().enumerate() {
            for &inst in &chunks_to_execute[chunk_id] {
                first_at.entry(inst).or_insert(pos);
                last_at.insert(inst, pos);
            }
        }
        let mut live = 0usize;
        let mut peak_live = 0usize;
        let mut peak_pos = 0usize;
        let mut starts: HashMap<usize, usize> = HashMap::new();
        let mut ends: HashMap<usize, usize> = HashMap::new();
        for (&inst, &pos) in first_at.iter() {
            *starts.entry(pos).or_insert(0) += 1;
            *ends.entry(last_at[&inst]).or_insert(0) += 1;
        }
        for pos in 0..ordered_chunks.len() {
            live += starts.get(&pos).copied().unwrap_or(0);
            if live > peak_live {
                peak_live = live;
                peak_pos = pos;
            }
            live -= ends.get(&pos).copied().unwrap_or(0);
        }
        // The airs live at the peak, so their trace sizes can be summed offline.
        let mut peak_airs: Vec<String> = first_at
            .iter()
            .filter(|(inst, &f)| f <= peak_pos && last_at[inst] >= peak_pos)
            .filter_map(|(inst, _)| pctx.dctx_get_instance_info(*inst).ok())
            .map(|(ag, air)| format!("{ag}:{air}"))
            .collect();
        peak_airs.sort();
        let chunks_done = collect_stats.chunks.load(Ordering::Relaxed);
        let chunk_wall_ms = collect_stats.wall_ns.load(Ordering::Relaxed) as f64 / 1e6;
        let chunk_cpu_ms = collect_stats.cpu_ns.load(Ordering::Relaxed) as f64 / 1e6;
        let replay_ms = replay.as_secs_f64() * 1000.0;
        // Pool capacity over the replay window. The gap between this and chunk_wall_ms is
        // pool time NOT spent processing chunks — rayon idle/spin or work-starvation from
        // an unbalanced chunk set.
        let pool_capacity_ms = replay_ms * n_workers as f64;
        tracing::info!(
            "COLLECT_SPLIT guard_wait={:.1}ms plan={:.1}ms bus_build={:.1}ms replay={:.1}ms \
             chunks={} empty={} done={} workers={} | chunk_wall_sum={:.1}ms chunk_cpu_sum={:.1}ms \
             cpu/wall={:.2} max_chunk_wall={:.1}ms (chunk {}) | pool_capacity={:.1}ms idle={:.1}ms ({:.1}%) \
             | minflt={} majflt={} faults_per_chunk={:.0} | malloc_d_mmap={}KB d_inuse={}KB d_free={}KB \
             | collector_pairs={} inst_per_chunk mean={:.2} max={} \
             | direct_rows_peak_live={} of {} at pos {} airs=[{}]",
            guard_wait.as_secs_f64() * 1000.0,
            plan_time.as_secs_f64() * 1000.0,
            bus_build.as_secs_f64() * 1000.0,
            replay_ms,
            n_chunks,
            n_empty,
            chunks_done,
            n_workers,
            chunk_wall_ms,
            chunk_cpu_ms,
            if chunk_wall_ms > 0.0 { chunk_cpu_ms / chunk_wall_ms } else { 0.0 },
            collect_stats.max_wall_ns.load(Ordering::Relaxed) as f64 / 1e6,
            collect_stats.max_chunk.load(Ordering::Relaxed),
            pool_capacity_ms,
            (pool_capacity_ms - chunk_wall_ms).max(0.0),
            if pool_capacity_ms > 0.0 {
                100.0 * (pool_capacity_ms - chunk_wall_ms).max(0.0) / pool_capacity_ms
            } else {
                0.0
            },
            minflt,
            majflt,
            if chunks_done > 0 { minflt as f64 / chunks_done as f64 } else { 0.0 },
            d_mmap / 1024,
            d_inuse / 1024,
            d_free / 1024,
            collector_pairs,
            if non_empty > 0 { collector_pairs as f64 / non_empty as f64 } else { 0.0 },
            max_inst,
            peak_live,
            first_at.len(),
            peak_pos,
            peak_airs.join(","),
        );

        // Optional benchmark: replay every chunk again with a bus that has NO collectors
        // attached. That pass costs emulation + bus dispatch with no consumers, so
        // (real emulator pass - this) is the collector-push cost, and this is the floor an
        // instance-axis replay would pay per instance. Side-effect free: with no
        // collectors there is nothing to write. Gated because it doubles the replay.
        if std::env::var("ZISK_COLLECT_BENCH").is_ok() {
            let t_bench = Instant::now();
            let empty: Vec<usize> = Vec::new();
            let bench_ns = AtomicU64::new(0);
            ordered_chunks.par_iter().for_each(|&chunk_id| {
                if chunks_to_execute[chunk_id].is_empty() {
                    return;
                }
                if let Ok(mut bus) = crate::StaticDataBusCollect::for_chunk(
                    pctx,
                    &secn_instances,
                    ChunkId(chunk_id),
                    &empty,
                    zisk_rom.as_ref(),
                ) {
                    let t = Instant::now();
                    ZiskEmulator::process_emu_traces::<F, _, _>(
                        &zisk_rom,
                        min_traces,
                        chunk_id,
                        &mut bus,
                    );
                    bench_ns.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
                }
            });
            let emu_ms = collect_stats.emu_ns.load(Ordering::Relaxed) as f64 / 1e6;
            let no_col_ms = bench_ns.load(Ordering::Relaxed) as f64 / 1e6;
            tracing::info!(
                "COLLECT_BENCH no_collector_replay: emu_with_collectors={:.1}ms \
                 emu_no_collectors={:.1}ms => collector_push={:.1}ms ({:.1}% of emu pass) \
                 | irreducible_floor={:.1}% | wall={:.1}ms",
                emu_ms,
                no_col_ms,
                (emu_ms - no_col_ms).max(0.0),
                if emu_ms > 0.0 { 100.0 * (emu_ms - no_col_ms).max(0.0) / emu_ms } else { 0.0 },
                if emu_ms > 0.0 { 100.0 * no_col_ms / emu_ms } else { 0.0 },
                t_bench.elapsed().as_secs_f64() * 1000.0,
            );
        }

        // Cost model for an instance-axis replay. `n=<inst>:<count>x<mean_ms>` per bucket.
        let buckets: Vec<String> = (0..COST_BUCKETS)
            .filter_map(|b| {
                let c = collect_stats.bucket_count[b].load(Ordering::Relaxed);
                (c > 0).then(|| {
                    let ms = collect_stats.bucket_ns[b].load(Ordering::Relaxed) as f64 / 1e6 / c as f64;
                    format!("n={b}:{c}x{ms:.2}ms")
                })
            })
            .collect();
        tracing::info!(
            "COLLECT_COST emu_pass={:.1}ms drain={:.1}ms | per-chunk wall by instance count — {}",
            collect_stats.emu_ns.load(Ordering::Relaxed) as f64 / 1e6,
            collect_stats.drain_ns.load(Ordering::Relaxed) as f64 / 1e6,
            buckets.join(" ")
        );

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

            // Per-chunk wall and CPU. One straggler chunk points at a lock or a cold
            // buffer; every chunk uniformly slower points at CPU starvation.
            let cpu0 = proofman::thread_cpu_time();
            let t0 = std::time::Instant::now();
            Self::process_one_chunk(chunk_id, data_bus, ctx);
            ctx.collect_stats.record(
                chunk_id,
                ctx.chunks_to_execute[chunk_id].len(),
                t0.elapsed(),
                cpu0.zip(proofman::thread_cpu_time()).map(|(a, b)| b.saturating_sub(a)),
            );
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
        let t_emu = Instant::now();
        ZiskEmulator::process_emu_traces::<F, _, _>(
            ctx.zisk_rom,
            ctx.min_traces,
            chunk_id,
            &mut data_bus,
        );
        ctx.collect_stats.emu_ns.fetch_add(t_emu.elapsed().as_nanos() as u64, Ordering::Relaxed);

        // Drain device collectors and build per-instance entries.
        let t_drain = Instant::now();
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
        ctx.collect_stats.drain_ns.fetch_add(t_drain.elapsed().as_nanos() as u64, Ordering::Relaxed);

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
