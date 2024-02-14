use crate::public_inputs::PublicInputs;
use crate::provers_manager::Prover;
use pilout::pilout_proxy::PilOutProxy;
use log::{debug, info, error};
use serde::de::DeserializeOwned;

use crate::provers_manager::ProversManager;

use crate::executor::Executor;
use crate::executor::executors_manager::{ExecutorsManager, ExecutorsManagerSequential};
use crate::proof_manager_config::{ExecutorsConfiguration, ProverConfiguration, MetaConfiguration};
use crate::proof_manager_config::ProofManConfig;

use crate::proof_ctx::ProofCtx;

use util::colors::colors::*;

// PROOF MANAGER
// ================================================================================================
#[derive(Debug, PartialEq)]
pub enum ProverStatus {
    OpeningsPending,
    OpeningsCompleted,
}

// PROOF MANAGER SEQUENTIAL
// ================================================================================================
#[allow(dead_code)]
pub struct ProofManager<T, E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    proofman_config: ProofManConfig<E, P, M>,
    proof_ctx: ProofCtx<T>,
    wc_manager: ExecutorsManagerSequential<T, E, P, M>,
    provers_manager: ProversManager<T>,
}

impl<T, E, P, M> ProofManager<T, E, P, M>
where
    T: Default + Clone,
    E: ExecutorsConfiguration + DeserializeOwned,
    P: ProverConfiguration + DeserializeOwned,
    M: MetaConfiguration + DeserializeOwned,
{
    const MY_NAME: &'static str = "proofMan";

    pub fn new(
        proofman_config: ProofManConfig<E, P, M>,
        wc: Vec<Box<dyn Executor<T, E, P, M>>>,
        prover: Box<dyn Prover<T>>,
    ) -> Self {
        println!("    {}{}PROOFMAN by Polygon Labs v{}{}", BOLD, PURPLE, env!("CARGO_PKG_VERSION"), RESET);

        println!("{}{}{} {}", GREEN, format!("{: >12}", "Pilout"), RESET, proofman_config.get_pilout());

        debug!("{}: Initializing", Self::MY_NAME);

        let pilout = PilOutProxy::new(proofman_config.get_pilout());

        let proof_ctx = ProofCtx::<T>::new(pilout);

        // Add WitnessCalculatorManager
        debug!("{}: ··· Creating proof executors manager", Self::MY_NAME);
        let wc_manager = ExecutorsManager::new(wc);

        // Add ProverManager
        debug!("{}: ··· Creating prover manager", Self::MY_NAME);
        let provers_manager = ProversManager::new(prover);

        Self { proofman_config, proof_ctx, wc_manager, provers_manager }
    }

    pub fn setup() {
        unimplemented!();
    }

    pub fn prove(&mut self, public_inputs: Option<Box<dyn PublicInputs<T>>>) -> &mut ProofCtx<T> {
        if !self.proofman_config.only_check {
            info!("{}: ==> INITIATING PROOF GENERATION", Self::MY_NAME);
        } else {
            info!("{}: ==> INITIATING PILOUT VERIFICATION", Self::MY_NAME);
        }

        self.proof_ctx.initialize_proof(public_inputs);

        let mut prover_status = ProverStatus::OpeningsPending;
        let mut stage_id: u32 = 1;
        let num_stages = self.proof_ctx.pilout.num_challenges.len() as u32;

        while prover_status != ProverStatus::OpeningsCompleted {
            let stage_str = if stage_id <= num_stages + 1 { "STAGE" } else { "OPENINGS" };

            info!("{}: ==> {} {}", Self::MY_NAME, stage_str, stage_id);

            self.wc_manager.witness_computation(&self.proofman_config, stage_id, &mut self.proof_ctx);

            if stage_id == 1 {
                self.provers_manager.setup(/*&setup*/);
            }

            prover_status = self.provers_manager.compute_stage(stage_id, &mut self.proof_ctx);

            info!("{}: <== {} {}", Self::MY_NAME, stage_str, stage_id);

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
                    return &mut self.proof_ctx;
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

        &mut self.proof_ctx
    }

    pub fn verify() {
        unimplemented!();
    }
}
