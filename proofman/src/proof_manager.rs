use crate::public_input::PublicInput;
use crate::{prover::Prover, executor::Executor};
use pilout::load_pilout;
use log::debug;

use math::FieldElement;
use crate::provers_manager::ProversManager;
use crate::witness_calculator_manager::WitnessCalculatorManager;

use crate::proof_ctx::ProofCtx;

// PROOF MANAGER OPTIONS
// ================================================================================================
#[derive(Debug)]
pub struct ProofManOpt {
    pub debug: bool,
    pub only_check: bool,
}

impl Default for ProofManOpt {
    fn default() -> Self {
        Self {
            debug: false,
            only_check: false,
        }
    }
}

// PROOF MANAGER
// ================================================================================================
pub struct ProofManager<T> {
    options: ProofManOpt,
    proof_ctx: ProofCtx<T>,
    wc_manager: WitnessCalculatorManager<T>,
    provers_manager: ProversManager,
}

impl<T> ProofManager<T>
where T: FieldElement,
{
    const MY_NAME: &'static str = "proofman";

    pub fn new(pilout_path: &str, wc: Vec<Box<dyn Executor<T>>>, prover: Box<dyn Prover>, options: ProofManOpt) -> Self {
        env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();

        let reset = "\x1b[37;0m";
        let bold = "\x1b[1m";
        let purple = "\x1b[35m";
        let green = "\x1b[32;1m";
        println!("{}{}Proof Manager {} by Polygon Labs{}", bold, purple, env!("CARGO_PKG_VERSION"), reset);
        println!("{}{}{} {}", green, format!("{: >13}", "Loaded:"), reset, std::env::current_exe().unwrap().display().to_string().as_str());
        println!("{}{}{} {}", green, format!("{: >13}", "Main PID:"), reset, std::process::id().to_string().as_str());
        println!("");
        println!("{}PROVE COMMAND{}", green, reset);
        // println!("{}{}{} {}", green, format!("{: >13}", "ProofMan:"), reset, "TODO");
        println!("{}{}{} {}", green, format!("{: >13}", "Pilout:"), reset, str::replace(pilout_path, "\\", "/"));
        // println!("{}{}{} {}", green, format!("{: >13}", "Executors:"), reset, "TODO");
        // println!("{}{}{} {}", green, format!("{: >13}", "Prover:"), reset, "TODO");
        println!("");
    
    
        debug!("{}> Initializing...", Self::MY_NAME);    

        let pilout = load_pilout(pilout_path);

        // TODO! Have we to take in account from here the FinitieField choosed in the pilout?

        let proof_ctx = ProofCtx::<T>::new(pilout);

        // Add WitnessCalculatorManager
        let wc_manager = WitnessCalculatorManager::new(wc);

        // Add ProverManager
        let provers_manager = ProversManager::new(prover);

        Self {
            options,
            proof_ctx,
            wc_manager,
            provers_manager,
        }
    }

    pub fn prove(&mut self, public_inputs: Option<Box<dyn PublicInput>>) {
        if !self.options.only_check {
            debug!("{}> INITIATING PROOF GENERATION", Self::MY_NAME);
        } else {
            debug!("{}> INITIATING PILOUT VERIFICATION", Self::MY_NAME);
        }

        self.proof_ctx.initialize_proof(public_inputs);

        //     let proverStatus = PROVER_OPENINGS_PENDING;
        //     for (let stageId = 1; proverStatus !== PROVER_OPENINGS_COMPLETED; stageId++) {
        //         let str = stageId <= this.proofCtx.airout.numStages + 1 ? "STAGE" : "OPENINGS";
        //         log.info(`[${this.name}]`, `==> ${str} ${stageId}`);

        //         await this.wcManager.witnessComputation(stageId, publics);

        //         if (stageId === 1) await this.proversManager.setup(setup);

        //         proverStatus = await this.proversManager.computeStage(stageId, publics, this.options);

        //         log.info(`[${this.name}]`, `<== ${str} ${stageId}`);

        //         if(stageId === this.proofCtx.airout.numStages) {
        //             for(let i = 0; i < this.proofCtx.airout.subproofs.length; i++) {
        //                 const subproof = this.proofCtx.airout.subproofs[i];
        //                 const subAirValues = subproof.subproofvalues;
        //                 if(subAirValues === undefined) continue;
        //                 const instances = this.proofCtx.airInstances.filter(airInstance => airInstance.subproofId === i);
        //                 for(let j = 0; j < subAirValues.length; j++) {
        //                     const aggType = subAirValues[j].aggType;
        //                     for(const instance of instances) {
        //                         const subproofValue = instance.ctx.subAirValues[j];
        //                         this.proofCtx.subAirValues[i][j] = aggType === 0 
        //                             ? this.proofCtx.F.add(this.proofCtx.subAirValues[i][j], subproofValue) 
        //                             : this.proofCtx.F.mul(this.proofCtx.subAirValues[i][j], subproofValue);
        //                     }
        //                 }
        //             }
        //         }

        //         // If onlyCheck is true, we check the constraints stage by stage from stage1 to stageQ - 1 and do not generate the proof
        //         if(this.options.onlyCheck) {
        //             log.info(`[${this.name}]`, `==> CHECKING CONSTRAINTS STAGE ${stageId}`);

        //             const valid = await this.proversManager.verifyConstraints(stageId);
        //             if(!valid) {
        //                 log.error(`[${this.name}]`, `Constraints verification failed.`);
        //                 throw new Error(`[${this.name}]`, `Constraints verification failed.`);
        //             }

        //             log.info(`[${this.name}]`, `<== CHECKING CONSTRAINTS STAGE ${stageId}`);

        //             if(stageId === this.proofCtx.airout.numStages) {
        //                 log.info(`[${this.name}]`, `==> CHECKING GLOBAL CONSTRAINTS.`);

        //                 const validG = await this.proversManager.verifyGlobalConstraints();

        //                 if(!validG) {
        //                     log.error(`[${this.name}]`, `Global constraints verification failed.`);
        //                     throw new Error(`[${this.name}]`, `Global constraints verification failed.`);
        //                 }
                        
        //                 log.info(`[${this.name}]`, `<== CHECKING GLOBAL CONSTRAINTS.`);
        //                 return true;
        //             }
        //         }
        //     }

        debug!("{}> PROOF SUCCESSFULLY GENERATED", Self::MY_NAME);

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
    }

    pub fn verify() {
        unimplemented!();
    }
}