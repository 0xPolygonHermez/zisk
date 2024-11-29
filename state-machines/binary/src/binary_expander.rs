use proofman_common::WitnessPilout;
use sm_common::Expander;
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst};


// impl Expander for BinaryExpander {
//     fn expand(
//         &self,
//         _buffer: &mut [u8],
//         _offset: usize,
//     ) -> Result<(), Box<dyn std::error::Error + Send>> {
//         Ok(())
//     }
// }

// impl InstObserver for BinaryExpander {
//     fn on_instruction(&mut self, chunk_id: usize, _inst: &ZiskInst, _inst_ctx: &InstContext) {}
// }

// pub struct BinaryCollector {
//     inputs: Vec<ZiskRequiredOperation>,
// }

// impl BinaryCollector {
//     pub fn new(pilout: &WitnessPilout, airgroup_id: usize, air_id: usize) -> Self {
//         let air = pilout.get_air(airgroup_id, air_id);
//         BinaryCollector { inputs: Vec::with_capacity(air.num_rows()) }
//     }
// }

// impl InstObserver for BinaryCollector {
//     fn on_instruction(
//         &mut self,
//         inst: &ZiskInst,
//         inst_ctx: &InstContext,
//     ) {
//         self.inputs.push(ZiskRequiredOperation {
//             step: inst_ctx.step,
//             opcode: inst.op,
//             a: if inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a },
//             b: if inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b },
//         });
//     }
// }
