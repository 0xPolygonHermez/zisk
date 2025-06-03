use std::{
    fs::File,
    io::{BufWriter, Write},
};

use zisk_pil::MemTrace;

use crate::MemHelpers;

#[derive(Default, Debug, Clone)]
struct MemOp {
    addr: u32,
    flags: u16, // internal(1) | order(1) |  offset(3) | bytes(3) |
    step: u64,
}

#[derive(Default, Debug)]
pub struct MemDebug {
    ops: Vec<MemOp>,
    prepared: bool,
}

#[allow(dead_code)]
impl MemDebug {
    pub fn new() -> Self {
        MemDebug { ops: Vec::new(), prepared: false }
    }
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    pub fn add(&mut self, data: &MemDebug) {
        assert!(!self.prepared);
        self.ops.extend_from_slice(&data.ops);
    }
    pub fn log(&mut self, addr: u32, step: u64, bytes: u8, is_write: bool, is_internal: bool) {
        if addr < 0xA000_0000 {
            return;
        }
        assert!(!self.prepared);
        let addr_w = MemHelpers::get_addr_w(addr);
        let aligned_addr = addr_w * 8;
        if MemHelpers::is_aligned(addr, bytes) {
            self.ops.push(MemOp {
                addr: aligned_addr,
                step,
                flags: Self::to_flags(0, 0, is_write, 0, is_internal),
            });
            return;
        }
        let offset = MemHelpers::get_byte_offset(addr);
        let is_double = MemHelpers::is_double(addr, bytes);
        self.ops.push(MemOp {
            addr: aligned_addr,
            step,
            flags: Self::to_flags(offset, bytes, false, 0, is_internal),
        });
        if is_double {
            self.ops.push(MemOp {
                addr: aligned_addr + 8,
                step,
                flags: Self::to_flags(offset, bytes, false, 1, is_internal),
            });
        }
        if is_write {
            self.ops.push(MemOp {
                addr: aligned_addr,
                step: step + 1,
                flags: Self::to_flags(offset, bytes, true, 0, is_internal),
            });
            if is_double {
                self.ops.push(MemOp {
                    addr: aligned_addr + 8,
                    step: step + 1,
                    flags: Self::to_flags(offset, bytes, true, 1, is_internal),
                });
            }
        }
    }
    fn to_flags(offset: u8, bytes: u8, is_write: bool, order: u8, internal: bool) -> u16 {
        let offset = (offset as u16) & 0x07;
        let bytes = match bytes {
            0 => 0,
            1 => 1,
            2 => 2,
            4 => 3,
            8 => 4,
            _ => panic!("Invalid bytes {}", bytes),
        };
        let order = (order as u16) & 0x01;
        let flags: u16 = if is_write { 0x100 } else { 0 }
            | if internal { 0x80 } else { 0 }
            | (order << 6)
            | (offset << 3)
            | bytes;
        flags
    }
    pub fn flags_to_order(flags: u16) -> u8 {
        ((flags >> 6) & 0x01) as u8
    }
    pub fn flags_to_offset(flags: u16) -> u8 {
        ((flags >> 3) & 0x07) as u8
    }
    pub fn flags_to_bytes(flags: u16) -> u8 {
        match flags & 0x07 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 4,
            4 => 8,
            _ => panic!("Invalid bytes on flag {:X}", flags),
        }
    }
    pub fn flags_to_write(flags: u16) -> bool {
        (flags & 0x100) != 0
    }
    pub fn flags_to_internal(flags: u16) -> bool {
        (flags & 0x80) != 0
    }
    pub fn flags_set_internal(flags: u16) -> u16 {
        flags | 0x80
    }
    fn prepare(&mut self) -> bool {
        if self.prepared {
            return false;
        }
        if self.ops.is_empty() {
            self.prepared = true;
            return false;
        }
        println!("[MemDebug] sorting information .....");
        self.ops.sort_by_key(|op| (op.addr, op.step));

        true
    }
    pub fn save_to_file(&mut self, file_name: &str) {
        if !self.prepare() {
            return;
        }
        println!("[MemDebug] writing information {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = MemTrace::<usize>::NUM_ROWS;
        for (i, op) in self.ops.iter().enumerate() {
            let extra = if (i % num_rows) == 0 {
                format!(" ======= INSTANCE {} ==========", i / num_rows)
            } else {
                "".to_string()
            };
            let flags_info = if op.flags == 0 {
                "".to_string()
            } else if op.flags == 0x100 {
                "W".to_string()
            } else {
                format!(
                    " {} {}bytes:{} offset:{} order:{}",
                    if Self::flags_to_write(op.flags) { "W" } else { "R" },
                    if Self::flags_to_internal(op.flags) { "(internal) " } else { "" },
                    Self::flags_to_bytes(op.flags),
                    Self::flags_to_offset(op.flags),
                    Self::flags_to_order(op.flags),
                )
            };
            writeln!(writer, "{:#010X} {:#12}{}{}", op.addr, op.step, flags_info, extra).unwrap();
        }
        println!("[MemDebug] done");
    }
}
