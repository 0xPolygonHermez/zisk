//! The `ZiskExecutor` module serves as the core orchestrator for executing the ZisK ROM program
//! and generating witness computations. It manages the execution of the state machines, from initial
//! planning to witness computation, ensuring efficient parallel processing and resource
//! utilization.
//!
//! This module handles both main and secondary state machines, integrating complex tasks such as
//! planning, configuration, and witness generation into a streamlined process.
//!
//! ## Executor Workflow
//! The execution is divided into distinct, sequential phases:
//!
//! 1. **Minimal Traces**: Rapidly process the ROM to collect minimal traces with minimal overhead.
//! 2. **Counting**: Creates the metrics required for the secondary state machine instances.
//! 3. **Planning**: Strategically plan the execution of instances to optimize resource usage.
//! 4. **Instance Creation**: Creates the AIR instances for the main and secondary state machines.
//! 5. **Witness Computation**: Compute the witnesses for all AIR instances, leveraging parallelism
//!    for efficiency.
//!
//! By structuring these phases, the `ZiskExecutor` ensures high-performance execution while
//! maintaining clarity and modularity in the computation process.

use itertools::Itertools;
use p3_field::PrimeField;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use witness::WitnessComponent;

use rayon::prelude::*;

use data_bus::{BusDevice, DataBus, PayloadType, OPERATION_BUS_ID};
use sm_common::{
    BusDeviceMetrics, BusDeviceMetricsWrapper, BusDeviceWrapper, CheckPoint, ComponentBuilder,
    Instance, InstanceCtx, InstanceType, Plan,
};
use sm_main::{MainInstance, MainPlanner, MainSM};
use zisk_pil::ZiskPublicValues;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, EmuTrace, ZiskEmulator};

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField> {
    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    pub zisk_rom: Arc<ZiskRom>,

    /// Path to the input data file.
    pub input_data_path: PathBuf,

    /// Registered secondary state machines.
    secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>,

    /// Planning information for main state machines.
    pub min_traces: RwLock<Vec<EmuTrace>>,
    pub main_planning: RwLock<Vec<Plan>>,
    pub secn_planning: RwLock<Vec<Vec<Plan>>>,
}

impl<F: PrimeField> ZiskExecutor<F> {
    /// The number of threads to use for parallel processing when computing minimal traces.
    const NUM_THREADS: usize = 16;

    /// The size in rows of the minimal traces
    const MIN_TRACE_SIZE: u64 = 1 << 18;

    const MAX_NUM_STEPS: u64 = 1 << 32;

    /// Creates a new instance of the `ZiskExecutor`.
    ///
    /// # Arguments
    /// * `input_data_path` - Path to the input data file.
    /// * `zisk_rom` - An `Arc`-wrapped ZisK ROM instance.
    pub fn new(input_data_path: PathBuf, zisk_rom: Arc<ZiskRom>) -> Self {
        Self {
            input_data_path,
            zisk_rom,
            secondary_sm: Vec::new(),
            min_traces: RwLock::new(Vec::new()),
            main_planning: RwLock::new(Vec::new()),
            secn_planning: RwLock::new(Vec::new()),
        }
    }

