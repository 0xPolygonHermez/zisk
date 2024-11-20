use proofman_common::WitnessPilout;
use sm_common::{LayoutPlanner, MinimalTraceStart, OutputPlan};
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};
use zisk_pil::{BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, ZISK_AIRGROUP_ID};

#[derive(Debug)]
pub struct BinaryPlan {
    pub details: String,
}

impl OutputPlan for BinaryPlan {}

#[derive(Default)]
pub struct BinaryPlanner {
    num_binary_slice: u64,
    num_binary_e_slice: u64,
    pub emu_trace_slices: Vec<MinimalTraceStart>,
    pub emu_trace_slices_e: Vec<MinimalTraceStart>,

    pub num_binary_inst: u64,
    pub num_binary_e_inst: u64,
    pub total_inst: u64,
}

impl BinaryPlanner {
    pub fn new() -> Self {
        BinaryPlanner {
            num_binary_slice: 0,
            num_binary_e_slice: 0,
            emu_trace_slices: Vec::new(),
            emu_trace_slices_e: Vec::new(),
            num_binary_inst: 0,
            num_binary_e_inst: 0,
            total_inst: 0,
        }
    }

    #[inline(always)]
    fn on_binary_instruction(&mut self, inst_ctx: &InstContext) {
        if self.num_binary_inst == 0 || self.num_binary_inst % self.num_binary_slice == 0 {
            self.emu_trace_slices.push(MinimalTraceStart { step: inst_ctx.step });
        }
        self.num_binary_inst += 1;
    }

    #[inline(always)]
    fn on_binary_e_instruction(&mut self, inst_ctx: &InstContext) {
        if self.num_binary_e_inst == 0 || self.num_binary_e_inst % self.num_binary_e_slice == 0 {
            self.emu_trace_slices_e.push(MinimalTraceStart { step: inst_ctx.step });
        }
        self.num_binary_e_inst += 1;
    }
}

impl LayoutPlanner for BinaryPlanner {
    fn new_session(&mut self, pilout: &WitnessPilout) {
        self.num_binary_slice =
            pilout.get_air(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]).num_rows() as u64;
        self.num_binary_e_slice =
            pilout.get_air(ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]).num_rows() as u64;

        self.emu_trace_slices.clear();
        self.emu_trace_slices_e.clear();
        self.num_binary_inst = 0;
        self.num_binary_e_inst = 0;
        self.total_inst = 0;
    }

    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) {
        match zisk_inst.op_type {
            ZiskOperationType::Binary => {
                self.on_binary_instruction(inst_ctx);
            }
            ZiskOperationType::BinaryE => {
                self.on_binary_e_instruction(inst_ctx);
            }
            _ => {}
        }

        self.total_inst += 1;
    }

    fn get_plan(&self) -> Box<dyn OutputPlan> {
        Box::new(BinaryPlan {
            details: format!(
                "Binary: {} slices, Binary Extension: {} slices, Total: {} instructions emu_Trace_slices: {:?}  emu_Trace_slices_e: {:?}",
                self.num_binary_inst, self.num_binary_e_inst, self.total_inst, self.emu_trace_slices, self.emu_trace_slices_e
            ),
        })
    }
}
