use proofman::{
    executor,
    executor::ExecutorBase,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::{Message, Payload},
    proof_ctx::ProofCtx,
    trace,
    task::TasksTable
};
use math::fields::f64::BaseElement;
use pilout::find_subproof_id_by_name;

use log::debug;

executor!(FibonacciExecutor: BaseElement);

impl Executor<BaseElement> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &ProofCtx<BaseElement>, tasks: &TasksTable, tx: &SenderB<Message>, _rx: &ReceiverB<Message>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let subproof_id = find_subproof_id_by_name(&proof_ctx.pilout, "Fibonacci").expect("Subproof not found");
        let air_id = 1;
        let num_rows = proof_ctx.pilout.subproofs[subproof_id].airs[air_id].num_rows.unwrap() as usize;

        trace!(Fibonacci {
            a: BaseElement,
            b: BaseElement
        });
        let mut fib = Fibonacci::new(num_rows);

        let public_inputs = proof_ctx.public_inputs.as_ref();
        fib.a[0] = public_inputs.unwrap()[0];
        fib.b[0] = public_inputs.unwrap()[1];

        for i in 1..num_rows as usize {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        let trace_id = proof_ctx.add_trace_to_air_instance(subproof_id, air_id, fib)
            .expect("Failed to add trace to AIR instance");

        self.broadcast(tx, Payload::NewTrace { subproof_id, air_id, trace_id });

        println!("FibonacciExecutor> Waiting for resolve...");
        tasks.wait_column("Fibonacci".to_string(), subproof_id, air_id, "XXX".to_string());
        println!("FibonacciExecutor> Resolved!");
    }
}