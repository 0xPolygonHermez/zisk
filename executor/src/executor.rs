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

use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rom_merkle::gen_elf_hash;
use witness::WitnessComponent;

use rayon::prelude::*;

use data_bus::{BusDevice, DataBus, PayloadType, OPERATION_BUS_ID};
use sm_common::{
    BusDeviceMetrics, BusDeviceMetricsWrapper, BusDeviceWrapper, CheckPoint, ComponentBuilder,
    Instance, InstanceCtx, InstanceType, Plan,
};
use sm_main::{MainInstance, MainPlanner, MainSM};
use zisk_pil::{RomRomTrace, ZiskPublicValues, MAIN_AIR_IDS};

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
    pub input_data_path: Option<PathBuf>,

    pub rom_path: Option<PathBuf>,

    /// Registered secondary state machines.
    secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>,

    /// Planning information for main state machines.
    pub min_traces: RwLock<Vec<EmuTrace>>,
    pub main_planning: RwLock<Vec<Plan>>,
    pub secn_planning: RwLock<Vec<Vec<Plan>>>,

    pub main_instances: RwLock<HashMap<usize, MainInstance>>,
    pub secn_instances: RwLock<HashMap<usize, Box<dyn Instance<F>>>>,
    std: Arc<Std<F>>,
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
    pub fn new(
        input_data_path: Option<PathBuf>,
        rom_path: Option<PathBuf>,
        zisk_rom: Arc<ZiskRom>,
        std: Arc<Std<F>>,
    ) -> Self {
        Self {
            input_data_path,
            rom_path,
            zisk_rom,
            secondary_sm: Vec::new(),
            min_traces: RwLock::new(Vec::new()),
            main_planning: RwLock::new(Vec::new()),
            secn_planning: RwLock::new(Vec::new()),
            main_instances: RwLock::new(HashMap::new()),
            secn_instances: RwLock::new(HashMap::new()),
            std,
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
            let global_id = pctx.add_instance(plan.airgroup_id, plan.air_id);

            plan.set_global_id(global_id);
        });
    }

    /// Creates main state machine instance based on a main planning.
    ///
    /// # Arguments
    /// * `global_id` - Global ID of the main instance to be created.
    ///
    /// # Returns
    /// A vector of `MainInstance` object.
    fn create_main_instance(&self, global_id: usize) -> MainInstance {
        let mut main_planning_guard = self.main_planning.write().unwrap();

        let plan_idx = main_planning_guard.iter().position(|x| x.global_id.unwrap() == global_id);
        if plan_idx.is_none() {
            panic!("Main instance not found");
        }

        let plan_idx = plan_idx.unwrap();
        let plan = main_planning_guard.remove(plan_idx);

        let global_id = plan.global_id.unwrap();
        let is_last_segment = *plan.meta.as_ref().unwrap().downcast_ref::<bool>().unwrap();

        MainInstance::new(InstanceCtx::new(global_id, plan), is_last_segment)
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
                if plan.instance_type == InstanceType::Instance {
                    let global_id = pctx.add_instance(plan.airgroup_id, plan.air_id);
                    plan.set_global_id(global_id);
                } else {
                    let global_id = pctx.add_instance_all(plan.airgroup_id, plan.air_id);
                    plan.set_global_id(global_id);
                }
            })
        });
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

    fn create_secn_instance(&self, global_id: usize) -> Box<dyn Instance<F>> {
        let mut secn_planning_guard = self.secn_planning.write().unwrap();

        let plan_idx = secn_planning_guard.iter().enumerate().find_map(|(outer_idx, plans)| {
            plans
                .iter()
                .position(|plan| plan.global_id.unwrap() == global_id)
                .map(|inner_idx| (outer_idx, inner_idx))
        });
        if plan_idx.is_none() {
            panic!("Secondary instance not found");
        }

        let plan_idx = plan_idx.unwrap();
        let plan = secn_planning_guard[plan_idx.0].remove(plan_idx.1);

        let global_id = plan.global_id.unwrap();

        // let is_mine = pctx.dctx_is_my_instance(global_id);
        // let is_table = plan.instance_type == InstanceType::Table;
        // if is_mine || is_table {
        //     let ictx = InstanceCtx::new(global_id, plan);
        //     let instance = (global_id, self.secondary_sm[plan_idx.0].build_instance(ictx));
        // }

        // // TODO! What to do if it's not mine and is not a table ?
        // instance

        let ictx = InstanceCtx::new(global_id, plan);
        self.secondary_sm[plan_idx.0].build_instance(ictx)
    }

    /// Expands and computes witnesses for a main instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_instance` - Main instance to compute witness for
    fn witness_main_instance(&self, pctx: &ProofCtx<F>, main_instance: &mut MainInstance) {
        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = &*min_traces_guard;

        let air_instance = MainSM::compute_witness(
            &self.zisk_rom,
            min_traces,
            Self::MIN_TRACE_SIZE,
            main_instance,
            self.std.clone(),
        );

        pctx.add_air_instance(air_instance, main_instance.ictx.global_id);
    }

    /// Expands and computes witness for a secondary state machines instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_id` - Global ID of the secondary state machine instance.
    /// * `secn_instance` - Secondary state machine instance to compute witness for
    fn witness_secn_instance(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        secn_instance: &mut Box<dyn Instance<F>>,
    ) {
        if secn_instance.instance_type() != InstanceType::Instance {
            panic!("Instance is not a secondary state machine instance");
        }

        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = &*min_traces_guard;

        // Group the instances by the chunk they need to process
        let chunks_to_execute = self.chunks_to_execute(min_traces, secn_instance);

        // Create data buses for each chunk
        let mut data_buses = self.get_data_bus_collectors(secn_instance, chunks_to_execute);

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
        let collectors_by_instance = self.close_data_bus_collectors(data_buses);

        if let Some(air_instance) =
            secn_instance.compute_witness(pctx, sctx, collectors_by_instance)
        {
            pctx.add_air_instance(air_instance, global_id);
        }
    }

    /// Computes and generates witness for secondary state machine instance of type `Table`.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_id` - Global ID of the secondary state machine instance.
    /// * `table_instance` - Secondary state machine table instance to compute witness for
    fn witness_table(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        table_instance: &mut Box<dyn Instance<F>>,
    ) {
        if table_instance.instance_type() != InstanceType::Table {
            panic!("Instance is not a table");
        }

        if let Some(air_instance) = table_instance.compute_witness(pctx, sctx, vec![]) {
            if pctx.dctx_is_my_instance(global_id) {
                pctx.add_air_instance(air_instance, global_id);
            }
        }
    }

    /// Computes all the chunks to be executed to generate the witness given an instance.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces
    /// * `secn_instance` - Secondary state machine instance to group.
    ///
    /// # Returns
    /// A vector of booleans indicating which chunks to execute.
    fn chunks_to_execute(
        &self,
        min_traces: &[EmuTrace],
        secn_instance: &Box<dyn Instance<F>>,
    ) -> Vec<bool> {
        let mut chunks_to_execute = vec![false; min_traces.len()];

        match secn_instance.check_point() {
            CheckPoint::None => {}
            CheckPoint::Single(chunk_id) => {
                chunks_to_execute[chunk_id] = true;
            }
            CheckPoint::Multiple(chunk_ids) => {
                chunk_ids.iter().for_each(|&chunk_id| {
                    chunks_to_execute[chunk_id] = true;
                });
            }
        };
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
    /// # Arguments
    /// * `secn_instance` - Secondary state machine instance.
    /// * `chunks_to_execute` - A vector of booleans indicating which chunks to execute.
    ///
    /// # Returns
    /// A vector of data buses with attached collectors for each chunk to be executed
    fn get_data_bus_collectors(
        &self,
        secn_instance: &Box<dyn Instance<F>>,
        chunks_to_execute: Vec<bool>,
    ) -> Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>> {
        chunks_to_execute
            .iter()
            .enumerate()
            .map(|(chunk_id, to_be_executed)| {
                if !to_be_executed {
                    return None;
                }

                let mut data_bus: DataBus<u64, BusDeviceWrapper<PayloadType>> = DataBus::new();

                let bus_device = secn_instance.build_inputs_collector(chunk_id);
                if let Some(bus_device) = bus_device {
                    // TODO!!!!!!!!  Canviar!!! Treure aquest BusDeviceWrapper!!!!!
                    let bus_device = Box::new(BusDeviceWrapper::new(Some(1), bus_device));
                    data_bus.connect_device(bus_device.bus_id(), bus_device);
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
    fn close_data_bus_collectors(
        &self,
        mut data_buses: Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>>,
    ) -> Vec<(usize, Box<BusDeviceWrapper<u64>>)> {
        let mut collectors_by_instance = Vec::new();

        for (chunk_id, data_bus) in data_buses.iter_mut().enumerate() {
            if let Some(data_bus) = data_bus {
                let collectors = data_bus.detach_devices();
                for collector in collectors {
                    if let Some(_) = collector.instance_idx() {
                        // TODO!!!! Remove this Some...
                        collectors_by_instance.push((chunk_id, collector));
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
    fn execute(&self, pctx: Arc<ProofCtx<F>>) -> Vec<usize> {
        // Call emulate with these options
        let input_data = {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(self.input_data_path.as_ref().unwrap().display().to_string());
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

        let main_global_ids =
            main_planning.iter().map(|plan| plan.global_id.unwrap()).collect::<Vec<_>>();
        let secn_global_ids = secn_planning
            .iter()
            .map(|plans| plans.iter().map(|plan| plan.global_id.unwrap()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let secn_global_ids = secn_global_ids.iter().flatten().map(|x| *x).collect::<Vec<_>>();

        *self.min_traces.write().unwrap() = min_traces;
        *self.main_planning.write().unwrap() = main_planning;
        *self.secn_planning.write().unwrap() = secn_planning;

        [main_global_ids, secn_global_ids].concat()
    }

    /// Computes the witness for the main and secondary state machines.
    ///
    /// # Arguments
    /// * `stage` - The current stage id
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    fn calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
    ) {
        if stage != 1 {
            return;
        }

        global_ids.iter().for_each(|&global_id| {
            let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);
            println!("Calculating witness for instance: {:?} air_id: {:?}", global_id, air_id);

            if MAIN_AIR_IDS.contains(&air_id) {
                let mut main_instances_guard = self.main_instances.write().unwrap();

                let main_instance = main_instances_guard
                    .entry(global_id)
                    .or_insert_with(|| self.create_main_instance(global_id));

                self.witness_main_instance(&pctx, main_instance);
            } else {
                let mut secn_instances_guard = self.secn_instances.write().unwrap();

                let secn_instance = secn_instances_guard
                    .entry(global_id)
                    .or_insert_with(|| self.create_secn_instance(global_id));

                match secn_instance.instance_type() {
                    InstanceType::Instance => {
                        self.witness_secn_instance(&pctx, &sctx, global_id, secn_instance)
                    }
                    InstanceType::Table => {
                        self.witness_table(&pctx, &sctx, global_id, secn_instance)
                    }
                }
            }
        });
    }

    /// Debugs the main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    fn debug(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx<F>>, global_ids: &[usize]) {
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

    fn gen_custom_commits_fixed(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        check: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_name = pctx.get_custom_commits_fixed_buffer("rom")?;

        let setup = sctx.get_setup(RomRomTrace::<usize>::AIRGROUP_ID, RomRomTrace::<usize>::AIR_ID);
        let blowup_factor =
            1 << (setup.stark_info.stark_struct.n_bits_ext - setup.stark_info.stark_struct.n_bits);

        gen_elf_hash(&self.rom_path.clone().unwrap(), file_name, blowup_factor, check)?;
        Ok(())
    }
}
