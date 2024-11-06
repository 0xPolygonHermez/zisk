use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::{InputDataSM, MemAlignSM, MemSM};
use p3_field::PrimeField;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use zisk_core::{ZiskRequiredMemory, RAM_ADDR, SYS_ADDR};

use proofman::{WitnessComponent, WitnessManager};

pub enum MemOps {
    OneRead,
    OneWrite,
    TwoReads,
    TwoWrites,
}

pub struct MemProxy<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM>,
    input_data_sm: Arc<InputDataSM<F>>,
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_sm = MemSM::new(wcm.clone());
        let mem_align_sm = MemAlignSM::new(wcm.clone());
        let input_data_sm = InputDataSM::new(wcm.clone());

        let mem_proxy = Self {
            registered_predecessors: AtomicU32::new(0),
            mem_sm: mem_sm.clone(),
            mem_align_sm: mem_align_sm.clone(),
            input_data_sm: input_data_sm.clone(),
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
            self.mem_sm.unregister_predecessor();
            // self.mem_align_sm.unregister_predecessor::<F>();
        }
    }

    pub fn prove(
        &self,
        mut operations: [Vec<ZiskRequiredMemory>; 2],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut aligned = std::mem::take(&mut operations[0]);
        let non_aligned = std::mem::take(&mut operations[1]);
        let mut new_aligned = Vec::new();

        // Step 1. Sort the aligned memory accesses
        timer_start_debug!(MEM_SORT);
        aligned.sort_by_key(|mem| mem.address);
        timer_stop_and_log_debug!(MEM_SORT);

        // Step 2. For each non-aligned memory access
        non_aligned.iter().for_each(|unaligned_access| {
            let mem_ops = Self::get_mem_ops(unaligned_access);

            // Step 2.1 Find the possible aligned memory access
            let aligned_accesses = self.get_aligned_accesses(&unaligned_access, mem_ops, &aligned);

            // Step 2.2 Align memory access using mem_align state machine
            // self.mem_aligned_sm.align_mem_accesses(potential_aligned_mem, mem, &mut new_aligned);

            // Step 2.3 Store the new aligned memory access(es)
            new_aligned.extend(aligned_accesses);
        });

        // Step 3. Concatenate the new aligned memory accesses with the original aligned memory
        // accesses
        aligned.extend(new_aligned);

        timer_start_debug!(MEM_SORT_2);
        aligned.sort_by_key(|mem| mem.address);
        timer_stop_and_log_debug!(MEM_SORT_2);

        let mut idx = 0;
        while aligned[idx].address < RAM_ADDR && idx < aligned.len() {
            idx += 1;
        }
        let (_input_aligned, aligned) = aligned.split_at_mut(idx);

        // Step 4. Prove the aligned memory accesses using mem state machine
        self.mem_sm.prove(aligned);

        Ok(())
    }

    #[inline(always)]
    fn get_aligned_accesses(
        &self,
        unaligned_access: &ZiskRequiredMemory,
        mem_ops: MemOps,
        aligned_accesses: &[ZiskRequiredMemory],
    ) -> Vec<ZiskRequiredMemory> {
        // Align down to a 8 byte addres
        let addr = unaligned_access.address & !7;
        match mem_ops {
            MemOps::OneRead => {
                // Look for last write to the same address
                let last_write_addr =
                    Self::get_last_write(addr, unaligned_access.step, aligned_accesses);
                let last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: 0,
                });
                vec![last_write_addr]
            }
            MemOps::OneWrite => {
                // Look for last write to the same address
                let last_write_addr =
                    Self::get_last_write(addr, unaligned_access.step, aligned_accesses);

                // Modify the value of the write to the same address
                let mut last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: true,
                    address: addr,
                    width: 8,
                    value: 0,
                });

                Self::write_value(&unaligned_access, &mut last_write_addr);
                vec![last_write_addr]
            }
            MemOps::TwoReads => {
                // Look for last write to the same address and same address + 8
                let last_write_addr =
                    Self::get_last_write(addr, unaligned_access.step, aligned_accesses);
                let last_write_addr_p =
                    Self::get_last_write(addr + 8, unaligned_access.step, aligned_accesses);

                let last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: 0,
                });

                let last_write_addr_p = last_write_addr_p.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: false,
                    address: addr + 8,
                    width: 8,
                    value: 0,
                });

                vec![last_write_addr, last_write_addr_p]
            }
            MemOps::TwoWrites => {
                // Look for last write to the same address and same address + 8
                let last_write_addr =
                    Self::get_last_write(addr, unaligned_access.step, aligned_accesses);
                let last_write_addr_p =
                    Self::get_last_write(addr + 8, unaligned_access.step, aligned_accesses);

                let mut last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: true,
                    address: addr,
                    width: 8,
                    value: 1,
                });

                let mut last_write_addr_p = last_write_addr_p.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: true,
                    address: addr + 8,
                    width: 8,
                    value: 1,
                });

                Self::write_values(&unaligned_access, &mut last_write_addr, &mut last_write_addr_p);
                vec![last_write_addr, last_write_addr_p]
            }
        }
    }

    #[inline(always)]
    fn get_last_write(
        addr: u64,
        step: u64,
        aligned_accesses: &[ZiskRequiredMemory],
    ) -> Option<ZiskRequiredMemory> {
        // Step 1: Find the start of the range for `addr`
        let start_index =
            match aligned_accesses.binary_search_by_key(&addr, |access| access.address) {
                Ok(mut index) => {
                    // Backtrack to find the first occurrence of `addr`
                    while index > 0 && aligned_accesses[index - 1].address == addr {
                        index -= 1;
                    }
                    index
                }
                Err(index) => index, // If no match, use the insertion point as before
            };

        // Step 2: Iterate from start_index forward, storing the last valid write
        let mut last_write = None;
        for access in &aligned_accesses[start_index..] {
            if access.address != addr {
                break; // Stop if we move past the given address
            }
            if access.step >= step {
                break; // Stop if step is not less than the given step
            }
            if access.is_write {
                last_write = Some(access.clone()); // Update last write if conditions are met
            }
        }

        last_write
    }

    #[inline(always)]
    fn write_value(unaligned: &ZiskRequiredMemory, aligned: &mut ZiskRequiredMemory) {
        let offset = 8 - (unaligned.address & 7);
        let width_in_bits = unaligned.width * 8;

        let mask = !(((1u64 << width_in_bits) - 1) << ((offset - unaligned.width) * 8));

        aligned.value =
            (aligned.value & mask) | (unaligned.value << ((offset - unaligned.width) * 8));
    }

    #[inline(always)]
    fn write_values(
        unaligned: &ZiskRequiredMemory,
        aligned: &mut ZiskRequiredMemory,
        aligned_next: &mut ZiskRequiredMemory,
    ) {
        let offset = unaligned.address & 7;
        let bytes_to_write = 8 - offset;
        let right_bits = (unaligned.width - bytes_to_write) * 8;

        // Left write
        let left_value = unaligned.value >> right_bits;
        let left_memory =
            ZiskRequiredMemory { width: bytes_to_write, value: left_value, ..*unaligned };
        Self::write_value(&left_memory, aligned);

        // Right write
        let mask = (1u64 << right_bits) - 1;
        let right_value = unaligned.value & mask;

        let right_memory = ZiskRequiredMemory {
            address: 0,
            width: unaligned.width - bytes_to_write,
            value: right_value,
            ..*unaligned
        };
        Self::write_value(&right_memory, aligned_next);
    }

    #[inline(always)]
    pub fn get_mem_ops(input: &ZiskRequiredMemory) -> MemOps {
        let addr = input.address;
        let width = input.width;
        let offset = addr & 7;
        match (input.is_write, offset + width > 8) {
            (false, false) => MemOps::OneRead,
            (true, false) => MemOps::OneWrite,
            (false, true) => MemOps::TwoReads,
            (true, true) => MemOps::TwoWrites,
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemProxy<F> {}
