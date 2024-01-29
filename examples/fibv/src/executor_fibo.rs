use proofman::{
    executor,
    executor::ExecutorBase,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::{Message, Payload},
    proof_ctx::ProofCtx,
    trace,
    task::TasksTable,
};

use std::sync::{Arc, RwLock};
use goldilocks::Goldilocks;
use pilout::find_subproof_id_by_name;

use log::debug;

executor!(FibonacciExecutor: Goldilocks);

impl Executor<Goldilocks> for FibonacciExecutor {
    fn witness_computation(
        &self,
        _config: String,
        stage_id: u32,
        proof_ctx: Arc<RwLock<&mut ProofCtx<Goldilocks>>>,
        tasks: &TasksTable,
        tx: &SenderB<Message>,
        _rx: &ReceiverB<Message>,
    ) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let proof_ctx = proof_ctx.read().unwrap();

        let subproof_id = find_subproof_id_by_name(&proof_ctx.pilout, "Fibonacci").expect("Subproof not found");
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

        self.broadcast(tx, Payload::new_trace(subproof_id, Box::new(fib)));

        println!("FibonacciExecutor> Waiting for resolve...");
        tasks.wait_column("Fibonacci".to_string(), subproof_id, air_id, "XXX".to_string());
        println!("FibonacciExecutor> Resolved!");
    }
}
