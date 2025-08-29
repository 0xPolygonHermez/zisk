use crate::{MemAlignInput, MemHelpers};
use mem_common::MemAlignCheckPoint;

use std::collections::VecDeque;
use zisk_common::{BusDevice, BusId, CollectCounter, MemBusData, MEM_BUS_ID};

pub struct MemAlignCollector {
    /// Collected inputs
    pub inputs: Vec<MemAlignInput>,

    full_5: CollectCounter,
    full_3: CollectCounter,
    full_2: CollectCounter,
    read_byte: CollectCounter,
    write_byte: CollectCounter,
}

impl MemAlignCollector {
    pub fn new(mem_align_checkpoint: &MemAlignCheckPoint) -> Self {
        Self {
            inputs: Vec::new(),
            full_5: mem_align_checkpoint.full_5,
            full_3: mem_align_checkpoint.full_3,
            full_2: mem_align_checkpoint.full_2,
            read_byte: mem_align_checkpoint.read_byte,
            write_byte: mem_align_checkpoint.write_byte,
        }
    }
    fn input_push_read(&mut self, addr: u32, bytes: u8, data: &[u64]) {
        let step = MemBusData::get_step(data);
        if step >= 58692093 && step <= 58692095 {
            println!("\x1B[1;36mMEM_DEBUG: INPUT COLLECTOR addr:{addr} step:{step} bytes:{bytes} write:false\x1B[0m");
        }
        let mem_values = MemBusData::get_mem_values(data);
        self.inputs.push(MemAlignInput {
            addr,
            is_write: false,
            width: bytes,
            value: MemHelpers::get_read_value(addr, bytes, mem_values),
            step: MemBusData::get_step(data),
            mem_values,
        });
    }
    fn input_push_write(&mut self, addr: u32, bytes: u8, data: &[u64]) {
        let step = MemBusData::get_step(data);
        if step >= 58692093 && step <= 58692095 {
            println!("\x1B[1;36mMEM_DEBUG: INPUT COLLECTOR addr:{addr} step:{step} bytes:{bytes} write:true\x1B[0m");
        }

        self.inputs.push(MemAlignInput {
            addr,
            is_write: true,
            width: bytes,
            value: MemBusData::get_value(data),
            step: MemBusData::get_step(data),
            mem_values: MemBusData::get_mem_values(data),
        });
    }
    pub fn count(&self) -> u32 {
        self.full_2.count()
            + self.full_3.count()
            + self.full_5.count()
            + self.read_byte.count()
            + self.write_byte.count()
    }
}

impl BusDevice<u64> for MemAlignCollector {
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == MEM_BUS_ID);

        let bytes = MemBusData::get_bytes(data);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        let step = MemBusData::get_step(data);
        let debug = step >= 58692093 && step <= 58692095;
        {
            let addr = MemBusData::get_addr(data);

            if debug {
                println!("\x1B[1;36mMEM_ALIGN_DEBUG: INPUT PROCESS DATA addr:{addr} step:{step} bytes:{bytes} write:false data:{:?}\x1B[0m", data);
            }
        }
        if bytes == 1 {
            if is_write {
                if (MemBusData::get_value(data) & 0xFFFF_FFFF_FFFF_FF00) == 0 {
                    if !self.write_byte.should_skip() {
                        if debug {
                            println!("\x1B[1;36mMEM_ALIGN_DEBUG: COLLECT WRITE_BYTE\x1B[0m");
                        }
                        self.input_push_write(MemBusData::get_addr(data), bytes, data);
                    } else if debug {
                        println!("\x1B[1;36mMEM_ALIGN_DEBUG: SKIP WRITE_BYTE\x1B[0m");
                    }
                    return true;
                }
            } else {
                if !self.read_byte.should_skip() {
                    if debug {
                        println!(
                            "\x1B[1;36mMEM_ALIGN_DEBUG: COLLECT READ_BYTE ({}/{})\x1B[0m",
                            self.read_byte.collected, self.read_byte.collect_count
                        );
                    }
                    self.input_push_read(MemBusData::get_addr(data), bytes, data);
                } else if debug {
                    println!("\x1B[1;36mMEM_ALIGN_DEBUG: SKIP READ_BYTE\x1B[0m");
                }
                return true;
            }
        }
        let addr = MemBusData::get_addr(data);
        if MemHelpers::is_aligned(addr, bytes) {
            return true;
        }
        let ops_by_addr = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
        let rows = if is_write { 1 + 2 * ops_by_addr } else { 1 + ops_by_addr };
        match rows as u8 {
            5 => {
                if !self.full_5.should_skip() {
                    if debug {
                        println!("\x1B[1;36mMEM_ALIGN_DEBUG: COLLECT FULL_5\x1B[0m");
                    }

                    self.input_push_write(addr, bytes, data);
                } else if debug {
                    println!("\x1B[1;36mMEM_ALIGN_DEBUG: SKIP FULL_5\x1B[0m");
                }
            }
            3 => {
                if !self.full_3.should_skip() {
                    if debug {
                        println!("\x1B[1;36mMEM_ALIGN_DEBUG: COLLECT FULL_3\x1B[0m");
                    }
                    if is_write {
                        self.input_push_write(addr, bytes, data);
                    } else {
                        self.input_push_read(addr, bytes, data);
                    }
                } else if debug {
                    println!("\x1B[1;36mMEM_ALIGN_DEBUG: SKIP FULL_3\x1B[0m");
                }
            }
            2 => {
                if !self.full_2.should_skip() {
                    if debug {
                        println!("\x1B[1;36mMEM_ALIGN_DEBUG: COLLECT FULL_2\x1B[0m");
                    }
                    self.input_push_read(addr, bytes, data);
                } else if debug {
                    println!("\x1B[1;36mMEM_ALIGN_DEBUG: SKIP FULL_2\x1B[0m");
                }
            }
            _ => panic!("Invalid mem_align_op_rows {}", rows),
        };
        true
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
