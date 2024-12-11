use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZiskRequiredOperation, ZiskRom};
use zisk_pil::ArithTrace;
use ziskemu::{EmuTrace, ZiskEmulator};

use crate::ArithFullSM;

pub struct ArithFullInstance<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
    arith_full_sm: Arc<ArithFullSM>,
    iectx: InstanceExpanderCtx,

    skipping: bool,
    skipped: u64,
    inputs: Vec<ZiskRequiredOperation>,
    arith_trace: ArithTrace<F>,
}

impl<F: PrimeField> ArithFullInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        arith_full_sm: Arc<ArithFullSM>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        let arith_trace = ArithTrace::new();

        Self {
            wcm,
            arith_full_sm,
            iectx,
            skipping: true,
            skipped: 0,
            inputs: Vec::new(),
            arith_trace,
        }
    }
}

unsafe impl<F: PrimeField> Sync for ArithFullInstance<F> {}

impl<F: PrimeField> Instance<F> for ArithFullInstance<F> {
    fn collect(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let chunk_id = self.iectx.plan.checkpoint.as_ref().unwrap().chunk_id;
        let observer: &mut dyn InstObserver = self;

        ZiskEmulator::process_rom_slice_plan::<F>(zisk_rom, &min_traces, chunk_id, observer);
        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        timer_start_debug!(PROVE_ARITH);
        self.arith_full_sm.prove_instance(&self.inputs, &mut self.arith_trace);
        timer_stop_and_log_debug!(PROVE_ARITH);

        let instance =
            AirInstance::new_from_trace(self.wcm.get_sctx(), FromTrace::new(&mut self.arith_trace));

        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> InstObserver for ArithFullInstance<F> {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
        if zisk_inst.op_type != ZiskOperationType::Arith {
            return false;
        }

        if self.skipping {
            let checkpoint = self.iectx.plan.checkpoint.as_ref().unwrap();
            if checkpoint.skip == 0 || self.skipped == checkpoint.skip {
                self.skipping = false;
            } else {
                self.skipped += 1;
                return false;
            }
        }

        let a = if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a };
        let b = if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b };

        self.inputs.push(ZiskRequiredOperation { step: inst_ctx.step, opcode: zisk_inst.op, a, b });

        self.inputs.len() == ArithTrace::<F>::NUM_ROWS
    }
}
