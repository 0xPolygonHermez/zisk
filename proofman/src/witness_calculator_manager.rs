use log::debug;
use crate::executor::Executor;
use crate::proof_ctx;
use crate::message::Message;
use crossbeam_channel::{unbounded, Receiver, Sender};

// WITNESS CALCULATOR MANAGER
// ================================================================================================
pub struct WitnessCalculatorManager<T> {
    wc: Vec<Box<dyn Executor<T>>>
}

impl<T: Send + Sync + std::fmt::Debug> WitnessCalculatorManager<T> {
    const MY_NAME: &'static str = "witnessm";

    pub fn new(wc: Vec<Box<dyn Executor<T>>>) -> Self {
        debug!("{}> Initializing...", Self::MY_NAME);

        WitnessCalculatorManager {
            wc
        }
    }

    pub fn witness_computation(&self, stage_id: usize, proof_ctx: &proof_ctx::ProofCtx<T>) {
        debug!("{}> Computing witness stage {}", Self::MY_NAME, stage_id);

        let (tx, rx): (Sender<Message>, Receiver<Message>) = unbounded();

        if stage_id == 1 {            
            std::thread::scope(|s| {
                for (subproof_id, subproof) in proof_ctx.pilout.subproofs.iter().enumerate() {
                    for wc in self.wc.iter() {
                        if subproof.name == Some(wc.get_name().to_string()) {
                            let tx = tx.clone();
                            let rx = rx.clone();
                            s.spawn(move || {
                                wc.witness_computation(stage_id as u32, subproof_id as u32, -1, proof_ctx, tx, rx);
                            });        
                        }
                    }
                }
// println!("MASTER THREAD 1");
//                 //MASTER
//                 loop {
//                     let msg = rx.recv().unwrap();
//                     match msg.payload {
//                         Payload::Halt => {
//                             println!("Halt!");
//                             break;
//                         },
//                         _ => {
//                             println!("Not done yet!");
//                         }
//                     }
//                 }
// println!("MASTER THREAD 2");
            });
        } else {
            std::thread::scope(|s| {
                for (instance_id, air) in proof_ctx.airs.iter().enumerate() {
                    let wc = &self.wc[air.subproof_id];
                    let tx = tx.clone();
                    let rx = rx.clone();
                    s.spawn(move || {
                        println!("thread spawned with pid: {:?}", std::thread::current().id());        
                        wc.witness_computation(stage_id as u32, air.subproof_id as u32, instance_id as i32, proof_ctx, tx, rx);
                    });        
                }
            });
        }

        // // const regulars = this.wc.filter(wc => wc.type === ModuleTypeEnum.REGULAR);
        // // const executors = [];

        // // this.wcDeferredLock = new TargetLock(regulars.length, 0);

        // // // NOTE: The first witness calculator is always the witness calculator deferred
        // // executors.push(this.witnessComputationDeferred(stageId));

        // //let arc_executors: Arc<Mutex<Vec<Box<dyn Executor>>>> = Arc::new(Mutex::new(self.witness_calculators));
        // if stage_id == 1 {
        //     //STEP 1:
        //     // Iterate over all witness_calculators and call witness_computation in a thread of each witnesscalculator

        //     //STEP 2:
        //     let wc = self.witness_calculators.lock().unwrap().pop().unwrap();
        //     //for wc in self.witness_calculators.lock().unwrap().iter() {
        //     let wc_cloned = Arc::new(Mutex::new(wc));
        //     let prooc_ctx_cloned = self.proof_ctx.clone().unwrap();
        // //                 let handle = thread::spawn(move || {
        // //                     // Access the executor inside the thread
        // //                     let wc = wc_cloned.lock().unwrap();

        // // //                    wc.witness_computation(stage_id, 0, -1, prooc_ctx_cloned);
        // //                 });

        // // let handle = thread::spawn(move || {
        // //     println!("hi!!!");
        // //     wc_cloned.witness_computation(stage_id, 0, -1, self.proof_ctx.clone().unwrap());
        // // });
        // //                handle.join().unwrap();
        // //println!("proof_ctx: {:?}", self.proof_ctx.clone().unwrap());

        // //                wc.witness_computation(stage_id, 0, -1, self.proof_ctx.clone().unwrap());
        // //}
        // } else {
        // }
        // // if(stageId === 1) {
        // //     for(const subproof of this.proofCtx.airout.subproofs) {
        // //         for (const wc of regulars) {
        // //             if(!wc.sm || subproof.name === wc.sm) {
        // //                 executors.push(wc._witnessComputation(stageId, subproof.subproofId, -1, -1, publics));
        // //             }
        // //         }
        // //     }
        // // } else {
        // //     for (const wc of regulars) {
        // //         for(const airInstance of this.proofCtx.airInstances) {
        // //             const subproof = this.proofCtx.airout.subproofs[airInstance.subproofId];

        // //             if(!wc.sm || subproof.name === wc.sm) {
        // //                 executors.push(wc._witnessComputation(stageId, airInstance.subproofId, airInstance.airId, airInstance.instanceId, publics));
        // //             }
        // //         }
        // //     }
        // // }

        // // //Executor deferred exits before it has to do it...
        // // await Promise.all(executors);

        // // if(this.airBus.hasPendingPayloads()) {
        // //     log.error(`[${this.name}]`, `Some witness calculators have pending payloads for stage ${stageId}. Unable to continue`);
        // //     throw new Error(`Some witness calculators have pending payloads for stage ${stageId}. Unable to continue`);
        // // }

        // debug!("[{}] > Computing witness stage {}", self.name, stage_id);
    }
}
