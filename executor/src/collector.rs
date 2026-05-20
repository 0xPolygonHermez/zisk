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

use anyhow::Result;
use crossbeam::atomic::AtomicCell;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tracing::error;
use zisk_common::{CheckPoint, EmuTrace, Instance, Stats};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use crate::AsmRunnerRH;
use crate::{state::ChunkCollector, ExecutionState, StaticSMBundle};

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

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> Result<()> {
        self.sm_bundle.set_rom(zisk_rom)
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) -> Result<()> {
        self.sm_bundle.set_rh_data(rh_data)
    }

    /// Computes which chunks need to be executed for each instance.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces from execution.
    /// * `secn_instances` - Map of global ID to secondary instances.
    ///
    /// # Returns
    /// Tuple of (chunks_to_execute, global_id_chunks) where:
    /// - chunks_to_execute[chunk_id] = list of global_ids that need this chunk
    /// - global_id_chunks[global_id] = list of chunk_ids this instance needs
    #[allow(clippy::borrowed_box)]
    pub fn compute_chunks_to_execute(
        &self,
        min_traces: &[EmuTrace],
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
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
    #[allow(clippy::borrowed_box)]
    pub fn collect_single(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        global_id: usize,
        instance: &Box<dyn Instance<F>>,
    ) -> Result<()> {
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
    #[allow(clippy::borrowed_box)]
    pub fn collect(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        secn_instances: HashMap<usize, &Box<dyn Instance<F>>>,
    ) -> Result<()> {
        let min_traces_guard = state
            .min_traces
            .read()
            .map_err(|e| anyhow::anyhow!("min_traces lock poisoned: {e}"))?;
        let min_traces = min_traces_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("min_traces should not be None"))?;

        // Compute chunks to execute
        let (chunks_to_execute, global_id_chunks) =
            self.compute_chunks_to_execute(min_traces, &secn_instances);

        let ordered_chunks = self.order_chunks(&chunks_to_execute, &global_id_chunks);
        let global_ids: Vec<usize> = secn_instances.keys().copied().collect();

        let collect_start_times: Vec<AtomicCell<Option<Instant>>> =
            global_ids.iter().map(|_| AtomicCell::new(None)).collect();

        let global_ids_map: HashMap<usize, usize> =
            global_ids.iter().enumerate().map(|(idx, &id)| (id, idx)).collect();

        // Build one data bus per chunk in parallel.
        let data_buses: Vec<_> = chunks_to_execute
            .par_iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                crate::StaticDataBusCollect::for_chunk(
                    &self.sm_bundle,
                    pctx,
                    &secn_instances,
                    chunk_id,
                    global_idxs,
                )
            })
            .collect::<Result<_>>()
            .map_err(|e| anyhow::anyhow!("Failed to build data bus collectors: {e}"))?;

        // Wrap each so chunk-player threads can write to them concurrently.
        let data_buses: Vec<_> = data_buses.into_iter().map(Mutex::new).collect();

        let n_chunks_left: Vec<AtomicUsize> = global_ids
            .iter()
            .map(|global_id| {
                global_id_chunks
                    .get(global_id)
                    .map(|chunks| AtomicUsize::new(chunks.len()))
                    .ok_or_else(|| anyhow::anyhow!("global_id {global_id} not in global_id_chunks"))
            })
            .collect::<Result<Vec<_>>>()?;

        // Initialize collectors and stats
        for global_id in global_ids.iter() {
            let (airgroup_id, air_id) = pctx.dctx_get_instance_info(*global_id).map_err(|e| {
                anyhow::anyhow!("Failed to get instance info for global_id {global_id}: {e}")
            })?;
            let n_chunks = global_id_chunks
                .get(global_id)
                .ok_or_else(|| anyhow::anyhow!("global_id {global_id} not in global_id_chunks"))?
                .len();
            let stats = Stats::new_pending_collection(airgroup_id, air_id, n_chunks);

            state
                .collectors_by_instance
                .write()
                .map_err(|e| anyhow::anyhow!("collectors_by_instance lock poisoned: {e}"))?
                .insert(*global_id, (0..n_chunks).map(|_| None).collect());
            state.stats.insert_witness_stats(*global_id, stats);
        }

        let next_chunk = AtomicUsize::new(0);
        let zisk_rom = state.get_rom()?;
        let errors: Mutex<Vec<anyhow::Error>> = Mutex::new(Vec::new());

        rayon::in_place_scope(|scope| {
            for _ in 0..rayon::current_num_threads() {
                let next_chunk = &next_chunk;
                let n_chunks_left = &n_chunks_left;
                let collectors_by_instance = &state.collectors_by_instance;
                let collect_start_times = &collect_start_times;
                let stats = &state.stats;
                let min_traces = &min_traces;
                let data_buses = &data_buses;
                let zisk_rom = &zisk_rom;
                let global_ids_map = &global_ids_map;
                let global_id_chunks = &global_id_chunks;
                let ordered_chunks = &ordered_chunks;
                let chunks_to_execute = &chunks_to_execute;
                let pctx = &pctx;
                let errors = &errors;

                scope.spawn(move |_| loop {
                    let next_chunk_id = next_chunk.fetch_add(1, Ordering::Relaxed);
                    if next_chunk_id >= ordered_chunks.len() {
                        break;
                    }
                    let chunk_id = ordered_chunks[next_chunk_id];

                    // Acquire lock and get data bus for this chunk
                    if let Ok(mut lock) = data_buses[chunk_id].lock() {
                        if let Some(mut data_bus) = lock.take() {
                            drop(lock);

                            // Mark collection start time for each affected instance
                            let mut affected_globals: Vec<(usize, usize)> = Vec::new();
                            for global_id in chunks_to_execute[chunk_id].iter() {
                                if let Some(&global_id_idx) = global_ids_map.get(global_id) {
                                    let start_time_cell = &collect_start_times[global_id_idx];
                                    if start_time_cell.load().is_none() {
                                        start_time_cell.store(Some(Instant::now()));
                                    }
                                    affected_globals.push((*global_id, global_id_idx));
                                } else {
                                    let _ = errors.lock().map(|mut errs| {
                                        errs.push(anyhow::anyhow!("global_id {global_id} not in global_ids_map"));
                                    });
                                }
                            }

                            // Process emulator traces for this chunk
                            ZiskEmulator::process_emu_traces::<F, _, _>(zisk_rom, min_traces, chunk_id, &mut data_bus);

                            // Collect device results and build entries for all affected instances
                            let devices = data_bus.into_devices(false);
                            let mut entries: Vec<(usize, usize, Option<ChunkCollector>)> = Vec::new();

                            for (global_id, collector) in devices {
                                if let Some(global_id) = global_id {
                                    if let Some(chunk_order) = global_id_chunks.get(&global_id) {
                                        if let Some(position) = chunk_order.iter().position(|&id| id == chunk_id) {
                                            if let Some(col) = collector {
                                                entries.push((global_id, position, Some((chunk_id, col))));
                                            } else {
                                                let _ = errors.lock().map(|mut errs| {
                                                    errs.push(anyhow::anyhow!("collector is None for global_id {global_id}"));
                                                });
                                            }
                                        } else {
                                            let _ = errors.lock().map(|mut errs| {
                                                errs.push(anyhow::anyhow!("chunk_id {chunk_id} not in chunk_order for global_id {global_id}"));
                                            });
                                        }
                                    } else {
                                        let _ = errors.lock().map(|mut errs| {
                                            errs.push(anyhow::anyhow!("global_id {global_id} not found in global_id_chunks"));
                                        });
                                    }
                                }
                            }

                            // Update collectors: store collected data for each instance
                            if let Ok(mut guard) = collectors_by_instance.write() {
                                for (global_id, position, entry) in entries.into_iter() {
                                    if let Some(vec) = guard.get_mut(&global_id) {
                                        vec[position] = entry;
                                    } else {
                                        let _ = errors.lock().map(|mut errs| {
                                            errs.push(anyhow::anyhow!("global_id {global_id} not in collectors_by_instance"));
                                        });
                                    }
                                }
                            } else {
                                let _ = errors.lock().map(|mut errs| {
                                    errs.push(anyhow::anyhow!("collectors_by_instance lock poisoned"));
                                });
                            }

                            // Update atomic counters and mark ready instances
                            for (global_id, global_id_idx) in affected_globals {
                                if n_chunks_left[global_id_idx].fetch_sub(1, Ordering::SeqCst) == 1 {
                                    pctx.set_witness_ready(global_id, true);

                                    if let Some(collect_start_time) = collect_start_times[global_id_idx].load() {
                                        let collect_duration = collect_start_time.elapsed().as_millis() as u64;
                                        match (pctx.dctx_get_instance_info(global_id), global_id_chunks.get(&global_id)) {
                                            (Ok((airgroup_id, air_id)), Some(chunks)) => {
                                                let new_stats = Stats::new_with_collection(
                                                    airgroup_id,
                                                    air_id,
                                                    chunks.len(),
                                                    collect_start_time,
                                                    collect_duration,
                                                );
                                                stats.insert_witness_stats(global_id, new_stats);
                                            }
                                            (Err(e), _) => {
                                                let _ = errors.lock().map(|mut errs| {
                                                    errs.push(anyhow::anyhow!("Failed to get instance info for global_id {global_id}: {e}"));
                                                });
                                            }
                                            (Ok(_), None) => {
                                                let _ = errors.lock().map(|mut errs| {
                                                    errs.push(anyhow::anyhow!("global_id {global_id} not in global_id_chunks"));
                                                });
                                            }
                                        }
                                    } else {
                                        let _ = errors.lock().map(|mut errs| {
                                            errs.push(anyhow::anyhow!("collect_start_time not set for global_id {global_id}"));
                                        });
                                    }
                                }
                            }
                        }
                    } else {
                        let _ = errors.lock().map(|mut errs| {
                            errs.push(anyhow::anyhow!("data_buses lock poisoned for chunk {chunk_id}"));
                        });
                    }
                });
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
            let combined = err_vec
                .iter()
                .enumerate()
                .map(|(i, e)| format!("[Error {}] {:#}", i + 1, e))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(anyhow::anyhow!(
                "Chunk data collection failed ({} errors):\n{}",
                err_vec.len(),
                combined
            ));
        }

        Ok(())
    }
}
