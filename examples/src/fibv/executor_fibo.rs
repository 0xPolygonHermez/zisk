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
    phantom: std::marker::PhantomData<T>,
}

impl<T> FibonacciExecutor<T> {
    pub fn new() -> Self {
        FibonacciExecutor {
            phantom: std::marker::PhantomData
        }
    }

}

impl<T: FieldElement> Executor<T> for FibonacciExecutor<T> {
    fn witness_computation(&self, stage_id: u32, _subproof_id: i32, _instance_id: i32, proof_ctx: &ProofCtx<T>, tx: Sender<Message>, _rx: Receiver<Message>) {
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
        let subproof_id = proof_ctx.pilout.subproofs.iter().position(|x| x.name == Some("Fibonacci".to_string())).unwrap();
        tx.send(Message {
            src: "Fibonacci".to_string(),
            dst: "brocadcast".to_string(),
            payload: Payload::NewTrace { subproof_id: subproof_id as u32, air_id: 0 }
        }).unwrap();
    }
}