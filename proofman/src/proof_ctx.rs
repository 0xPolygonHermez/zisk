use std::{os::raw::c_void, sync::RwLock};
use pilout::pilout_proxy::PilOutProxy;

use std::sync::Arc;

use crate::trace::trace::Trace;
use std::fmt;

use log::debug;
use util::{timer_start, timer_stop_and_log};

/// Proof context for managing proofs, including information about airs and air instances.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProofCtx<T> {
    /// The Pilout associated with the proof context.
    pub pilout: PilOutProxy,
    /// The public inputs associated with the proof context.
    pub public_inputs: Vec<T>,
    /// The challenges associated with the proof context.
    challenges: Vec<Vec<T>>,
    /// The subproofs associated with the proof context.
    pub subproofs: Vec<SubproofCtx<T>>,
    /// The subproof values associated with the proof context.
    pub subproof_values: Vec<Vec<T>>,
    // NOTE: remove this ptr when vadcops ready, now it's used while developing
    pub proof: *mut c_void,
}

impl<T: Default + Clone> ProofCtx<T> {
    const MY_NAME: &'static str = "proofCtx";

    /// Creates a new `ProofCtx` given a `Pilout`.
    pub fn new(pilout: PilOutProxy) -> Self {
        timer_start!(CREATING_PROOF_CTX);
        debug!("{}: ··· Creating proof context", Self::MY_NAME);

        if pilout.subproofs.len() == 0 {
            panic!("No subproofs found in PilOut");
        }

        pilout.print_pilout_info();

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

        let proof_ctx = ProofCtx {
            pilout,
            public_inputs: Vec::new(),
            challenges,
            subproofs,
            subproof_values: Vec::new(),
            proof: std::ptr::null_mut(),
        };

        timer_stop_and_log!(CREATING_PROOF_CTX);

        proof_ctx
    }

