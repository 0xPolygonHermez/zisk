use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::AirInstance;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZiskRequiredOperation, ZiskRom};
use zisk_pil::{BinaryTableTrace, BinaryTrace};
use ziskemu::{EmuTrace, ZiskEmulator};

pub struct BinaryBasicTableInstance<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
    iectx: InstanceExpanderCtx,

    binary_table_trace: BinaryTableTrace<F>,
}

impl<F: PrimeField> BinaryBasicTableInstance<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, iectx: InstanceExpanderCtx) -> Self {
        let pctx = wcm.get_pctx();
        let plan = &iectx.plan;
        let air = pctx.pilout.get_air(plan.airgroup_id, plan.air_id);

        let binary_table_trace = BinaryTableTrace::new(air.num_rows());

        Self { wcm, iectx, binary_table_trace }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryBasicTableInstance<F> {}

impl<F: PrimeField> Instance for BinaryBasicTableInstance<F> {
    fn expand(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn prove(
        &mut self,
        _min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        // timer_start_debug!(PROVE_BINARY);
        // self.binary_basic_sm.prove_instance(&self.inputs, &mut self.binary_trace);
        // timer_stop_and_log_debug!(PROVE_BINARY);

        // timer_start_debug!(CREATE_BINARY_AIR_INSTANCE);
        // let buffer = std::mem::take(&mut self.binary_trace.buffer);
        // let buffer: Vec<F> = unsafe { std::mem::transmute(buffer) };
        // let air_instance = AirInstance::new(
        //     self.wcm.get_sctx(),
        //     self.iectx.plan.airgroup_id,
        //     self.iectx.plan.air_id,
        //     None,
        //     buffer,
        // );

        // let air_instance_repo = &self.wcm.get_pctx().air_instance_repo;
        // air_instance_repo.add_air_instance(air_instance, Some(self.iectx.instance_global_idx));
        // timer_stop_and_log_debug!(CREATE_BINARY_AIR_INSTANCE);

        Ok(())
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}

// impl<F: PrimeField> InstObserver for BinaryInstance<F> {
//     #[inline(always)]
//     fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
//         if zisk_inst.op_type != ZiskOperationType::Binary {
//             return false;
//         }

//         if self.skipping && self.skipped < self.iectx.plan.checkpoint.skip {
//             self.skipped += 1;
//             return false;
//         }

//         let required_operation = ZiskRequiredOperation {
//             step: inst_ctx.step - 1,
//             opcode: zisk_inst.op,
//             a: if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a },
//             b: if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b },
//         };
//         self.inputs.push(required_operation);

//         self.expanded += 1;

//         self.expanded == self.num_rows
//     }
// }
