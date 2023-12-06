use std::sync::RwLock;
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

    pub fn find_air_instance(&self, subproof_id: usize, air_id: usize) -> Result<usize, &'static str> {
        if let Some(index) = self.air_instances.iter().position(|instance| instance.subproof_id == subproof_id && instance.air_id == air_id) {
            Ok(index)
        } else {
            Err("Air instance not found")
        }
    }

    pub fn add_trace_to_air_instance(&self, subproof_id: usize, air_id: usize, trace: Box<dyn Trace>) {
        if let Ok(index) = self.find_air_instance(subproof_id, air_id) {
            self.air_instances[index].add_trace(trace);
        } else {
            panic!("Could not find air instance with subproof_id {} and air_id {}", subproof_id, air_id);
        }
    }
}

#[allow(dead_code)]
pub struct AirInstance {
    subproof_id: usize,
    air_id: usize,
    traces: RwLock<Vec<Box<dyn Trace>>>,
}

impl AirInstance {
    pub fn new(subproof_id: usize, air_id: usize) -> Self {
        AirInstance {
            subproof_id,
            air_id,
            traces: RwLock::new(Vec::new()),
        }
    }

    pub fn add_trace(&self, trace: Box<dyn Trace>) {
        self.traces.write().unwrap().push(trace);
    }
}

impl fmt::Debug for AirInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AirInstance")
            .field("subproof_id", &self.subproof_id)
            .field("air_id", &self.air_id)
            .field("traces", &self.traces)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::trace;
    use super::ProofCtx;
    use std::sync::Arc;

    #[test]
    fn test_proof_ctx() {
        let mut proof_ctx = ProofCtx::new();
        proof_ctx.air_instances.push(super::AirInstance::new(0, 0));

        let proof_ctx = Arc::new(proof_ctx);
        let cloned_write = Arc::clone(&proof_ctx);

        let write_handle = std::thread::spawn(move || {
            let proof_ctx = cloned_write;

            // Create trace
            trace!(Simple { field1: usize });
            let mut simple = Simple::new(16);

            for i in 0..16 {
                simple.field1[i] = i;
            }

            proof_ctx.add_trace_to_air_instance(0, 0, simple);
        });

        write_handle.join().unwrap();
    }
}