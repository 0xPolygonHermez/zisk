use core::trace::Trace;
use core::trace::trace_layout::TraceLayout;

use std::sync::{Arc, Mutex};

// PROOF CONTEXT
// ================================================================================================
#[derive(Debug)]
pub struct ProofCtx {
    pub counter: usize,
    _air_instances: Vec<Arc<Mutex<AirInstance>>>,
}

#[allow(dead_code)]
impl ProofCtx {
    pub fn new() -> Self {
        ProofCtx {
            counter: 0,
            _air_instances: Vec::new(),
        }
    }

    pub fn add_air_instance(&mut self, _subproof_id: usize, _air_id: usize, /*trace_layout: &'a TraceLayout*/) {
        //let instance_id = self.air_instances.len();
        //let air_instance = Arc::new(Mutex::new(AirInstance::new(subproof_id, air_id, instance_id, trace_layout)));
        //self.air_instances.push(air_instance);
        println!("add_air_instance");
    }
}

// AIR INSTANCE CONTEXT
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct AirInstance {
    subproof_id: usize,
    air_id: usize,
    instance_id: usize,
    //trace_layout: &TraceLayout,
    traces: Vec<Trace>,
}

#[allow(dead_code)]
impl AirInstance {
    pub fn new(subproof_id: usize, air_id: usize, instance_id: usize, _trace_layout: &TraceLayout) -> Self {
        AirInstance {
            subproof_id,
            air_id,
            instance_id,
            //trace_layout,
            traces: Vec::new(),
        }
    }
}