    /// Registers a secondary state machine with the executor.
    ///
    /// # Arguments
    /// * `sm` - The state machine to register.
    pub fn register_sm(&mut self, sm: Arc<dyn ComponentBuilder<F>>) {
        self.secondary_sm.push(sm);
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `input_data` - Input data for the ROM execution.
    /// * `num_threads` - Number of threads to use for parallel execution.
    ///
    /// # Returns
    /// A vector of `EmuTrace` instances representing minimal traces.
    fn compute_minimal_traces(&self, input_data: Vec<u8>, num_threads: usize) -> Vec<EmuTrace> {
        assert!(Self::MIN_TRACE_SIZE.is_power_of_two());

        // Settings for the emulator
        let emu_options = EmuOptions {
            trace_steps: Some(Self::MIN_TRACE_SIZE),
            max_steps: Self::MAX_NUM_STEPS,
            ..EmuOptions::default()
        };

        ZiskEmulator::compute_minimal_traces::<F>(
            &self.zisk_rom,
            &input_data,
            &emu_options,
            num_threads,
        )
        .expect("Error during emulator execution")
    }

    /// Adds main state machine instances to the proof context and assigns global IDs.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_planning` - Planning information for main state machines.
    fn assign_main_instances(&self, pctx: &ProofCtx<F>, main_planning: &mut [Plan]) {
        main_planning.iter_mut().for_each(|plan| {
            let (_, global_idx) = pctx.dctx_add_instance(
                plan.airgroup_id,
                plan.air_id,
                pctx.get_weight(plan.airgroup_id, plan.air_id),
            );

            plan.set_global_id(global_idx);
        });
    }

    /// Creates main state machine instances based on the main plannings.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    ///
    /// # Returns
    /// A vector of `MainInstance` objects.
    fn create_main_instances(&self, pctx: &ProofCtx<F>) -> Vec<MainInstance> {
        let mut main_planning_guard = self.main_planning.write().unwrap();
        let main_planning = std::mem::take(&mut *main_planning_guard);

        let len = main_planning.len() - 1;
        main_planning
            .into_iter()
            .filter_map(|plan| {
                let global_id = plan.global_id.unwrap();
                let is_last_segment = plan.segment_id.unwrap() == len;
                if pctx.dctx_is_my_instance(global_id) {
                    Some(MainInstance::new(InstanceCtx::new(global_id, plan), is_last_segment))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Counts metrics for secondary state machines based on minimal traces.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    ///
    /// # Returns
    /// A vector of metrics grouped by chunk ID.
    #[allow(clippy::type_complexity)]
    fn count(
        &self,
        min_traces: &[EmuTrace],
    ) -> (Vec<(usize, Box<dyn BusDeviceMetrics>)>, Vec<Vec<(usize, Box<dyn BusDeviceMetrics>)>>)
    {
        let (mut main_metrics_slices, mut secn_metrics_slices): (Vec<_>, Vec<_>) = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut data_bus = self.get_data_bus_counters();

                ZiskEmulator::process_emu_trace::<F, BusDeviceMetricsWrapper>(
                    &self.zisk_rom,
                    minimal_trace,
                    &mut data_bus,
                );

                let (mut main, mut secondary) = (Vec::new(), Vec::new());

                let result = self.close_data_bus_counters(data_bus);
                for (is_secondary, counter) in result {
                    if is_secondary {
                        secondary.push(counter);
                    } else {
                        main.push(counter);
                    }
                }
                (main, secondary)
            })
            .unzip();

        // Group counters by chunk_id and counter type
        let mut secn_vec_counters =
            (0..secn_metrics_slices[0].len()).map(|_| Vec::new()).collect::<Vec<_>>();

        for (chunk_id, counter_slice) in secn_metrics_slices.iter_mut().enumerate() {
            for (i, counter) in counter_slice.drain(..).enumerate() {
                secn_vec_counters[i].push((chunk_id, counter));
            }
        }

        let mut main_vec_counters = Vec::new();

        for (chunk_id, counter_slice) in main_metrics_slices.iter_mut().enumerate() {
            for counter in counter_slice.drain(..) {
                main_vec_counters.push((chunk_id, counter));
            }
        }

        (main_vec_counters, secn_vec_counters)
    }

    /// Plans the secondary state machines by generating plans from the counted metrics.
    ///
    /// # Arguments
    /// * `vec_counters` - A vector of counters grouped by chunk ID.
    ///
    /// # Returns
    /// A vector of plans for each secondary state machine.
    fn plan_sec(
        &self,
        mut vec_counters: Vec<Vec<(usize, Box<dyn BusDeviceMetrics>)>>,
    ) -> Vec<Vec<Plan>> {
        self.secondary_sm.iter().map(|sm| sm.build_planner().plan(vec_counters.remove(0))).collect()
    }

    /// Prepares and configures the secondary instances using the provided plans before their
    /// creation.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `plannings` - A vector of vectors containing plans for each secondary state machine.
    ///
    /// # Panics
    /// This function will panic if the length of `plannings` does not match the length of
    /// `self.secondary_sm`.
    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Vec<Plan>]) {
        self.secondary_sm
            .iter()
            .enumerate()
            .for_each(|(i, sm)| sm.configure_instances(pctx, &plannings[i]));
    }

    /// Adds secondary state machine instances to the proof context and assigns global IDs.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_planning` - Planning information for main state machines.
    #[allow(clippy::type_complexity)]
    fn assign_secn_instances(&self, pctx: &ProofCtx<F>, secn_planning: &mut [Vec<Plan>]) {
        secn_planning.iter_mut().for_each(|plans_by_sm| {
            plans_by_sm.iter_mut().for_each(move |plan| {
                let global_id = pctx.dctx_add_instance_no_assign(
                    plan.airgroup_id,
                    plan.air_id,
                    pctx.get_weight(plan.airgroup_id, plan.air_id),
                );
                plan.set_global_id(global_id);
            })
        });

        pctx.dctx_assign_instances();
    }

    /// Creates secondary state machine instances based on the plans.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    ///
    /// # Returns
    /// A tuple containing two vectors:
    /// * A vector of table instances grouped by global ID.
    /// * A vector of non-table instances grouped by global ID.
    #[allow(clippy::type_complexity)]
    fn create_secn_instances(
        &self,
        pctx: &ProofCtx<F>,
    ) -> (
        Vec<(usize, Box<dyn Instance<F>>)>, // Table instances
        Vec<(usize, Box<dyn Instance<F>>)>, // Non-table instances
    ) {
        let mut table_instances = Vec::new();
        let mut other_instances = Vec::new();

        let mut secn_planning_guard = self.secn_planning.write().unwrap();
        let secn_planning = std::mem::take(&mut *secn_planning_guard);

        secn_planning.into_iter().enumerate().for_each(|(i, plans_by_sm)| {
            plans_by_sm.into_iter().for_each(|plan| {
                let global_idx = plan.global_id.unwrap();
                let is_mine = pctx.dctx_is_my_instance(global_idx);
                let is_table = plan.instance_type == InstanceType::Table;
                if is_mine || is_table {
                    let ictx = InstanceCtx::new(global_idx, plan);
                    let instance = (global_idx, self.secondary_sm[i].build_instance(ictx));
                    if is_table {
                        table_instances.push(instance);
                    } else {
                        other_instances.push(instance);
                    }
                }
            })
        });

        (table_instances, other_instances)
    }

    /// Expands and computes witnesses for main state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_instances` - Main state machine instances to compute witnesses for
    fn witness_main_instances(&self, pctx: &ProofCtx<F>, main_instances: Vec<MainInstance>) {
        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = &*min_traces_guard;

        main_instances.into_par_iter().for_each(|mut main_instance| {
            let air_instance = MainSM::compute_witness(
                &self.zisk_rom,
                min_traces,
                Self::MIN_TRACE_SIZE,
                &mut main_instance,
            );

            pctx.air_instance_repo.add_air_instance(air_instance, main_instance.ictx.global_id);
        });
    }

    /// Expands and computes witnesses for secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `secn_instances` - Secondary state machine instances to compute witnesses for
    fn witness_secn_instances(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        secn_instances: Vec<(usize, Box<dyn Instance<F>>)>,
    ) {
        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = &*min_traces_guard;

        // Group the instances by the chunk they need to process
        let instances_by_chunk = self.chunks_to_execute(min_traces, &secn_instances);

        // Create data buses for each chunk
        let mut data_buses = self.get_data_bus_collectors(&secn_instances, instances_by_chunk);

        // Execute collect process for each chunk
        data_buses.par_iter_mut().enumerate().for_each(|(chunk_id, data_bus)| {
            if let Some(data_bus) = data_bus {
                ZiskEmulator::process_emu_traces::<F, BusDeviceWrapper<u64>>(
                    &self.zisk_rom,
                    min_traces,
                    chunk_id,
                    data_bus,
                );
            }
        });

        // Close the data buses and get for each instance its collectors
        let collectors_by_instance = self.close_data_bus_collectors(secn_instances, data_buses);

        collectors_by_instance.into_par_iter().for_each(|(global_idx, mut instance, collector)| {
            if let Some(air_instance) = instance.compute_witness(pctx, sctx, collector) {
                pctx.air_instance_repo.add_air_instance(air_instance, global_idx);
            }
        });
    }

    /// Computes and generates witnesses for secondary state machine instances of type `Table`.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `table_instances` - Secondary state machine table instances to compute witnesses for
    fn witness_tables(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        table_instances: Vec<(usize, Box<dyn Instance<F>>)>,
    ) {
        let mut instances = table_instances
            .into_iter()
            .filter(|(_, secn_instance)| secn_instance.instance_type() == InstanceType::Table)
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|(a, _), (b, _)| {
                let (airgroup_id_a, air_id_a) = pctx.dctx_get_instance_info(*a);
                let (airgroup_id_b, air_id_b) = pctx.dctx_get_instance_info(*b);

                airgroup_id_a.cmp(&airgroup_id_b).then(air_id_a.cmp(&air_id_b))
            })
            .collect::<Vec<_>>();

        instances.iter_mut().for_each(|(global_idx, secn_instance)| {
            if secn_instance.instance_type() == InstanceType::Table {
                if let Some(air_instance) = secn_instance.compute_witness(pctx, sctx, vec![]) {
                    if pctx.dctx_is_my_instance(*global_idx) {
                        pctx.air_instance_repo.add_air_instance(air_instance, *global_idx);
                    }
                }
            }
        });
    }

