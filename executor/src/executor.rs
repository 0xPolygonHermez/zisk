use p3_field::PrimeField;
use proofman_common::ProofCtx;
use witness::WitnessComponent;

use rayon::prelude::*;

use sm_common::{
    BusDeviceInstance, BusDeviceInstanceWrapper, BusDeviceMetrics, BusDeviceMetricsWrapper,
    CheckPoint, ComponentProvider, InstanceCtx, InstanceType, Plan,
};
use sm_main::{MainInstance, MainPlanner, MainSM};
use zisk_common::{DataBus, PayloadType, OPERATION_BUS_ID};

use std::{fs, path::PathBuf, sync::Arc};
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, EmuTrace, ZiskEmulator};

pub struct ZiskExecutor<F: PrimeField> {
    /// ZisK ROM, a binary file that contains the ZisK program to be executed
    pub zisk_rom: Arc<ZiskRom>,

    /// Path to the public inputs file
    pub public_inputs_path: PathBuf,

    /// Secondary State Machines
    secondary_sm: Vec<Arc<dyn ComponentProvider<F>>>,
}

impl<F: PrimeField> ZiskExecutor<F> {
    const NUM_THREADS: usize = 8;

    pub fn new(public_inputs_path: PathBuf, zisk_rom: Arc<ZiskRom>) -> Self {
        Self { public_inputs_path, zisk_rom, secondary_sm: Vec::new() }
    }

    pub fn register_sm(&mut self, sm: Arc<dyn ComponentProvider<F>>) {
        self.secondary_sm.push(sm);
    }

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

    fn plan_sec(
        &self,
        mut vec_counters: Vec<Vec<(usize, Box<dyn BusDeviceMetrics>)>>,
    ) -> Vec<Vec<Plan>> {
        self.secondary_sm.iter().map(|sm| sm.get_planner().plan(vec_counters.remove(0))).collect()
    }

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
                        let iectx = InstanceCtx::new(global_idx, plan);
                        Some((global_idx, self.secondary_sm[i].get_instance(iectx)))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn expand_and_witness_sec(
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

    fn witness_tables_sec(
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

    fn get_data_bus_counters(&self) -> DataBus<PayloadType, BusDeviceMetricsWrapper> {
        let mut data_bus = DataBus::new();
        self.secondary_sm.iter().for_each(|sm| {
            let counter = sm.get_counter();

            data_bus
                .connect_device(counter.bus_id(), Box::new(BusDeviceMetricsWrapper::new(counter)));
        });

        data_bus
    }

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

    fn get_data_bus_collectors(
        &self,
        sec_instance: Box<dyn BusDeviceInstance<F>>,
    ) -> DataBus<u64, BusDeviceInstanceWrapper<F>> {
        let mut data_bus = DataBus::new();

        let bus_device_instance = sec_instance;
        data_bus.connect_device(
            vec![OPERATION_BUS_ID],
            Box::new(BusDeviceInstanceWrapper::new(bus_device_instance)),
        );

        self.secondary_sm.iter().for_each(|sm| {
            if let Some(input_generator) = sm.get_inputs_generator() {
                data_bus.connect_device(
                    vec![OPERATION_BUS_ID],
                    Box::new(BusDeviceInstanceWrapper::new(input_generator)),
                );
            }
        });
        data_bus
    }

    fn close_data_bus_collectors(
        &self,
        mut data_bus: DataBus<u64, BusDeviceInstanceWrapper<F>>,
    ) -> Box<dyn BusDeviceInstance<F>> {
        data_bus.devices.remove(0).inner
    }
}

impl<F: PrimeField> WitnessComponent<F> for ZiskExecutor<F> {
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
        let sec_expanded = self.expand_and_witness_sec(&pctx, min_traces, sec_instances);
        self.witness_tables_sec(&pctx, sec_expanded);

        // Wait for the main task to finish
        main_task.join().unwrap();
    }
}
