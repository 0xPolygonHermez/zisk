use log::{error, debug};

use std::cell::RefCell;
use std::rc::Rc;

use crate::witness_calculator_manager::WitnessCalculatorManager;

use crate::executor::Executor;
use crate::proof_ctx::ProofCtx;

#[derive(Debug, PartialEq)]
enum ProverStatus {
    OpeningsPending,
    OpeningsCompleted,
}

// PROOF ORCHESTRATOR
// ================================================================================================
pub struct ProofOrchestrator {
    name: String,
    initialized: bool,
    wc_manager: WitnessCalculatorManager,
    // TODO! Add Option<>
    proof_ctx: Rc<RefCell<ProofCtx>>,
}

#[allow(dead_code)]
impl ProofOrchestrator {
    pub fn new() -> Self {
        ProofOrchestrator {
            name: String::from("ProofOrch "),
            initialized: false,
            wc_manager: WitnessCalculatorManager::new(),
            proof_ctx: Rc::new(RefCell::new(ProofCtx::new())),
        }
    }

    pub fn check_initialized(&self) {
        assert!(self.initialized, "ProofOrchestrator is not initialized");
    }

    pub fn initialize(&mut self, _config: &str, options: &str, witness_calculators: Vec<Box<dyn Executor>>) {
        if self.initialized {
            error!("[{}] ProofOrchestrator is already initialized", self.name);
            panic!("ProofOrchestrator is already initialized");
        }

        debug!("[{}] > Initializing...", self.name);

        // self.options = options;

        /*
         * config is a JSON object containing the following fields:
         * - name: name of the proof
         * - airout: airout of the proof
         * - witnessCalculators: array of witnessCalculator types
         * - prover: prover 
         * - setup: setup data
         */
        // if (!await configValid(config)) {
        //     log.error(`[${this.name}]`, "Invalid proof orchestrator config.");
        //     throw new Error("Invalid proof orchestrator config.");
        // }
        // this.config = config;

        // const airout = new AirOut(this.config.airout.filename);

        // // Create the finite field object
        // const finiteField = FiniteFieldFactory.createFiniteField(airout.baseField);
        //self.proof_ctx = RefCell::new(ProofCtx::new());
        // this.proofCtx = ProofCtx.createProofCtxFromAirout(this.config.name, airout, finiteField);
        
        self.wc_manager.initialize(Rc::clone(&self.proof_ctx), witness_calculators, options);
        
        // this.proversManager = new ProversManager();
        // await this.proversManager.initialize(config.prover, this.proofCtx, this.options);

        self.initialized = true;
        
        // return;

        // async function configValid(config) {
        //     const fields = ["airout", "witnessCalculators", "prover"];
        //     for (const field of fields) {
        //         if (config[field] === undefined) {
        //             log.error(`[${this.name}]`, `No ${field} provided in config.`);
        //             return false;
        //         }
        //     }

        //     if (config.name === undefined) {
        //         config.name = "proof-" + Date.now();
        //         log.warn(`[${this.name}]`, `No name provided in config, assigning a default name ${config.name}.`);
        //     }

        //     if (config.witnessCalculators.length === 0) {
        //         log.error(`[${this.name}]`, "No witnessCalculators provided in config.");
        //         return false;
        //     }
    
        //     if (!await validateFileNameCorrectness(config)) {
        //         log.error(`[${this.name}]`, "Invalid config.");
        //         return false;
        //     }
    
        //     //TODO !!!!
        //     /// If setup exists check that it has a valid format

        //     return true;
        // }

        // async function validateFileNameCorrectness(config) {
        //     const airoutFilename =  path.join(__dirname, "..", config.airout.airoutFilename);
        //     if (!await fileExists(airoutFilename)) {
        //         log.error(`[${this.name}]`, `Airout ${airoutFilename} does not exist.`);
        //         return false;
        //     }
        //     config.airout.filename = airoutFilename;
        //     console.log(airoutFilename, config.airout.airoutFilename);


        //     for(const witnessCalculator of config.witnessCalculators) {
        //         const witnessCalculatorLib =  path.join(__dirname, "..", witnessCalculator.filename);

        //         if (!await fileExists(witnessCalculatorLib)) {
        //             log.error(`[${this.name}]`, `WitnessCalculator ${witnessCalculator.filename} does not exist.`);
        //             return false;
        //         }
        //         witnessCalculator.witnessCalculatorLib = witnessCalculatorLib;
        //     }                
    
        //     const proverFilename =  path.join(__dirname, "..", config.prover.filename);

        //     if (!await fileExists(proverFilename)) {
        //         log.error(`[${this.name}]`, `Prover ${proverFilename} does not exist.`);
        //         return false;
        //     }
        //     config.prover.proverFilename = proverFilename;

        //     if (config.setup !== undefined) {
        //         // TODO
        //     }

        //     return true;
        // }
    }

    fn new_proof(&self/*publics*/) {
        //await this.proofCtx.initialize(publics);
    }

    pub fn generate_proof(&self, /*setup, publics*/) {
        self.check_initialized();

        // try {
        //     if(this.options.onlyCheck === undefined || this.options.onlyCheck === false) {
        //         log.info(`[${this.name}]`, `==> STARTING GENERATION OF THE PROOF '${this.config.name}'.`);
        //     }

        self.new_proof(/*publics*/);

        let mut prover_status = ProverStatus::OpeningsPending;
        let mut stage_id = 1_u32;
        //Replicate the following loop in RUST
        while prover_status != ProverStatus::OpeningsCompleted {
            let str = if stage_id <= 1/*this.proofCtx.airout.numStages + 1*/ { "STAGE" } else { "OPENINGS" };
            debug!("[{}] > {} {} ============================", self.name, str, stage_id);

            self.wc_manager.witness_computation(stage_id/*, publics*/);

        //         if (stageId === 1) await this.proversManager.setup(setup);

        //         proverStatus = await this.proversManager.computeStage(stageId, publics, this.options);

            debug!("[{}] < {} {} ============================", self.name, str, stage_id);

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
            prover_status = ProverStatus::OpeningsCompleted;
            stage_id += 1;
        }
        //     log.info(`[${this.name}]`, `<== PROOF '${this.config.name}' SUCCESSFULLY GENERATED.`);

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

        // } catch (error) {
        //     log.error(`[${this.name}]`, `Error while generating proof: ${error}`);
        //     throw error;
        // }
    }
}
