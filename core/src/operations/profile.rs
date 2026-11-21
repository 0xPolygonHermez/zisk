use crate::{zisk_ops::OpStats, InstContext};

pub fn opc_profile(ctx: &mut InstContext) {
    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Profile can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_profile(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_profile() is not implemented");
}

#[inline(always)]
pub fn ops_profile(_ctx: &InstContext, _stats: &mut dyn OpStats) {}

/// Allocation mark (`SYSCALL_ALLOC_MARK_ID`): a no-op, the emulator stats read its operands.
#[inline(always)]
pub fn opc_alloc_mark(ctx: &mut InstContext) {
    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented. The allocation mark is only meaningful with its instruction context.
#[inline(always)]
pub fn op_alloc_mark(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_alloc_mark() is not implemented");
}

#[inline(always)]
pub fn ops_alloc_mark(_ctx: &InstContext, _stats: &mut dyn OpStats) {}
