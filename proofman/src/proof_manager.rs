use std::collections::HashMap;

use crate::provers_manager::ProverBuilder;
use colored::Colorize;
use goldilocks::AbstractField;
// use pilout::pilout::AggregationType;
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
    CommitStage,
    OpeningStage,
    StagesCompleted,
}

// PROOF MANAGER SEQUENTIAL
// ================================================================================================
#[allow(dead_code)]
pub struct ProofManager<'a, T, PB> {
    proofman_config: ProofManConfig,
    proof_ctx: ProofCtx<T>,
    wc_manager: ExecutorsManagerSequential<'a, T>,
    provers_manager: ProversManager<T, PB>,
}

impl<'a, T, PB> ProofManager<'a, T, PB>
where
    T: Default + Copy + Clone + AbstractField,
    PB: ProverBuilder<T>,
{
    const MY_NAME: &'static str = "proofMan";

    pub fn new<I>(
        proofman_config: ProofManConfig,
        wc: I,
        prover_builders: HashMap<String, PB>,
        // TODO! This flag is used only while developing vadcops. After that it must be removed.
        // TODO! It allows us to inidicate that we are using a BIG trace matrix instead of a fully enhanced vadcops as it is used in the current zkEVM implementation.
        // TODO! It allows us to indicate we are using a fake pilout instead of a real pilout.
        // TODO! This flag must be removed after the implementation of vadcops.
        dev_use_feature: bool,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        PB: ProverBuilder<T>,
        I: IntoIterator<Item = &'a dyn Executor<T>>,
    {
        print_banner(true);

        println!("············ {}", proofman_config.get_name().bright_purple().bold());

        println!("{} {}", format!("{: >12}", "Pilout").bright_green().bold(), proofman_config.get_pilout());
        println!("");

        debug!("{}: Initializing", Self::MY_NAME);

        let pilout = PilOutProxy::new(proofman_config.get_pilout(), dev_use_feature)?;

        //let's filter pilout symbols where type = WitnessCol
        // let witness_cols =
        //     pilout.symbols.iter().filter(|s| s.r#type == SymbolType::WitnessCol as i32).collect::<Vec<_>>();
        // println!("witness_cols: {:?}", witness_cols);

        let proof_ctx = ProofCtx::<T>::new(pilout);

        // Add WitnessCalculatorManager
        debug!("{}: ··· Creating proof executors manager", Self::MY_NAME);
        let wc_manager = ExecutorsManager::new(wc);

        // Add ProverManager
        debug!("{}: ··· Creating prover manager", Self::MY_NAME);
        let provers_manager = ProversManager::new(prover_builders, dev_use_feature);

        Ok(Self { proofman_config, proof_ctx, wc_manager, provers_manager })
    }

    pub fn setup() {
        unimplemented!();
    }

    pub fn prove(&mut self, public_inputs: Option<Vec<T>>) -> Result<&mut ProofCtx<T>, Box<dyn std::error::Error>> {
        if !self.proofman_config.only_check {
            info!("{}: ==> INITIATING PROOF GENERATION", Self::MY_NAME);
        } else {
            info!("{}: ==> INITIATING PILOUT VERIFICATION", Self::MY_NAME);
        }

        self.proof_ctx.initialize_proof(public_inputs);

        let mut prover_status = ProverStatus::CommitStage;
        let mut stage_id = 1u32;

        while prover_status != ProverStatus::StagesCompleted {
            if prover_status == ProverStatus::CommitStage {
                self.wc_manager.witness_computation(stage_id, &mut self.proof_ctx);
            }

            // After computing the witness on stage 1, we assume we know the value of N for all air instances.
            // This allows us to construct each air instance prover depending on its features.
            if stage_id == 1 {
                self.provers_manager.init_provers(&self.proof_ctx);
            }

            prover_status = self.provers_manager.compute_stage(stage_id, &mut self.proof_ctx);

            // If onlyCheck is true, we check the constraints stage by stage from stage1 to stageQ - 1 and do not generate the proof
            if self.proofman_config.only_check {
                info!("{}: ==> CHECKING CONSTRAINTS STAGE {}", Self::MY_NAME, stage_id);

                let verified = self.provers_manager.verify_constraints(stage_id);
                if verified {
                    info!("{}: CONSTRAINTS VERIFICATION PASSED", Self::MY_NAME);
                } else {
                    error!("{}: CONSTRAINTS VERIFICATION FAILED", Self::MY_NAME);
                }

                info!("{}: <== CHECKING CONSTRAINTS STAGE {} FINISHED", Self::MY_NAME, stage_id);

                if stage_id == self.provers_manager.num_stages().unwrap() {
                    info!("{}: ==> CHECKING GLOBAL CONSTRAINTS", Self::MY_NAME);

                    let verified_global = self.provers_manager.verify_global_constraints();
                    if verified_global {
                        info!("{}: GLOBAL CONSTRAINTS VERIFICATION PASSED", Self::MY_NAME);
                    } else {
                        error!("{}: GLOBAL CONSTRAINTS VERIFICATION FAILED", Self::MY_NAME);
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

    /// Computes subproof values for the proof context.
    ///
    /// This function iterates over the subproofs in the proof context,
    /// aggregates their subproof values based on aggregation type, and updates
    /// the proof context accordingly.
    fn _compute_subproof_values(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (subproof_id, subproof_pilout) in self.proof_ctx.pilout.subproofs.iter().enumerate() {
            let subproof_values_pilout = &subproof_pilout.subproofvalues;

            if subproof_values_pilout.is_empty() {
                log::warn!("{}: No subproof values for subproof {}", Self::MY_NAME, subproof_id);
                continue;
            }

            // let subproof_ctx = &self.proof_ctx.subproofs[subproof_id];

            // for (subproof_value_id, subproof_value_pilout) in subproof_values_pilout.iter().enumerate() {
            //     for air_ctx in &subproof_ctx.airs {
            //         for instance_ctx in &air_ctx.instances {
            //             let subproof_value = instance_ctx.subproof_values[subproof_value_id].clone();

            //             match AggregationType::try_from(subproof_value_pilout.agg_type).unwrap() {
            //                 AggregationType::Sum => {
            //                     // self.proof_ctx.subproof_values[subproof_id][subproof_value_id] += subproof_value;
            //                 }
            //                 AggregationType::Prod => {
            //                     // self.proof_ctx.subproof_values[subproof_id][subproof_value_id] *= subproof_value;
            //                 }
            //             }
            //         }
            //     }
            // }
        }

        Ok(())
    }

    pub fn verify() {
        unimplemented!();
    }
}