    /// Groups secondary state machine instances by the chunk they need to process.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces
    /// * `secn_instances` - Secondary state machine instances to group.
    ///
    /// # Returns
    /// A vector of vectors containing the indices of secondary state machine instances to execute
    /// for each chunk.
    fn chunks_to_execute(
        &self,
        min_traces: &[EmuTrace],
        secn_instances: &[(usize, Box<dyn Instance<F>>)],
    ) -> Vec<Vec<usize>> {
        let mut chunks_to_execute = vec![Vec::new(); min_traces.len()];
        secn_instances.iter().enumerate().for_each(|(idx, (_, secn_instance))| match secn_instance
            .check_point()
        {
            CheckPoint::None => {}
            CheckPoint::Single(chunk_id) => {
                chunks_to_execute[chunk_id].push(idx);
            }
            CheckPoint::Multiple(chunk_ids) => {
                chunk_ids.iter().for_each(|&chunk_id| {
                    chunks_to_execute[chunk_id].push(idx);
                });
            }
        });
        chunks_to_execute
    }

    /// Retrieves a `DataBus` configured with counters for each secondary state machine.
    ///
    /// # Returns
    /// A `DataBus` instance with connected counters for each registered secondary state machine.
    fn get_data_bus_counters(&self) -> DataBus<PayloadType, BusDeviceMetricsWrapper> {
        let mut data_bus = DataBus::new();

        let counter = MainSM::build_counter();

        data_bus.connect_device(
            counter.bus_id(),
            Box::new(BusDeviceMetricsWrapper::new(counter, false)),
        );

        self.secondary_sm.iter().for_each(|sm| {
            let counter = sm.build_counter();

            data_bus.connect_device(
                counter.bus_id(),
                Box::new(BusDeviceMetricsWrapper::new(counter, true)),
            );
        });

        data_bus
    }

