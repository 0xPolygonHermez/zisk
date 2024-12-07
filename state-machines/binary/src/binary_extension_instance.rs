use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::AirInstance;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZiskRequiredOperation, ZiskRom};
use zisk_pil::BinaryExtensionTrace;
use ziskemu::{EmuTrace, ZiskEmulator};

use crate::BinaryExtensionSM;

pub struct BinaryExtensionInstance<F: PrimeField> {
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,
    wcm: Arc<WitnessManager<F>>,
    iectx: InstanceExpanderCtx,

    skipping: bool,
    skipped: u64,
    num_rows: u64,
    inputs: Vec<ZiskRequiredOperation>,
    binary_e_trace: BinaryExtensionTrace<F>,
}

impl<F: PrimeField> BinaryExtensionInstance<F> {
    pub fn new(
        binary_extension_sm: Arc<BinaryExtensionSM<F>>,
        wcm: Arc<WitnessManager<F>>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        let pctx = wcm.get_pctx();
        let plan = &iectx.plan;
        let air = pctx.pilout.get_air(plan.airgroup_id, plan.air_id);

        let binary_e_trace = BinaryExtensionTrace::new(air.num_rows());

        Self {
            binary_extension_sm,
            wcm,
            iectx,
            skipping: true,
            skipped: 0,
            num_rows: air.num_rows() as u64,
            inputs: Vec::new(),
            binary_e_trace,
        }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryExtensionInstance<F> {}

impl<F: PrimeField> Instance for BinaryExtensionInstance<F> {
    fn expand(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let chunk_id = self.iectx.plan.checkpoint.chunk_id;
        let observer: &mut dyn InstObserver = self;

        ZiskEmulator::process_rom_slice_plan::<F>(zisk_rom, &min_traces, chunk_id, observer);
        Ok(())
    }

    fn prove(
        &mut self,
        _min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        timer_start_debug!(PROVE_BINARY);
        self.binary_extension_sm.prove_instance(&self.inputs, &mut self.binary_e_trace);
        timer_stop_and_log_debug!(PROVE_BINARY);

        timer_start_debug!(CREATE_BINARY_EXTENSION_AIR_INSTANCE);
        let buffer = std::mem::take(&mut self.binary_e_trace.buffer);
        let buffer: Vec<F> = unsafe { std::mem::transmute(buffer) };
        let air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            self.iectx.plan.airgroup_id,
            self.iectx.plan.air_id,
            None,
            buffer,
        );

        let air_instance_repo = &self.wcm.get_pctx().air_instance_repo;
        air_instance_repo.add_air_instance(air_instance, Some(self.iectx.instance_global_idx));
        timer_stop_and_log_debug!(CREATE_BINARY_EXTENSION_AIR_INSTANCE);

        Ok(())
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> InstObserver for BinaryExtensionInstance<F> {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
        if zisk_inst.op_type != ZiskOperationType::BinaryE {
            return false;
        }

        if self.skipping {
            if self.iectx.plan.checkpoint.skip == 0 ||
                self.skipped == self.iectx.plan.checkpoint.skip
            {
                self.skipping = false;
            } else {
                self.skipped += 1;
                return false;
            }
        }

        let a = if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a };
        let b = if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b };

        self.inputs.push(ZiskRequiredOperation { step: inst_ctx.step, opcode: zisk_inst.op, a, b });

        self.inputs.len() == self.num_rows as usize
    }
}
