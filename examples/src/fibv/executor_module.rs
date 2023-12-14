use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use proofman::message::Payload;
use proofman::channel::{SenderB, ReceiverB};
use proofman::message::Message;
use proofman::trace;
use math::fields::f64::BaseElement;
use log::{info, debug, error};

pub struct ModuleExecutor;

impl Executor<BaseElement> for ModuleExecutor {
    fn witness_computation(&self, stage_id: u32, _subproof_id: i32, _instance_id: i32, proof_ctx: &ProofCtx<BaseElement>, _tx: SenderB<Message>, rx: ReceiverB<Message>) {
        if stage_id != 1 {
            info!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        println!("ModuleEx> Waiting for message...");
        let msg = rx.recv().unwrap();

        if msg.payload == Payload::Halt {
            return;
        }

        
        match msg.payload {
            Payload ::Halt => {
            },
            Payload::NewTrace { subproof_id, air_id } => {
                // Search pilout.subproof index with name Fibonacci inside proof_ctx.pilout.subproofs
                let subproof_id2 = proof_ctx.pilout.subproofs
                    .iter()
                    .position(|x| x.name == Some("Fibonacci".to_string()))
                    .unwrap();

                if subproof_id2 == subproof_id as usize {        
                    // TODO! We need to know the trace_id!!!! Pass it with the message
                    let trace_id = 0;
                    let air_id = proof_ctx.find_air_instance(subproof_id as usize, air_id as usize).unwrap();
                    
                    let trace = proof_ctx.airs[air_id].get_trace(trace_id).unwrap();

                    trace!(Module {
                        x: BaseElement,
                        q: BaseElement,
                        x_mod: BaseElement
                    });
                    let mut module = Module::new(trace.num_rows());
                    
                    // TODO how to convert public inputs to BaseElement ina generic way?
                    let public_inputs = proof_ctx.public_inputs.as_ref();
                    let mut a = public_inputs.unwrap()[0];
                    let mut b = public_inputs.unwrap()[1];
                    let m = public_inputs.unwrap()[2];
        
                    for i in 1..trace.num_rows() {
                        module.x[i] = a * a + b * b;
        
                        module.q[i] = module.x[i] / m;
                        module.x_mod[i] = module.x[i]; // TODO: % m;
        
                        b = a;
                        a = module.x_mod[i];
                    }

                    match proof_ctx.add_trace_to_air_instance(subproof_id as usize, 0, module) {
                        Ok(_) => debug!("Successfully added trace to AIR instance"),
                        Err(e) => error!("Failed to add trace to AIR instance: {}", e)
                    }
                }
            },
        }
    }
}