    /// Finalizes a `DataBus` with counters, detaching and closing all devices.
    ///
    /// # Arguments
    /// * `data_bus` - A `DataBus` instance with attached counters.
    ///
    /// # Returns
    /// A vector containing all detached counters after closing their associated devices.
    fn close_data_bus_counters(
        &self,
        mut data_bus: DataBus<u64, BusDeviceMetricsWrapper>,
    ) -> Vec<(bool, Box<dyn BusDeviceMetrics>)> {
        data_bus
            .detach_devices()
            .into_iter()
            .map(|mut device| {
                device.on_close();
                (device.is_secondary, device.inner)
            })
            .collect::<Vec<_>>()
    }

    /// Retrieves a data bus for managing collectors in secondary state machines.
    ///
    /// # Arguments
    /// * `secn_instances` - A vector of secondary state machine instances
    /// * `chunks_to_execute` - A vector of chunk IDs to execute
    ///
    /// # Returns
    /// A vector of `DataBus` instances, each configured with collectors for the secondary state
    fn get_data_bus_collectors(
        &self,
        secn_instances: &[(usize, Box<dyn Instance<F>>)],
        chunks_to_execute: Vec<Vec<usize>>,
    ) -> Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>> {
        chunks_to_execute
            .iter()
            .enumerate()
            .map(|(chunk_id, secn_indices)| {
                if secn_indices.is_empty() {
                    return None;
                }

                let mut data_bus: DataBus<u64, BusDeviceWrapper<PayloadType>> = DataBus::new();

                for idx in secn_indices {
                    let (_, secn_instance) = &secn_instances[*idx];
                    let bus_device = secn_instance.build_inputs_collector(chunk_id);
                    if let Some(bus_device) = bus_device {
                        let bus_device = Box::new(BusDeviceWrapper::new(Some(*idx), bus_device));
                        data_bus.connect_device(bus_device.bus_id(), bus_device);
                    }
                }

                self.secondary_sm.iter().for_each(|sm| {
                    let inputs_generator = sm.build_inputs_generator();

                    if let Some(inputs_generator) = inputs_generator {
                        data_bus.connect_device(
                            vec![OPERATION_BUS_ID],
                            Box::new(BusDeviceWrapper::new(None, inputs_generator)),
                        );
                    }
                });

                Some(data_bus)
            })
            .collect::<Vec<_>>()
    }

