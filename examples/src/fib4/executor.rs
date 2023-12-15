use proofman::executor::Executor;
use proofman::channel::{SenderB, ReceiverB};
use proofman::message::Message;
use proofman::proof_ctx::ProofCtx;
use proofman::trace;
use math::fields::f64::BaseElement;

use log::{debug, error};

pub struct FibonacciExecutor;

impl Executor<BaseElement> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, _subproof_id: Option<usize>, _air_id: Option<usize>, proof_ctx: &ProofCtx<BaseElement>, _tx: SenderB<Message>, _rx: ReceiverB<Message>) {
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

        let subproof_id = proof_ctx.pilout.subproofs.iter().position(|x| x.name == Some("Fibonacci".to_string())).unwrap();

        match proof_ctx.add_trace_to_air_instance(subproof_id, 0, fibonacci) {
            Ok(_) => debug!("Successfully added trace to AIR instance"),
            Err(e) => error!("Failed to add trace to AIR instance: {}", e)
        }
    }
}