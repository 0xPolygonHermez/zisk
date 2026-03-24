use crate::{zisk_ops::OpStats, EmulationMode, InstContext};

pub fn opc_profile(ctx: &mut InstContext) {
    if ctx.emulation_mode == EmulationMode::Mem {
        ctx.c = 0;
        ctx.flag = false;
    } else {
        ctx.c = 0;
        ctx.flag = false;
    }
}

/// Unimplemented.  Profile can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_profile(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_profile() is not implemented");
}

#[inline(always)]
pub fn ops_profile(_ctx: &InstContext, _stats: &mut dyn OpStats) {}
