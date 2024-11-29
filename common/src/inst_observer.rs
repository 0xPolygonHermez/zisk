use zisk_core::{InstContext, ZiskInst};

// use crate::EmuTraceStart;

// pub struct RegisterStatus {
//     pub step: u64,
//     pub a: u64,
//     pub b: u64,
//     pub c: u64,
//     pub last_c: u64,
//     pub pc: u64,
//     pub sp: u64,
// }

// impl RegisterStatus {
//     pub fn new(step: u64, a: u64, b: u64, c: u64, last_c: u64, pc: u64, sp: u64) -> RegisterStatus {
//         RegisterStatus { step, a, b, c, last_c, pc, sp }
//     }
// }

pub trait InstObserver {
    fn on_instruction(&mut self, inst: &ZiskInst, inst_ctx: &InstContext);
}
