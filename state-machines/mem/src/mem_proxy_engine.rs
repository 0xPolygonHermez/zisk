use std::{collections::VecDeque, sync::Arc};

use crate::{
    MemAlignInput, MemAlignResponse, MemAlignSM, MemHelpers, MemInput, MemUnmapped, MAX_MEM_ADDR,
    MAX_MEM_OPS_PER_MAIN_STEP, MEM_ADDR_MASK, MEM_BYTES,
};
use log::info;
use p3_field::PrimeField;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use zisk_core::ZiskRequiredMemory;

#[cfg(feature = "debug_mem_proxy_engine")]
const DEBUG_ADDR: u32 = 0xA0008F10;

macro_rules! debug_info {
    ($prefix:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_mem_proxy_engine")]
        {
            info!(concat!("MemPE   : ",$prefix), $($arg)*);
        }
    };
}

pub trait MemModule<F>: Send + Sync {
    fn send_inputs(&self, mem_op: &[MemInput]);
    fn get_addr_ranges(&self) -> Vec<(u32, u32)>;
    fn get_flush_input_size(&self) -> u32;
}

trait MemAlignSm {
    fn get_mem_op(&self, mem_op: &MemInput, phase: u8) -> MemAlignResponse;
}

struct MemModuleData {
    pub name: String,
    pub inputs: Vec<MemInput>,
    pub flush_input_size: u32,
}

struct MemAlignOperation {
    addr: u32,
    input: MemAlignInput,
}

#[derive(Debug)]
pub struct AddressRegion {
    from_address: u32,
    to_address: u32,
    module_id: u8,
}
pub struct MemProxyEngine<F: PrimeField> {
    modules: Vec<Arc<dyn MemModule<F>>>,
    modules_data: Vec<MemModuleData>,
    open_mem_align_ops: VecDeque<MemAlignOperation>,
    address_map: Vec<AddressRegion>,
    address_map_closed: bool,
    last_addr: u32,
    last_addr_value: u64,
    current_module_id: usize,
    current_module: String,
    module_end_addr: u32,
}

