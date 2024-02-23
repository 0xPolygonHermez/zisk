use proofman::{executor, executor::Executor, proof_ctx::ProofCtx, trace};

use goldilocks::Goldilocks;

use log::debug;

executor!(ModuleExecutor);

impl Executor<Goldilocks> for ModuleExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &mut ProofCtx<Goldilocks>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        // Search pilout.subproof index with name Fibonacci inside proof_ctx.pilout.subproofs
        let subproof_id_fibo = proof_ctx.pilout.find_subproof_id_by_name("Fibonacci").expect("Subproof not found");

        trace!(Module { x: Goldilocks, q: Goldilocks, x_mod: Goldilocks });

        let num_rows = proof_ctx.pilout.subproofs[subproof_id_fibo].airs[0].num_rows.unwrap() as usize;
        let mut module = Module::new(num_rows);

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

        debug!("modleExe: ··· Module trace generated");
    }
}
