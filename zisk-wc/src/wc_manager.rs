// use goldilocks::{AbstractField, Goldilocks};
// use log::{info, trace};
// use pil2_components::ZiskProcessor;
// use pilout::pilout_proxy::PilOutProxy;
// use common::{ExecutionCtx, ProofCtx, WitnessPilOut};
// use wcmanager::{DefaultWitnessPlanner, WitnessManager, WitnessPlanner};
// use std::env;

// type F = Goldilocks;

// #[allow(dead_code)]
// pub struct BasicZiskWCPlugin<'a, F> {
//     // pilout: WitnessPilOut,
//     zisk_processor: ZiskProcessor<'a, F>,
//     witness_planner: Box<dyn WitnessPlanner<F>>,
// }

// impl<'a, F: AbstractField + 'static> BasicZiskWCPlugin<'a, F> {
//     const MY_NAME: &'static str = "ZiskWC  ";

//     pub fn new(witness_planner: Option<Box<dyn WitnessPlanner<F>>>) -> Self {
//         let current_dir = env::current_dir().unwrap();
//         let pilout_path = current_dir.join("./zisk-wc/src/pil/pilout/basic.pilout");
//         // let pilout = PilOutProxy::new(pilout_path.to_str().unwrap(), false).unwrap_or_else(|error| {
//         //     panic!("Failed to load pilout: {}", error);
//         // });

//         // TODO Check hash!

//         let zisk_processor = ZiskProcessor::<F>::new(&pilout);

//         let witness_planner = match witness_planner {
//             Some(witness_planner) => witness_planner,
//             None => Box::new(DefaultWitnessPlanner::new()),
//         };

//         Self { /*pilout,*/ zisk_processor, witness_planner: witness_planner }
//     }
// }

// #[allow(unused_variables)]
// impl<'a, F: AbstractField + 'static> WitnessManager<F> for BasicZiskWCPlugin<'a, F> {
//     fn initialize(&self) {
//         env_logger::builder()
//             .format_timestamp(None)
//             .format_level(true)
//             .format_target(false)
//             .filter_level(log::LevelFilter::Trace)
//             .init();
//     }

//     fn get_pilout(&self) -> &PilOutProxy {
//         &self.pilout
//     }

//     fn start_proof(&mut self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx) {
//         info!("{}: ··· Starting proof", Self::MY_NAME);
//         for (module_name, module) in self.zisk_processor.get_modules() {
//             module.borrow_mut().start_proof(proof_ctx, execution_ctx, &self.pilout);
//         }

//         info!("{}: ··· Executing", Self::MY_NAME);
//         self.zisk_processor.execute(proof_ctx, execution_ctx);
//     }

//     fn end_proof(&mut self, proof_ctx: &ProofCtx<F>) {
//         trace!("Ending proof");
//         for (module_name, module) in self.zisk_processor.get_modules() {
//             module.borrow_mut().end_proof(proof_ctx);
//         }
//     }

//     fn calculate_air_instances_map(&self, proof_ctx: &ProofCtx<F>) {
//         self.witness_planner.calculate_air_instances_map(&proof_ctx.air_instances_map);
//     }

//     fn calculate_witness(&self, stage: u32, pilout: &PilOutProxy, proof_ctx: &ProofCtx<F>) {
//         trace!("Calculate witness for stage {}", stage);
//         let modules = self.zisk_processor.get_modules();

//         for (air_group_id, air_instances) in proof_ctx.air_instances_map.inner.iter().enumerate() {
//             for air_instance in air_instances {
//                 // TODO Hardcoded to be removed
//                 // let module_pilout = pilout.get_air(air_group_id, air_instance.air_id as usize);

//                 // let module = modules.get(module_pilout.name()).unwrap_or_else(|| {
//                 //     panic!("Module {} not found", module_pilout.name());
//                 // });

//                 let module_name = match air_instance.air_id {
//                     0 | 1 => "Arith",
//                     2 => "Binary",
//                     3 | 4 => "Main",
//                     7 => "Memory",
//                     _ => panic!("Module not found"),
//                 };
//                 let module = modules.get(module_name).unwrap_or_else(|| {
//                     panic!("Module not found");
//                 });

//                 module.borrow().calculate_witness(stage, proof_ctx, air_instance);
//             }
//         }
//     }
// }

// #[no_mangle]
// pub extern "Rust" fn create_plugin() -> Box<dyn WitnessManager<F>> {
//     Box::new(BasicZiskWCPlugin::new(None))
// }
