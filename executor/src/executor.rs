//! The `ZiskExecutor` module provides the main logic for orchestrating the execution of the ZisK
//! ROM program to generate the witness computation. It is responsible for managing state machines,
//! planning instances, and computing witnesses for both main and secondary state machines,
//! leveraging parallel processing for efficiency.

use itertools::Itertools;
use p3_field::PrimeField;
use proofman_common::ProofCtx;
use witness::WitnessComponent;

use rayon::prelude::*;

use sm_common::{
    BusDeviceInstance, BusDeviceInstanceWrapper, BusDeviceMetrics, BusDeviceMetricsWrapper,
    CheckPoint, ComponentBuilder, InstanceCtx, InstanceType, Plan,
};
use sm_main::{MainInstance, MainPlanner, MainSM};
use zisk_common::{DataBus, PayloadType};

use std::{fs, path::PathBuf, sync::Arc};
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, EmuTrace, ZiskEmulator};

use rayon::iter::Either;

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField> {
    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    pub zisk_rom: Arc<ZiskRom>,

    /// Path to the public inputs file.
    pub public_inputs_path: PathBuf,

    /// Registered secondary state machines.
    secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>,
}

impl<F: PrimeField> ZiskExecutor<F> {
    /// The number of threads to use for parallel processing.
    const NUM_THREADS: usize = 8;
    const MIN_TRACE_SIZE: u64 = 1 << 21;

