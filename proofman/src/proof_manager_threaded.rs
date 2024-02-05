// use core::fmt;

// use crate::public_inputs::PublicInputs;
// use crate::prover::Prover;
// use pilout::load_pilout;
// use log::{debug, info, error};

// use crate::prover::provers_manager::ProversManager;

// use crate::executor::ExecutorBase;
// use crate::executor::executors_manager::ExecutorsManager;
// use crate::executor::executors_manager_thread::WitnessCalculatorManagerThread;

// use crate::proof_ctx::ProofCtx;
// use crate::config::Config;

// // PROOF MANAGER OPTIONS
// // ================================================================================================
// #[derive(Debug)]
// pub struct ProofManOpt {
//     pub debug: bool,
//     pub only_check: bool,
// }

// impl Default for ProofManOpt {
//     fn default() -> Self {
//         Self { debug: false, only_check: false }
//     }
// }

// // PROOF MANAGER
// // ================================================================================================
// #[derive(Debug, PartialEq)]
// pub enum ProverStatus {
//     OpeningsPending,
//     OpeningsCompleted,
// }

// pub struct ProofManagerThreaded<T> {
//     options: ProofManOpt,
//     proof_ctx: ProofCtx<T>,
//     wc_manager: WitnessCalculatorManagerThread<T>,
//     config: Box<dyn Config>,
//     provers_manager: ProversManager,
// }

// impl<T: Default + Clone + Send + Sync + fmt::Debug> ProofManagerThreaded<T> {
//     const MY_NAME: &'static str = "proofman";

//     pub fn new(
//         pilout_path: &str,
//         wc: Vec<Box<dyn ExecutorBase<T>>>,
//         prover: Box<dyn Prover>,
//         config: Box<dyn Config>,
//         options: ProofManOpt,
//     ) -> Self {
//         let reset = "\x1b[37;0m";
//         let purple = "\x1b[35m";
//         let green = "\x1b[32;1m";
//         let bold = "\x1b[1m";
//         println!("    {}{}PROOFMAN by Polygon Labs v{}{}", bold, purple, env!("CARGO_PKG_VERSION"), reset);
//         // println!(
//         //     "{}{}{} {}",
//         //     green,
//         //     format!("{: >12}", "Loaded"),
//         //     reset,
//         //     std::env::current_exe().unwrap().display().to_string().as_str()
//         // );
//         // println!("{}{}{} {}", green, format!("{: >12}", "Main PID"), reset, std::process::id().to_string().as_str());
//         println!("{}{}{} {}", green, format!("{: >12}", "Pilout"), reset, str::replace(pilout_path, "\\", "/"));
//         // println!("{}{}{} {}", green, format!("{: >13}", "Executors:"), reset, "TODO");
//         // println!("{}{}{} {}", green, format!("{: >13}", "Prover:"), reset, "TODO");
//         println!("");

//         debug!("{}: Initializing...", Self::MY_NAME);

//         let pilout = load_pilout(pilout_path);

//         let proof_ctx = ProofCtx::<T>::new(pilout);

//         // Add WitnessCalculatorManager
//         let wc_manager = WitnessCalculatorManagerThread::new(wc);

//         // Add ProverManager
//         let provers_manager = ProversManager::new(prover);

//         Self { options, proof_ctx, wc_manager, config, provers_manager }
//     }

//     pub fn setup() {
//         unimplemented!();
//     }

//     pub fn prove(&mut self, public_inputs: Option<Box<dyn PublicInputs<T>>>) {
//         if !self.options.only_check {
//             info!("{}: ==> INITIATING PROOF GENERATION", Self::MY_NAME);
//         } else {
//             info!("{}: ==> INITIATING PILOUT VERIFICATION", Self::MY_NAME);
//         }

//         self.proof_ctx.initialize_proof(public_inputs);

//         let mut prover_status = ProverStatus::OpeningsPending;
//         let mut stage_id: u32 = 1;
//         let num_stages = self.proof_ctx.pilout.num_challenges.len() as u32;

//         while prover_status != ProverStatus::OpeningsCompleted {
//             let stage_str = if stage_id <= num_stages + 1 { "STAGE" } else { "OPENINGS" };

//             info!("{}: ==> {} {}", Self::MY_NAME, stage_str, stage_id);

//             self.wc_manager.witness_computation(stage_id, &self.config, &mut self.proof_ctx);

//             if stage_id == 1 {
//                 self.provers_manager.setup(/*&setup*/);
//             }

//             prover_status = self.provers_manager.compute_stage(stage_id /*&public_inputs, &self.options*/);

//             info!("{}: <== {} {}", Self::MY_NAME, stage_str, stage_id);

//             // if stage_id == num_stages {
//             //     for i in 0..self.proof_ctx.pilout.subproofs.len() {
//             //         let subproof = self.proof_ctx.pilout.subproofs[i];
//             //         let sub_air_values = subproof.subproofvalues;
//             //         if sub_air_values.is_none() {
//             //             continue;
//             //         }
//             //         let instances = self.proof_ctx.air_instances.iter().filter(|air_instance| air_instance.subproof_id == i);
//             //         for j in 0..sub_air_values.unwrap().len() {
//             //             let agg_type = sub_air_values.unwrap()[j].agg_type;
//             //             for instance in instances {
//             //                 let subproof_value = instance.ctx.sub_air_values[j];
//             //                 self.proof_ctx.sub_air_values[i][j] = if agg_type == 0 {
//             //                     self.proof_ctx.F.add(self.proof_ctx.sub_air_values[i][j], subproof_value)
//             //                 } else {
//             //                     self.proof_ctx.F.mul(self.proof_ctx.sub_air_values[i][j], subproof_value)
//             //                 };
//             //             }
//             //         }
//             //     }
//             // }

//             // If onlyCheck is true, we check the constraints stage by stage from stage1 to stageQ - 1 and do not generate the proof
//             if self.options.only_check {
//                 info!("{}: ==> CHECKING CONSTRAINTS STAGE {}", Self::MY_NAME, stage_id);

//                 if !self.provers_manager.verify_constraints(stage_id) {
//                     error!("{}: CONSTRAINTS VERIFICATION FAILED", Self::MY_NAME);
//                 }

//                 info!("{}: <== CHECKING CONSTRAINTS STAGE {} FINISHED", Self::MY_NAME, stage_id);

//                 if stage_id == num_stages {
//                     info!("{}: ==> CHECKING GLOBAL CONSTRAINTS", Self::MY_NAME);

//                     if !self.provers_manager.verify_global_constraints() {
//                         error!("{}: Global constraints verification failed", Self::MY_NAME);
//                     }

//                     info!("{}: <== CHECKING GLOBAL CONSTRAINTS FINISHED", Self::MY_NAME);
//                     return;
//                 }
//             }

//             stage_id += 1;
//         }

//         info!("{}: <== PROOF SUCCESSFULLY GENERATED", Self::MY_NAME);

//         //     let proofs = [];

//         //     for(const airInstance of this.proofCtx.airInstances) {
//         //         airInstance.proof.subproofId = airInstance.subproofId;
//         //         airInstance.proof.airId = airInstance.airId;
//         //         proofs.push(airInstance.proof);
//         //     }

//         //     return {
//         //         proofs,
//         //         challenges: this.proofCtx.challenges.slice(0, this.proofCtx.airout.numStages + 3),
//         //         challengesFRISteps: this.proofCtx.challenges.slice(this.proofCtx.airout.numStages + 3).map(c => c[0]),
//         //         subAirValues: this.proofCtx.subAirValues,
//         //     };
//     }

//     pub fn verify() {
//         unimplemented!();
//     }
// }
