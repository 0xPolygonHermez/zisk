use std::{
    collections::HashMap,
    fmt::Binary,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    binary, BinaryBasicSM, BinaryBasicTableSM, BinaryExtensionSM, BinaryExtensionTableSM,
    BinarySurveyor,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use rayon::Scope;
use sm_common::{
    plan, CheckPoint, ChunkId, ComponentProvider, Expander, InstCount, OpResult, Plan, Planner,
    Provable, StateMachine, Survey, SurveyCounter, Surveyor, WitnessBuffer,
};
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskRequiredOperation};
use zisk_pil::{
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BINARY_EXTENSION_TABLE_AIR_IDS, BINARY_TABLE_AIR_IDS,
    ZISK_AIRGROUP_ID,
};
use ziskemu::EmuTrace;

const PROVE_CHUNK_SIZE: usize = 1 << 16;

#[allow(dead_code)]
pub struct BinarySM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs_basic: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_extension: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
    binary_basic_sm: Arc<BinaryBasicSM<F>>,
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,
}

impl<F: PrimeField> BinarySM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let binary_basic_table_sm =
            BinaryBasicTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS);
        let binary_basic_sm = BinaryBasicSM::new(
            wcm.clone(),
            binary_basic_table_sm,
            ZISK_AIRGROUP_ID,
            BINARY_AIR_IDS,
        );

        let binary_extension_table_sm = BinaryExtensionTableSM::new(
            wcm.clone(),
            ZISK_AIRGROUP_ID,
            BINARY_EXTENSION_TABLE_AIR_IDS,
        );
        let binary_extension_sm = BinaryExtensionSM::new(
            wcm.clone(),
            std,
            binary_extension_table_sm,
            ZISK_AIRGROUP_ID,
            BINARY_EXTENSION_AIR_IDS,
        );

        let binary_sm = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            inputs_basic: Mutex::new(Vec::new()),
            inputs_extension: Mutex::new(Vec::new()),
            binary_basic_sm,
            binary_extension_sm,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone(), None, None);

        binary_sm.binary_basic_sm.register_predecessor();
        binary_sm.binary_extension_sm.register_predecessor();

        binary_sm
    }

    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredOperation>,
        is_extension: bool,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        if !is_extension {
            self.binary_basic_sm.prove_instance(operations, prover_buffer, offset);
        } else {
            self.binary_extension_sm.prove_instance(operations, prover_buffer, offset);
        }
    }

    // pub fn prove_binary(
    //     &self,
    //     zisk_rom: &ZiskRom,
    //     vec_traces: &[EmuTrace],
    //     iectx: &mut InstanceExtensionCtx<F>,
    //     pctx: &ProofCtx<F>,
    // ) {
    //     let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]);

    //     timer_start_debug!(PROCESS_BINARY);
    //     let inputs = ZiskEmulator::process_slice_required::<F>(
    //         zisk_rom,
    //         vec_traces,
    //         ZiskOperationType::None,
    //         &iectx.emu_trace_start,
    //         air.num_rows(),
    //     );
    //     timer_stop_and_log_debug!(PROCESS_BINARY);

    //     timer_start_debug!(PROVE_BINARY);
    //     self.binary_sm.prove_instance(inputs, false, &mut iectx.prover_buffer, iectx.offset);
    //     timer_stop_and_log_debug!(PROVE_BINARY);

    //     timer_start_debug!(CREATE_AIR_INSTANCE);
    //     let buffer = std::mem::take(&mut iectx.prover_buffer);
    //     iectx.air_instance = Some(AirInstance::new(
    //         self.wcm.get_sctx(),
    //         ZISK_AIRGROUP_ID,
    //         BINARY_AIR_IDS[0],
    //         None,
    //         buffer,
    //     ));
    //     timer_stop_and_log_debug!(CREATE_AIR_INSTANCE);
    // }

    // pub fn prove_binary_extension(
    //     &self,
    //     zisk_rom: &ZiskRom,
    //     vec_traces: &[EmuTrace],
    //     iectx: &mut InstanceExtensionCtx<F>,
    //     pctx: &ProofCtx<F>,
    // ) {
    //     let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]);

    //     let inputs = ZiskEmulator::process_slice_required::<F>(
    //         zisk_rom,
    //         vec_traces,
    //         ZiskOperationType::None,
    //         &iectx.emu_trace_start,
    //         air.num_rows(),
    //     );

    //     self.binary_sm.prove_instance(inputs, true, &mut iectx.prover_buffer, iectx.offset);

    //     let buffer = std::mem::take(&mut iectx.prover_buffer);
    //     iectx.air_instance = Some(AirInstance::new(
    //         self.wcm.get_sctx(),
    //         ZISK_AIRGROUP_ID,
    //         BINARY_EXTENSION_AIR_IDS[0],
    //         None,
    //         buffer,
    //     ));
    // }
}

pub struct BinaryPlanner<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
}

impl<F: PrimeField> BinaryPlanner<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Self {
        Self { wcm }
    }
}

