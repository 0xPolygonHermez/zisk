use std::{
    any::Any,
    collections::HashMap,
    fmt::Binary,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    binary, BinaryBasicSM, BinaryBasicTableSM, BinaryExtensionSM, BinaryExtensionTableSM,
    BinaryPlanner, BinarySurveyor,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use rayon::Scope;
use sm_common::{
    plan, CheckPoint, ChunkId, ComponentProvider, InstCount, InstanceExpanderCtx, InstanceXXXX,
    OpResult, Plan, Planner, Provable, StateMachine, Survey, SurveyCounter, Surveyor,
    WitnessBuffer,
};
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZiskRequiredOperation, ZiskRom};
use zisk_pil::{
    BinaryTrace, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BINARY_EXTENSION_TABLE_AIR_IDS,
    BINARY_TABLE_AIR_IDS, ZISK_AIRGROUP_ID,
};
use ziskemu::{EmuTrace, ZiskEmulator};

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

impl<F: PrimeField> ComponentProvider<F> for BinarySM<F> {
    fn get_surveyor(&self) -> Box<dyn Surveyor> {
        Box::new(BinarySurveyor::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::new(self.wcm.clone()))
    }

    fn get_instance(self: Arc<Self>, iectx: InstanceExpanderCtx<F>) -> Box<dyn InstanceXXXX> {
        Box::new(BinaryInstance::new(self.clone(), self.wcm.clone(), iectx))
    }
}

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

pub struct BinaryInstance<F: PrimeField> {
    binary_sm: Arc<BinarySM<F>>,
    wcm: Arc<WitnessManager<F>>,
    skipping: bool,
    skipped: u64,
    expanded: u64,
    num_rows: u64,
    iectx: InstanceExpanderCtx<F>,
    inputs: Vec<ZiskRequiredOperation>,
    inputs_e: Vec<ZiskRequiredOperation>,
}

impl<F: PrimeField> BinaryInstance<F> {
    pub fn new(
        binary_sm: Arc<BinarySM<F>>,
        wcm: Arc<WitnessManager<F>>,
        iectx: InstanceExpanderCtx<F>,
    ) -> Self {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]);
        Self {
            binary_sm,
            wcm,
            skipping: true,
            skipped: 0,
            expanded: 0,
            num_rows: air.num_rows() as u64,
            iectx,
            inputs: Vec::new(),
            inputs_e: Vec::new(),
        }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryInstance<F> {}

impl<F: PrimeField> InstanceXXXX for BinaryInstance<F> {
    fn expand(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let observer: &mut dyn InstObserver = self;
        ZiskEmulator::process_slice_plan::<F>(zisk_rom, &min_traces, 0, observer);
        Ok(())
    }

    fn prove(
        &mut self,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        timer_start_debug!(PROVE_BINARY);
        let inputs = std::mem::take(&mut self.inputs);
        self.binary_sm.prove_instance(
            inputs,
            false,
            &mut self.iectx.buffer.buffer,
            self.iectx.buffer.offset as u64,
        );
        timer_stop_and_log_debug!(PROVE_BINARY);

        timer_start_debug!(CREATE_AIR_INSTANCE);
        let buffer = std::mem::take(&mut self.iectx.buffer.buffer);
        let air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            ZISK_AIRGROUP_ID,
            BINARY_AIR_IDS[0],
            None,
            buffer,
        );

        self.wcm
            .get_pctx()
            .air_instance_repo
            .add_air_instance(air_instance, Some(self.iectx.instance_global_idx));

        timer_stop_and_log_debug!(CREATE_AIR_INSTANCE);
        Ok(())
    }
}

impl<F: PrimeField> InstObserver for BinaryInstance<F> {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
        if zisk_inst.op_type != ZiskOperationType::Binary
            && zisk_inst.op_type != ZiskOperationType::BinaryE
        {
            return false;
        }

        if self.skipping {
            if self.skipped < self.iectx.plan.checkpoint.skip {
                self.skipped += 1;
                return false;
            }
        }

        if zisk_inst.op_type == ZiskOperationType::Binary {
            let required_operation = ZiskRequiredOperation {
                step: inst_ctx.step - 1,
                opcode: zisk_inst.op,
                a: if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a },
                b: if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b },
            };
            self.inputs.push(required_operation);
        } else {
            let required_operation = ZiskRequiredOperation {
                step: inst_ctx.step - 1,
                opcode: zisk_inst.op,
                a: if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a },
                b: if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b },
            };
            self.inputs_e.push(required_operation);
        }
        self.expanded += 1;

        return self.expanded == self.num_rows;
    }
}
