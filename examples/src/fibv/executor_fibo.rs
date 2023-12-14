use math::FieldElement;
use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use proofman::message::{Payload, Message};
use proofman::trace;
use math::fields::f64::BaseElement;
use proofman::channel::{SenderB, ReceiverB};

use log::debug;

/// `FibonacciExecutor` is an executor for computing Fibonacci sequences in the Fibonacci vadcop example.
pub struct FibonacciExecutor<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> FibonacciExecutor<T> {
    /// Creates a new instance of `FibonacciExecutor`.
    pub fn new() -> Self {
        FibonacciExecutor { phantom: std::marker::PhantomData }
    }
}

impl<T: FieldElement> Executor<T> for FibonacciExecutor<T> {
    fn witness_computation(&self, stage_id: u32, _subproof_id: i32, _instance_id: i32, proof_ctx: &ProofCtx<T>, tx: SenderB<Message>, _rx: ReceiverB<Message>) {
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

        fib.a[0] = BaseElement::new(1);
        fib.b[0] = BaseElement::new(1);

        for i in 1..num_rows {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        let subproof_id = proof_ctx.pilout.subproofs
            .iter()
            .position(|x| x.name == Some("Fibonacci".to_string()))
            .unwrap();

        proof_ctx.add_trace_to_air_instance(subproof_id, 0, fib);

        let msg = Message {  
            src: "Fibonacci".to_string(),
            dst: "*".to_string(),
            payload: Payload::new_trace(subproof_id as u32, 0)
        };

        tx.send(msg);
    }
}