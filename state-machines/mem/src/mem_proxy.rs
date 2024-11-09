use std::collections::VecDeque;
use std::fmt;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::{MemAlignResponse, MemAlignRomSM, MemAlignSM, MemSM};
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};

const MEM_ADDR_MASK: u64 = 0xFFFF_FFFF_FFFF_FFF8;
const MEM_BYTES: u64 = 8;

const MAX_MEM_STEP_OFFSET: u64 = 2;
const MAX_MEM_OPS_PER_MAIN_STEP: u64 = (MAX_MEM_STEP_OFFSET + 1) * 2;

pub trait MemModule<F>: Send + Sync {
    fn send_inputs(&self, mem_op: &[ZiskRequiredMemory]);
    fn get_addr_ranges(&self) -> Vec<(u64, u64)>;
    fn get_flush_input_size(&self) -> u64;
    fn unregister_predecessor(&self);
    fn register_predecessor(&self);
}

struct MemModuleData {
    pub inputs: Vec<ZiskRequiredMemory>,
    pub addr_ranges: Vec<(u64, u64)>,
    pub flush_input_size: u64,
}

impl fmt::Debug for MemAlignResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "more:{} step:{} value:{:2}({:3})",
            self.more_address,
            self.step,
            format_hex(self.value.unwrap_or(0)),
            self.value.unwrap_or(0)
        )
    }
}
pub struct MemProxy<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Secondary State machines
    //  mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM<F>>,
    modules: Vec<Arc<dyn MemModule<F>>>,
    modules_data: Vec<MemModuleData>,
}

pub struct MemOperation {
    pub step: u64,
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}

pub struct MemAlignOperation {
    pub address: u64,
    pub mem_op: ZiskRequiredMemory,
    pub mem_value: [u64; 2],
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let mem_align_rom_sm = MemAlignRomSM::new(wcm.clone());
        let mem_align_sm = MemAlignSM::new(wcm.clone(), std, mem_align_rom_sm);

        let mut modules: Vec<Arc<dyn MemModule<F>>> = Vec::new();

        modules.push(MemSM::new(wcm.clone()).clone());
        let mut modules_data: Vec<MemModuleData> = Vec::new();

        for module in modules.iter_mut() {
            modules_data.push(Self::init_module(module));
        }
        let mem_proxy = Self {
            registered_predecessors: AtomicU32::new(0),
            mem_align_sm,
            modules,
            modules_data,
        };
        let mem_proxy = Arc::new(mem_proxy);

        wcm.register_component(mem_proxy.clone(), None, None);

