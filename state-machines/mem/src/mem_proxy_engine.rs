//! The `MemProxyEngine` module is designed to facilitate dividing the proxy logic into smaller,
//! more manageable pieces of code.
//!
//! The engine is created through MemProxy on a static call, which creates the `MemProxyEngine`.
//! `MemProxyEngine` has state, and this state allows the implementation of smaller, focused
//! methods, making the codebase easier to maintain and extend.
//!
//!
//! ## Creation and Setup of the `MemProxyEngine`
//!
//! When creating the `MemProxyEngine`, a state machine is provided to handle alignment of memory
//! accesses. This state machine is responsible for demostrate unaligned accesses based on aligned
//! ones.
//!
//! Once the `MemProxyEngine` is created, all memory modules are registered. These modules must
//! implement the `MemModule` trait, which serves three purposes:
//!
//! 1. To define the range of addresses (regions) they are responsible for handling.
//! 2. To specify the frequency (number of inputs) at which they expect to receive inputs.
//! 3. To define the "callback" used to send inputs to the module
//!
//!
//! ## Inputs from `MemProxyEngine`
//!
//! The inputs to the `MemProxyEngine` are represented as an enumeration to optimize memory usage
//! and performance. This design ensures efficient handling of both common and rare cases,
//! balancing memory allocation and computational efficiency.
//!
//! The enumeration has two variants:
//!  1. `Basic`: The primary input type, used for the majority of memory accesses. This variant is
//!     highly optimized to minimize overhead and ensure efficient processing in typical scenarios.
//!  2. `Extended`: A specialized input type used exclusively for handling unaligned memory
//!     accesses. This variant is appended to the vector immediately after the corresponding `Basic`
//!     instance that generates it. The `Extended` input contains the aligned memory values required
//!     to process the unaligned access (in word case two values)
//!
//! By adopting this design, the `MemProxyEngine` avoids penalizing the commonly used `Basic` type
//! due to the less frequent unaligned cases that requires addicional `Extended` type. This
//! separation ensures that unaligned access handling introduces minimal overhead to the overall
//! system, while still providing the flexibility to unaligned access.
//!
//!
//! ## Logic of the `MemProxyEngine`
//!
//! Step 1. Sort the aligned memory accesses
//! original vector is sorted by step, sort_by_key is stable, no reordering of elements with
//! the same key.
//!
//! Step 2. Add a final mark mem_op to force flush of open_mem_align_ops, because always the
//! last operation is mem_op.
//!
//! Step 3. Composing information for memory operation (access). In this step, all necessary
//! information is gathered and composed to perform a memory operation. The process involves
//! reading the next input from the input vector, which defines the nature of the operation.
//!
//!  - For standard (aligned) operations, only the `Basic` input is required, and the operation
//!    proceeds directly.
//!  - For unaligned operations, the `Extended` input is also read. This additional input provides
//!    the extra values required to handle the unaligned operation.
//!
//! Step 4. Process each memory operation ordered by address and step. When a non-aligned
//! memory access there are two possible situations:
//!
//!  1. The operation applies only applies to one memory address (read or read+write). In this case
//!     mem_align helper return the aligned operation for this address, and loop continues.
//!
//!  2. The operation applies to two consecutive memory addresses, mem_align helper returns the
//!     aligned operation involved for the current address, and the second part of the operation is
//!     enqueued to open_mem_align_ops, it will processed when processing next address.
//!
//! First, we verify if there are any "previous" open memory alignment operations
//! (`open_mem_align_ops`) that need to be processed before handling the current `mem_op`. If such
//! operations exist, they are processed first, and then the current `mem_op` is executed.
//!
//! At the end of Step 2, a final marker is used to ensure a forced flush of any remaining
//! `open_mem_align_ops`. This guarantees that all pending alignment operations are completed,
//! as the last operation in this step is always a `mem_op`.
//!
//!
//! ## Handling Large Gaps Between Steps
//!
//! One challenge in the design is addressing cases where the distance between steps becomes
//! more large than max range check MEMORY_MAX_DIFF (current 2^24). This solve this situation
//! the proxy add extra intermediate internal reads (internal because don't send to bus), each
//! increase step in MEMORY_MAX_DIFF to arrive to the final step.

