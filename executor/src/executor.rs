use p3_field::PrimeField;
use proofman::WitnessManager;
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};

use rayon::prelude::*;

use sm_common::{CheckPoint, ComponentProvider, InstanceExpanderCtx, InstanceType, Plan};
use sm_main::{MainInstance, MainSM};

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use zisk_core::ZiskRom;
use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};
use ziskemu::{EmuOptions, EmuTrace, ZiskEmulator};

use crate::MetricsProxy;

pub struct ZiskExecutor<F: PrimeField> {
    /// Witness Manager
    pub wcm: Arc<WitnessManager<F>>,

    /// ZisK ROM, a binary file that contains the ZisK program to be executed
    pub zisk_rom: Arc<ZiskRom>,

    /// Main State Machine
    pub main_sm: Arc<MainSM<F>>,

    /// Secondary State Machines
    secondary_sm: Vec<Arc<dyn ComponentProvider<F>>>,
}

impl<F: PrimeField> ZiskExecutor<F> {
    const NUM_THREADS: usize = 8;

    pub fn new(wcm: Arc<WitnessManager<F>>, zisk_rom: Arc<ZiskRom>) -> Self {
        let main_sm = MainSM::new(wcm.clone());

        Self { wcm, zisk_rom, main_sm, secondary_sm: Vec::new() }
    }

    pub fn register_sm(&mut self, sm: Arc<dyn ComponentProvider<F>>) {
        self.secondary_sm.push(sm);
    }

    pub fn execute(
        &self,
        public_inputs_path: &Path,
        _: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        _: Arc<SetupCtx>,
    ) {
        // Call emulate with these options
        let public_inputs = {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(public_inputs_path.display().to_string());
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
        let mut main_layouts = self.create_main_instances(&mut main_planning);

        // PATH A PHASE 3. Expand the Minimal Traces to fill the Main Traces
        // ---------------------------------------------------------------------------------
        let main_task = {
            let main_sm = self.main_sm.clone();
            let zisk_rom = self.zisk_rom.clone();
            let minimal_traces = min_traces.clone();

            std::thread::spawn(move || {
                main_layouts.par_iter_mut().for_each(|main_instance| {
                    main_sm.prove_main(&zisk_rom, &minimal_traces, main_instance);
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
        for (i, plans_by_sm) in plans.iter_mut().enumerate() {
            for plan in plans_by_sm.drain(..) {
                let (is_mine, global_idx) =
                    ectx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1);

                if is_mine || plan.instance_type == InstanceType::Table {
                    let iectx = InstanceExpanderCtx::new(global_idx, plan);

                    let instance = self.secondary_sm[i].get_instance(iectx);
                    sec_instances.push(instance);
                }
            }
        }

        // PATH B PHASE 3. Expand the Minimal Traces to fill the Secondary SM Traces
        // ---------------------------------------------------------------------------------
        sec_instances.par_iter_mut().for_each(|sec_instance| {
            if sec_instance.instance_type() == InstanceType::Instance {
                let _ = sec_instance.expand(&self.zisk_rom, min_traces.clone());
                let _ = sec_instance.prove(min_traces.clone());
            }
        });

        sec_instances.par_iter_mut().for_each(|sec_instance| {
            if sec_instance.instance_type() == InstanceType::Table {
                let _ = sec_instance.prove(min_traces.clone());
            }
        });

        // Drop memory
        std::thread::spawn(move || {
            drop(min_traces);
        });

        // Wait for the main task to finish
        main_task.join().unwrap();
    }

    fn compute_plans(&self, min_traces: Arc<Vec<EmuTrace>>) -> Vec<Vec<Plan>> {
        timer_start_debug!(PROCESS_OBSERVER);
        let mut metrics_slices = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut metrics_proxy = MetricsProxy::new();
                self.secondary_sm.iter().for_each(|sm| {
                    metrics_proxy.register_metrics(sm.get_counter());
                });
                ZiskEmulator::process_rom_slice_counters::<F>(
                    &self.zisk_rom,
                    minimal_trace,
                    &mut metrics_proxy,
                );
                metrics_proxy
            })
            .collect::<Vec<_>>();
        timer_stop_and_log_debug!(PROCESS_OBSERVER);

        // Group counters by chunk_id and counter type
        let mut vec_counters =
            (0..metrics_slices[0].metrics.len()).map(|_| Vec::new()).collect::<Vec<_>>();

        for (chunk_id, counter_slice) in metrics_slices.iter_mut().enumerate() {
            for (i, counter) in counter_slice.metrics.drain(..).enumerate() {
                vec_counters[i].push((chunk_id, counter));
            }
        }

        self.secondary_sm
            .iter()
            .map(|sm| sm.get_planner().plan(vec_counters.drain(..1).next().unwrap()))
            .collect()
    }

    fn compute_minimal_traces(&self, public_inputs: Vec<u8>, num_threads: usize) -> Vec<EmuTrace> {
        timer_start_debug!(PHASE1_FAST_PROCESS_ROM);

        // Prepare the settings for the emulator
        let emu_options = EmuOptions {
            elf: None,    //Some(rom_path.to_path_buf().display().to_string()),
            inputs: None, //Some(public_inputs_path.display().to_string()),
            trace_steps: Some(MainTrace::<F>::NUM_ROWS as u64 - 1),
            ..EmuOptions::default()
        };

        let min_traces = ZiskEmulator::process_rom_min_trace::<F>(
            &self.zisk_rom,
            &public_inputs,
            &emu_options,
            num_threads,
        )
        .expect("Error during emulator execution");
        timer_stop_and_log_debug!(PHASE1_FAST_PROCESS_ROM);

        min_traces
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

    fn create_main_instances(&self, main_planning: &mut Vec<Plan>) -> Vec<MainInstance<F>> {
        let mut main_instances = Vec::new();
        let ectx = self.wcm.get_ectx();

        for plan in main_planning.drain(..) {
            if let (true, global_idx) = ectx.dctx_add_instance(plan.airgroup_id, plan.air_id, 1) {
                let iectx = InstanceExpanderCtx::new(global_idx, plan);
                main_instances.push(self.main_sm.get_instance(iectx));
            }
        }

        main_instances
    }
}
