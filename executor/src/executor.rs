use p3_field::PrimeField;
use proofman_common::ProofCtx;
use witness::WitnessComponent;

use rayon::prelude::*;

use sm_common::{
    BusDeviceInstance, BusDeviceInstanceWrapper, BusDeviceMetrics, BusDeviceMetricsWrapper,
    CheckPoint, CollectInfoSkip, ComponentProvider, InstanceExpanderCtx, InstanceType, Plan,
};
use sm_main::{MainInstance, MainSM};
use zisk_common::{DataBus, PayloadType, OPERATION_BUS_ID};

use std::{fs, path::PathBuf, sync::Arc};
use zisk_core::ZiskRom;
use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};
use ziskemu::{EmuOptions, EmuTrace, ZiskEmulator};

pub struct ZiskExecutor<F: PrimeField> {
    pub public_inputs_path: PathBuf,

    /// ZisK ROM, a binary file that contains the ZisK program to be executed
    pub zisk_rom: Arc<ZiskRom>,

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
        // Prepare the settings for the emulator
        let emu_options = EmuOptions {
            elf: None,    //Some(rom_path.to_path_buf().display().to_string()),
            inputs: None, //Some(public_inputs_path.display().to_string()),
            trace_steps: Some(MainTrace::<F>::NUM_ROWS as u64 - 1),
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

    fn plan_main(&self, min_traces: &[EmuTrace]) -> Vec<Plan> {
        min_traces
            .iter()
            .enumerate()
            .map(|(segment_id, _minimal_trace)| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    MAIN_AIR_IDS[0],
                    Some(segment_id),
                    InstanceType::Instance,
                    CheckPoint::Single(segment_id),
                    Some(Box::new(CollectInfoSkip::new(0))),
                    None,
                )
            })
            .collect()
    }

    fn create_main_instances(
        &self,
        pctx: &ProofCtx<F>,
        mut main_planning: Vec<Plan>,
    ) -> Vec<MainInstance> {
        main_planning
            .drain(..)
            .filter_map(|plan| {
                if let (true, global_idx) = pctx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1)
                {
                    let iectx = InstanceExpanderCtx::new(global_idx, plan);
                    Some(MainInstance::new(iectx))
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
                let mut data_bus = DataBus::<PayloadType, BusDeviceMetricsWrapper>::new();
                self.secondary_sm.iter().for_each(|sm| {
                    let counter = sm.get_counter();
                    let bus_ids = counter.bus_id();

                    data_bus
                        .connect_device(bus_ids, Box::new(BusDeviceMetricsWrapper::new(counter)));
                });

                ZiskEmulator::process_rom_slice_counters::<F, BusDeviceMetricsWrapper>(
                    &self.zisk_rom,
                    minimal_trace,
                    &mut data_bus,
                );

                data_bus
                    .devices
                    .into_iter()
                    .map(|mut device| {
                        device.on_close();
                        device.inner
                    })
                    .collect::<Vec<Box<dyn BusDeviceMetrics>>>()
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
        self.secondary_sm
            .iter()
            .map(|sm| sm.get_planner().plan(vec_counters.drain(..1).next().unwrap()))
            .collect()
    }

    fn create_sec_instances(
        &self,
        pctx: &ProofCtx<F>,
        mut plans: Vec<Vec<Plan>>,
    ) -> Vec<(usize, Box<dyn BusDeviceInstance<F>>)> {
        // Create the buffer ta the distribution context
        let mut sec_instances = Vec::new();
        for (i, plans_by_sm) in plans.iter_mut().enumerate() {
            for plan in plans_by_sm.drain(..) {
                let (is_mine, global_idx) =
                    pctx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1);

                if is_mine || plan.instance_type == InstanceType::Table {
                    let iectx = InstanceExpanderCtx::new(global_idx, plan);

                    let instance = self.secondary_sm[i].get_instance(iectx);
                    sec_instances.push((global_idx, instance));
                }
            }
        }
        sec_instances
    }

    fn expand_sec(
        &self,
        pctx: &ProofCtx<F>,
        min_traces: Arc<Vec<EmuTrace>>,
        mut sec_instances: Vec<(usize, Box<dyn BusDeviceInstance<F>>)>,
    ) -> Vec<(usize, Box<dyn BusDeviceInstance<F>>)> {
        let collected_instances: Vec<_> = sec_instances
            .par_drain(..)
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
            .collect();

        collected_instances
    }

    fn witness_sec(
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
        let mut data_bus = DataBus::<PayloadType, BusDeviceInstanceWrapper<F>>::new();

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

        for chunk_id in chunk_ids {
            ZiskEmulator::process_rom_slice_plan::<F, BusDeviceInstanceWrapper<F>>(
                &self.zisk_rom,
                min_traces,
                *chunk_id,
                &mut data_bus,
            );
        }

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
        let main_planning = self.plan_main(&min_traces);
        let main_instances = self.create_main_instances(&pctx, main_planning);
        let main_task =
            self.expand_and_witness_main(pctx.clone(), min_traces.clone(), main_instances);

        // --- PATH B Secondary SM instances
        // Count, Plan and create the Secondary SM instances + Expand and Compute the Witnesses
        // --------------------------------------------------------------------------------------------------
        let sec_count = self.count_sec(&min_traces);
        let sec_planning = self.plan_sec(sec_count);
        let sec_instances = self.create_sec_instances(&pctx, sec_planning);
        let sec_expanded = self.expand_sec(&pctx, min_traces, sec_instances);
        self.witness_sec(&pctx, sec_expanded);

        // Wait for the main task to finish
        main_task.join().unwrap();
    }
}
