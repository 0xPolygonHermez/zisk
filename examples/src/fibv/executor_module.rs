use proofman::{
    executor,
    executor::Executor,
    channel::{SenderB, ReceiverB},
    message::{Message, Payload},
    proof_ctx::ProofCtx,
    trace,
    task::TasksTable
};
use math::fields::f64::BaseElement;
use pilout::find_subproof_id_by_name;

use log::{info, debug, error};

executor!(ModuleExecutor: BaseElement);

impl Executor<BaseElement> for ModuleExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &ProofCtx<BaseElement>, tasks: &TasksTable, _tx: &SenderB<Message>, rx: &ReceiverB<Message>) {
        if stage_id != 1 {
            info!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        println!("ModuleEx> Waiting for message...");

        let msg = rx.recv().expect("Failed to receive message");

        if msg.payload == Payload::Halt { return; }

        if let Payload::NewTrace { subproof_id, air_id, trace_id } = msg.payload {
            // Search pilout.subproof index with name Fibonacci inside proof_ctx.pilout.subproofs
            let subproof_id_fibo = find_subproof_id_by_name(&proof_ctx.pilout, "Fibonacci").expect("Subproof not found");
            if subproof_id != subproof_id_fibo {
                error!("Subproof id {} does not match Fibonacci subproof id {}", subproof_id, subproof_id_fibo);
                return;
            }

            
            let trace = proof_ctx.get_trace(subproof_id, air_id, trace_id).expect("Failed to get trace");

            trace!(Module {
                x: BaseElement,
                q: BaseElement,
                x_mod: BaseElement
            });
            let mut module = Module::new(trace.num_rows());

            // TODO how to convert public inputs to BaseElement lika a downcast?
            let public_inputs = proof_ctx.public_inputs.as_ref().expect("Failed to get public inputs");
            let mut a = public_inputs[0];
            let mut b = public_inputs[1];
            let m = public_inputs[2];

            for i in 1..trace.num_rows() {
                module.x[i] = a * a + b * b;

                module.q[i] = module.x[i] / m;
                module.x_mod[i] = module.x[i]; // TODO: % m;

                b = a;
                a = module.x_mod[i];
            }

            if let Err(e) = proof_ctx.add_trace_to_air_instance(subproof_id as usize, 0, module) {
                error!("Failed to add trace to AIR instance: {}", e)
            } else {
                debug!("Successfully added trace to AIR instance");
            }

            println!("ModuleEx> Resolving task...");
            tasks.resolve_task(0).unwrap();
        }
    }
}
