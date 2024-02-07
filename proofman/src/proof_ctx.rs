use std::{os::raw::c_void, sync::RwLock};
use pilout::pilout::PilOut;

use std::sync::Arc;

use crate::trace::trace::Trace;
use std::fmt;
use crate::public_inputs::PublicInputs;

/// Context for managing proofs, including information about Air instances.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProofCtx<T> {
    pub pilout: PilOut,
    pub public_inputs: Vec<T>,
    challenges: Vec<Vec<T>>,
    pub subproofs: Vec<SubproofCtx>,
    //pub subAirValues = Vec<T>,
    // NOTE: remove this ptr when vadcops ready, now it's used while developing
    pub proof: Option<*mut c_void>,
}

impl<T: Default + Clone> ProofCtx<T> {
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

        //TODO!
        // for i in 0..pilout.num_challenges.len() {
        //     println!("pilout.num_challenges[{}]: {}", i, pilout.num_challenges[i]);
        // }

        //TODO!
        // proofCtx.stepsFRI = stepsFRI;

        //TODO!
        // for(let i = 0; i < airout.subproofs.length; i++) {
        //     proofCtx.subAirValues[i] = [];
        //     for(let j = 0; j < airout.subproofs[i].subproofvalues?.length; j++) {
        //         const aggType = airout.subproofs[i].subproofvalues[j].aggType;
        //         proofCtx.subAirValues[i][j] = aggType === 0 ? zero : one;
        //     }
        // }
        let mut subproofs = Vec::new();
        for (subproof_index, _subproof) in pilout.subproofs.iter().enumerate() {
            let subproof = SubproofCtx { subproof_id: subproof_index, airs: Vec::new() };
            subproofs.push(subproof);

            for (air_index, _air) in pilout.subproofs[subproof_index].airs.iter().enumerate() {
                let air = AirCtx::new(subproof_index, air_index);
                subproofs[subproof_index].airs.push(air);
            }
        }

        ProofCtx { pilout, public_inputs: Vec::new(), challenges, subproofs, proof: None }
    }

    /// Initializes the proof context with optional public inputs
    pub fn initialize_proof(&mut self, public_inputs: Option<Box<dyn PublicInputs<T>>>) {
        if let Some(public_inputs) = public_inputs {
            self.public_inputs = public_inputs.to_vec();
        }

        for subproof in self.subproofs.iter() {
            for air in subproof.airs.iter() {
                air.instances.write().unwrap().clear();
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
    pub fn add_trace_to_air_instance(
        &self,
        subproof_id: usize,
        air_id: usize,
        trace: Box<dyn Trace>,
    ) -> Result<usize, &'static str> {
        // Check if subproof_id and air_id are valid
        assert!(subproof_id < self.subproofs.len(), "Subproof ID out of bounds");
        assert!(air_id < self.subproofs[subproof_id].airs.len(), "Air ID out of bounds");

        Ok(self.subproofs[subproof_id].airs[air_id].add_trace(trace))
    }

    pub fn get_trace(
        &self,
        subproof_id: usize,
        air_id: usize,
        trace_id: usize,
    ) -> Result<Arc<Box<dyn Trace>>, &'static str> {
        // Check if subproof_id and air_id are valid
        assert!(subproof_id < self.subproofs.len(), "Subproof ID out of bounds");
        assert!(air_id < self.subproofs[subproof_id].airs.len(), "Air ID out of bounds");

        self.subproofs[subproof_id].airs[air_id].get_trace(trace_id)
    }
}

/// Represents an instance of a Subproof within a proof.
#[derive(Debug)]
#[allow(dead_code)]
pub struct SubproofCtx {
    pub subproof_id: usize,
    pub airs: Vec<AirCtx>,
}

/// Represents an instance of an Air within a proof.
#[allow(dead_code)]
pub struct AirCtx {
    pub subproof_id: usize,
    pub air_id: usize,
    pub instances: RwLock<Vec<Arc<Box<dyn Trace>>>>,
}

impl AirCtx {
    /// Creates a new AirCtx.
    ///
    /// # Arguments
    ///
    /// * `subproof_id` - The subproof ID associated with the AirCtx.
    /// * `air_id` - The air ID associated with the AirCtx.
    pub fn new(subproof_id: usize, air_id: usize) -> Self {
        AirCtx { subproof_id, air_id, instances: RwLock::new(Vec::new()) }
    }

    /// Adds a trace to the AirCtx.
    ///
    /// # Arguments
    ///
    /// * `trace` - The trace to add to the AirCtx.
    pub fn add_trace(&self, trace: Box<dyn Trace>) -> usize {
        let mut traces = self.instances.write().unwrap();
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
        let traces = self.instances.read().unwrap();

        assert!(trace_id < traces.len(), "Trace ID out of bounds");

        Ok(Arc::clone(&traces[trace_id]))
    }
}

impl fmt::Debug for AirCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AirCtx")
            .field("subproof_id", &self.subproof_id)
            .field("air_id", &self.air_id)
            .field("traces", &self.instances)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use goldilocks::Goldilocks;
    // use crate::trace;

    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_proof_ctx() {
        let proof_ctx = ProofCtx {
            pilout: PilOut::default(),
            public_inputs: Vec::new(),
            challenges: vec![vec![Goldilocks::default(); 0]],
            subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
            ptr: std::ptr::null_mut(),
            proof: None,
        };

        let proof_ctx = Arc::new(proof_ctx);
        let _cloned_write = Arc::clone(&proof_ctx);

        // let write_handle = std::thread::spawn(move || {
        //     let proof_ctx = cloned_write;

        //     // Create trace
        //     trace!(Simple { field1: usize });
        //     let mut simple = Simple::new(16);

        //     for i in 0..16 {
        //         simple.field1[i] = i;
        //     }

        //     let res = proof_ctx.add_trace_to_air_instance(0, 0, Box::new(simple));
        //     assert!(res.is_ok());
        // });

        // write_handle.join().unwrap();
    }
}
