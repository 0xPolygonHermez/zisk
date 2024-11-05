use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{MemAlignSM, MemSM};
use p3_field::{Field, PrimeField};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{MemOp, MemUnalignedOp};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};

#[allow(dead_code)]
const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct MemProxy<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs_aligned: Mutex<Vec<MemOp>>,
    inputs_unaligned: Mutex<Vec<MemUnalignedOp>>,

    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM>,
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_sm = MemSM::new(wcm.clone());
        let mem_align_sm = MemAlignSM::new(wcm.clone());

        let mem_proxy = Self {
            registered_predecessors: AtomicU32::new(0),
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_sm: mem_sm.clone(),
            mem_align_sm: mem_align_sm.clone(),
        };
        let mem_proxy = Arc::new(mem_proxy);

        wcm.register_component(mem_proxy.clone(), None, None);

        // For all the secondary state machines, register the main state machine as a predecessor
        mem_sm.register_predecessor();
        mem_align_sm.register_predecessor();

        mem_proxy
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            // self.mem_sm.unregister_predecessor();
            // self.mem_align_sm.unregister_predecessor::<F>();
        }
    }

    pub fn prove(
        &self,
        mut operations: [Vec<ZiskRequiredMemory>; 2],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut aligned = std::mem::take(&mut operations[0]);
        let non_aligned = std::mem::take(&mut operations[1]);
        let new_aligned = Vec::new();

        // Step 1. Sort the aligned memory accesses
        timer_start_debug!(MEM_SORT);
        aligned.sort_by_key(|mem| mem.address);
        timer_stop_and_log_debug!(MEM_SORT);

        // Step 2. For each non-aligned memory access
        non_aligned.iter().for_each(|mem| {
            // Step 2.1 Find the possible aligned memory access
            let potential_aligned_mem = self.get_potential_aligned_mem(&aligned, &mem);

            // Step 2.2 Align memory access using mem_align state machine
            // self.mem_aligned_sm.align_mem_accesses(potential_aligned_mem, mem, &mut new_aligned);

            // Step 2.3 Store the new aligned memory access(es)
        });

        // Step 3. Concatenate the new aligned memory accesses with the original aligned memory accesses
        aligned.extend(new_aligned);

        // Step 4. Sort the (full) aligned memory accesses
        timer_start_debug!(MEM_SORT_2);
        aligned.sort_by_key(|mem| mem.address);
        timer_stop_and_log_debug!(MEM_SORT_2);

        // Step 5. Prove the aligned memory accesses using mem state machine

        println!("Proving MemSM");
        println!("Aligned: {:?}", operations[0].len());
        println!("Non aligned: {:?}", operations[1].len());
        Ok(())
    }

    fn get_potential_aligned_mem(
        &self,
        aligned_accesses: &[ZiskRequiredMemory],
        unaligned_access: &ZiskRequiredMemory,
    ) -> Vec<ZiskRequiredMemory> {
        let mut aligned_mem = Vec::new();
        aligned_mem
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemProxy<F> {}