use std::{collections::VecDeque, sync::Arc};

use crate::{
    MemAlignInput, MemAlignResponse, MemAlignSM, MemHelpers, MemInput, MemModule, MemUnmapped,
    MAX_MAIN_STEP, MAX_MEM_ADDR, MAX_MEM_OPS_BY_MAIN_STEP, MAX_MEM_STEP, MAX_MEM_STEP_OFFSET,
    MEMORY_MAX_DIFF, MEM_ADDR_MASK, MEM_BYTES, MEM_BYTES_BITS,
};

use p3_field::PrimeField;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use zisk_core::ZiskRequiredMemory;

#[cfg(feature = "debug_mem_proxy_engine")]
const DEBUG_ADDR: u32 = 0x90000008;

macro_rules! debug_info {
    ($prefix:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_mem_proxy_engine")]
        {
            info!(concat!("MemProxy: ",$prefix), $($arg)*);
        }
    };
}

struct MemModuleData {
    pub name: String,
    pub inputs: Vec<MemInput>,
    pub flush_input_size: usize,
}

#[derive(Debug)]
pub struct AddressRegion {
    from_addr: u32,
    to_addr: u32,
    module_id: u8,
}
pub struct MemProxyEngine<F: PrimeField> {
    modules: Vec<Arc<dyn MemModule<F>>>,
    modules_data: Vec<MemModuleData>,
    open_mem_align_ops: VecDeque<MemAlignInput>,
    addr_map: Vec<AddressRegion>,
    addr_map_fetched: bool,
    current_module_id: usize,
    current_module: String,
    module_end_addr: u32,
    mem_align_sm: Arc<MemAlignSM<F>>,
    next_open_addr: u32,
    next_open_step: u64,
    last_value: u64,
    last_addr: u32,
    last_step: u64,
    intermediate_cases: u32,
    intermediate_steps: u32,
}

const NO_OPEN_ADDR: u32 = 0xFFFF_FFFF;
const NO_OPEN_STEP: u64 = 0xFFFF_FFFF_FFFF_FFFF;

impl<F: PrimeField> MemProxyEngine<F> {
    pub fn new(mem_align_sm: Arc<MemAlignSM<F>>) -> Self {
        Self {
            modules: Vec::new(),
            modules_data: Vec::new(),
            current_module_id: 0,
            current_module: String::new(),
            module_end_addr: 0,
            open_mem_align_ops: VecDeque::new(),
            addr_map: Vec::new(),
            addr_map_fetched: false,
            mem_align_sm,
            next_open_addr: NO_OPEN_ADDR,
            next_open_step: NO_OPEN_STEP,
            last_value: 0,
            last_addr: 0xFFFF_FFFF,
            last_step: 0,
            intermediate_cases: 0,
            intermediate_steps: 0,
        }
    }

    pub fn add_module(&mut self, name: &str, module: Arc<dyn MemModule<F>>) {
        if self.modules.is_empty() {
            self.current_module = String::from(name);
        }
        let module_id = self.modules.len() as u8;
        self.modules.push(module.clone());

        let ranges = module.get_addr_ranges();
        let flush_input_size = module.get_flush_input_size();

        for range in ranges.iter() {
            debug_info!("adding range 0x{:X} 0x{:X} to {}", range.0, range.1, name);
            self.insert_address_range(range.0, range.1, module_id);
        }
        self.modules_data.push(MemModuleData {
            name: String::from(name),
            inputs: Vec::new(),
            flush_input_size: if flush_input_size == 0 {
                0xFFFF_FFFF_FFFF_FFFF
            } else {
                flush_input_size as usize
            },
        });
    }
    /* insert in sort way the address map and verify that */
    fn insert_address_range(&mut self, from_addr: u32, to_addr: u32, module_id: u8) {
        let region = AddressRegion { from_addr, to_addr, module_id };
        if let Some(index) = self.addr_map.iter().position(|x| x.from_addr >= from_addr) {
            self.addr_map.insert(index, region);
        } else {
            self.addr_map.push(region);
        }
    }

