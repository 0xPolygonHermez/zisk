use zisk_core::{InstContext, ZiskInst};

pub trait InstObserver {
    fn on_instruction(&mut self, inst: &ZiskInst, inst_ctx: &InstContext) -> bool;
}
