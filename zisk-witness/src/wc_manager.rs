extern crate proofman_common;

use goldilocks::Goldilocks;
use pil2_components::ZiskProcessor;
use pilout::pilout_proxy::PilOutProxy;
use proofman_common::{AirInstanceMap, DefaultWitnessPlanner, ExecutionCtx, ProofCtx, WitnessManager, WitnessPlanner};
use std::env;

type F = Goldilocks;

#[allow(dead_code)]
pub struct BasicZiskWCPlugin<F> {
    pilout: PilOutProxy,
    zisk_processor: ZiskProcessor<F>,
    witness_planner: Box<dyn WitnessPlanner<F>>,
}

impl<F: 'static> BasicZiskWCPlugin<F> {
    pub fn new(witness_planner: Option<Box<dyn WitnessPlanner<F>>>) -> Self {
        let current_dir = env::current_dir().unwrap();
        let pilout_path = current_dir.join("../../zisk-witness/src/pil/pilout/basic.pilout");
        let pilout = PilOutProxy::new(pilout_path.to_str().unwrap(), false).unwrap_or_else(|error| {
            panic!("Failed to load pilout: {}", error);
        });

        let zisk_processor = ZiskProcessor::<F>::new(&pilout);

        let witness_planner = match witness_planner {
            Some(witness_planner) => witness_planner,
            None => Box::new(DefaultWitnessPlanner::new()),
        };

        Self { pilout, zisk_processor, witness_planner: witness_planner }
    }
}

#[allow(unused_variables)]
impl<F: 'static> WitnessManager<F> for BasicZiskWCPlugin<F> {
    fn get_pilout(&self) -> &PilOutProxy {
        &self.pilout
    }

    fn start_proof(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        for (module_name, module) in self.zisk_processor.get_modules() {
            module.start_proof(proof_ctx, execution_ctx);
        }

        self.zisk_processor.execute(proof_ctx, execution_ctx);
    }

    fn end_proof(&mut self, proof_ctx: &ProofCtx<F>) {
        for (module_name, module) in self.zisk_processor.get_modules() {
            module.end_proof(proof_ctx);
        }
    }

    fn get_air_instances_map(&self, proof_ctx: &ProofCtx<F>) -> AirInstanceMap {
        self.witness_planner.get_air_instances_map(proof_ctx)
    }

    fn calculate_witness(&self, stage: u32, pilout: &PilOutProxy, proof_ctx: &ProofCtx<F>) {
        let modules = self.zisk_processor.get_modules();

        for (subproof_id, air_instances) in proof_ctx.air_instance_map.inner.iter() {
            let module = modules.get("Main").unwrap_or_else(|| {
                panic!("Main module not found");
            });

            for (air_id, air_instance) in air_instances.iter() {
                module.calculate_witness(stage, proof_ctx, air_instance);
            }
        }
    }
}

#[no_mangle]
pub extern "Rust" fn create_plugin() -> Box<dyn WitnessManager<F>> {
    Box::new(BasicZiskWCPlugin::new(None))
}