    pub fn prove(
        &mut self,
        mem_operations: &mut Vec<ZiskRequiredMemory>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.init_prove();

        // Sort the aligned memory accesses
        // original vector is sorted by step, sort_by_key is stable, no reordering of elements with
        // the same key.

        timer_start_debug!(MEM_SORT);
        mem_operations.sort_by_key(|mem| (mem.get_address() & 0xFFFF_FFF8));
        timer_stop_and_log_debug!(MEM_SORT);

        // Add a final mark mem_op to force flush of open_mem_align_ops, because always the
        // last operation is mem_op.

        self.push_end_of_memory_mark(mem_operations);

        let mut index = 0;
        let count = mem_operations.len();
        while index < count {
            if let ZiskRequiredMemory::Basic {
                step,
                value,
                address,
                is_write,
                width,
                step_offset,
            } = mem_operations[index]
            {
                let extend_values = if !Self::is_aligned(address, width) {
                    debug_assert!(index + 1 < count, "expected one element more extended !!");
                    if let ZiskRequiredMemory::Extended { address: _, values } =
                        mem_operations[index + 1]
                    {
                        index += 1;
                        values
                    } else {
                        panic!("MemProxy::prove() unexpected Basic variant");
                    }
                } else {
                    [0, 0]
                };
                index += 1;
                if !self.prove_one(
                    address,
                    MemHelpers::main_step_to_address_step(step, step_offset),
                    value,
                    is_write,
                    width,
                    extend_values,
                ) {
                    break;
                }
            } else {
                panic!("MemProxy::prove() unexpected Extended variant");
            }
        }
        self.finish_prove();
        Ok(())
    }

    fn prove_one(
        &mut self,
        addr: u32,
        mem_step: u64,
        value: u64,
        is_write: bool,
        width: u8,
        extend_values: [u64; 2],
    ) -> bool {
        let is_aligned: bool = Self::is_aligned(addr, width);
        let aligned_mem_addr = Self::to_aligned_addr(addr);

        // Check if there are open mem align operations to be processed in this moment,
        // with address (or step) less than the aligned of current
        // mem_op.
        self.process_all_previous_open_mem_align_ops(aligned_mem_addr, mem_step);

        // check if we are at end of loop
        if self.check_if_end_of_memory_mark(addr, mem_step) {
            return false;
        }

        // all open mem align operations are processed, check if new mem operation is
        // aligned
        if !is_aligned {
            // In this point found non-aligned memory access, phase-0
            let mem_align_input = MemAlignInput {
                addr,
                value,
                width,
                mem_values: extend_values,
                is_write,
                step: mem_step,
            };
            let mem_align_response = self.mem_align_sm.get_mem_op(&mem_align_input, 0);

            #[cfg(feature = "debug_mem_proxy_engine")]
            Self::debug_mem_align_api(&mem_align_input, &mem_align_response, 0);

            // if operation applies to two consecutive memory addresses, add the second
            // part is enqueued to be processed in future when
            // processing next address on phase-1
            self.push_mem_align_response_ops(
                aligned_mem_addr,
                extend_values[0],
                &mem_align_input,
                &mem_align_response,
            );
            if mem_align_response.more_addr {
                self.open_mem_align_ops.push_back(mem_align_input);
                self.update_next_open_mem_align();
            }
        } else {
            self.push_aligned_op(is_write, addr, value, mem_step);
        }
        true
    }

