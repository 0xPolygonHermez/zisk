use proofman::{
    executor,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::Message,
    proof_ctx::ProofCtx,
    task::TasksTable,
    trace,
    executor::ExecutorBase,
    message::Payload,
};
use math::fields::f64::BaseElement as GoldiLocks;

use log::debug;

executor!(FibonacciExecutor: GoldiLocks);

impl Executor<GoldiLocks> for FibonacciExecutor {
    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &ProofCtx<GoldiLocks>,
        _tasks: &TasksTable,
        tx: &SenderB<Message>,
        _rx: &ReceiverB<Message>,
    ) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        println!("proof_ctx.pilout = {:?}", proof_ctx.pilout);
        debug!("FibExect> --> Computing witness");
        let subproof_id = 0;
        let air_id = 0;
        let num_rows = proof_ctx.pilout.subproofs[subproof_id].airs[air_id].num_rows.unwrap() as usize;

        trace!(Fibonacci { a: GoldiLocks, b: GoldiLocks });
        let mut fib = Fibonacci::new(num_rows);

        let public_inputs = proof_ctx.public_inputs.as_ref();
        fib.a[0] = public_inputs.unwrap()[0];
        fib.b[0] = public_inputs.unwrap()[1];

        for i in 1..num_rows as usize {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        self.broadcast(tx, Payload::new_trace(subproof_id, fib));
        debug!("FibExect> <-- Computing witness finished");
    }
}