    /// Initializes the proof context with optional public inputs
    pub fn initialize_proof<U: Into<Vec<T>>>(&mut self, public_inputs: Option<U>) {
        if let Some(public_inputs) = public_inputs {
            self.public_inputs = public_inputs.into();
        }

        for subproof in self.subproofs.iter_mut() {
            for air in subproof.airs.iter_mut() {
                air.instances.clear();
            }
        }

        self.proof = std::ptr::null_mut();
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
        &mut self,
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

/// Subproof context for managing subproofs, including information about airs and air instances.
#[derive(Debug)]
#[allow(dead_code)]
pub struct SubproofCtx<T> {
    pub subproof_id: usize,
    pub airs: Vec<AirCtx<T>>,
}

/// Air context for managing airs, including information about air instances.
#[allow(dead_code)]
pub struct AirCtx<T> {
    pub subproof_id: usize,
    pub air_id: usize,
    pub instances: Vec<AirInstanceCtx<T>>,
}

/// Air instance context for managing air instances (traces)
#[derive(Debug)]
pub struct AirInstanceCtx<T> {
    pub subproof_id: usize,
    pub air_id: usize,
    pub instance_id: usize,
    pub trace: RwLock<Arc<Box<dyn Trace>>>,
    pub subproof_values: Vec<T>,
}

impl<T> AirCtx<T> {
    /// Creates a new AirCtx.
    ///
    /// # Arguments
    ///
    /// * `subproof_id` - The subproof ID associated with the AirCtx.
    /// * `air_id` - The air ID associated with the AirCtx.
    pub fn new(subproof_id: usize, air_id: usize) -> Self {
        AirCtx { subproof_id, air_id, instances: Vec::new() }
    }

    /// Adds a trace to the AirCtx.
    ///
    /// # Arguments
    ///
    /// * `trace` - The trace to add to the AirCtx.
    pub fn add_trace(&mut self, trace: Box<dyn Trace>) -> usize {
        let len = self.instances.len();

        self.instances.push(AirInstanceCtx {
            subproof_id: self.subproof_id,
            air_id: self.air_id,
            instance_id: len,
            trace: RwLock::new(Arc::new(trace)),
            // TODO! Review this, has to be resized from the beginning?????
            subproof_values: Vec::new(),
        });
        self.instances.len() - 1
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
    pub fn get_trace(&self, instance_id: usize) -> Result<Arc<Box<dyn Trace>>, &'static str> {
        assert!(instance_id < self.instances.len(), "Trace ID out of bounds");

        Ok(Arc::clone(&self.instances[instance_id].trace.read().unwrap()))
    }
}

impl<T> fmt::Debug for AirCtx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AirCtx")
            .field("subproof_id", &self.subproof_id)
            .field("air_id", &self.air_id)
            .field("instances", &self.instances.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::sync::Arc;
    // use std::thread;
    use goldilocks::Goldilocks;
    use crate::trace;

    // Define a trait for types that support downcasting
    // Mock trace implementation for testing
    #[derive(Debug)]
    struct MockTrace;

    impl Trace for MockTrace {
        fn num_rows(&self) -> usize {
            0
        }

        fn row_size(&self) -> usize {
            0
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_add_trace_to_air_instance() {
        let mut proof_ctx = ProofCtx {
            pilout: PilOutProxy::default(),
            public_inputs: Vec::new(),
            challenges: vec![vec![Goldilocks::default(); 0]],
            subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
            subproof_values: Vec::new(),
            proof: std::ptr::null_mut(),
        };

        // Add a trace to the first Air instance of the first subproof
        let subproof_id = 0;
        let air_id = 0;
        let trace_id = proof_ctx.add_trace_to_air_instance(subproof_id, air_id, Box::new(MockTrace)).unwrap();

        // Check if the trace was added successfully
        assert_eq!(trace_id, 0);
    }

    #[test]
    fn test_get_trace() {
        let mut proof_ctx = ProofCtx {
            pilout: PilOutProxy::default(),
            public_inputs: Vec::new(),
            challenges: vec![vec![Goldilocks::default(); 0]],
            subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
            subproof_values: Vec::new(),
            proof: std::ptr::null_mut(),
        };

        // Add a trace to the first Air instance of the first subproof
        let subproof_id = 0;
        let air_id = 0;

        // Fille Simple trace with fake values
        trace!(Simple { field1: usize });
        let mut simple = Simple::new(16);
        for i in 0..16 {
            simple.field1[i] = i;
        }

        let mut simple2 = Simple::new(16);
        for i in 0..16 {
            simple2.field1[i] = i * 2;
        }

        let result = proof_ctx.add_trace_to_air_instance(subproof_id, air_id, Box::new(simple));
        assert!(result.is_ok());

        let result2 = proof_ctx.add_trace_to_air_instance(subproof_id, air_id, Box::new(simple2));
        assert!(result2.is_ok());

        let index = result.unwrap();
        let index2 = result2.unwrap();

        // Retrieve the added traces
        let trace_result = proof_ctx.get_trace(subproof_id, air_id, index);
        assert!(trace_result.is_ok());

        let trace_result2 = proof_ctx.get_trace(subproof_id, air_id, index2);
        assert!(trace_result2.is_ok());

        // Downcast the trait object to a concrete type
        let trace = trace_result.unwrap();
        let simple_p = trace.as_any().downcast_ref::<Simple>().unwrap();

        // Check if the retrieved trace is the same as the added trace
        for i in 0..16 {
            assert_eq!(simple_p.field1[i], i);
        }

        let trace2 = trace_result2.unwrap();
        let simple2_p = trace2.as_any().downcast_ref::<Simple>().unwrap();

        // Check if the retrieved trace is the same as the added trace
        for i in 0..16 {
            assert_eq!(simple2_p.field1[i], i * 2);
        }
    }

    // #[test]
    // fn test_concurrent_add_trace() {
    //     let mut proof_ctx = ProofCtx {
    //         pilout: PilOutProxy::default(),
    //         public_inputs: Vec::new(),
    //         challenges: vec![vec![Goldilocks::default(); 0]],
    //         subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
    //         proof: None,
    //     };

    //     // Number of threads for concurrent addition of traces
    //     let num_threads = 10;

    //     // Vector to store thread handles
    //     let mut handles = vec![];

    //     // Concurrently add traces to the first Air instance of the first subproof
    //     for _ in 0..num_threads {
    //         let proof_ctx_ref = &mut proof_ctx;
    //         let handle = thread::spawn(move || {
    //             proof_ctx_ref.add_trace_to_air_instance(0, 0, Box::new(MockTrace)).unwrap();
    //         });
    //         handles.push(handle);
    //     }

    //     // Wait for all threads to finish
    //     for handle in handles {
    //         handle.join().unwrap();
    //     }

    //     // Check if all traces were added successfully
    //     assert_eq!(proof_ctx.subproofs[0].airs[0].instances.len(), num_threads);
    // }
}
