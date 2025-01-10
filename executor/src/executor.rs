//! The `ZiskExecutor` module provides the main logic for orchestrating the execution of the ZisK
//! ROM program to generate the witness computation. It is responsible for managing state machines,
//! planning instances, and computing witnesses for both main and secondary state machines,
//! leveraging parallel processing for efficiency.

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
        // Settings for the emulator
        let emu_options = EmuOptions {
            trace_steps: Some(MainSM::non_continuation_rows::<F>()),
            ..EmuOptions::default()
        };

        ZiskEmulator::process_rom_min_trace::<F>(
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
                if let (true, global_idx) = pctx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1)
                {
                    Some(MainInstance::new(InstanceCtx::new(global_idx, plan)))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Expands and computes witnesses for main state machines in parallel.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    /// * `main_layouts` - Main instances to compute witnesses for.
    ///
    /// # Returns
    /// A thread handle for the witness computation task.
    fn expand_and_witness_main(
        &self,
        pctx: Arc<ProofCtx<F>>,
        min_traces: Arc<Vec<EmuTrace>>,
        mut main_layouts: Vec<MainInstance>,
    ) -> std::thread::JoinHandle<Vec<MainInstance>> {
        let zisk_rom = self.zisk_rom.clone();
        std::thread::spawn(move || {
            main_layouts.par_iter_mut().for_each(|main_instance| {
                MainSM::prove_main(pctx.clone(), &zisk_rom, &min_traces, main_instance);
            });

            main_layouts
        })
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

                ZiskEmulator::process_rom_slice_counters::<F, BusDeviceMetricsWrapper>(
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

    /// Creates secondary state machine instances based on their plans.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `plans` - A vector of plans for each secondary state machine.
    ///
    /// # Returns
    /// A vector of collected instances paired with their global indices.
    fn create_sec_instances(
        &self,
        pctx: &ProofCtx<F>,
        plans: Vec<Vec<Plan>>,
    ) -> Vec<(usize, Box<dyn BusDeviceInstance<F>>)> {
        plans
            .into_iter()
            .enumerate()
            .flat_map(|(i, plans_by_sm)| {
                plans_by_sm.into_iter().filter_map(move |plan| {
                    let (is_mine, global_idx) =
                        pctx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1);

                    if is_mine || plan.instance_type == InstanceType::Table {
                        let ictx = InstanceCtx::new(global_idx, plan);
                        Some((global_idx, self.secondary_sm[i].build_inputs_collector(ictx)))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Expands and computes witnesses for secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    /// * `sec_instances` - Secondary state machine instances to compute witnesses for.
    ///
    /// # Returns
    /// A vector of expanded secondary instances paired with their global indices.
    fn collect_and_witness_sec(
        &self,
        pctx: &ProofCtx<F>,
        min_traces: Arc<Vec<EmuTrace>>,
        sec_instances: Vec<(usize, Box<dyn BusDeviceInstance<F>>)>,
    ) -> Vec<(usize, Box<dyn BusDeviceInstance<F>>)> {
        sec_instances
            .into_par_iter()
            .map(|(global_idx, mut sec_instance)| {
                if sec_instance.instance_type() == InstanceType::Instance {
                    match sec_instance.check_point() {
                        CheckPoint::None => {}
                        CheckPoint::Single(chunk_id) => {
                            sec_instance =
                                self.process_checkpoint(&min_traces, sec_instance, &[chunk_id]);
                        }
                        CheckPoint::Multiple(chunk_ids) => {
                            sec_instance =
                                self.process_checkpoint(&min_traces, sec_instance, &chunk_ids);
                        }
                    }

                    if let Some(air_instance) = sec_instance.compute_witness(pctx) {
                        pctx.air_instance_repo.add_air_instance(air_instance, Some(global_idx));
                    }
                }
                (global_idx, sec_instance)
            })
            .collect()
    }

    /// Computes and generates witnesses for secondary state machine instances of type `Table`.
    ///
    /// # Arguments
    /// * `pctx` - Proof context for managing air instances.
    /// * `collected_instances` - A vector of collected secondary state machine instances.
    fn witness_sec_tables(
        &self,
        pctx: &ProofCtx<F>,
        mut collected_instances: Vec<(usize, Box<dyn BusDeviceInstance<F>>)>,
    ) {
        collected_instances.par_iter_mut().for_each(|(global_idx, sec_instance)| {
            if sec_instance.instance_type() == InstanceType::Table {
                if let Some(air_instance) = sec_instance.compute_witness(pctx) {
                    pctx.air_instance_repo.add_air_instance(air_instance, Some(*global_idx));
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
    ) -> Box<dyn BusDeviceInstance<F>> {
        let mut data_bus = self.get_data_bus_collectors(sec_instance);

        chunk_ids.iter().for_each(|&chunk_id| {
            ZiskEmulator::process_rom_slice_plan::<F, BusDeviceInstanceWrapper<F>>(
                &self.zisk_rom,
                min_traces,
                chunk_id,
                &mut data_bus,
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
        // --------------------------------------------------------------------------------------------------
        let min_traces = self.compute_minimal_traces(public_inputs, Self::NUM_THREADS);
        let min_traces = Arc::new(min_traces);

        // --- PATH A Main SM instances
        // Count, Plan and create the Main SM instances + Compute the Main Witnesses
        // --------------------------------------------------------------------------------------------------
        let main_planning = MainPlanner::plan(&min_traces);
        let main_instances = self.create_main_instances(&pctx, main_planning);
        let main_task =
            self.expand_and_witness_main(pctx.clone(), min_traces.clone(), main_instances);

        // --- PATH B Secondary SM instances
        // Count, Plan and create the Secondary SM instances + Expand and Compute the Witnesses
        // --------------------------------------------------------------------------------------------------
        let sec_count = self.count_sec(&min_traces);
        let sec_planning = self.plan_sec(sec_count);
        let sec_instances = self.create_sec_instances(&pctx, sec_planning);
        let sec_expanded = self.collect_and_witness_sec(&pctx, min_traces, sec_instances);
        self.witness_sec_tables(&pctx, sec_expanded);

        // Wait for the main task to finish
        main_task.join().unwrap();
    }
}
