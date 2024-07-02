use goldilocks::Goldilocks;
use log::trace;
use pil2_components::ZiskProcessor;
use pilout::pilout_proxy::PilOutProxy;
use pil2_stark::{AirInstanceMap, DefaultWitnessPlanner, ExecutionCtx, ProofCtx, WitnessManager, WitnessPlanner};
use std::env;

type F = Goldilocks;

#[allow(dead_code)]
pub struct BasicZiskWCPlugin<F> {
    pilout: PilOutProxy,
    zisk_processor: ZiskProcessor<F>,
    witness_planner: Box<dyn WitnessPlanner<F>>,
}

impl<F: 'static> BasicZiskWCPlugin<F> {
    const MY_NAME: &'static str = "ZiskWC  ";

    pub fn new(witness_planner: Option<Box<dyn WitnessPlanner<F>>>) -> Self {
        let current_dir = env::current_dir().unwrap();
        let pilout_path = current_dir.join("./zisk-wc/src/pil/pilout/basic.pilout");
        let pilout = PilOutProxy::new(pilout_path.to_str().unwrap(), false).unwrap_or_else(|error| {
            panic!("Failed to load pilout: {}", error);
        });

        // TODO Check hash!

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
    fn initialize(&self) {
        env_logger::builder()
            .format_timestamp(None)
            .format_level(true)
            .format_target(false)
            .filter_level(log::LevelFilter::Trace)
            .init();
    }

    fn get_pilout(&self) -> &PilOutProxy {
        &self.pilout
    }

    fn start_proof(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Starting Proof", Self::MY_NAME);
        for (module_name, module) in self.zisk_processor.get_modules() {
            module.borrow_mut().start_proof(proof_ctx, execution_ctx);
        }

        self.zisk_processor.execute(proof_ctx, execution_ctx);
    }

    fn end_proof(&mut self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof");
        for (module_name, module) in self.zisk_processor.get_modules() {
            module.borrow_mut().end_proof(proof_ctx);
        }
    }

    fn get_air_instances_map(&self, proof_ctx: &ProofCtx<F>) -> AirInstanceMap {
        trace!("Get air instances map");
        self.witness_planner.get_air_instances_map(proof_ctx)
    }

    fn calculate_witness(&self, stage: u32, pilout: &PilOutProxy, proof_ctx: &ProofCtx<F>) {
        trace!("Calculate witness for stage {}", stage);
        let modules = self.zisk_processor.get_modules();

        for (subproof_id, air_instances) in proof_ctx.air_instance_map.inner.iter() {
            let module = modules.get("Main").unwrap_or_else(|| {
                panic!("Main module not found");
            });

            for (air_id, air_instance) in air_instances.iter() {
                module.borrow().calculate_witness(stage, proof_ctx, air_instance);
            }
        }
    }
}

#[no_mangle]
pub extern "Rust" fn create_plugin() -> Box<dyn WitnessManager<F>> {
    Box::new(BasicZiskWCPlugin::new(None))
}
