use crate::{EmulationMode, InstContext};

/// Tells the emulator to keep a copy of `[a, a + b)` tagged with the temporal reference the guest
/// most recently requested, so that a later `mt` DMA operation carrying that same temporal
/// reference can read the region back after it has been overwritten.
///
/// This is a pure hint: it does not touch the guest-visible state beyond `c`, and it exists only
/// because the emulator cannot afford to keep the whole memory history. `c` carries the temporal
/// reference the copy was tagged with, which is what a variant of the pattern with a real
/// destination register would hand back to the guest.
///
/// `a` is the region address (a register) and `b` its length in bytes (an immediate).
#[inline(always)]
pub fn opc_execute_advice(ctx: &mut InstContext) {
    let addr = ctx.a;
    let count = ctx.b;

    match ctx.emulation_mode {
        // Memory is only maintained in these two modes; in ConsumeMemReads the source words come
        // back from the minimal trace, so there is nothing to capture.
        EmulationMode::Mem | EmulationMode::GenerateMemReads => {
            #[cfg(feature = "log_dma_ops")]
            println!(
                "opc_execute_advice 0x{addr:08X} {count} T:{} STEP:{}",
                ctx.temporal_ref, ctx.step
            );
            ctx.mem.capture_snapshot(ctx.temporal_ref, addr, count);
        }
        EmulationMode::ConsumeMemReads => {}
    }

    ctx.c = ctx.temporal_ref;
    ctx.flag = false;
}

/// `execute_advice` that opens the temporal reference it captures under, instead of reusing the one
/// a preceding `flag` request left in the context, and hands it back in `c`.
///
/// This is the two-instruction sequence the guest almost always wants -- request a reference,
/// advise one region -- folded into a single operation, so it costs one step where the pair costs
/// two.  Reusing the current `step` as the reference is what the `flag` request does too, so the
/// value means exactly the same thing to the `mt` operations and the two forms interoperate: a
/// reference opened here can still receive further regions from later `execute_advice`s, because
/// this leaves it as the context's current reference.
///
/// `a` is the region address (a register) and `b` its length in bytes (an immediate).
#[inline(always)]
pub fn opc_execute_advice_ref(ctx: &mut InstContext) {
    ctx.temporal_ref = ctx.step;
    opc_execute_advice(ctx);
}

/// Unimplemented. ExecuteAdvice needs the memory, so it can only be called from the system call
/// context via InstContext. This is provided just for completeness.
#[inline(always)]
pub fn op_execute_advice(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_execute_advice() is not implemented");
}

/// Unimplemented, for the same reason as [`op_execute_advice`].
#[inline(always)]
pub fn op_execute_advice_ref(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_execute_advice_ref() is not implemented");
}
