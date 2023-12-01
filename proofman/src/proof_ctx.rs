use core::trace::Trace;

use std::sync::{Arc, Mutex};

// PROOF CONTEXT
// ================================================================================================
#[derive(Debug)]
pub struct ProofCtx {

    air_instances: Vec<Arc<Mutex<AirInstance>>>,
}

#[allow(dead_code)]
impl ProofCtx {
    pub fn new() -> Self {
        ProofCtx {
            air_instances: Vec::new(),
        }
    }

    pub fn add_air_instance(&mut self, subproof_id: usize, air_id: usize, trace: Arc<Mutex<Trace>>) {
        let instance_id = self.air_instances.len();
        let air_instance = Arc::new(Mutex::new(AirInstance::new(subproof_id, air_id, instance_id, trace)));
        self.air_instances.push(air_instance);
    }
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
    pub fn new(subproof_id: usize, air_id: usize, instance_id: usize, trace: Arc<Mutex<Trace>>) -> Self {
        AirInstance {
            subproof_id,
            air_id,
            instance_id,
            //trace_layout,
            traces: vec![trace],
        }
    }
}