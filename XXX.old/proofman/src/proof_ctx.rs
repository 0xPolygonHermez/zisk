use core::trace::Trace;

use std::sync::{Arc, Mutex};

// PROOF CONTEXT
// ================================================================================================
#[derive(Debug)]
pub struct ProofCtx<T: Default> {
    //publics: Vec<T>,
    //subAirValues: Vec<T>,
    //challenges: Vec<T>,
    //steps_fri: Vec<T>,
    air_instances: Vec<Arc<Mutex<AirInstance>>>,
    xxx: T,
}

#[allow(dead_code)]
impl<T: Default> ProofCtx<T> {
    pub fn new() -> Self {
        ProofCtx::<T> {
            //     publics: Vec::new(),
            air_instances: Vec::new(),
            xxx: T::default(),
        }
    }

    // TODO! Unblock when PROTOBUFFER library installed and imported
    // API functions related to PILOUT
    // pub fn getAirout() {}

    // API functions related to transcript and challenges
    pub fn addChallengeToTranscript(/*challenge*/) {}
    pub fn computeGlobalChallenge(/*stageId*/) {}
    pub fn getChallenge(/*stageId*/) {}

    // API functions related to AIR instances
    pub fn add_air_instance(
        &mut self,
        subproof_id: usize,
        air_id: usize,
        trace: Arc<Mutex<Trace>>,
    ) {
        let instance_id = self.air_instances.len();
        let air_instance =
            Arc::new(Mutex::new(AirInstance::new(subproof_id, air_id, instance_id, trace)));
        self.air_instances.push(air_instance);
    }

    pub fn set_filled(mut self, subproof_id: usize, instance_id: usize, column_name: &str) {}

    pub fn getAirInstancesBySubproofIdAirId(&self, subproof_id: usize, air_id: usize) {
        let mut air_instances = Vec::new();
        for air_instance in self.air_instances.iter() {
            let air_instance = air_instance.lock().unwrap();
            if air_instance.subproof_id == subproof_id && air_instance.air_id == air_id {
                air_instances.push(air_instance);
            }
        }
    }

    pub fn getAirInstanceColumn(&self, subproof_id: usize, instance_id: usize, column_name: &str) {}

    // TODO! unblock when PROTOBUFFER library installed and imported
    pub fn createProofCtxFromAirout(/*name, airout, stepsFRI, finiteField*/) {}
}

// AIR INSTANCE CONTEXT
// ================================================================================================
#[derive(Debug)]
#[allow(dead_code)]
pub struct AirInstance {
    subproof_id: usize,
    air_id: usize,
    instance_id: usize,
    //trace_layout: &TraceLayout,
    traces: Vec<Arc<Mutex<Trace>>>,
}

#[allow(dead_code)]
impl AirInstance {
    pub fn new(
        subproof_id: usize,
        air_id: usize,
        instance_id: usize,
        trace: Arc<Mutex<Trace>>,
    ) -> Self {
        AirInstance {
            subproof_id,
            air_id,
            instance_id,
            //trace_layout,
            traces: vec![trace],
        }
    }
}
