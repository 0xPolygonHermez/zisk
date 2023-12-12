use std::thread::sleep;

use math::FieldElement;
use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use crossbeam_channel::{Receiver, Sender};
use proofman::message::{Message, Payload};
use proofman::trace;
use math::fields::f64::BaseElement;
use std::time::Duration;

use log::debug;

pub struct FibonacciExecutor<T> {
    pub name: String,
    phantom: std::marker::PhantomData<T>,
}

impl<T> FibonacciExecutor<T> {
    pub fn new(name: String) -> Self {
        FibonacciExecutor {
            name,
            phantom: std::marker::PhantomData
        }
    }

}

impl<T: FieldElement> Executor<T> for FibonacciExecutor<T> {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    fn witness_computation(&self, stage_id: u32, _subproof_id: u32, _instance_id: i32, proof_ctx: &ProofCtx<T>, tx: Sender<Message>, _rx: Receiver<Message>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        // NOTE! This is a hack to get the Fibonacci example working.
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

        proof_ctx.add_trace_to_air_instance(0, 0, fibonacci);

        sleep(Duration::from_millis(500));
        tx.send(Message {
            src: self.name.clone(),
            dst: "brocadcast".to_string(),
            payload: Payload::NewTrace { subproof_id: 0, air_id: 0 }
        }).unwrap();
    }
}