    /// Creates a new instance of the `ZiskExecutor`.
    ///
    /// # Arguments
    /// * `public_inputs_path` - Path to the public inputs file.
    /// * `zisk_rom` - An `Arc`-wrapped ZisK ROM instance.
    pub fn new(public_inputs_path: PathBuf, zisk_rom: Arc<ZiskRom>) -> Self {
        Self { public_inputs_path, zisk_rom, secondary_sm: Vec::new() }
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
    /// * `public_inputs` - Public inputs for the ROM execution.
    /// * `num_threads` - Number of threads to use for parallel execution.
    ///
    /// # Returns
    /// A vector of `EmuTrace` instances representing minimal traces.
    fn compute_minimal_traces(&self, public_inputs: Vec<u8>, num_threads: usize) -> Vec<EmuTrace> {
        assert!(Self::MIN_TRACE_SIZE.is_power_of_two());

        // Settings for the emulator
        let emu_options =
            EmuOptions { trace_steps: Some(Self::MIN_TRACE_SIZE), ..EmuOptions::default() };

        ZiskEmulator::compute_minimal_traces::<F>(
            &self.zisk_rom,
            &public_inputs,
            &emu_options,
            num_threads,
        )
        .expect("Error during emulator execution")
    }

    /// Creates main state machine instances based on the provided planning.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `main_planning` - Planning information for main state machines.
    ///
    /// # Returns
    /// A vector of `MainInstance` objects.
    fn create_main_instances(
        &self,
        pctx: &ProofCtx<F>,
        main_planning: Vec<Plan>,
    ) -> Vec<MainInstance> {
        main_planning
            .into_iter()
            .filter_map(|plan| {
                if let (true, global_idx) = pctx.dctx_add_instance(
                    plan.airgroup_id,
                    plan.air_id,
                    pctx.get_weight(plan.airgroup_id, plan.air_id),
                ) {
                    Some(MainInstance::new(InstanceCtx::new(global_idx, plan)))
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
    fn count_sec(&self, min_traces: &[EmuTrace]) -> Vec<Vec<(usize, Box<dyn BusDeviceMetrics>)>> {
        if self.secondary_sm.is_empty() {
            return Vec::new();
        }

        let mut metrics_slices = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut data_bus = self.get_data_bus_counters();

                ZiskEmulator::process_emu_trace::<F, BusDeviceMetricsWrapper>(
                    &self.zisk_rom,
                    minimal_trace,
                    &mut data_bus,
                );

                self.close_data_bus_counters(data_bus)
            })
            .collect::<Vec<_>>();

        // Group counters by chunk_id and counter type
        let mut vec_counters = (0..metrics_slices[0].len()).map(|_| Vec::new()).collect::<Vec<_>>();

        for (chunk_id, counter_slice) in metrics_slices.iter_mut().enumerate() {
            for (i, counter) in counter_slice.drain(..).enumerate() {
                vec_counters[i].push((chunk_id, counter));
            }
        }

        vec_counters
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
    /// * `pctx` - A reference to the proof context, providing shared resources for configuration.
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

    /// Creates secondary state machine instances based on their plans.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `plans` - A vector of plans for each secondary state machine.
    ///
    /// # Returns
    /// A tuple containing two vectors:
    /// * A vector of table instances.
    /// * A vector of non-table instances.
    #[allow(clippy::type_complexity)]
    fn create_sec_instances(
        &self,
        pctx: &ProofCtx<F>,
        plans: Vec<Vec<Plan>>,
    ) -> (
        Vec<(usize, Box<dyn BusDeviceInstance<F>>)>, // Table instances
        Vec<(usize, Box<dyn BusDeviceInstance<F>>)>, // Non-table instances
    ) {
        let gids: Vec<_> = plans
            .into_iter()
            .enumerate()
            .flat_map(|(i, plans_by_sm)| {
                plans_by_sm.into_iter().map(move |plan| {
                    Some((
                        pctx.dctx_add_instance_no_assign(
                            plan.airgroup_id,
                            plan.air_id,
                            pctx.get_weight(plan.airgroup_id, plan.air_id),
                        ),
                        plan.instance_type == InstanceType::Table,
                        plan,
                        i,
                    ))
                })
            })
            .collect();

        pctx.dctx_assign_instances();

        let mut table_instances = Vec::new();
        let mut other_instances = Vec::new();

        gids.into_iter().for_each(|item| {
            if let Some((global_idx, is_table, plan, i)) = item {
                let is_mine = pctx.dctx_is_my_instance(global_idx);
                if is_mine || is_table {
                    let ictx = InstanceCtx::new(global_idx, plan);
                    let instance = (global_idx, self.secondary_sm[i].build_inputs_collector(ictx));
                    if is_table {
                        table_instances.push(instance);
                    } else {
                        other_instances.push(instance);
                    }
                }
            }
        });

        (table_instances, other_instances)
    }

    /// Expands and computes witnesses for main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    /// * `main_instances` - Main state machine instances to compute witnesses for
    /// * `secn_instances` - Secondary state machine instances to compute witnesses for
    fn witness_instances(
        &self,
        pctx: &ProofCtx<F>,
        min_traces: &[EmuTrace],
        main_instances: Vec<MainInstance>,
        secn_instances: Vec<(usize, Box<dyn BusDeviceInstance<F>>)>,
    ) {
        // Combine main_instances and secn_instances into a single parallel iterator
        let main_iter = main_instances.into_par_iter().map(|mut main_instance| {
            Either::Left(move || {
                MainSM::prove_main(
                    pctx,
                    &self.zisk_rom,
                    min_traces,
                    Self::MIN_TRACE_SIZE,
                    &mut main_instance,
                );
            })
        });

        let secn_iter = secn_instances.into_par_iter().map(|(global_id, mut secn_instance)| {
            Either::Right(move || {
                match secn_instance.check_point() {
                    CheckPoint::None => {}
                    CheckPoint::Single(chunk_id) => {
                        secn_instance =
                            self.process_checkpoint(min_traces, secn_instance, &[chunk_id], false);
                    }
                    CheckPoint::Multiple(chunk_ids) => {
                        secn_instance =
                            self.process_checkpoint(min_traces, secn_instance, &chunk_ids, true);
                    }
                }

                if let Some(air_instance) = secn_instance.compute_witness(pctx) {
                    pctx.air_instance_repo.add_air_instance(air_instance, global_id);
                }
            })
        });

        // Chain the two iterators and process them concurrently
        main_iter.chain(secn_iter).for_each(|task| match task {
            Either::Left(mut main_task) => main_task(),
            Either::Right(sec_task) => sec_task(),
        });
    }

    /// Computes and generates witnesses for secondary state machine instances of type `Table`.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `collected_instances` - A vector of collected secondary state machine instances.
    fn witness_tables(
        &self,
        pctx: &ProofCtx<F>,
        table_instances: Vec<(usize, Box<dyn BusDeviceInstance<F>>)>,
    ) {
        let mut instances = table_instances
            .into_iter()
            .filter(|(_, sec_instance)| sec_instance.instance_type() == InstanceType::Table)
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|(a, _), (b, _)| {
                let (airgroup_id_a, air_id_a) = pctx.dctx_get_instance_info(*a);
                let (airgroup_id_b, air_id_b) = pctx.dctx_get_instance_info(*b);

                airgroup_id_a.cmp(&airgroup_id_b).then(air_id_a.cmp(&air_id_b))
            })
            .collect::<Vec<_>>();

        instances.iter_mut().for_each(|(global_idx, sec_instance)| {
            if sec_instance.instance_type() == InstanceType::Table {
                if let Some(air_instance) = sec_instance.compute_witness(pctx) {
                    if pctx.dctx_is_my_instance(*global_idx) {
                        pctx.air_instance_repo.add_air_instance(air_instance, *global_idx);
                    }
                }
            }
        });
    }

    /// Processes a checkpoint to compute the witness for a secondary state machine instance.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    /// * `sec_instance` - The secondary state machine instance to process.
    /// * `chunk_ids` - The chunk IDs that the instance needs to process.
    ///
    /// # Returns
    /// The updated secondary instance after processing the checkpoint.
    fn process_checkpoint(
        &self,
        min_traces: &[EmuTrace],
        sec_instance: Box<dyn BusDeviceInstance<F>>,
        chunk_ids: &[usize],
        is_multiple: bool,
    ) -> Box<dyn BusDeviceInstance<F>> {
        let mut data_bus = self.get_data_bus_collectors(sec_instance);
        chunk_ids.iter().for_each(|&chunk_id| {
            ZiskEmulator::process_emu_traces::<F, BusDeviceInstanceWrapper<F>>(
                &self.zisk_rom,
                min_traces,
                chunk_id,
                &mut data_bus,
                is_multiple,
            );
        });

        self.close_data_bus_collectors(data_bus)
    }

    /// Retrieves a `DataBus` configured with counters for each secondary state machine.
    ///
    /// # Returns
    /// A `DataBus` instance with connected counters for each registered secondary state machine.
    fn get_data_bus_counters(&self) -> DataBus<PayloadType, BusDeviceMetricsWrapper> {
        let mut data_bus = DataBus::new();
        self.secondary_sm.iter().for_each(|sm| {
            let counter = sm.build_counter();

            data_bus
                .connect_device(counter.bus_id(), Box::new(BusDeviceMetricsWrapper::new(counter)));
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
    ) -> Vec<Box<dyn BusDeviceMetrics>> {
        data_bus
            .detach_devices()
            .into_iter()
            .map(|mut device| {
                device.on_close();
                device.inner
            })
            .collect::<Vec<_>>()
    }

    /// Retrieves a data bus for managing collectors in secondary state machines.
    ///
    /// # Arguments
    /// * `sec_instance` - The secondary state machine instance to manage.
    ///
    /// # Returns
    /// A `DataBus` instance with collectors connected.
    fn get_data_bus_collectors(
        &self,
        sec_instance: Box<dyn BusDeviceInstance<F>>,
    ) -> DataBus<u64, BusDeviceInstanceWrapper<F>> {
        let mut data_bus = DataBus::new();

        let bus_device_instance = sec_instance;
        data_bus.connect_device(
            bus_device_instance.bus_id(),
            Box::new(BusDeviceInstanceWrapper::new(bus_device_instance)),
        );

        self.secondary_sm.iter().for_each(|sm| {
            if let Some(input_generator) = sm.build_inputs_generator() {
                data_bus.connect_device(
                    input_generator.bus_id(),
                    Box::new(BusDeviceInstanceWrapper::new(input_generator)),
                );
            }
        });
        data_bus
    }

    /// Closes a data bus used for managing collectors and returns the first instance.
    ///
    /// # Arguments
    /// * `data_bus` - The `DataBus` instance to close.
    ///
    /// # Returns
    /// The first `BusDeviceInstance` after detaching the bus.
    fn close_data_bus_collectors(
        &self,
        mut data_bus: DataBus<u64, BusDeviceInstanceWrapper<F>>,
    ) -> Box<dyn BusDeviceInstance<F>> {
        data_bus.devices.remove(0).inner
    }
}

impl<F: PrimeField> WitnessComponent<F> for ZiskExecutor<F> {
    /// Executes the ZisK ROM program and computes all necessary witnesses.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances and computation.
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        // Call emulate with these options
        let public_inputs = {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(self.public_inputs_path.display().to_string());
            fs::read(path).expect("Could not read inputs file")
        };

        // PHASE 1. MINIMAL TRACES. Process the ROM super fast to collect the Minimal Traces
        let min_traces = self.compute_minimal_traces(public_inputs, Self::NUM_THREADS);

        // PHASE 2. COUNTING. Count the metrics for the Secondary SM instances
        let sec_count = self.count_sec(&min_traces);

        // PHASE 3. PLANNING. Plan the instances
        let main_planning = MainPlanner::plan::<F>(&min_traces, Self::MIN_TRACE_SIZE);
        let sec_planning = self.plan_sec(sec_count);

        // PHASE 4. PLANNING. Plan the instances
        self.configure_instances(&pctx, &sec_planning);

        // PHASE 5. INSTANCES. Create the instances
        let main_instances = self.create_main_instances(&pctx, main_planning);
        let (table_instances, secn_instances) = self.create_sec_instances(&pctx, sec_planning);

        // PHASE 6. WITNESS. Compute the witnesses
        self.witness_instances(&pctx, &min_traces, main_instances, secn_instances);
        self.witness_tables(&pctx, table_instances);
    }
}
