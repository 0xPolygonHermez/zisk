use std::sync::RwLock;
use math::FieldElement;
use pilout::pilout::PilOut;

use std::sync::Arc;

use crate::trace::Trace;
use std::fmt;
use crate::public_input::PublicInput;

/// Context for managing proofs, including information about Air instances.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProofCtx<T> {
    pub pilout: PilOut,
    pub public_inputs: Option<Vec<T>>,
    challenges: Vec<Vec<T>>,
    pub instances: Vec<Vec<AirContext>>,
}

impl<T: FieldElement + Default> ProofCtx<T> {
    /// Creates a new `ProofCtx` with the given `PilOut`.
    pub fn new(pilout: PilOut) -> Self {
        // NOTE: consider Vec::with_capacity() instead of Vec::new()
        let mut challenges = Vec::<Vec<T>>::new();

        // TODO! Review this
        if !pilout.num_challenges.is_empty() {
            for i in 0..pilout.num_challenges.len() {
                challenges.push(vec![T::default(); pilout.num_challenges[i] as usize]);
            }
        } else {
            challenges.push(vec![]);
        }
        
        // qStage, evalsStage and friStage
        challenges.push(vec![T::default(); 1]);
        challenges.push(vec![T::default(); 1]);
        challenges.push(vec![T::default(); 2]);

        // for i in 0..pilout.num_challenges.len() {
        //     println!("pilout.num_challenges[{}]: {}", i, pilout.num_challenges[i]);
        // }

        // proofCtx.stepsFRI = stepsFRI;
        
        // for(let i = 0; i < airout.subproofs.length; i++) {
        //     proofCtx.subAirValues[i] = [];
        //     for(let j = 0; j < airout.subproofs[i].subproofvalues?.length; j++) {
        //         const aggType = airout.subproofs[i].subproofvalues[j].aggType;
        //         proofCtx.subAirValues[i][j] = aggType === 0 ? zero : one;
        //     }
        // }
        let mut instances = Vec::new();
        for (subproof_index, subproof) in pilout.subproofs.iter().enumerate() {   
            let mut air_contexts = Vec::new();
            for (air_index, _air) in subproof.airs.iter().enumerate() {
                air_contexts.push(AirContext::new(subproof_index, air_index));
            }
            instances.push(air_contexts);
        }

        ProofCtx {
            pilout,
            public_inputs: None,
            challenges,
            instances,
        }
    }

    /// Initializes the proof context with optional public inputs
    pub fn initialize_proof(&mut self, public_inputs: Option<Box<dyn PublicInput<T>>>) {
        if let Some(public_inputs) = public_inputs {
            self.public_inputs = Some(public_inputs.to_elements());
        }

        for subproof in self.instances.iter() {
            for air_context in subproof.iter() {
                air_context.traces.write().unwrap().clear();
            }
        }
    }

    /// Adds a trace to the specified Air instance.
    ///
    /// # Arguments
    ///
    /// * `subproof_id` - The subproof ID of the target Air instance.
    /// * `air_id` - The air ID of the target Air instance.
    /// * `trace` - The trace to add to the Air instance.
    ///
    /// # Panics
    ///
    /// Panics if the specified Air instance is not found.
    pub fn add_trace_to_air_instance(&self, subproof_id: usize, air_id: usize, trace: Box<dyn Trace>) -> Result<usize, &'static str> {
        // Check if subproof_id and air_id are valid
        if subproof_id >= self.instances.len() {
            return Err("Subproof ID out of bounds");
        }
        if air_id >= self.instances[subproof_id].len() {
            return Err("Air ID out of bounds");
        }

        Ok(self.instances[subproof_id][air_id].add_trace(trace))
    }
}

/// Represents an instance of an Air within a proof.
#[allow(dead_code)]
pub struct AirContext {
    pub subproof_id: usize,
    pub air_id: usize,
    pub traces: RwLock<Vec<Arc<Box<dyn Trace>>>>,
}

impl AirContext {
    /// Creates a new AirContext.
    ///
    /// # Arguments
    ///
    /// * `subproof_id` - The subproof ID associated with the AirContext.
    /// * `air_id` - The air ID associated with the AirContext.
    pub fn new(subproof_id: usize, air_id: usize) -> Self {
        AirContext {
            subproof_id,
            air_id,
            traces: RwLock::new(Vec::new()),
        }
    }

    /// Adds a trace to the AirContext.
    ///
    /// # Arguments
    ///
    /// * `trace` - The trace to add to the AirContext.
    pub fn add_trace(&self, trace: Box<dyn Trace>) -> usize {
        let mut traces = self.traces.write().unwrap();
        traces.push(Arc::new(trace));
        traces.len() - 1
    }

    /// Returns a reference to the trace at the specified index.
    /// 
    /// # Arguments
    /// 
    /// * `trace_id` - The index of the trace to return.
    /// 
    /// # Returns
    /// 
    /// Returns a reference to the trace at the specified index.
    pub fn get_trace(&self, trace_id: usize) -> Result<Arc<Box<dyn Trace>>, &'static str> {
        let traces = self.traces.read().unwrap();
    
        if trace_id < traces.len() {
            Ok(Arc::clone(&traces[trace_id]))
        } else {
            Err("Trace not found")
        }
    }
}

impl fmt::Debug for AirContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AirContext")
            .field("subproof_id", &self.subproof_id)
            .field("air_id", &self.air_id)
            .field("traces", &self.traces)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use math::fields::f64::BaseElement;

    use crate::trace;
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_proof_ctx() {
        let proof_ctx = ProofCtx {
            pilout: PilOut::default(),
            public_inputs: None,
            challenges: vec![vec![BaseElement::default(); 0]],
            instances: vec![vec![AirContext::new(0, 0)]],
        };

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

            let res = proof_ctx.add_trace_to_air_instance(0, 0, simple);
            assert!(res.is_ok());
        });

        write_handle.join().unwrap();
    }
}