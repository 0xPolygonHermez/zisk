use std::sync::{Arc, RwLock};
use crate::trace::Trace;
use std::fmt;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ProofCtx {
    air_instances: Vec<AirInstance>,
}

impl ProofCtx {
    pub fn new() -> Self {
        ProofCtx {
            air_instances: Vec::new(),
        }
    }

    pub fn find_air_instance(&self, subproof_id: usize, air_id: usize) -> Option<usize> {
        // Search for the index of the air instance with the given subproof_id and air_id
        self.air_instances.iter().position(|instance| instance.subproof_id == subproof_id && instance.air_id == air_id)
    }
    
    pub fn add_trace_to_air_instance(&mut self, subproof_id: usize, air_id: usize, trace: Box<dyn Trace>) {
        // Search for the index of the air instance
        if let Some(index) = self.find_air_instance(subproof_id, air_id) {
            // If found, add the trace to the existing instance
            self.air_instances[index].add_trace(trace);
        } else {
            // If not found, create a new instance and add the trace to it
            let mut air_instance = AirInstance::new(subproof_id, air_id);
            air_instance.add_trace(trace);
            self.air_instances.push(air_instance);
        }
    }    
}

#[allow(dead_code)]
pub struct AirInstance {
    subproof_id: usize,
    air_id: usize,
    traces: Vec<Arc<RwLock<Box<dyn Trace>>>>,
}

impl AirInstance {
    pub fn new(subproof_id: usize, air_id: usize) -> Self {
        AirInstance {
            subproof_id,
            air_id,
            traces: Vec::new(),
        }
    }

    pub fn add_trace(&mut self, trace: Box<dyn Trace>) {
        let trace = Arc::new(RwLock::new(trace));
        self.traces.push(trace);
    }
}

impl fmt::Debug for AirInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AirInstance")
            .field("subproof_id", &self.subproof_id)
            .field("air_id", &self.air_id)
            //.field("traces", &self.traces)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::trace;

    #[test]
    fn test_proof_ctx() {
        let mut proof_ctx = super::ProofCtx::new();

        // Create trace
        trace!(Simple { field1: usize });
        let mut simple = Simple::new(16);

        for i in 0..16 {
            simple.field1[i] = i;
        }

        proof_ctx.add_trace_to_air_instance(0, 0, simple);

        println!("{:?}", proof_ctx);
    }
}