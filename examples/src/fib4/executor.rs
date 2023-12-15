use proofman::{
    executor,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::Message,
    proof_ctx::ProofCtx,
    trace
};
use math::fields::f64::BaseElement;
use pilout::find_subproof_id_by_name;

use log::{debug, error};

executor!(FibonacciExecutor: BaseElement);

impl Executor<BaseElement> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &ProofCtx<BaseElement>, _tx: &SenderB<Message>, _rx: &ReceiverB<Message>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let subproof_id = find_subproof_id_by_name(&proof_ctx.pilout, "Fibonacci").expect("Subproof not found");

        let num_rows = 16;

        trace!(Fibonacci {
            a: BaseElement,
            b: BaseElement
        });
        let mut fibonacci = Fibonacci::new(num_rows);

        fibonacci.a[0] = BaseElement::new(1);
        fibonacci.b[0] = BaseElement::new(1);

        for i in 1..num_rows {
            fibonacci.a[i] = fibonacci.b[i - 1];
            fibonacci.b[i] = fibonacci.a[i - 1] + fibonacci.b[i - 1];
        }

        match proof_ctx.add_trace_to_air_instance(subproof_id, 0, fibonacci) {
            Ok(_) => debug!("Successfully added trace to AIR instance"),
            Err(e) => error!("Failed to add trace to AIR instance: {}", e)
        }
    }
}