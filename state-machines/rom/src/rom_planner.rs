use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskPcHistogram};

#[derive(Default)]
pub struct RomPlanner {
    pub histogram: ZiskPcHistogram,
}

impl RomPlanner {
    pub fn new() -> Self {
        RomPlanner { histogram: ZiskPcHistogram::default() }
    }
}

// impl LayoutPlanner for RomPlanner {
//     fn get_plan(&self) -> Vec<InstMetricEnum> {
//         // vec![Plan {
//         //     airgroup_id: ZISK_AIRGROUP_ID,
//         //     air_id: ROM_AIR_IDS[0],
//         //     segment_id: None,
//         //     emu_trace_start: None,
//         // }]
//         Vec::new()
//     }
// }

// impl InstObserver for RomPlanner {
//     fn on_instruction(&mut self, chunk_id: usize, instruction: &ZiskInst, inst_ctx: &InstContext) {
//         let count = self.histogram.map.entry(inst_ctx.pc).or_default();
//         *count += 1;

//         if instruction.end {
//             self.histogram.end_pc = inst_ctx.pc;
//             self.histogram.steps = inst_ctx.step + 1;
//         }
//     }
// }