    fn update_next_open_mem_align(&mut self) {
        if self.open_mem_align_ops.is_empty() {
            self.next_open_addr = NO_OPEN_ADDR;
            self.next_open_step = NO_OPEN_STEP;
        } else if self.open_mem_align_ops.len() == 1 {
            let mem_align_input = self.open_mem_align_ops.front().unwrap();
            self.next_open_addr = Self::next_aligned_addr(mem_align_input.addr);
            self.next_open_step = mem_align_input.step;
        }
    }

    fn process_all_previous_open_mem_align_ops(&mut self, mem_addr: u32, mem_step: u64) {
        // Two possible situations to process open mem align operations:
        //
        // 1) the address of open operation is less than the aligned address.
        // 2) the address of open operation is equal to the aligned address, but the step of the
        //    open operation is less than the step of the current operation.

        while let Some(open_op) = self.get_next_open_mem_align_input(mem_addr, mem_step) {
            // call to mem_align to get information of the aligned memory access needed
            // to prove the unaligned open operation.
            let mem_align_resp = self.mem_align_sm.get_mem_op(&open_op, 1);

            #[cfg(feature = "debug_mem_proxy_engine")]
            Self::debug_mem_align_api(&open_op, &mem_align_resp, 1);

            // push the aligned memory operations for current address (read or read+write) and
            // update last_address and last_value.
            self.push_mem_align_response_ops(
                Self::next_aligned_addr(open_op.addr),
                open_op.mem_values[1],
                &open_op,
                &mem_align_resp,
            );
        }
    }

    pub fn main_step_to_mem_step(step: u64, step_offset: u8) -> u64 {
        1 + MAX_MEM_OPS_BY_MAIN_STEP * step + 2 * step_offset as u64
    }

    #[inline(always)]
    fn is_aligned(address: u32, width: u8) -> bool {
        ((address & 0x07) == 0) && (width == 8)
    }

    fn push_aligned_op(&mut self, is_write: bool, addr: u32, value: u64, step: u64) {
        self.update_mem_module(addr);
        let w_addr = Self::to_aligned_word_addr(addr);

        // check if step difference is too large
        if self.last_addr == w_addr && (step - self.last_step) > MEMORY_MAX_DIFF {
            self.push_intermediate_internal_reads(w_addr, self.last_value, self.last_step, step);
        }

        self.last_step = step;
        self.last_addr = w_addr;
        self.last_value = value;

        let mem_op = MemInput { step, is_write, is_internal: false, addr: w_addr, value };
        debug_info!(
            "route ==> {}[{:X}] {} {} #{}",
            self.current_module,
            mem_op.addr << MEM_BYTES_BITS,
            if is_write { "W" } else { "R" },
            value,
            step,
        );
        self.internal_push_mem_op(mem_op);
    }

    fn push_intermediate_internal_reads(
        &mut self,
        addr: u32,
        value: u64,
        last_step: u64,
        final_step: u64,
    ) {
        let mut step = last_step;
        self.intermediate_cases += 1;
        while (final_step - step) > MEMORY_MAX_DIFF {
            self.intermediate_steps += 1;
            step += MEMORY_MAX_DIFF;
            let mem_op = MemInput { step, is_write: false, is_internal: true, addr, value };
            self.internal_push_mem_op(mem_op);
        }
    }

