use proofman::{
    executor,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::{Message, Payload},
    proof_ctx::ProofCtx,
    trace
};
use math::fields::f64::BaseElement;
use pilout::find_subproof_id_by_name;

use log::debug;

executor!(FibonacciExecutor: BaseElement);

impl Executor<BaseElement> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &ProofCtx<BaseElement>, tx: SenderB<Message>, _rx: ReceiverB<Message>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let subproof_id = find_subproof_id_by_name(&proof_ctx.pilout, "Fibonacci").expect("Subproof not found");
        let air_id = 1;
        let air = &proof_ctx.pilout.subproofs[subproof_id].airs[air_id];

        trace!(Fibonacci {
            a: BaseElement,
            b: BaseElement
        });
        let mut fib = Fibonacci::new(air.num_rows() as usize);

        let public_inputs = proof_ctx.public_inputs.as_ref();
        fib.a[0] = public_inputs.unwrap()[0];
        fib.b[0] = public_inputs.unwrap()[1];

        for i in 1..air.num_rows() as usize {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        let trace_id = proof_ctx.add_trace_to_air_instance(subproof_id, air_id, fib)
            .expect("Failed to add trace to AIR instance");

        let msg = Message {  
            src: "Fibonacci".to_string(),
            dst: "*".to_string(),
            payload: Payload::new_trace(subproof_id, air_id, trace_id)
        };

        tx.send(msg);

        // channel.send(new_trace!(0, 0, 0));"))
    }
}