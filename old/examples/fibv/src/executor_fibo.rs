use std::ffi::c_void;

use proofman::{
    executor::{BufferManager, Executor},
    trace, ProofCtx,
};
use proofman::executor;

use goldilocks::Goldilocks;

use log::debug;

executor!(FibonacciExecutor /*{ /*buffer_manager: StarkBufferManager<Goldilocks>*/}*/);

impl Executor<Goldilocks> for FibonacciExecutor {
    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<Goldilocks>,
        buffer_manager: Option<&Box<dyn BufferManager<Goldilocks>>>,
    ) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let subproof_id = proof_ctx.pilout.find_subproof_id_by_name("Fibonacci").expect("Subproof not found");
        let air_id = 1;

        let num_rows = proof_ctx.pilout.subproofs[subproof_id].airs[air_id].num_rows.unwrap() as usize;

        let (buffer, trace_offset) =
            buffer_manager.as_ref().unwrap().get_buffer("Fibonacci").expect("Buffer not found");

        trace!(Fibonacci { a: Goldilocks, b: Goldilocks });

        let mut fib = Fibonacci::from_ptr(buffer.as_ptr() as *mut c_void, num_rows, trace_offset);

        fib.a[0] = proof_ctx.public_inputs[0];
        fib.b[0] = proof_ctx.public_inputs[1];

        for i in 1..num_rows as usize {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        proof_ctx.add_instance(subproof_id, air_id, buffer, fib).expect("Error adding trace to air instance");

        debug!("FbnccExe: ··· Fibonacci trace generated");
    }
}
