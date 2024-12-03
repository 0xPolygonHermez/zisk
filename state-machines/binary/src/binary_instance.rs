use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::AirInstance;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{Instance, InstanceExpanderCtx};
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZiskRequiredOperation, ZiskRom};
use zisk_pil::BINARY_AIR_IDS;
use ziskemu::{EmuTrace, ZiskEmulator};

use crate::BinarySM;

pub struct BinaryInstance<F: PrimeField> {
    binary_sm: Arc<BinarySM<F>>,
    wcm: Arc<WitnessManager<F>>,
    iectx: InstanceExpanderCtx<F>,

    op_type: ZiskOperationType,
    skipping: bool,
    skipped: u64,
    expanded: u64,
    num_rows: u64,
    inputs: Vec<ZiskRequiredOperation>,
}

impl<F: PrimeField> BinaryInstance<F> {
    pub fn new(
        binary_sm: Arc<BinarySM<F>>,
        wcm: Arc<WitnessManager<F>>,
        iectx: InstanceExpanderCtx<F>,
    ) -> Self {
        let pctx = wcm.get_pctx();
        let plan = &iectx.plan;
        let air = pctx.pilout.get_air(plan.airgroup_id, plan.air_id);
        let op_type = if plan.air_id == BINARY_AIR_IDS[0] {
            ZiskOperationType::Binary
        } else {
            ZiskOperationType::BinaryE
        };

        Self {
            binary_sm,
            wcm,
            iectx,
            op_type,
            skipping: true,
            skipped: 0,
            expanded: 0,
            num_rows: air.num_rows() as u64,
            inputs: Vec::new(),
        }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryInstance<F> {}

impl<F: PrimeField> Instance for BinaryInstance<F> {
    fn expand(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let observer: &mut dyn InstObserver = self;
        ZiskEmulator::process_rom_slice_plan::<F>(zisk_rom, &min_traces, 0, observer);
        Ok(())
    }

    fn prove(
        &mut self,
        _min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        timer_start_debug!(PROVE_BINARY);
        let inputs = std::mem::take(&mut self.inputs);

        self.binary_sm.prove_instance(
            inputs,
            self.op_type == ZiskOperationType::BinaryE,
            &mut self.iectx.buffer.buffer,
            self.iectx.buffer.offset as u64,
        );
        timer_stop_and_log_debug!(PROVE_BINARY);

        timer_start_debug!(CREATE_AIR_INSTANCE);
        let buffer = std::mem::take(&mut self.iectx.buffer.buffer);
        let air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            self.iectx.plan.airgroup_id,
            self.iectx.plan.air_id,
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
        if zisk_inst.op_type != self.op_type {
            return false;
        }

        if self.skipping {
            if self.skipped < self.iectx.plan.checkpoint.skip {
                self.skipped += 1;
                return false;
            }
        }

        let required_operation = ZiskRequiredOperation {
            step: inst_ctx.step - 1,
            opcode: zisk_inst.op,
            a: if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a },
            b: if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b },
        };
        self.inputs.push(required_operation);

        self.expanded += 1;

        return self.expanded == self.num_rows;
    }
}
