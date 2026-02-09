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
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use zisk_common::{CheckPoint, EmuTrace, Instance, Stats};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

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

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) {
        self.sm_bundle.set_rom(zisk_rom);
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
        let min_traces_guard = state.min_traces.read().unwrap();
        let min_traces = min_traces_guard.as_ref().expect("min_traces should not be None");

        // Compute chunks to execute
        let (chunks_to_execute, global_id_chunks) =
            self.compute_chunks_to_execute(min_traces, &secn_instances);

        let ordered_chunks = self.order_chunks(&chunks_to_execute, &global_id_chunks);
        let global_ids: Vec<usize> = secn_instances.keys().copied().collect();

        let collect_start_times: Vec<AtomicCell<Option<Instant>>> =
            global_ids.iter().map(|_| AtomicCell::new(None)).collect();

        let global_ids_map: HashMap<usize, usize> =
            global_ids.iter().enumerate().map(|(idx, &id)| (id, idx)).collect();

        // Create data buses for each chunk
        let data_buses = self
            .sm_bundle
            .build_data_bus_collectors(pctx, &secn_instances, &chunks_to_execute)
            .into_iter()
            .map(Mutex::new)
            .collect::<Vec<_>>();

        let n_chunks_left: Vec<AtomicUsize> = global_ids
            .iter()
            .map(|global_id| AtomicUsize::new(global_id_chunks[global_id].len()))
            .collect();

        // Initialize collectors and stats
        for global_id in global_ids.iter() {
            let (airgroup_id, air_id) =
                pctx.dctx_get_instance_info(*global_id).expect("Failed to get instance info");
            let stats = Stats::new_pending_collection(
                airgroup_id,
                air_id,
                global_id_chunks[global_id].len(),
            );

            state
                .collectors_by_instance
                .write()
                .unwrap()
                .insert(*global_id, (0..global_id_chunks[global_id].len()).map(|_| None).collect());
            state.stats.insert_witness_stats(*global_id, stats);
        }

        let next_chunk = AtomicUsize::new(0);
        let zisk_rom = state.get_rom()?;

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

                scope.spawn(move |_| loop {
                    let next_chunk_id = next_chunk.fetch_add(1, Ordering::Relaxed);
                    if next_chunk_id >= ordered_chunks.len() {
                        break;
                    }
                    let chunk_id = ordered_chunks[next_chunk_id];

                    if let Some(mut data_bus) = data_buses[chunk_id].lock().unwrap().take() {
                        for global_id in chunks_to_execute[chunk_id].iter() {
                            let start_time_cell = &collect_start_times[global_ids_map[global_id]];
                            if start_time_cell.load().is_none() {
                                start_time_cell.store(Some(Instant::now()));
                            }
                        }

                        ZiskEmulator::process_emu_traces::<F, _, _>(
                            zisk_rom,
                            min_traces,
                            chunk_id,
                            &mut data_bus,
                        );

                        // Collect all device results locally
                        let devices = data_bus.into_devices(false);
                        let mut entries: Vec<(usize, usize, Option<ChunkCollector>)> = Vec::new();
                        let mut affected_globals: Vec<(usize, usize)> = Vec::new();

                        for (global_id, collector) in devices {
                            if let Some(global_id) = global_id {
                                let global_id_idx = *global_ids_map
                                    .get(&global_id)
                                    .expect("Global ID not found in map");

                                let chunk_order = &global_id_chunks[&global_id];
                                let position = chunk_order
                                    .iter()
                                    .position(|&id| id == chunk_id)
                                    .expect("Chunk ID not found in order");

                                entries.push((
                                    global_id,
                                    position,
                                    Some((chunk_id, collector.unwrap())),
                                ));
                                affected_globals.push((global_id, global_id_idx));
                            }
                        }

                        // Single write-lock acquisition
                        {
                            let mut guard = collectors_by_instance.write().unwrap();
                            for (global_id, position, entry) in entries.iter_mut() {
                                guard.get_mut(global_id).unwrap()[*position] = entry.take();
                            }
                        }

                        // Update atomic counters and mark ready instances
                        for (global_id, global_id_idx) in affected_globals {
                            if n_chunks_left[global_id_idx].fetch_sub(1, Ordering::SeqCst) == 1 {
                                pctx.set_witness_ready(global_id, true);

                                let collect_start_time = collect_start_times[global_id_idx]
                                    .load()
                                    .expect("Collect start time was not set");
                                let collect_duration =
                                    collect_start_time.elapsed().as_millis() as u64;

                                let (airgroup_id, air_id) = pctx
                                    .dctx_get_instance_info(global_id)
                                    .expect("Failed to get instance info");
                                let new_stats = Stats::new_with_collection(
                                    airgroup_id,
                                    air_id,
                                    global_id_chunks[&global_id].len(),
                                    collect_start_time,
                                    collect_duration,
                                );

                                stats.insert_witness_stats(global_id, new_stats);
                            }
                        }
                    }
                });
            }
        });

        Ok(())
    }
}
