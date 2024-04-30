use std::ffi::c_void;

use proofman::{
    executor::{BufferManager, Executor},
    trace, ProofCtx,
};
use proofman::executor;

use goldilocks::Goldilocks;

use log::debug;

executor!(ModuleExecutor { /*buffer_manager: StarkBufferManager<Goldilocks>*/});

impl Executor<Goldilocks> for ModuleExecutor {
    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<Goldilocks>,
        buffer_manager: Option<&Box<dyn BufferManager<Goldilocks>>>,
    ) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        // Search pilout.subproof index with name Fibonacci inside proof_ctx.pilout.subproofs
        let subproof_id_fibo = proof_ctx.pilout.find_subproof_id_by_name("Fibonacci").expect("Subproof not found");

        trace!(Module { x: Goldilocks, q: Goldilocks, x_mod: Goldilocks });

        let subproof_id = proof_ctx.pilout.find_subproof_id_by_name("Module").expect("Subproof not found");
        let air_id = 0;

        let num_rows = proof_ctx.pilout.subproofs[subproof_id_fibo].airs[0].num_rows.unwrap() as usize;

        let (buffer, trace_offset) =
            buffer_manager.as_ref().unwrap().get_buffer("Fibonacci").expect("Buffer not found");

        let mut module = Module::from_ptr(buffer.as_ptr() as *mut c_void, num_rows, trace_offset);

        // TODO how to convert public inputs to Goldilocks like a downcast?
        let mut a = proof_ctx.public_inputs[0];
        let mut b = proof_ctx.public_inputs[1];
        let m = proof_ctx.public_inputs[2];

        for i in 1..num_rows {
            module.x[i] = a * a + b * b;

            module.q[i] = module.x[i] / m;
            module.x_mod[i] = module.x[i]; // TODO: % m;

            b = a;
            a = module.x_mod[i];
        }

        proof_ctx.add_instance(subproof_id, air_id, buffer, module).expect("Error adding trace to air instance");

        debug!("modleExe: ··· Module trace generated");
    }
}
