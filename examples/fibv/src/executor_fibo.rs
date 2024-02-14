use proofman::{executor, executor::Executor, proof_ctx::ProofCtx, trace};

use proofman::proof_manager_config::ProofManConfig;

use goldilocks::Goldilocks;
use estark::config::{executors_config::ExecutorsConfig, estark_config::EStarkConfig, meta_config::MetaConfig};

use log::debug;

executor!(FibonacciExecutor);

impl Executor<Goldilocks, ExecutorsConfig, EStarkConfig, MetaConfig> for FibonacciExecutor {
    fn witness_computation(
        &self,
        _config: &ProofManConfig<ExecutorsConfig, EStarkConfig, MetaConfig>,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<Goldilocks>,
    ) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let subproof_id = proof_ctx.pilout.find_subproof_id_by_name("Fibonacci").expect("Subproof not found");
        let air_id = 1;
        let num_rows = proof_ctx.pilout.subproofs[subproof_id].airs[air_id].num_rows.unwrap() as usize;

        trace!(Fibonacci { a: Goldilocks, b: Goldilocks });
        let mut fib = Fibonacci::new(num_rows);

        fib.a[0] = proof_ctx.public_inputs[0];
        fib.b[0] = proof_ctx.public_inputs[1];

        for i in 1..num_rows as usize {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        debug!("FbnccExe: ··· Fibonacci trace generated");
    }
}
