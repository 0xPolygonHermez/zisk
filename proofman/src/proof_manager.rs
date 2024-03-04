use crate::provers_manager::ProverBuilder;
use colored::Colorize;
// use pilout::pilout::SymbolType;
use pilout::pilout_proxy::PilOutProxy;
use log::{debug, info, error};

use crate::provers_manager::ProversManager;

use crate::executor::Executor;
use crate::executor::executors_manager::{ExecutorsManager, ExecutorsManagerSequential};
use crate::proof_manager_config::ProofManConfig;

use crate::proof_ctx::ProofCtx;

use util::cli::*;

// PROOF MANAGER
// ================================================================================================
#[derive(Debug, PartialEq)]
pub enum ProverStatus {
    StagesPending,
    StagesCompleted,
}

// PROOF MANAGER SEQUENTIAL
// ================================================================================================
#[allow(dead_code)]
pub struct ProofManager<T> {
    proofman_config: ProofManConfig,
    proof_ctx: ProofCtx<T>,
    wc_manager: ExecutorsManagerSequential<T>,
    provers_manager: ProversManager<T>,
}

impl<T> ProofManager<T>
where
    T: Default + Clone,
{
    const MY_NAME: &'static str = "proofMan";

    pub fn new(
        proofman_config: ProofManConfig,
        wc: Vec<Box<dyn Executor<T>>>,
        prover_builder: Box<dyn ProverBuilder<T>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        print_banner(true);

        println!("············ {}", proofman_config.get_name().bright_purple().bold());

        println!("{} {}", format!("{: >12}", "Pilout").bright_green().bold(), proofman_config.get_pilout());
        println!("");

        debug!("{}: Initializing", Self::MY_NAME);

        let pilout = PilOutProxy::new(proofman_config.get_pilout())?;

        //let's filter pilout symbols where type = WitnessColl
        // let witness_cols =
        //     pilout.symbols.iter().filter(|s| s.r#type == SymbolType::WitnessCol as i32).collect::<Vec<_>>();
        // println!("witness_cols: {:?}", witness_cols);

        let proof_ctx = ProofCtx::<T>::new(pilout);

        // Add WitnessCalculatorManager
        debug!("{}: ··· Creating proof executors manager", Self::MY_NAME);
        let wc_manager = ExecutorsManager::new(wc);

        // Add ProverManager
        debug!("{}: ··· Creating prover manager", Self::MY_NAME);
        let provers_manager = ProversManager::new(prover_builder);

        Ok(Self { proofman_config, proof_ctx, wc_manager, provers_manager })
    }

    pub fn setup() {
        unimplemented!();
    }

    pub fn prove(&mut self, public_inputs: Option<Vec<T>>) -> Result<&mut ProofCtx<T>, &str> {
        if !self.proofman_config.only_check {
            info!("{}: ==> INITIATING PROOF GENERATION", Self::MY_NAME);
        } else {
            info!("{}: ==> INITIATING PILOUT VERIFICATION", Self::MY_NAME);
        }

        self.proof_ctx.initialize_proof(public_inputs);

        let mut prover_status = ProverStatus::StagesPending;
        let mut stage_id: u32 = 1;
        // TODO! Uncomment this when pilout done!!!!
        // let num_stages = proof_ctx.pilout.get_num_stages();
        let num_stages = 3;

        while prover_status != ProverStatus::StagesCompleted {
            if stage_id <= num_stages {
                self.wc_manager.witness_computation(stage_id, &mut self.proof_ctx);
            }

            // Once the first witness computation is done we assume we have initialized the air instances.
            // So, we know the number of row for each air instance and we can select the setup for each air instance.
            if stage_id == 1 {
                // TODO!
                self.provers_manager.setup(/*&setup*/);
            }

            prover_status = self.provers_manager.compute_stage(stage_id, &mut self.proof_ctx);

            // if stage_id == num_stages {
            //     for i in 0..self.proof_ctx.pilout.subproofs.len() {
            //         let subproof = self.proof_ctx.pilout.subproofs[i];
            //         let sub_air_values = subproof.subproofvalues;
            //         if sub_air_values.is_none() {
            //             continue;
            //         }
            //         let instances = self.proof_ctx.air_instances.iter().filter(|air_instance| air_instance.subproof_id == i);
            //         for j in 0..sub_air_values.unwrap().len() {
            //             let agg_type = sub_air_values.unwrap()[j].agg_type;
            //             for instance in instances {
            //                 let subproof_value = instance.ctx.sub_air_values[j];
            //                 self.proof_ctx.sub_air_values[i][j] = if agg_type == 0 {
            //                     self.proof_ctx.F.add(self.proof_ctx.sub_air_values[i][j], subproof_value)
            //                 } else {
            //                     self.proof_ctx.F.mul(self.proof_ctx.sub_air_values[i][j], subproof_value)
            //                 };
            //             }
            //         }
            //     }
            // }

            // If onlyCheck is true, we check the constraints stage by stage from stage1 to stageQ - 1 and do not generate the proof
            if self.proofman_config.only_check {
                info!("{}: ==> CHECKING CONSTRAINTS STAGE {}", Self::MY_NAME, stage_id);

                if !self.provers_manager.verify_constraints(stage_id) {
                    error!("{}: CONSTRAINTS VERIFICATION FAILED", Self::MY_NAME);
                }

                info!("{}: <== CHECKING CONSTRAINTS STAGE {} FINISHED", Self::MY_NAME, stage_id);

                if stage_id == num_stages {
                    info!("{}: ==> CHECKING GLOBAL CONSTRAINTS", Self::MY_NAME);

                    if !self.provers_manager.verify_global_constraints() {
                        error!("{}: Global constraints verification failed", Self::MY_NAME);
                    }

                    info!("{}: <== CHECKING GLOBAL CONSTRAINTS FINISHED", Self::MY_NAME);
                    return Ok(&mut self.proof_ctx);
                }
            }

            stage_id += 1;
        }

        info!("{}: <== PROOF SUCCESSFULLY GENERATED", Self::MY_NAME);

        //     let proofs = [];

        //     for(const airInstance of this.proofCtx.airInstances) {
        //         airInstance.proof.subproofId = airInstance.subproofId;
        //         airInstance.proof.airId = airInstance.airId;
        //         proofs.push(airInstance.proof);
        //     }

        //     return {
        //         proofs,
        //         challenges: this.proofCtx.challenges.slice(0, this.proofCtx.airout.numStages + 3),
        //         challengesFRISteps: this.proofCtx.challenges.slice(this.proofCtx.airout.numStages + 3).map(c => c[0]),
        //         subAirValues: this.proofCtx.subAirValues,
        //     };

        Ok(&mut self.proof_ctx)
    }

    pub fn verify() {
        unimplemented!();
    }
}
