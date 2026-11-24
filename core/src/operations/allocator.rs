//! Operations of the allocator precompile (`precompiles/allocator`).
//!
//! They have no opcode: the guest reaches them through the allocation mark CSR with the kinds
//! `ALLOC_OP_*` of `zisk_definitions::alloc_mark`, and each one is transpiled to a `copyb` whose
//! `func` is one of these (see [`alloc_op_func`]). The prover sees the `copyb`; the address an
//! alloc returns is a free value for it, as the result of an fcall is.
//!
//! PROTOTYPE: only the plain emulation (`EmulationMode::Mem`, i.e. ziskemu) executes them. There
//! is no minimal trace, no assembly emulator and no witness, so a program that uses them cannot
//! be proved yet.
//!
//! | operation | a | b | c |
//! |---|---|---|---|
//! | create (clean, dirty) | addr | size | 0 |
//! | alloc, calloc | size | 0 | addr |
//! | dealloc | addr | 0 | 0 |
//!
//! An operation the allocator cannot serve (no free block, an address that is not in use, ...)
//! has no proof, so the emulation stops there.

use zisk_definitions::{
    ALLOC_OP_ALLOC_ID, ALLOC_OP_CALLOC_ID, ALLOC_OP_CREATE_CLEAN_ID, ALLOC_OP_CREATE_DIRTY_ID,
    ALLOC_OP_DEALLOC_ID,
};

use crate::{
    zisk_ops::OpStats, AllocError, AllocHeap, EmulationMode, InstContext,
    DMA_64_ALIGNED_MEMSET_COST, DMA_64_ALIGNED_MEMSET_DIVISOR, DMA_MEMSET_COST,
};

/// The `func` of the `copyb` an operation of the allocator precompile is transpiled to, by the
/// kind of its mark; `None` for a kind that is not one of its operations.
pub fn alloc_op_func(kind: u8) -> Option<fn(&mut InstContext)> {
    match kind {
        ALLOC_OP_CREATE_CLEAN_ID => Some(opc_alloc_create_clean),
        ALLOC_OP_CREATE_DIRTY_ID => Some(opc_alloc_create_dirty),
        ALLOC_OP_ALLOC_ID => Some(opc_alloc_alloc),
        ALLOC_OP_CALLOC_ID => Some(opc_alloc_calloc),
        ALLOC_OP_DEALLOC_ID => Some(opc_alloc_dealloc),
        _ => None,
    }
}

/// State of the allocator precompile along an execution.
#[derive(Debug, Default)]
pub struct AllocInstContext {
    pub heap: AllocHeap,
    /// Rows the last operation needed (1 plus its joins), for the cost model.
    pub last_rows: u64,
    /// Words the last operation cleared (a `calloc` over a used block), for the cost model.
    pub last_memset_words: u64,
    /// Zeroed allocations, how many of them had to clear the block, and the words cleared.
    pub callocs: u64,
    pub memsets: u64,
    pub memset_words: u64,
}

fn check_mode(ctx: &InstContext) {
    assert!(
        ctx.emulation_mode == EmulationMode::Mem,
        "the allocator precompile is a prototype, only the plain emulation (ziskemu) executes it"
    );
}

fn fail(ctx: &InstContext, op: &str, error: AllocError) -> ! {
    panic!("{op} at step {} pc 0x{:08x}: {error}", ctx.step, ctx.pc)
}

fn create(ctx: &mut InstContext, dirty: bool) {
    check_mode(ctx);
    if let Err(error) = ctx.alloc.heap.create(ctx.a, ctx.b, dirty) {
        fail(ctx, if dirty { "alloc_create_dirty" } else { "alloc_create_clean" }, error);
    }
    ctx.alloc.last_rows = 1;
    ctx.alloc.last_memset_words = 0;
    ctx.c = 0;
    ctx.flag = false;
}

#[inline(always)]
pub fn opc_alloc_create_clean(ctx: &mut InstContext) {
    create(ctx, false);
}

#[inline(always)]
pub fn opc_alloc_create_dirty(ctx: &mut InstContext) {
    create(ctx, true);
}

fn alloc(ctx: &mut InstContext, zeroed: bool) {
    check_mode(ctx);
    let size = ctx.a;
    let outcome = match ctx.alloc.heap.alloc(size) {
        Ok(outcome) => outcome,
        Err(error) => fail(ctx, if zeroed { "alloc_calloc" } else { "alloc_alloc" }, error),
    };
    ctx.alloc.last_rows = 1 + outcome.joins;
    ctx.alloc.last_memset_words = 0;
    if zeroed {
        ctx.alloc.callocs += 1;
        if outcome.dirty {
            // the precompile launches the memset, as the main would
            ctx.mem.memset(outcome.addr, size, 0);
            ctx.alloc.last_memset_words = size / 8;
            ctx.alloc.memsets += 1;
            ctx.alloc.memset_words += size / 8;
        }
    }
    ctx.c = outcome.addr;
    ctx.flag = false;
}

#[inline(always)]
pub fn opc_alloc_alloc(ctx: &mut InstContext) {
    alloc(ctx, false);
}

#[inline(always)]
pub fn opc_alloc_calloc(ctx: &mut InstContext) {
    alloc(ctx, true);
}

#[inline(always)]
pub fn opc_alloc_dealloc(ctx: &mut InstContext) {
    check_mode(ctx);
    let joins = match ctx.alloc.heap.dealloc(ctx.a) {
        Ok(joins) => joins,
        Err(error) => fail(ctx, "alloc_dealloc", error),
    };
    ctx.alloc.last_rows = 1 + joins;
    ctx.alloc.last_memset_words = 0;
    ctx.c = 0;
    ctx.flag = false;
}

/// Cost of the operation just executed: its rows in the allocator air, and for a `calloc` over a
/// used block the memset the precompile launches (which has no main step of its own). The step
/// of the main is charged as the one of its `copyb`.
pub fn ops_alloc(ctx: &InstContext, stats: &mut dyn OpStats) {
    let mut cost = ctx.alloc.last_rows * ctx.alloc.heap.policy().row_cost();
    let words = ctx.alloc.last_memset_words;
    if words > 0 {
        stats.mem_align_write(ctx.c, words as usize);
        cost += DMA_MEMSET_COST
            + words.div_ceil(DMA_64_ALIGNED_MEMSET_DIVISOR) * DMA_64_ALIGNED_MEMSET_COST;
    }
    stats.set_variable_cost(cost);
}
