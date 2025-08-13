use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use zisk_common::{
    create_atomic_vec, BusDeviceMetrics, ComponentBuilder, Instance, InstanceCtx, Planner,
};
use zisk_pil::FrequentOpsTrace;

use crate::{FrequentOpsCounter, FrequentOpsInstance, FrequentOpsPlanner};

pub struct FrequentOpsSM<F: PrimeField64> {
    /// The multiplicity table, shared across threads.
    _phantom: std::marker::PhantomData<F>,
    multiplicities: Vec<Vec<AtomicU64>>,
    calculated: AtomicBool,
}

impl<F: PrimeField64> FrequentOpsSM<F> {
    const MY_NAME: &'static str = "FrOpsSM ";

    /// Creates a new instance of the `FrequentOps` state machine.
    ///
    /// # Returns
    pub fn new(_std: Arc<Std<F>>) -> Arc<Self> {
        let mut multiplicities = Vec::new();
        for _ in 0..FrequentOpsTrace::<usize>::ROW_SIZE {
            multiplicities.push(create_atomic_vec(FrequentOpsTrace::<usize>::NUM_ROWS));
        }
        Arc::new(Self {
            multiplicities,
            calculated: AtomicBool::new(false),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values representing the input data.
    pub fn update_input(&self, index: usize, value: u64) {
        if self.calculated.load(Ordering::Relaxed) {
            return;
        }
        self.multiplicities[0][index].fetch_add(value, Ordering::Relaxed);
    }

    /// Detaches and returns the current multiplicity table.
    ///
    /// # Returns
    /// A vector containing the multiplicity table.
    pub fn detach_multiplicities(&self) -> &[Vec<AtomicU64>] {
        &self.multiplicities
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::Relaxed);
    }

    /// Computes the witness for frequent operations.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed frequent operations trace data.
    pub fn compute_witness() -> AirInstance<F> {
        let mut frequent_ops_trace = FrequentOpsTrace::new_zeroes();
        let trace_len = FrequentOpsTrace::<F>::NUM_ROWS as u64;
        tracing::info!(
            "{}: ··· Creating FrequentOps instance [{} rows filled]",
            Self::MY_NAME,
            trace_len,
        );
        // For every frequent operation, fill its corresponding trace entry
        //for (i, inst_builder) in frequent_ops.insts.clone().into_iter().enumerate() {
        /*
        for (i, key) in frequent_ops.insts.keys().sorted().enumerate() {
            // Get the frequent operation entry
            let inst = &frequent_ops.insts[key].i;

            // Calculate the multiplicity, i.e. the number of times this operation is used in this
            // execution
            let mut multiplicity: u64;
            if metadata.frequent_ops.inst_count.is_empty() {
                multiplicity = 1; // If the histogram is empty, we use 1 for all operations
            } else {
                let counter = metadata.frequent_ops.inst_count.get(&inst.paddr);
                if counter.is_some() {
                    multiplicity = *counter.unwrap();
                    if inst.paddr == metadata.frequent_ops.end_pc {
                        multiplicity += main_trace_len - metadata.frequent_ops.steps % main_trace_len;
                    }
                } else {
                    continue; // We skip those operations that are not used in this execution
                }
            }
            frequent_ops_trace[i].multiplicity = F::from_u64(multiplicity);
        }
        */
        AirInstance::new_from_trace(FromTrace::new(&mut frequent_ops_trace))
    }

    pub fn build_frequent_ops_counter(&self) -> FrequentOpsCounter {
        FrequentOpsCounter::new()
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for FrequentOpsSM<F> {
    /// Builds and returns a new counter for monitoring frequent operations.
    ///
    /// # Returns
    /// A boxed implementation of `FrequentOpsCounter`.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(FrequentOpsCounter::new()))
    }

    /// Builds a planner for frequent operations related instances.
    ///
    /// # Returns
    /// A boxed implementation of `FrequentOpsPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(FrequentOpsPlanner {})
    }

    /// Builds an instance of the FrequentOps state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated data.
    ///
    /// # Returns
    /// A boxed implementation of `FrequentOpsInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            FrequentOpsTrace::<usize>::AIR_ID => Box::new(FrequentOpsInstance::new(ictx)),
            _ => panic!("FrequentOpsSM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
