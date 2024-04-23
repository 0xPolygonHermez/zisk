use proofman::{executor, executor::Executor, ProofCtx, trace};

use goldilocks::Goldilocks;

use log::debug;

executor!(FibonacciExecutor);

impl Executor<Goldilocks> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &mut ProofCtx<Goldilocks>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let subproof_id = proof_ctx.pilout.find_subproof_id_by_name("Fibonacci").expect("Subproof not found");
        let air_id = 1;

        let num_rows = proof_ctx.pilout.subproofs[subproof_id].airs[air_id].num_rows.unwrap() as usize;

        trace!(Fibonacci { a: Goldilocks, b: Goldilocks });
        let mut fib = Fibonacci::new(num_rows);

        fib.a[0] = proof_ctx.public_inputs[0];
        fib.b[0] = proof_ctx.public_inputs[1];

        for i in 1..num_rows as usize {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        proof_ctx.add_trace_to_air_instance(subproof_id, air_id, fib).expect("Error adding trace to air instance");

        debug!("FbnccExe: ··· Fibonacci trace generated");
    }
}