    fn internal_push_mem_op(&mut self, mem_op: MemInput) {
        self.modules_data[self.current_module_id].inputs.push(mem_op);
        self.check_flush_inputs();
    }
    // method to add aligned read operation
    #[inline(always)]
    fn push_aligned_read(&mut self, addr: u32, value: u64, step: u64) {
        self.push_aligned_op(false, addr, value, step);
    }
    // method to add aligned write operation
    #[inline(always)]
    fn push_aligned_write(&mut self, addr: u32, value: u64, step: u64) {
        self.push_aligned_op(true, addr, value, step);
    }
    /// Process information of mem_op and mem_align_op to push mem_op operation. Only two possible
    /// situations:
    /// 1) read, only on single mem_op is pushed
    /// 2) read+write, two mem_op are pushed, one read and one write.
    ///
    /// This process is used for each aligned memory address, means that the "second part" of non
    /// aligned memory operation is processed on addr + MEM_BYTES.
    fn push_mem_align_response_ops(
        &mut self,
        mem_addr: u32,
        mem_value: u64,
        mem_align_input: &MemAlignInput,
        mem_align_resp: &MemAlignResponse,
    ) {
        self.push_aligned_read(mem_addr, mem_value, mem_align_resp.step);
        if mem_align_input.is_write {
            self.push_aligned_write(
                mem_addr,
                mem_align_resp.value.unwrap(),
                mem_align_resp.step + 1,
            );
        }
    }
    fn set_active_region(&mut self, region_id: usize) {
        self.current_module_id = self.addr_map[region_id].module_id as usize;
        self.current_module = self.modules_data[self.current_module_id].name.clone();
        self.module_end_addr = self.addr_map[region_id].to_addr;
    }
    fn update_mem_module_id(&mut self, addr: u32) {
        debug_info!("search module for address 0x{:X}", addr);
        if let Some(index) =
            self.addr_map.iter().position(|x| x.from_addr <= addr && x.to_addr >= addr)
        {
            self.set_active_region(index);
        } else {
            panic!("out-of-memory 0x{:X}", addr);
        }
    }
    fn update_mem_module(&mut self, addr: u32) {
        // check if need to reevaluate the module id
        if addr > self.module_end_addr {
            self.update_mem_module_id(addr);
        }
    }
    fn check_flush_inputs(&mut self) {
        // check if need to flush the inputs of the module
        let mid = self.current_module_id;
        let inputs = self.modules_data[mid].inputs.len();
        if inputs >= self.modules_data[mid].flush_input_size {
            // TODO: optimize passing ownership of inputs to module, and creating a new input
            // object
            debug_info!("flush {} inputs => {}", inputs, self.current_module);
            self.modules[mid].send_inputs(&self.modules_data[mid].inputs);
            self.modules_data[mid].inputs.clear();
        }
    }

    fn get_next_open_mem_align_input(&mut self, addr: u32, step: u64) -> Option<MemAlignInput> {
        if self.next_open_addr < addr || (self.next_open_addr == addr && self.next_open_step < step)
        {
            let open_op = self.open_mem_align_ops.pop_front().unwrap();
            self.update_next_open_mem_align();
            Some(open_op)
        } else {
            None
        }
    }
    // method to process open mem align operations, second part of non aligned memory operations
    // applies to two consecutive memory addresses.

    fn push_end_of_memory_mark(&mut self, mem_operations: &mut Vec<ZiskRequiredMemory>) {
        mem_operations.push(ZiskRequiredMemory::Basic {
            step: MAX_MAIN_STEP,
            step_offset: MAX_MEM_STEP_OFFSET as u8,
            is_write: false,
            address: MAX_MEM_ADDR as u32,
            width: MEM_BYTES as u8,
            value: 0,
        });
        mem_operations
            .push(ZiskRequiredMemory::Extended { address: MAX_MEM_ADDR as u32, values: [0, 0] });
    }

