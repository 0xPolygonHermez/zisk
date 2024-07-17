use log::info;

use crate::{Prover, WCPilout};

#[allow(dead_code)]
pub struct ProofCtx<F> {
    pub public_inputs: Vec<u8>,
    pub pilout: WCPilout,
    pub challenges: Vec<Vec<F>>,
    pub air_instances: Vec<AirInstanceCtx>,
    pub provers: Vec<Box<dyn Prover<F>>>,
}

impl<F> ProofCtx<F> {
    const MY_NAME: &'static str = "ProofCtx";

    pub fn create_ctx(pilout: WCPilout, public_inputs: Vec<u8>) -> Self {
        info!("{}: ··· Creating proof context", Self::MY_NAME);

        if pilout.air_groups().len() == 0 {
            panic!("No subproofs found in PilOut");
        }

        // pilout.print_pilout_info();

        // NOTE: consider Vec::with_capacity() instead of Vec::new()
        let challenges = Vec::<Vec<F>>::new();

        // TODO! Review this
        // if !pilout.num_challenges.is_empty() {
        //     for i in 0..pilout.num_challenges.len() {
        //         challenges.push(vec![T::default(); pilout.num_challenges[i] as usize]);
        //     }
        // } else {
        //     challenges.push(vec![]);
        // }

        // qStage, evalsStage and friStage
        // challenges.push(vec![F::default(); 1]);
        // challenges.push(vec![F::default(); 1]);
        // challenges.push(vec![F::default(); 2]);

        Self { public_inputs, pilout, challenges, air_instances: Vec::new(), provers: Vec::new() }
    }

    pub fn find_air_instances(&self, air_group_id: usize, air_id: usize) -> Vec<&AirInstanceCtx> {
        self.air_instances
            .iter()
            .filter(|air_instance| air_instance.air_group_id == air_group_id && air_instance.air_id == air_id)
            .collect()
    }
}

/// Air instance context for managing air instances (traces)
#[derive(Debug)]
#[allow(dead_code)]
pub struct AirInstanceCtx {
    pub air_group_id: usize,
    pub air_id: usize,
    pub buffer: Vec<u8>,
}

impl AirInstanceCtx {
    pub fn new(air_group_id: usize, air_id: usize) -> Self {
        AirInstanceCtx { air_group_id, air_id, buffer: Vec::new() }
    }

    pub fn get_buffer_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr() as *mut u8
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     // use std::sync::Arc;
//     // use std::thread;
//     use goldilocks::Goldilocks;
//     use crate::trace;

//     // Define a trait for types that support downcasting
//     // Mock trace implementation for testing
//     #[derive(Debug)]
//     struct MockTrace;

//     impl Trace for MockTrace {
//         fn num_rows(&self) -> usize {
//             0
//         }

//         fn row_size(&self) -> usize {
//             0
//         }

//         fn as_any(&self) -> &dyn std::any::Any {
//             self
//         }
//     }

//     #[test]
//     fn test_add_trace_to_air_instance() {
//         let mut proof_ctx = ProofCtx {
//             pilout: PilOutProxy::default(),
//             public_inputs: Vec::new(),
//             challenges: vec![vec![Goldilocks::default(); 0]],
//             subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
//             subproof_values: Vec::new(),
//             proof: std::ptr::null_mut(),
//         };

//         // Add a trace to the first Air instance of the first subproof
//         let subproof_id = 0;
//         let air_id = 0;
//         let buffer: Vec<u8> = vec![0; 16];
//         let trace_id = proof_ctx.add_instance(subproof_id, air_id, buffer, MockTrace).unwrap();

//         // Check if the trace was added successfully
//         assert_eq!(trace_id, 0);
//     }

//     #[test]
//     fn test_get_trace() {
//         let mut proof_ctx = ProofCtx {
//             pilout: PilOutProxy::default(),
//             public_inputs: Vec::new(),
//             challenges: vec![vec![Goldilocks::default(); 0]],
//             subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
//             subproof_values: Vec::new(),
//             proof: std::ptr::null_mut(),
//         };

//         // Add a trace to the first Air instance of the first subproof
//         let subproof_id = 0;
//         let air_id = 0;

//         // Fille Simple trace with fake values
//         trace!(Simple { field1: usize });
//         let mut simple = Simple::new(16);
//         for i in 0..16 {
//             simple.field1[i] = i;
//         }

//         let mut simple2 = Simple::new(16);
//         for i in 0..16 {
//             simple2.field1[i] = i * 2;
//         }

//         let buffer: Vec<u8> = vec![0; 16];
//         let buffer2: Vec<u8> = vec![0; 16];

//         let result = proof_ctx.add_instance(subproof_id, air_id, buffer, simple);
//         assert!(result.is_ok());

//         let result2 = proof_ctx.add_instance(subproof_id, air_id, buffer2, simple2);
//         assert!(result2.is_ok());

//         let index = result.unwrap();
//         let index2 = result2.unwrap();

//         // Retrieve the added traces
//         let trace_result = proof_ctx.get_trace(subproof_id, air_id, index);
//         assert!(trace_result.is_ok());

//         let trace_result2 = proof_ctx.get_trace(subproof_id, air_id, index2);
//         assert!(trace_result2.is_ok());

//         // Downcast the trait object to a concrete type
//         let trace = trace_result.unwrap();
//         let simple_p = trace.as_any().downcast_ref::<Simple>().unwrap();

//         // Check if the retrieved trace is the same as the added trace
//         for i in 0..16 {
//             assert_eq!(simple_p.field1[i], i);
//         }

//         let trace2 = trace_result2.unwrap();
//         let simple2_p = trace2.as_any().downcast_ref::<Simple>().unwrap();

//         // Check if the retrieved trace is the same as the added trace
//         for i in 0..16 {
//             assert_eq!(simple2_p.field1[i], i * 2);
//         }
//     }

//     // #[test]
//     // fn test_concurrent_add_trace() {
//     //     let mut proof_ctx = ProofCtx {
//     //         pilout: PilOutProxy::default(),
//     //         public_inputs: Vec::new(),
//     //         challenges: vec![vec![Goldilocks::default(); 0]],
//     //         subproofs: vec![SubproofCtx { subproof_id: 0, airs: vec![AirCtx::new(0, 0)] }],
//     //         proof: None,
//     //     };

//     //     // Number of threads for concurrent addition of traces
//     //     let num_threads = 10;

//     //     // Vector to store thread handles
//     //     let mut handles = vec![];

//     //     // Concurrently add traces to the first Air instance of the first subproof
//     //     for _ in 0..num_threads {
//     //         let proof_ctx_ref = &mut proof_ctx;
//     //         let handle = thread::spawn(move || {
//     //             proof_ctx_ref.add_trace_to_air_instance(0, 0, Box::new(MockTrace)).unwrap();
//     //         });
//     //         handles.push(handle);
//     //     }

//     //     // Wait for all threads to finish
//     //     for handle in handles {
//     //         handle.join().unwrap();
//     //     }

//     //     // Check if all traces were added successfully
//     //     assert_eq!(proof_ctx.subproofs[0].airs[0].instances.len(), num_threads);
//     // }
// }
