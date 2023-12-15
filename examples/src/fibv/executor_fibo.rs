use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use proofman::message::{Payload, Message};
use proofman::trace;
use math::fields::f64::BaseElement;
use proofman::channel::{SenderB, ReceiverB};

use log::{debug, error};

/// `FibonacciExecutor` is an executor for computing Fibonacci sequences in the Fibonacci vadcop example.
pub struct FibonacciExecutor;

impl Executor<BaseElement> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, _subproof_id: Option<usize>, _air_id: Option<usize>, proof_ctx: &ProofCtx<BaseElement>, tx: SenderB<Message>, _rx: ReceiverB<Message>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let num_rows = 16;

        trace!(Fibonacci {
            a: BaseElement,
            b: BaseElement
        });
        let mut fib = Fibonacci::new(num_rows);

        let public_inputs = proof_ctx.public_inputs.as_ref();
        fib.a[0] = public_inputs.unwrap()[0];
        fib.b[0] = public_inputs.unwrap()[1];

        for i in 1..num_rows {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        let subproof_id = proof_ctx.pilout.subproofs
            .iter()
            .position(|x| x.name == Some("Fibonacci".to_string()))
            .unwrap();

        match proof_ctx.add_trace_to_air_instance(subproof_id, 0, fib) {
            Ok(_) => debug!("Successfully added trace to AIR instance"),
            Err(e) => error!("Failed to add trace to AIR instance: {}", e)
        }

        let msg = Message {  
            src: "Fibonacci".to_string(),
            dst: "*".to_string(),
            payload: Payload::new_trace(subproof_id, 0)
        };

        tx.send(msg);
    }
}