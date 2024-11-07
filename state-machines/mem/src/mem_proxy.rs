use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::{MemAlignSM, MemSM};
use p3_field::PrimeField;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use zisk_core::{ZiskRequiredMemory, RAM_ADDR, SYS_ADDR};

use proofman::{WitnessComponent, WitnessManager};

#[derive(Debug, Clone)]
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
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_sm = MemSM::new(wcm.clone());
        let mem_align_sm = MemAlignSM::new(wcm.clone());

        let mem_proxy = Self {
            registered_predecessors: AtomicU32::new(0),
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
            self.mem_sm.unregister_predecessor();
            // self.mem_align_sm.unregister_predecessor::<F>();
        }
    }

    pub fn prove(
        &self,
        mut operations: &mut [Vec<ZiskRequiredMemory>; 2],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut aligned = std::mem::take(&mut operations[0]);
        let unaligned = std::mem::take(&mut operations[1]);
        let mut new_aligned = Vec::new();

        //trace[63927]: MemRow { addr: 2685533720, step: 5145, sel: 1, wr: 0, value: [2685534552, 0], addr_changes: 0, same_value: 0, first_addr_access_is_read: 0 }
        println!("-----------------");
        println!("-- Aligned inputs:");
        for i in 0..aligned.len() {
            if aligned[i].address == 2685534096 {
                println!("aligned[{}]: {:?} value: {:x}", i, aligned[i], aligned[i].value);
            }
        }
        println!("-- Unaligned inputs:");
        for i in 0..unaligned.len() {
            if unaligned[i].address >= (2685534096 - 8) && unaligned[i].address <= (2685534096 + 8)
            {
                println!("unaligned[{}]: {:?} value: {:x}", i, unaligned[i], unaligned[i].value);
            }
        }
        println!("-----------------");

        // Step 1. Sort the aligned memory accesses
        timer_start_debug!(MEM_SORT);
        aligned.sort_by_key(|mem| mem.address);
        timer_stop_and_log_debug!(MEM_SORT);

        // Step 2. For each unaligned memory access
        unaligned.iter().for_each(|unaligned_access| {
            let mem_ops = Self::get_mem_ops(unaligned_access);

            // Step 2.1 Find the possible aligned memory access
            // TODO! Remove mem_ops.clone()
            let aligned_accesses = self.get_aligned_accesses(
                &unaligned_access,
                mem_ops.clone(),
                &aligned,
                &new_aligned,
            );

            // Step 2.2 Align memory access using mem_align state machine
            // self.mem_align_sm.prove(&aligned_accesses, unaligned_access);

            for access in new_aligned.iter() {
                if access.step == 4682 {
                    println!("new_aligned: {:?}", access);
                }
            }

            // Step 2.3 Store the new aligned memory access(es)
            if unaligned_access.step == 5145 {
                println!("*** mem_ops: {:?}", mem_ops);
                println!("*** unaligned_access: {:?}", unaligned_access);
                println!("*** aligned_accesses: {:?}", aligned_accesses);
            }

            new_aligned.extend(aligned_accesses);
            new_aligned.sort_by_key(|mem| mem.address);
        });

        // Step 3. Concatenate the new aligned memory accesses with the original aligned memory
        // accesses
        aligned.extend(new_aligned);

        timer_start_debug!(MEM_SORT_2);
        aligned.sort_by_key(|mem| (mem.address, mem.step));
        timer_stop_and_log_debug!(MEM_SORT_2);

        let mut idx = 0;
        while aligned[idx].address < RAM_ADDR && idx < aligned.len() {
            idx += 1;
        }

        println!("Aligned len(): {:?}", aligned.len());

        let (_input_aligned, aligned) = aligned.split_at_mut(idx);

        // Filter where address = 2684391184
        println!("");
        for i in 0..aligned.len() {
            if aligned[i].address == 2685534096 {
                println!("OJO!!!! mem: {:?}", aligned[i]);
            }
        }

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
        new_aligned_accesses: &[ZiskRequiredMemory],
    ) -> Vec<ZiskRequiredMemory> {
        // Align down to a 8 byte addres
        let addr = unaligned_access.address & !7;
        match mem_ops {
            MemOps::OneRead => {
                // Look for last write to the same address
                let last_write_addr = Self::get_last_write(
                    addr,
                    unaligned_access.step,
                    aligned_accesses,
                    Some(new_aligned_accesses),
                );
                let mut last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: 0,
                });

                last_write_addr.step = unaligned_access.step;

                vec![last_write_addr]
            }
            MemOps::OneWrite => {
                // Look for last write to the same address
                let last_write_addr = Self::get_last_write(
                    addr,
                    unaligned_access.step,
                    aligned_accesses,
                    Some(new_aligned_accesses),
                );

                // Modify the value of the write to the same address
                let last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: true,
                    address: addr,
                    width: 8,
                    value: 0,
                });

                let mut last_write_addr_r = last_write_addr.clone();
                last_write_addr_r.step = unaligned_access.step;
                last_write_addr_r.is_write = false;

                let mut last_write_addr_w = last_write_addr;
                last_write_addr_w.step = unaligned_access.step;
                Self::write_value(&unaligned_access, &mut last_write_addr_w);

                vec![last_write_addr_r, last_write_addr_w]
            }
            MemOps::TwoReads => {
                // Look for last write to the same address and same address + 8
                let last_write_addr = Self::get_last_write(
                    addr,
                    unaligned_access.step,
                    aligned_accesses,
                    Some(new_aligned_accesses),
                );
                let last_write_addr_p = Self::get_last_write(
                    addr + 8,
                    unaligned_access.step,
                    aligned_accesses,
                    Some(new_aligned_accesses),
                );

                let mut last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: 0,
                });

                let mut last_write_addr_p = last_write_addr_p.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: false,
                    address: addr + 8,
                    width: 8,
                    value: 0,
                });

                last_write_addr.step = unaligned_access.step;
                last_write_addr_p.step = unaligned_access.step;

                vec![last_write_addr, last_write_addr_p]
            }
            MemOps::TwoWrites => {
                // Look for last write to the same address and same address + 8
                let last_write_addr = Self::get_last_write(
                    addr,
                    unaligned_access.step,
                    aligned_accesses,
                    Some(new_aligned_accesses),
                );
                let last_write_addr_p = Self::get_last_write(
                    addr + 8,
                    unaligned_access.step,
                    aligned_accesses,
                    Some(new_aligned_accesses),
                );

                // Modify the value of the write to the same address
                let last_write_addr = last_write_addr.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: true,
                    address: addr,
                    width: 8,
                    value: 0,
                });

                let mut last_write_addr_r = last_write_addr.clone();
                last_write_addr_r.step = unaligned_access.step;
                last_write_addr_r.is_write = false;

                let mut last_write_addr_w = last_write_addr;
                last_write_addr_w.step = unaligned_access.step;
                Self::write_value(&unaligned_access, &mut last_write_addr_w);

                let last_write_addr_p = last_write_addr_p.unwrap_or(ZiskRequiredMemory {
                    step: unaligned_access.step,
                    is_write: true,
                    address: addr + 8,
                    width: 8,
                    value: 0,
                });

                let mut last_write_addr_p_r = last_write_addr_p.clone();
                last_write_addr_p_r.step = unaligned_access.step;
                last_write_addr_p_r.is_write = false;

                let mut last_write_addr_p_w = last_write_addr_p;
                last_write_addr_p_w.step = unaligned_access.step;
                Self::write_value(&unaligned_access, &mut last_write_addr_p_w);

                Self::write_values(
                    &unaligned_access,
                    &mut last_write_addr_w,
                    &mut last_write_addr_p_w,
                );
                vec![last_write_addr_r, last_write_addr_w, last_write_addr_p_r, last_write_addr_p_w]
            }
        }
    }

    #[inline(always)]
    fn get_last_write(
        addr: u64,
        step: u64,
        aligned_accesses: &[ZiskRequiredMemory],
        new_aligned_accesses: Option<&[ZiskRequiredMemory]>,
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

        // Step 3: If `new_aligned_accesses` exists, check for a more recent write
        if let None = new_aligned_accesses {
            return last_write;
        }

        let new_aligned_accesses = new_aligned_accesses.unwrap();
        let last_new_write = Self::get_last_write(addr, step, new_aligned_accesses, None);

        if let None = last_write {
            return last_new_write;
        }

        if let Some(last_new_write) = last_new_write {
            if last_new_write.step > last_write.as_ref().unwrap().step {
                return Some(last_new_write);
            }
        }

        last_write
    }

    #[inline(always)]
    fn write_value(unaligned: &ZiskRequiredMemory, aligned: &mut ZiskRequiredMemory) {
        let offset = unaligned.address & 7;
        let width_in_bits = unaligned.width * 8;

        let mask = !(((1u64 << width_in_bits) - 1) << (offset * 8));

        aligned.value = (aligned.value & mask)
            | ((unaligned.value & ((1u64 << width_in_bits) - 1)) << (offset * 8));
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