    /// Closes a data bus used for managing collectors and returns the first instance.
    ///
    /// # Arguments
    /// * `secn_instances` - A vector of secondary state machine instances.
    /// * `data_buses` - A vector of data buses with attached collectors.
    ///
    /// # Returns
    /// A vector of tuples containing the global ID, secondary state machine instance, and a vector
    /// of collectors for each instance.
    #[allow(clippy::type_complexity)]
    fn close_data_bus_collectors(
        &self,
        secn_instances: Vec<(usize, Box<dyn Instance<F>>)>,
        mut data_buses: Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>>,
    ) -> Vec<(usize, Box<dyn Instance<F>>, Vec<(usize, Box<BusDeviceWrapper<u64>>)>)> {
        let mut collectors_by_instance = Vec::new();
        for (global_id, secn_instance) in secn_instances {
            collectors_by_instance.push((global_id, secn_instance, Vec::new()));
        }

        for (chunk_id, data_bus) in data_buses.iter_mut().enumerate() {
            if let Some(data_bus) = data_bus {
                let collectors = data_bus.detach_devices();
                for collector in collectors {
                    if let Some(idx) = collector.instance_idx() {
                        collectors_by_instance[idx].2.push((chunk_id, collector));
                    }
                }
            }
        }
        collectors_by_instance
    }
}

impl<F: PrimeField> WitnessComponent<F> for ZiskExecutor<F> {
    /// Executes the ZisK ROM program and calculate the plans for main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        // Call emulate with these options
        let input_data = {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(self.input_data_path.display().to_string());
            fs::read(path).expect("Could not read inputs file")
        };

        // PHASE 1. MINIMAL TRACES. Process the ROM super fast to collect the Minimal Traces
        timer_start_info!(COMPUTE_MINIMAL_TRACE);
        let min_traces = self.compute_minimal_traces(input_data, Self::NUM_THREADS);
        timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

        timer_start_info!(COUNT_AND_PLAN);
        // PHASE 2. COUNTING. Count the metrics for the Secondary SM instances
        let (main_count, secn_count) = self.count(&min_traces);

        // PHASE 3. PLANNING. Plan the instances
        let (mut main_planning, public_values) =
            MainPlanner::plan::<F>(&min_traces, main_count, Self::MIN_TRACE_SIZE);

        // Update pctx
        let mut publics = ZiskPublicValues::from_vec_guard(pctx.get_publics());

        for (index, value) in public_values.iter() {
            publics.inputs[*index as usize] = F::from_canonical_u32(*value);
        }

        let mut secn_planning = self.plan_sec(secn_count);

        // PHASE 4. PLANNING. Plan the instances
        self.configure_instances(&pctx, &secn_planning);

        timer_stop_and_log_info!(COUNT_AND_PLAN);

        // PHASE 5. INSTANCES. Assign the instances
        self.assign_main_instances(&pctx, &mut main_planning);
        self.assign_secn_instances(&pctx, &mut secn_planning);

        *self.min_traces.write().unwrap() = min_traces;
        *self.main_planning.write().unwrap() = main_planning;
        *self.secn_planning.write().unwrap() = secn_planning;
    }

    /// Computes the witness for the main and secondary state machines.
    ///
    /// # Arguments
    /// * `stage` - The current stage id
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    fn calculate_witness(&self, stage: u32, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        if stage == 1 {
            // PHASE 6. WITNESS. Compute the witnesses
            let main_instances = self.create_main_instances(&pctx);
            let (table_instances, secn_instances) = self.create_secn_instances(&pctx);

            self.witness_main_instances(&pctx, main_instances);
            self.witness_secn_instances(&pctx, &sctx, secn_instances);
            self.witness_tables(&pctx, &sctx, table_instances);
        }
    }

    /// Debugs the main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    fn debug(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        let (table_instances, secn_instances) = self.create_secn_instances(&pctx);

        MainSM::debug(&pctx, &sctx);

        let mut debug_airs: HashMap<(usize, usize), bool> = HashMap::new();

        secn_instances.iter().for_each(|(global_idx, secn_instance)| {
            let instance_info = pctx.dctx_get_instance_info(*global_idx);
            if secn_instance.instance_type() == InstanceType::Instance
                && !debug_airs.contains_key(&instance_info)
            {
                debug_airs.insert(instance_info, true);
                secn_instance.debug(&pctx, &sctx);
            }
        });

        table_instances.iter().for_each(|(global_idx, secn_instance)| {
            let instance_info = pctx.dctx_get_instance_info(*global_idx);
            if secn_instance.instance_type() == InstanceType::Table
                && pctx.dctx_is_my_instance(*global_idx)
                && !debug_airs.contains_key(&instance_info)
            {
                debug_airs.insert(instance_info, true);
                secn_instance.debug(&pctx, &sctx);
            }
        });
    }
}