impl<F: PrimeField> Planner for BinaryPlanner<F> {
    fn plan(&self, surveys: Vec<(ChunkId, Box<dyn Surveyor>)>) -> Vec<Plan> {
        let pctx = self.wcm.get_pctx();
        let binary_rows =
            pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]).num_rows() as u64;
        let binary_e_rows =
            pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]).num_rows() as u64;

        // Prepare counts for binary
        let count_inst_binary: Vec<_> = surveys
            .iter()
            .map(|(chunk_id, surveyor)| {
                let binary_surveyor = surveyor.as_any().downcast_ref::<BinarySurveyor>().unwrap();
                InstCount {
                    chunk_id: *chunk_id,
                    inst_count: binary_surveyor.binary.inst_count as u64,
                }
            })
            .collect();

        // Prepare counts for binary_extension
        let count_inst_binary_e: Vec<_> = surveys
            .iter()
            .map(|(chunk_id, surveyor)| {
                let binary_surveyor = surveyor.as_any().downcast_ref::<BinarySurveyor>().unwrap();
                InstCount {
                    chunk_id: *chunk_id,
                    inst_count: binary_surveyor.binary_extension.inst_count as u64,
                }
            })
            .collect();

        // Create plans for binary
        let plan_binary: Vec<_> = plan(count_inst_binary, binary_rows)
            .into_iter()
            .map(|checkpoint| Plan::new(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0], None, checkpoint))
            .collect();

        // Create plans for binary_extension
        let plan_binary_e: Vec<_> = plan(count_inst_binary_e, binary_e_rows)
            .into_iter()
            .map(|checkpoint| {
                Plan::new(ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0], None, checkpoint)
            })
            .collect();

        // Combine both sets of plans
        plan_binary.into_iter().chain(plan_binary_e.into_iter()).collect()
    }
}

pub struct BinaryExpander {}

impl<'a, F: PrimeField> Expander<'a, F> for BinaryExpander {
    fn expand(
        &self,
        plan: &Plan,
        min_traces: Arc<[EmuTrace]>,
        buffer: WitnessBuffer<'a, F>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }
}

impl<F: PrimeField> ComponentProvider<F> for BinarySM<F> {
    fn get_surveyor(&self) -> Box<dyn Surveyor> {
        Box::new(BinarySurveyor::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::new(self.wcm.clone()))
    }

    fn get_expander(&self) -> Box<dyn Expander<F>> {
        Box::new(BinaryExpander {})
    }
}
impl<F: PrimeField> StateMachine for BinarySM<F> {
    //     fn prove_x(
    //         &self,
    //         layout_planner: &dyn LayoutPlanner,
    //     ) -> Result<(), Box<dyn std::error::Error + Send>> {
    //         // if let Some(binary_planner) = layout_planner.as_any().downcast_ref::<BinaryPlanner>() {
    //         //     // Ok(self.prove(&self.zisk_rom, &binary_planner.histogram).unwrap())
    //         //     println!(
    //         //         "Binary planner: {:?} {}",
    //         //         binary_planner.num_binary_inst, binary_planner.num_binary_e_inst
    //         //     );
    //         //     Ok(())
    //         // } else {
    //         //     Err(Box::new(std::io::Error::new(
    //         //         std::io::ErrorKind::Other,
    //         //         "Failed to downcast layout planner to BinaryPlanner",
    //         //     )))
    //         // }
    //         Ok(())
    //     }

    //     fn register_predecessor(&self) {
    //         self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    //     }

    //     fn unregister_predecessor(&self) {
    //         if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
    //             /*<BinarySM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
    //                 self,
    //                 &[],
    //                 true,
    //                 scope,
    //             );*/
    //             //self.threads_controller.wait_for_threads();

    //             self.binary_basic_sm.unregister_predecessor();
    //             self.binary_extension_sm.unregister_predecessor();
    //         }
    //     }
}
// impl<F: PrimeField> ObserverProvider<F> for BinarySM<F> {
//     fn get_planner(&self) -> Box<dyn LayoutPlanner> {
//         Box::new(BinaryPlanner::new())
//     }

//     fn get_expander(&self, buffer: &[F], offset: usize) -> Option<Box<dyn Expander>> {
//         Some(Box::new(BinaryExpander::new(&self.wcm.get_pctx().pilout)))
//     }
// }

impl<F: PrimeField> WitnessComponent<F> for BinarySM<F> {}

impl<F: PrimeField> Provable<ZiskRequiredOperation, OpResult> for BinarySM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        let mut _inputs_basic = Vec::new();
        let mut _inputs_extension = Vec::new();

        let basic_operations = BinaryBasicSM::<F>::operations();
        let extension_operations = BinaryExtensionSM::<F>::operations();

        // TODO Split the operations into basic and extended operations in parallel
        for operation in operations {
            if basic_operations.contains(&operation.opcode) {
                _inputs_basic.push(operation.clone());
            } else if extension_operations.contains(&operation.opcode) {
                _inputs_extension.push(operation.clone());
            } else {
                panic!("BinarySM: Operator {:#04x} not found", operation.opcode);
            }
        }

        let mut inputs_basic = self.inputs_basic.lock().unwrap();
        inputs_basic.extend(_inputs_basic);

        while inputs_basic.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_basic.is_empty()) {
            let num_drained_basic = std::cmp::min(PROVE_CHUNK_SIZE, inputs_basic.len());
            let drained_inputs_basic = inputs_basic.drain(..num_drained_basic).collect::<Vec<_>>();

            let binary_basic_sm_cloned = self.binary_basic_sm.clone();

            binary_basic_sm_cloned.prove(&drained_inputs_basic, false, scope);
        }
        drop(inputs_basic);

        let mut inputs_extension = self.inputs_extension.lock().unwrap();
        inputs_extension.extend(_inputs_extension);

        while inputs_extension.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_extension.is_empty())
        {
            let num_drained_extension = std::cmp::min(PROVE_CHUNK_SIZE, inputs_extension.len());
            let drained_inputs_extension =
                inputs_extension.drain(..num_drained_extension).collect::<Vec<_>>();
            let binary_extension_sm_cloned = self.binary_extension_sm.clone();

            binary_extension_sm_cloned.prove(&drained_inputs_extension, false, scope);
        }
        drop(inputs_extension);
    }
}