    /// Check if the address is the "special" address inserted at the end of the memory operations
    #[inline(always)]
    fn check_if_end_of_memory_mark(&self, addr: u32, mem_step: u64) -> bool {
        if addr == MAX_MEM_ADDR as u32 && mem_step == MAX_MEM_STEP {
            debug_assert!(
                self.open_mem_align_ops.is_empty(),
                "open_mem_align_ops not empty, has {} elements",
                self.open_mem_align_ops.len()
            );
            true
        } else {
            false
        }
    }
    /// Encapsulates all tasks to be performed at the beginning of the witness computation (stage
    /// 1).
    ///
    /// This method fetches the address map and sets the initial values to prepare for the
    /// computation.
    fn init_prove(&mut self) {
        if !self.addr_map_fetched {
            self.fetch_address_map();
        }
        self.current_module_id = self.addr_map[0].module_id as usize;
        self.current_module = self.modules_data[self.current_module_id].name.clone();
        self.module_end_addr = self.addr_map[0].to_addr;
    }
    /// Encapsulates all tasks to be performed at the end of the witness computation (stage 1).
    ///
    /// This method flushes all module inputs to ensure they are finalized and ready for further
    /// processing.
    fn finish_prove(&self) {
        for (module_id, module) in self.modules.iter().enumerate() {
            debug_info!(
                "{}: flush all({}) inputs",
                self.modules_data[module_id].name,
                self.modules_data[module_id].inputs.len()
            );
            module.send_inputs(&self.modules_data[module_id].inputs);
        }
        debug_info!(
            "MemProxy: ··· Intermediate reads [cases:{} steps:{}]",
            self.intermediate_cases,
            self.intermediate_steps
        );
    }
    /// Fetches the address map, defining and calculating all necessary structures to manage the
    /// memory map.
    ///
    /// For undefined regions (such as memory between defined regions, or memory at the beginning or
    /// end of the memory map), this method assigns an unmapped module. If any access occurs
    /// within these unmapped memory regions, the method will trigger a panic.
    ///
    /// The unmapped module ensures that every address has an associated module to handle memory
    /// access, providing a safety mechanism to prevent undefined behavior.
    fn fetch_address_map(&mut self) {
        let unmapped_regions: Vec<(u32, u32)> = self.get_unmapped_regions();
        if !unmapped_regions.is_empty() {
            self.define_unmapped_module(&unmapped_regions);
        }
        self.addr_map_fetched = true;
    }

    /// Get list of regions (from_addr, to_addr) that are not defined in the memory map
    fn get_unmapped_regions(&self) -> Vec<(u32, u32)> {
        let mut next_addr = 0;
        let mut unmapped_regions: Vec<(u32, u32)> = Vec::new();
        for addr_region in self.addr_map.iter() {
            if next_addr < addr_region.from_addr {
                unmapped_regions.push((next_addr, addr_region.from_addr - 1));
            }
            next_addr = addr_region.to_addr + 1;
        }
        unmapped_regions
    }

    /// Define an unmapped module with all unmapped regions.
    fn define_unmapped_module(&mut self, unmapped_regions: &[(u32, u32)]) {
        let mut unmapped_module = MemUnmapped::<F>::new();
        for unmapped_region in unmapped_regions.iter() {
            unmapped_module.add_range(unmapped_region.0, unmapped_region.1);
        }
        self.add_module("unmapped", Arc::new(unmapped_module));
    }

    /// Calculate aligned address from regular address (aligned or not)
    #[inline(always)]
    fn to_aligned_addr(addr: u32) -> u32 {
        addr & MEM_ADDR_MASK
    }

    /// Calculate the next aligned address from regular address (aligned or not)
    #[inline(always)]
    fn next_aligned_addr(addr: u32) -> u32 {
        (addr & MEM_ADDR_MASK) + MEM_BYTES
    }

    /// Calculate the word address where word is MEM_BYTES
    #[inline(always)]
    fn to_aligned_word_addr(addr: u32) -> u32 {
        addr >> MEM_BYTES_BITS
    }

    #[cfg(feature = "debug_mem_proxy_engine")]
    fn debug_mem_align_api(
        mem_align_input: &MemAlignInput,
        mem_align_response: &MemAlignResponse,
        phase: u8,
    ) {
        if mem_align_input.addr >= DEBUG_ADDR - 8 && mem_align_input.addr <= DEBUG_ADDR + 8 {
            debug_info!(
                "mem_align_input_{:X}: phase:{} {:?}",
                mem_align_input.addr,
                phase,
                mem_align_input
            );
            debug_info!(
                "mem_align_response_{:X}: phase:{} {:?}",
                mem_align_input.addr,
                phase,
                mem_align_response
            );
        }
    }
}
