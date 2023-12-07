use log::{debug, error};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::{executor::Executor, proof_ctx::ProofCtx};

// WITNESS CALCULATOR MANAGER
// ================================================================================================
pub struct WitnessCalculatorManager<T: Default> {
    name: String,
    initialized: bool,
    proof_ctx: Option<Arc<RwLock<ProofCtx<T>>>>,
    witness_calculators: Arc<Mutex<Vec<Box<dyn Executor<T>>>>>,
}

#[allow(dead_code)]
impl<T: Default> WitnessCalculatorManager<T> {
    pub fn new() -> Self {
        WitnessCalculatorManager {
            name: String::from("WC Manager"),
            initialized: false,
            proof_ctx: None,
            witness_calculators: Arc::new(Mutex::new(Vec::<Box<dyn Executor<T>>>::new())),
        }
    }

    pub fn check_initialized(&self) {
        assert!(self.initialized, "WC Manager is not initialized");
    }

    pub fn initialize(
        &mut self,
        proof_ctx: Arc<RwLock<ProofCtx<T>>>,
        witness_calculators: Arc<Mutex<Vec<Box<dyn Executor<T>>>>>,
    ) {
        if self.initialized {
            error!("[{}] WC Manager is already initialized", self.name);
            panic!("WC Manager is already initialized");
        }

        debug!("[{}] > Initializing...", self.name);

        self.proof_ctx = Some(proof_ctx);
        self.witness_calculators = witness_calculators;
        self.initialized = true;
    }

    pub fn witness_computation(&self, stage_id: u32) {
        self.check_initialized();

        debug!("[{}] > Computing witness stage {}", self.name, stage_id);

        // const regulars = this.wc.filter(wc => wc.type === ModuleTypeEnum.REGULAR);
        // const executors = [];

        // this.wcDeferredLock = new TargetLock(regulars.length, 0);

        // // NOTE: The first witness calculator is always the witness calculator deferred
        // executors.push(this.witnessComputationDeferred(stageId));

        //let arc_executors: Arc<Mutex<Vec<Box<dyn Executor>>>> = Arc::new(Mutex::new(self.witness_calculators));
        if stage_id == 1 {
            //STEP 1:
            // Iterate over all witness_calculators and call witness_computation in a thread of each witnesscalculator

            //STEP 2:
            let wc = self.witness_calculators.lock().unwrap().pop().unwrap();
            //for wc in self.witness_calculators.lock().unwrap().iter() {
            let wc_cloned = Arc::new(Mutex::new(wc));
            let prooc_ctx_cloned = self.proof_ctx.clone().unwrap();
        //                 let handle = thread::spawn(move || {
        //                     // Access the executor inside the thread
        //                     let wc = wc_cloned.lock().unwrap();

        // //                    wc.witness_computation(stage_id, 0, -1, prooc_ctx_cloned);
        //                 });

        // let handle = thread::spawn(move || {
        //     println!("hi!!!");
        //     wc_cloned.witness_computation(stage_id, 0, -1, self.proof_ctx.clone().unwrap());
        // });
        //                handle.join().unwrap();
        //println!("proof_ctx: {:?}", self.proof_ctx.clone().unwrap());

        //                wc.witness_computation(stage_id, 0, -1, self.proof_ctx.clone().unwrap());
        //}
        } else {
        }
        // if(stageId === 1) {
        //     for(const subproof of this.proofCtx.airout.subproofs) {
        //         for (const wc of regulars) {
        //             if(!wc.sm || subproof.name === wc.sm) {
        //                 executors.push(wc._witnessComputation(stageId, subproof.subproofId, -1, -1, publics));
        //             }
        //         }
        //     }
        // } else {
        //     for (const wc of regulars) {
        //         for(const airInstance of this.proofCtx.airInstances) {
        //             const subproof = this.proofCtx.airout.subproofs[airInstance.subproofId];

        //             if(!wc.sm || subproof.name === wc.sm) {
        //                 executors.push(wc._witnessComputation(stageId, airInstance.subproofId, airInstance.airId, airInstance.instanceId, publics));
        //             }
        //         }
        //     }
        // }

        // //Executor deferred exits before it has to do it...
        // await Promise.all(executors);

        // if(this.airBus.hasPendingPayloads()) {
        //     log.error(`[${this.name}]`, `Some witness calculators have pending payloads for stage ${stageId}. Unable to continue`);
        //     throw new Error(`Some witness calculators have pending payloads for stage ${stageId}. Unable to continue`);
        // }

        debug!("[{}] > Computing witness stage {}", self.name, stage_id);
    }
}
