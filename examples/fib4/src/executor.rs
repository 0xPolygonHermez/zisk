use proofman::{
    executor,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::Message,
    proof_ctx::ProofCtx,
    task::TasksTable,
    trace,
};

use std::sync::{Arc, RwLock};
use goldilocks::{Goldilocks, AbstractField};
use pilout::find_subproof_id_by_name;

use log::debug;

executor!(FibonacciExecutor: Goldilocks);

impl Executor<Goldilocks> for FibonacciExecutor {
    fn witness_computation(
        &self,
        _config: String,
        stage_id: u32,
        proof_ctx: Arc<RwLock<&mut ProofCtx<Goldilocks>>>,
        _tasks: &TasksTable,
        _tx: &SenderB<Message>,
        _rx: &ReceiverB<Message>,
    ) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let proof_ctx = proof_ctx.read().unwrap();

        let subproof_id = find_subproof_id_by_name(&proof_ctx.pilout, "Fibonacci").expect("Subproof not found");
        let air_id = 0;
        let num_rows = proof_ctx.pilout.subproofs[subproof_id].airs[air_id].num_rows.unwrap() as usize;

        trace!(Fibonacci { a: Goldilocks, b: Goldilocks });
        let mut fib = Fibonacci::new(num_rows);

        fib.a[0] = Goldilocks::one();
        fib.b[0] = Goldilocks::one();

        for i in 1..num_rows {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        proof_ctx
            .add_trace_to_air_instance(subproof_id, air_id, Box::new(fib))
            .expect("Error adding trace to air instance");
    }
}
