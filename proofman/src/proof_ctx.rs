use std::sync::RwLock;
use math::FieldElement;
use pilout::pilout::PilOut;

use crate::trace::Trace;
use std::fmt;

use crate::public_input::PublicInput;

/// Context for managing proofs, including information about Air instances.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProofCtx<T> {
    pub pilout: PilOut,
    public_inputs: Option<Box<dyn PublicInput<T>>>,
    challenges: Vec<Vec<T>>,
    airs: Vec<AirContext>,
}

impl<T: FieldElement + Default> ProofCtx<T> {
    pub fn new(pilout: PilOut) -> Self {
        println!("pilout: {:?}", pilout.num_challenges);

        let mut challenges = Vec::<Vec<T>>::new();

        // TODO! Review this
        if pilout.num_challenges.len() > 0 {
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

        for i in 0..pilout.num_challenges.len() {
            println!("pilout.num_challenges[{}]: {}", i, pilout.num_challenges[i]);
        }

        // proofCtx.stepsFRI = stepsFRI;
        
        // for(let i = 0; i < airout.subproofs.length; i++) {
        //     proofCtx.subAirValues[i] = [];
        //     for(let j = 0; j < airout.subproofs[i].subproofvalues?.length; j++) {
        //         const aggType = airout.subproofs[i].subproofvalues[j].aggType;
        //         proofCtx.subAirValues[i][j] = aggType === 0 ? zero : one;
        //     }
        // }
        let mut airs = Vec::new();
        for (subproof_index, subproof) in pilout.subproofs.iter().enumerate() {            
            for (air_index, _air) in subproof.airs.iter().enumerate() {
                airs.push(AirContext::new(subproof_index, air_index));
            }
        }

        ProofCtx {
            pilout,
            public_inputs: None,
            challenges,
            airs,
        }
    }

    pub fn initialize_proof(&mut self, public_inputs: Option<Box<dyn PublicInput<T>>>) {
        self.public_inputs = public_inputs;

        // TODO!
        // const poseidon = await buildPoseidonGL();
        // this.transcript = new Transcript(poseidon);

        // TODO! remove existing traces
        for air in self.airs.iter_mut() {
            air.traces.write().unwrap().clear();
        }
    }

    /// Finds the index of the Air instance with the given subproof_id and air_id.
    ///
    /// # Arguments
    ///
    /// * `subproof_id` - The subproof ID to search for.
    /// * `air_id` - The air ID to search for.
    ///
    /// # Returns
    ///
    /// Returns `Some(index)` if the Air instance is found, or `None` otherwise.
    pub fn find_air_instance(&self, subproof_id: usize, air_id: usize) -> Result<usize, &'static str> {
        if let Some(index) = self.airs.iter().position(|instance| instance.subproof_id == subproof_id && instance.air_id == air_id) {
            Ok(index)
        } else {
            Err("Air instance not found")
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
    pub fn add_trace_to_air_instance(&self, subproof_id: usize, air_id: usize, trace: Box<dyn Trace>) {
        if let Ok(index) = self.find_air_instance(subproof_id, air_id) {
            self.airs[index].add_trace(trace);
        } else {
            // TODO: Better error handling
            panic!("Could not find air instance with subproof_id {} and air_id {}", subproof_id, air_id);
        }
    }
}

/// Represents an instance of an Air within a proof.
#[allow(dead_code)]
pub struct AirContext {
    subproof_id: usize,
    air_id: usize,
    traces: RwLock<Vec<Box<dyn Trace>>>,
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
    pub fn add_trace(&self, trace: Box<dyn Trace>) {
        self.traces.write().unwrap().push(trace);
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
        type T = BaseElement;
        let proof_ctx = ProofCtx {
            pilout: PilOut::default(),
            public_inputs: None,
            challenges: vec![vec![T::default(); 0]],
            airs: vec![AirContext::new(0, 0)],
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

            proof_ctx.add_trace_to_air_instance(0, 0, simple);
        });

        write_handle.join().unwrap();
    }

    #[test]
    fn test_find_air_instance_success() {
        type T = BaseElement;
        let proof_ctx = ProofCtx {
            pilout: PilOut::default(),
            public_inputs: None,
            challenges: vec![vec![T::default(); 0]],
            airs: vec![
                AirContext::new(0, 0),
                AirContext::new(1, 1),
            ],
        };

        let result = proof_ctx.find_air_instance(1, 1);
        assert_eq!(result, Ok(1));
    }

    #[test]
    fn test_find_air_instance_not_found() {
        type T = BaseElement;
        let proof_ctx = ProofCtx {
            public_inputs: None,
            pilout: PilOut::default(),
            challenges: vec![vec![T::default(); 0]],
            airs: vec![
                AirContext::new(0, 0),
                AirContext::new(1, 1),
            ],
        };

        let result = proof_ctx.find_air_instance(2, 2);
        assert_eq!(result, Err("Air instance not found"));
    }
}