impl<F: PrimeField> MemProxyEngine<F> {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            modules_data: Vec::new(),
            last_addr: 0,
            last_addr_value: 0,
            current_module_id: 0,
            current_module: String::new(),
            module_end_addr: 0,
            open_mem_align_ops: VecDeque::new(),
            address_map: Vec::new(),
            address_map_closed: false,
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
            debug_info!("adding range 0x{:X} 0x{:X}", range.0, range.1);
            self.insert_address_range(range.0, range.1, module_id);
        }
        self.modules_data.push(MemModuleData {
            name: String::from(name),
            inputs: Vec::new(),
            flush_input_size,
        });
    }
    /* insert in sort way the address map and verify that */
    fn insert_address_range(&mut self, from_address: u32, to_address: u32, module_id: u8) {
        let region = AddressRegion { from_address, to_address, module_id };
        if let Some(index) = self.address_map.iter().position(|x| x.from_address >= from_address) {
            self.address_map.insert(index, region);
        } else {
            self.address_map.push(region);
        }
    }

    pub fn prove(
        &mut self,
        mem_align_sm: &MemAlignSM<F>,
        mem_operations: &mut Vec<ZiskRequiredMemory>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.init_prove();

        // Step 1. Sort the aligned memory accesses
        // original vector is sorted by step, sort_by_key is stable, no reordering of elements with
        // the same key.
        timer_start_debug!(MEM_SORT);
        mem_operations.sort_by_key(|mem| (mem.address & 0xFFFF_FFF8));
        timer_stop_and_log_debug!(MEM_SORT);

        // Step2. Add a final mark mem_op to force flush of open_mem_align_ops, because always the
        // last operation is mem_op.
        mem_operations.push(Self::end_of_memory_mark());

        // Step3. Process each memory operation ordered by address and step. When a non-aligned
        // memory access there are two possible situations:
        //
        //  1) the operation applies only applies to one memory address (read or read+write). In
        //     this case mem_align helper return the aligned operation for this address, and loop
        //     continues.
        //  2) the operation applies to two consecutive memory addresses, mem_align helper returns
        //     the aligned operation involved for the current address, and the second part of the
        //     operation is enqueued to open_mem_align_ops, it will processed when processing next
        //     address.
        //
        // Inside loop, first of all, we verify if exists "previous" open mem align operations that
        // be processed before current mem_op, in this case process all "previous" and after process
        // the current mem_op.

        for mem_extern_op in mem_operations.iter_mut() {
            self.log_mem_op(mem_extern_op);
            let mem_op = MemInput::from(mem_extern_op);
            let aligned_mem_addr = Self::to_aligned_addr(mem_op.address);
            let mem_step = mem_op.step;

            // Check if there are open mem align operations to be processed in this moment, with
            // address (or step) less than the aligned of current mem_op.
            self.process_all_previous_open_mem_align_ops(aligned_mem_addr, mem_step, mem_align_sm);

            // check if we are at end of loop
            if self.check_if_end_of_memory_mark(&mem_op) {
                break;
            }

            // TODO: edge case special memory with free-input memory data as input data
            let mem_value = self.get_mem_value(aligned_mem_addr);

            // all open mem align operations are processed, check if new mem operation is aligned
            if !Self::is_aligned(&mem_extern_op) {
                // In this point found non-aligned memory access, phase-0
                let mem_align_input =
                    MemAlignInput::from(&mem_op, mem_extern_op.width, &[mem_value, 0]);
                let mem_align_response = mem_align_sm.get_mem_op(&mem_align_input, 0);

                #[cfg(feature = "debug_mem_proxy_engine")]
                if mem_align_input.address >= DEBUG_ADDR - 8 &&
                    mem_align_input.address <= DEBUG_ADDR + 8
                {
                    debug_info!(
                        "mem_align_input_{:X}: phase: 0 {:?}",
                        mem_align_input.address,
                        mem_align_input
                    );
                    debug_info!(
                        "mem_align_response_{:X}: phase: 0 {:?}",
                        mem_align_input.address,
                        mem_align_response
                    );
                }
                // if operation applies to two consecutive memory addresses, add the second part
                // is enqueued to be processed in future when processing next address on phase-1
                if mem_align_response.more_address {
                    self.push_open_mem_align_op(aligned_mem_addr, &mem_align_input);
                }
                self.push_mem_align_response_ops(
                    aligned_mem_addr,
                    mem_value,
                    &mem_align_input,
                    &mem_align_response,
                );
            } else {
                self.push_mem_op(&mem_op);
            }
        }
        self.finish_prove();
        Ok(())
    }

    fn process_all_previous_open_mem_align_ops(
        &mut self,
        mem_addr: u32,
        mem_step: u64,
        mem_align_sm: &MemAlignSM<F>,
    ) {
        // Two possible situations to process open mem align operations:
        //
        // 1) the address of open operation is less than the aligned address.
        // 2) the address of open operation is equal to the aligned address, but the step of the
        //    open operation is less than the step of the current operation.

        while self.has_open_mem_align_lt(mem_addr, mem_step) {
            let mut open_op = self.open_mem_align_ops.pop_front().unwrap();
            let mem_value = self.get_mem_value(open_op.addr);

            // call to mem_align to get information of the aligned memory access needed
            // to prove the unaligned open operation.
            open_op.input.mem_values[1] = mem_value;
            let mem_align_resp = mem_align_sm.get_mem_op(&open_op.input, 1);

            #[cfg(feature = "debug_mem_proxy_engine")]
            if open_op.input.address >= DEBUG_ADDR - 8 && open_op.input.address <= DEBUG_ADDR + 8 {
                debug_info!(
                    "mem_align_input_{:X}: phase:1 {:?}",
                    open_op.input.address,
                    open_op.input
                );
                debug_info!(
                    "mem_align_response_{:X}: phase:1 {:?}",
                    open_op.input.address,
                    mem_align_resp
                );
            }
            // push the aligned memory operations for current address (read or read+write) and
            // update last_address and last_value.
            self.push_mem_align_response_ops(
                open_op.addr,
                mem_value,
                &open_op.input,
                &mem_align_resp,
            );
        }
    }

    pub fn main_step_to_mem_step(step: u64, step_offset: u8) -> u64 {
        1 + MAX_MEM_OPS_PER_MAIN_STEP * step + 2 * step_offset as u64
    }

    /// Static method to decide it the memory operation needs to be processed by
    /// memAlign, because it isn't a 8-byte and 8-byte aligned memory access.
    fn is_aligned(mem_op: &ZiskRequiredMemory) -> bool {
        let aligned_mem_address = (mem_op.address as u64 & MEM_ADDR_MASK) as u32;
        aligned_mem_address == mem_op.address && mem_op.width == MEM_BYTES as u8
    }
    fn push_mem_op(&mut self, mem_op: &MemInput) {
        self.push_aligned_op(mem_op.is_write, mem_op.address, mem_op.value, mem_op.step);
    }

    fn push_aligned_op(&mut self, is_write: bool, addr: u32, value: u64, step: u64) {
        self.update_last_addr(addr, value);
        let mem_op = MemInput { step, is_write, address: addr as u32, value };
        debug_info!(
            "route ==> {}[{:X}] {} {} #{}",
            self.current_module,
            mem_op.address,
            if is_write { "W" } else { "R" },
            value,
            step,
        );
        self.modules_data[self.current_module_id].inputs.push(mem_op);
        self.last_addr_value = value;
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
            #[cfg(feature = "debug_mem_proxy_engine")]
            if mem_addr >= DEBUG_ADDR - 8 && mem_addr <= DEBUG_ADDR - 8 {
                debug_info!(
                    "push_mem_align_response_ops_{:X}-A: value:{} {:?}",
                    mem_addr,
                    mem_align_resp.value.unwrap(),
                    mem_align_resp
                );
                debug_info!(
                    "push_mem_align_response_ops_{:X}-B: mem_value:{} {:?}",
                    mem_addr,
                    mem_value,
                    mem_align_input
                );
            }
            self.push_aligned_write(
                mem_addr,
                mem_align_resp.value.unwrap(),
                mem_align_resp.step + 1,
            );
        }
    }
    fn create_modules_inputs(&self) -> Vec<Vec<MemInput>> {
        let mut mem_module_inputs: Vec<Vec<MemInput>> = Default::default();
        for _module in self.modules.iter() {
            mem_module_inputs.push(Vec::new());
        }
        mem_module_inputs
    }
    fn set_active_region(&mut self, region_id: usize) {
        self.current_module_id = self.address_map[region_id].module_id as usize;
        self.current_module = self.modules_data[self.current_module_id].name.clone();
        self.module_end_addr = self.address_map[region_id].to_address;
    }
    fn update_mem_module_id(&mut self, addr: u32) {
        debug_info!("search module for address 0x{:X}", addr);
        if let Some(index) =
            self.address_map.iter().position(|x| x.from_address <= addr && x.to_address >= addr)
        {
            self.set_active_region(index);
        } else {
            assert!(false, "out-of-memory 0x{:X}", addr);
        }
    }
    fn update_last_addr(&mut self, addr: u32, value: u64) {
        self.last_addr = addr;
        self.last_addr_value = value;
        self.update_mem_module(addr);
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
        let inputs = self.modules_data[mid].inputs.len() as u32;
        if inputs >= self.modules_data[mid].flush_input_size {
            // TODO: optimize passing ownership of inputs to module, and creating a new input
            // object
            debug_info!("flush {} inputs => {}", inputs, self.current_module);
            self.modules[mid].send_inputs(&self.modules_data[mid].inputs);
            self.modules_data[mid].inputs.clear();
        }
    }

    fn has_open_mem_align_lt(&self, addr: u32, step: u64) -> bool {
        self.open_mem_align_ops.len() > 0 &&
            (self.open_mem_align_ops[0].addr < addr ||
                (self.open_mem_align_ops[0].addr == addr &&
                    self.open_mem_align_ops[0].input.step < step))
    }
    // method to process open mem align operations, second part of non aligned memory operations
    // applies to two consecutive memory addresses.

    fn end_of_memory_mark() -> ZiskRequiredMemory {
        ZiskRequiredMemory {
            step: 0,
            step_offset: 0,
            is_write: false,
            address: MAX_MEM_ADDR as u32,
            width: MEM_BYTES as u8,
            value: 0,
        }
    }
    #[inline(always)]
    fn check_if_end_of_memory_mark(&self, mem_op: &MemInput) -> bool {
        // TODO: 0xFFFF_FFFF not valid address
        if mem_op.address == MAX_MEM_ADDR as u32 {
            assert!(
                self.open_mem_align_ops.len() == 0,
                "open_mem_align_ops not empty, has {} elements",
                self.open_mem_align_ops.len()
            );
            true
        } else {
            false
        }
    }
    fn init_prove(&mut self) {
        if !self.address_map_closed {
            self.close_address_map();
        }
        self.current_module_id = self.address_map[0].module_id as usize;
        self.current_module = self.modules_data[self.current_module_id].name.clone();
        self.module_end_addr = self.address_map[0].to_address;
    }
    fn finish_prove(&self) {
        for (module_id, module) in self.modules.iter().enumerate() {
            debug_info!(
                "{}: flush all({}) inputs",
                self.modules_data[module_id].name,
                self.modules_data[module_id].inputs.len()
            );
            module.send_inputs(&self.modules_data[module_id].inputs);
        }
    }
    fn get_mem_value(&self, addr: u32) -> u64 {
        if addr == self.last_addr {
            self.last_addr_value
        } else {
            0
        }
    }
    fn close_address_map(&mut self) {
        let mut next_address = 0;
        let mut unmapped_regions: Vec<(u32, u32)> = Vec::new();
        for address_region in self.address_map.iter() {
            if next_address < address_region.from_address {
                unmapped_regions.push((next_address, address_region.from_address - 1));
            }
            next_address = address_region.to_address + 1;
        }
        if !unmapped_regions.is_empty() {
            let mut unmapped_module = MemUnmapped::<F>::new();
            for unmapped_region in unmapped_regions.iter() {
                unmapped_module.add_range(unmapped_region.0, unmapped_region.1);
            }
            self.add_module("unmapped", Arc::new(unmapped_module));
        }
        self.address_map_closed = true;
    }

    #[inline(always)]
    fn push_open_mem_align_op(&mut self, aligned_mem_addr: u32, input: &MemAlignInput) {
        self.open_mem_align_ops.push_back(MemAlignOperation {
            addr: aligned_mem_addr + MEM_BYTES as u32,
            input: input.clone(),
        });
    }
    fn log_mem_op(&self, mem_op: &ZiskRequiredMemory) {
        debug_info!(
            "next input [0x{:x}] {} {} {}b #{} [0x{:x},{}]",
            mem_op.address,
            if mem_op.is_write { "W" } else { "R" },
            mem_op.value,
            mem_op.width,
            mem_op.step,
            self.last_addr,
            self.last_addr_value
        );
    }
    #[inline(always)]
    fn to_aligned_addr(addr: u32) -> u32 {
        (addr as u64 & MEM_ADDR_MASK) as u32
    }
}
