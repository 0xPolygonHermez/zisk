use p3_field::PrimeField;
use proofman_common::ProofCtx;
use witness::WitnessComponent;

use rayon::prelude::*;

use sm_common::{
    BusDeviceInstanceWrapper, BusDeviceMetrics, BusDeviceMetricsWrapper, CheckPoint,
    ComponentProvider, InstanceExpanderCtx, InstanceType, Plan,
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

    /// Main State Machine
    pub main_sm: Arc<MainSM<F>>,

    /// Secondary State Machines
    secondary_sm: Vec<Arc<dyn ComponentProvider<F>>>,
}

impl<F: PrimeField> ZiskExecutor<F> {
    const NUM_THREADS: usize = 8;

    pub fn new(public_inputs_path: PathBuf, zisk_rom: Arc<ZiskRom>) -> Self {
        let main_sm = MainSM::new();

        Self { public_inputs_path, zisk_rom, main_sm, secondary_sm: Vec::new() }
    }

    pub fn register_sm(&mut self, sm: Arc<dyn ComponentProvider<F>>) {
        self.secondary_sm.push(sm);
    }

    fn compute_plans(&self, min_traces: Arc<Vec<EmuTrace>>) -> Vec<Vec<Plan>> {
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
                    .map(|device| device.inner)
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

        self.secondary_sm
            .iter()
            .map(|sm| sm.get_planner().plan(vec_counters.drain(..1).next().unwrap()))
            .collect()
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

    fn create_main_plans(&self, min_traces: &[EmuTrace]) -> Vec<Plan> {
        min_traces
            .iter()
            .enumerate()
            .map(|(segment_id, _minimal_trace)| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    MAIN_AIR_IDS[0],
                    Some(segment_id),
                    InstanceType::Instance,
                    Some(CheckPoint::new(segment_id, 0)),
                    None,
                )
            })
            .collect()
    }

    fn create_main_instances(
        &self,
        pctx: Arc<ProofCtx<F>>,
        main_planning: &mut Vec<Plan>,
    ) -> Vec<MainInstance<F>> {
        main_planning
            .drain(..)
            .filter_map(|plan| {
                if let (true, global_idx) = pctx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1)
                {
                    let iectx = InstanceExpanderCtx::new(global_idx, plan);
                    Some(self.main_sm.get_instance(iectx))
                } else {
                    None
                }
            })
            .collect()
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
        // ---------------------------------------------------------------------------------
        let min_traces = self.compute_minimal_traces(public_inputs, Self::NUM_THREADS);
        let min_traces = Arc::new(min_traces);

        // =================================================================================
        // PATH A Main SM instances
        // =================================================================================

        // PATH A PHASE 2. Count & Reduce the Minimal Traces to get the Plans
        // ---------------------------------------------------------------------------------
        let mut main_planning = self.create_main_plans(&min_traces);
        let mut main_layouts = self.create_main_instances(pctx.clone(), &mut main_planning);

        // PATH A PHASE 3. Expand the Minimal Traces to fill the Main Traces
        // ---------------------------------------------------------------------------------
        let pctx_clone = pctx.clone();
        let main_task = {
            let main_sm = self.main_sm.clone();
            let zisk_rom = self.zisk_rom.clone();
            let minimal_traces = min_traces.clone();
            std::thread::spawn(move || {
                main_layouts.par_iter_mut().for_each(|main_instance| {
                    main_sm.prove_main(
                        pctx_clone.clone(),
                        &zisk_rom,
                        &minimal_traces,
                        main_instance,
                    );
                });
                main_layouts
            })
        };

        // =================================================================================
        // PATH B Secondary SM instances
        // =================================================================================

        // PATH B PHASE 2. Count & Reduce the Minimal Traces to get the Plans
        // ---------------------------------------------------------------------------------
        // Compute counters for each minimal trace
        let mut plans = self.compute_plans(min_traces.clone());

        // Create the buffer ta the distribution context
        let mut sec_instances = Vec::new();
        let pctx_clone = pctx.clone();
        for (i, plans_by_sm) in plans.iter_mut().enumerate() {
            for plan in plans_by_sm.drain(..) {
                let (is_mine, global_idx) =
                    pctx_clone.clone().dctx_add_instance(plan.airgroup_id, plan.air_id, 1);

                if is_mine || plan.instance_type == InstanceType::Table {
                    let iectx = InstanceExpanderCtx::new(global_idx, plan);

                    let instance = self.secondary_sm[i].get_instance(iectx);
                    sec_instances.push((global_idx, instance));
                }
            }
        }

        // PATH B PHASE 3. Expand the Minimal Traces to fill the Secondary SM Traces
        // ---------------------------------------------------------------------------------
        let mut collected_instances: Vec<_> = sec_instances
            .par_drain(..)
            .map(|(global_idx, mut sec_instance)| {
                if sec_instance.instance_type() == InstanceType::Instance {
                    let mut data_bus = DataBus::<PayloadType, BusDeviceInstanceWrapper<F>>::new();

                    if let Some(check_point) = sec_instance.check_point() {
                        let chunk_id = check_point.chunk_id;

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

                        ZiskEmulator::process_rom_slice_plan_2::<F, BusDeviceInstanceWrapper<F>>(
                            &self.zisk_rom,
                            &min_traces,
                            chunk_id,
                            &mut data_bus,
                        );

                        sec_instance = data_bus.devices.remove(0).inner;
                    }

                    if let Some(air_instance) = sec_instance.compute_witness(&pctx) {
                        pctx.clone()
                            .air_instance_repo
                            .add_air_instance(air_instance, Some(global_idx));
                    }
                }
                (global_idx, sec_instance)
            })
            .collect();

        // Drop memory
        std::thread::spawn(move || {
            drop(min_traces);
        });

        collected_instances.par_iter_mut().for_each(|(global_idx, sec_instance)| {
            if sec_instance.instance_type() == InstanceType::Table {
                if let Some(air_instance) = sec_instance.compute_witness(&pctx) {
                    pctx.air_instance_repo.add_air_instance(air_instance, Some(*global_idx));
                }
            }
        });

        // Wait for the main task to finish
        main_task.join().unwrap();
    }
}