        // For all the secondary state machines, register the main state machine as a predecessor
        mem_proxy.mem_align_sm.register_predecessor();
        mem_proxy
    }
    pub fn main_step_to_mem_step(step: u64, step_offset: u8) -> u64 {
        1 + MAX_MEM_OPS_PER_MAIN_STEP * step + 2 * step_offset as u64
    }
    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            for module in self.modules.iter() {
                module.unregister_predecessor();
            }
            // self.mem_sm.unregister_predecessor();
            self.mem_align_sm.unregister_predecessor();
        }
    }

    fn init_module(module: &Arc<dyn MemModule<F>>) -> MemModuleData {
        module.register_predecessor();
        let ranges = module.get_addr_ranges();
        let flush_input_size = module.get_flush_input_size();
        MemModuleData { inputs: Vec::new(), addr_ranges: ranges, flush_input_size }
    }

    /// Static method to decide it the memory operation needs to be processed by
    /// memAlign, because it isn't a 8-byte and 8-byte aligned memory access.
    fn is_aligned(mem_op: &ZiskRequiredMemory) -> bool {
        let aligned_mem_address = mem_op.address & MEM_ADDR_MASK;
        aligned_mem_address == mem_op.address && mem_op.width == MEM_BYTES
    }
    /// Process information of mem_op and mem_align_op to push mem_op operation. Only two possible situations:
    /// 1) read, only on single mem_op is pushed
    /// 2) read+write, two mem_op are pushed, one read and one write.
    ///
    /// This process is used for each aligned memory address, means that the "second part" of non aligned memory
    /// operation is processed on addr + MEM_BYTES.
    fn push_mem_align_op(
        &self,
        mem_addr: u64,
        mem_value: u64,
        mem_op: &ZiskRequiredMemory,
        mem_align_op: &MemAlignResponse,
        input: &mut Vec<ZiskRequiredMemory>,
    ) -> u64 {
        // Prepare aligned memory access
        let read = ZiskRequiredMemory {
            step: mem_align_op.step,
            is_write: false,
            address: mem_addr,
            width: MEM_BYTES,
            value: mem_value,
        };
        println!("  ##SEND2## mem_op: {0:?}", read);
        input.push(read);

        if mem_op.is_write {
            let mem_value = mem_align_op.value.expect("value returned by mem_align");
            let write = ZiskRequiredMemory {
                step: mem_align_op.step + 1,
                is_write: true,
                address: mem_addr,
                width: MEM_BYTES,
                value: mem_value,
            };
            println!("  ##SEND2## mem_op: {0:?}", write);
            input.push(write);
            mem_value
        } else {
            mem_value
        }
    }
    fn create_modules_inputs(&self) -> Vec<Vec<ZiskRequiredMemory>> {
        let mut mem_module_inputs: Vec<Vec<ZiskRequiredMemory>> = Default::default();
        for module in self.modules.iter() {
            mem_module_inputs.push(Vec::new());
        }
        mem_module_inputs
    }
    fn get_mem_module_id(&self, address: u64) -> (usize, u64) {
        let mem_module_id = 0;
        let next_addr_to_reevaluate = 0xFFFF_FFFF_FFFF;
        (mem_module_id, next_addr_to_reevaluate)
    }
    pub fn prove(
        &self,
        mem_operations: &mut Vec<ZiskRequiredMemory>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut open_mem_align_ops: VecDeque<MemAlignOperation> = VecDeque::new();
        let mut mem_module_inputs = self.create_modules_inputs();

        // Step 1. Sort the aligned memory accesses
        // original vector is sorted by step, sort_by_key is stable, no reordering of elements with
        // the same key.
        timer_start_debug!(MEM_SORT);
        mem_operations.sort_by_key(|mem| (mem.address & 0xFFFF_FFFF_FFFF_FFF8));
        timer_stop_and_log_debug!(MEM_SORT);

        // Initialize the last values of address and value on the sorted memory operations
        let mut last_addr = 0xFFFF_FFFF_FFFF_FFFFu64;
        let mut last_value = 0u64;

        // Add a final fake mem_op to force flush of open_mem_align_ops
        mem_operations.push(ZiskRequiredMemory {
            step: 0,
            is_write: false,
            address: MEM_ADDR_MASK,
            width: 8,
            value: 0,
        });

        // Initialize the module id and next module address to reevaluate the module id, it's done
        // to avoid check on each loop if memory address is inside one range or other
        let (mut mem_module_id, mut next_module_addr) = if mem_operations.is_empty() {
            (0, 0)
        } else {
            self.get_mem_module_id(mem_operations[0].address)
        };

        for mem_op in mem_operations.iter_mut() {
            println!(
                "##LOOP## mem_op: {0:?} 0x{1:#08X}({1}) 0x{2:#016X}({2})",
                mem_op, last_addr, last_value
            );
            let mut aligned_mem_address = mem_op.address & MEM_ADDR_MASK;

            // ONLY TO TEST
            if aligned_mem_address < 0xA0000000 {
                continue;
            }

            // Check if there are open mem align operations to be processed in this moment. Two possible
            // conditions to process open mem align operations:
            // 1) the address of open operation is less than the aligned address.
            // 2) the address of open operation is equal to the aligned address, but the step of the open
            // operation is less than the step of the current operation.

            while open_mem_align_ops.len() > 0
                && (open_mem_align_ops[0].address < aligned_mem_address
                    || (open_mem_align_ops[0].address == aligned_mem_address
                        && open_mem_align_ops[0].mem_op.step < mem_op.step))
            {
                let open_op = open_mem_align_ops.pop_front().unwrap();
                let mem_value = if open_op.address == last_addr { last_value } else { 0 };

                // call to mem_align to get information of the aligned memory access needed
                // to prove the unaligned open operation.
                let mem_align_op = mem_align_call(&open_op.mem_op, [mem_value, 0], 1);

                // remove element from top of queue, because we are on last phase, phase 1.
                open_mem_align_ops.pop_front();

                // check if need to reevaluate the module id
                if open_op.address >= next_module_addr {
                    (mem_module_id, next_module_addr) = self.get_mem_module_id(open_op.address);
                }
                // push the aligned memory operations for current address (read or read+write) and
                // update last_address and last_value.
                last_value = self.push_mem_align_op(
                    open_op.address,
                    mem_value,
                    &mem_op,
                    &mem_align_op,
                    &mut mem_module_inputs[mem_module_id],
                );
                last_addr = open_op.address;

                // check if need to flush the inputs of the module
                if (mem_module_inputs[mem_module_id].len() as u64)
                    >= self.modules_data[mem_module_id].flush_input_size
                {
                    self.modules[mem_module_id].send_inputs(&mut mem_module_inputs[mem_module_id]);
                }
            }

            aligned_mem_address = mem_op.address & MEM_ADDR_MASK;

            // check if the aligned address is the last address to avoid processing the last fake mem_op
            if aligned_mem_address == MEM_ADDR_MASK {
                assert!(
                    open_mem_align_ops.len() == 0,
                    "open_mem_align_ops not empty, has {} elements",
                    open_mem_align_ops.len()
                );
                break;
            }

            // check if need to reevaluate the module id
            if aligned_mem_address >= next_module_addr {
                (mem_module_id, next_module_addr) = self.get_mem_module_id(aligned_mem_address);
            }

            let mem_value = if aligned_mem_address == last_addr { last_value } else { 0 };

            // all open mem align operations are processed, check if new mem operation is aligned
            if !Self::is_aligned(&mem_op) {
                // In this point found non-aligned memory access, phase-0
                let mem_align_op = mem_align_call(mem_op, [mem_value, 0], 0);
                if mem_align_op.more_address {
                    open_mem_align_ops.push_back(MemAlignOperation {
                        address: aligned_mem_address + MEM_BYTES,
                        mem_op: mem_op.clone(),
                        mem_value: [mem_value, 0],
                    });
                }
                last_value = self.push_mem_align_op(
                    aligned_mem_address,
                    mem_value,
                    &mem_op,
                    &mem_align_op,
                    &mut mem_module_inputs[mem_module_id],
                );
                last_addr = aligned_mem_address
            } else {
                println!("  ##SEND1## mem_op: {0:?}", mem_op);
                mem_module_inputs[mem_module_id].push(mem_op.clone());
                last_value = mem_op.value;
                last_addr = aligned_mem_address
            }

            // check if need to flush the inputs of the module
            if (mem_module_inputs[mem_module_id].len() as u64)
                >= self.modules_data[mem_module_id].flush_input_size
            {
                self.modules[mem_module_id].send_inputs(&mut mem_module_inputs[mem_module_id]);
            }
        }

        Ok(())
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemProxy<F> {}

fn format_hex(value: u64) -> String {
    let hex_str = format!("{:016x}", value); // Format hexadecimal amb 16 dígits i padding de 0s
    hex_str
        .as_bytes() // Converteix a bytes per manipular fàcilment
        .chunks(4) // Separa en grups de 4 caràcters (2 bytes)
        .map(|chunk| std::str::from_utf8(chunk).unwrap()) // Converteix cada chunk a &str
        .collect::<Vec<_>>() // Recull els chunks com a un vector
        .join("_") // Uneix amb "_"
}

fn mem_align_call(
    mem_op: &ZiskRequiredMemory,
    mem_values: [u64; 2],
    phase: u8,
) -> MemAlignResponse {
    // DEBUG: only for testing
    let offset = (mem_op.address & 0x7) * 8;
    let width = (mem_op.width as u64) * 8;
    let double_address = (offset + width) > 64;
    let mem_value = mem_values[phase as usize];
    let mask = 0xFFFF_FFFF_FFFF_FFFFu64 >> (64 - width);
    /*println!("width: {} offset:{}", width, offset);
    println!("mem_value   {}", format_hex(mem_value));
    println!("mask        {}", format_hex(mask));*/
    if mem_op.is_write {
        if phase == 0 {
            /*println!("mask1       {}", format_hex(mask << offset));
            println!("mask2       {}", format_hex(0xFFFF_FFFF_FFFF_FFFFu64 ^ (mask << offset)));
            println!(
                "mask3       {}",
                format_hex((mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 ^ (mask << offset))))
            );
            println!("mask4       {}", format_hex((mem_op.value & mask) << offset));*/
            MemAlignResponse {
                more_address: double_address,
                step: mem_op.step + 1,
                value: Some(
                    (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 ^ (mask << offset)))
                        | ((mem_op.value & mask) << offset),
                ),
            }
        } else {
            /* println!("{} bits = {} bytes", (offset + width - 64), (offset + width - 64) >> 3);
            println!("ph1_1       {}", format_hex(mask << offset));
            println!(
                "ph1_2       {}",
                format_hex(0xFFFF_FFFF_FFFF_FFFFu64 << (offset + width - 64))
            );
            println!(
                "ph1_3       {}",
                format_hex(mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 << (offset + width - 64)))
            );
            println!("ph1_4       {}", format_hex((mem_op.value & mask) >> (128 - offset - width)));*/
            MemAlignResponse {
                more_address: false,
                step: mem_op.step + 1,
                value: Some(
                    (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 << (offset + width - 64)))
                        | ((mem_op.value & mask) >> (128 - offset - width)),
                ),
            }
        }
    } else {
        MemAlignResponse {
            more_address: double_address && phase == 0,
            step: mem_op.step + 1,
            value: None,
        }
    }
}
