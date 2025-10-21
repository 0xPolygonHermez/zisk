use std::{
    fs::File,
    io::{BufWriter, Write},
};

use zisk_common::MemBusData;

use crate::MemHelpers;

#[derive(Default, Debug, Clone)]
struct MemOp {
    addr: u32,
    flags: u16, // internal(1) | order(1) |  offset(3) | bytes(3) |
    step: u64,
    step_dual: u64,
    value: u64,
}

#[derive(Default, Debug)]
pub struct MemDebug {
    ops: Vec<MemOp>,
    prepared: bool,
    count: usize,
    indirect_count: usize,
    dual_count: usize,
    from_addr: u32,
    to_addr: u32,
    _is_dual: bool,
    max_step: u64,
}

#[derive(Debug, Copy, Clone)]
struct ChunkInfo {
    from_addr: u32,
    to_addr: u32,
    from_skip: u32,
    to_count: u32,
    to_single_count: u32,
    count: u32,
    dual_count: u32,
}

impl ChunkInfo {
    fn new() -> Self {
        Self {
            from_addr: 0xFFFF_FFFF,
            to_addr: 0,
            from_skip: 0,
            to_count: 0,
            to_single_count: 0,
            count: 0,
            dual_count: 0,
        }
    }
}

impl Default for ChunkInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl MemDebug {
    pub fn new(from_addr: u32, to_addr: u32, is_dual: bool) -> Self {
        MemDebug {
            ops: Vec::new(),
            prepared: false,
            count: 0,
            from_addr,
            to_addr,
            _is_dual: is_dual,
            dual_count: 0,
            indirect_count: 0,
            max_step: 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    pub fn add(&mut self, data: &MemDebug) {
        assert!(!self.prepared);
        self.ops.extend_from_slice(&data.ops);
    }
    pub fn log(
        &mut self,
        addr: u32,
        step: u64,
        bytes: u8,
        is_write: bool,
        is_internal: bool,
        data: &[u64],
    ) {
        assert!(bytes == 1 || bytes == 2 || bytes == 4 || bytes == 8);
        if addr < self.from_addr || addr > self.to_addr {
            return;
        }
        assert!(!self.prepared);
        self.count += 1;
        let addr_w = MemHelpers::get_addr_w(addr);
        let aligned_addr = addr_w * 8;
        if MemHelpers::is_aligned(addr, bytes) {
            let value = if is_write {
                MemBusData::get_value(data)
            } else {
                MemBusData::get_mem_values(data)[0]
            };
            self.push_op(
                aligned_addr,
                step,
                Self::to_flags(0, 0, is_write, 0, is_internal),
                value,
                false,
            );
            return;
        }
        let offset = MemHelpers::get_byte_offset(addr);
        let is_double = MemHelpers::is_double(addr, bytes);
        let mem_values = MemBusData::get_mem_values(data);
        self.push_op(
            aligned_addr,
            step,
            Self::to_flags(offset, bytes, false, 0, is_internal),
            mem_values[0],
            true,
        );
        if is_double {
            self.push_op(
                aligned_addr + 8,
                step,
                Self::to_flags(offset, bytes, false, 1, is_internal),
                mem_values[1],
                true,
            );
        }
        if is_write {
            let write_values =
                MemHelpers::get_write_values(addr, bytes, MemBusData::get_value(data), mem_values);
            self.push_op(
                aligned_addr,
                step + 1,
                Self::to_flags(offset, bytes, true, 0, is_internal),
                write_values[0],
                true,
            );
            if is_double {
                self.push_op(
                    aligned_addr + 8,
                    step + 1,
                    Self::to_flags(offset, bytes, true, 1, is_internal),
                    write_values[1],
                    true,
                );
            }
        }
    }
    fn push_op(&mut self, addr: u32, step: u64, flags: u16, value: u64, indirect: bool) {
        if indirect {
            self.indirect_count += 1;
        } else {
            self.count += 1;
        }
        if step > self.max_step {
            self.max_step = step + 1;
        };
        self.ops.push(MemOp { addr, step, flags, step_dual: 0, value });
    }
    fn to_flags(offset: u8, bytes: u8, is_write: bool, order: u8, internal: bool) -> u16 {
        let offset = (offset as u16) & 0x07;
        let bytes = match bytes {
            0 => 0,
            1 => 1,
            2 => 2,
            4 => 3,
            8 => 4,
            _ => panic!("Invalid bytes {bytes}"),
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
            _ => panic!("Invalid bytes on flag {flags:X}"),
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
    pub fn get_total(&self) -> usize {
        self.count + self.indirect_count
    }
    pub fn get_direct(&self) -> usize {
        self.count
    }
    pub fn get_indirect(&self) -> usize {
        self.indirect_count
    }
    pub fn get_dual(&self) -> usize {
        self.dual_count
    }
    pub fn prepare(&mut self) {
        if self.prepared {
            return;
        }
        if self.ops.is_empty() {
            self.prepared = true;
            return;
        }
        println!("[MemDebug] sorting information .....");
        self.ops.sort_by_key(|op| (op.addr, op.step));
        self.prepared = true;
    }
    pub fn apply_dual(&mut self) {
        self.prepare();
        assert!(self.dual_count == 0);
        let ops = std::mem::take(&mut self.ops);
        let mut index = 0;
        while index < ops.len() {
            let chunk_id = MemHelpers::mem_step_to_chunk(ops[index].step);
            let op = &ops[index];
            if index + 1 < ops.len() {
                let op2 = &ops[index + 1];
                let chunk_id_2 = MemHelpers::mem_step_to_chunk(op2.step);
                if op.addr == op2.addr && chunk_id == chunk_id_2 && !Self::flags_to_write(op2.flags)
                {
                    self.ops.push(MemOp {
                        addr: op.addr,
                        step: op.step,
                        flags: op.flags,
                        step_dual: op2.step,
                        value: op.value,
                    });
                    self.dual_count += 1;
                    index += 2;
                    continue;
                }
            }
            self.ops.push(MemOp {
                addr: op.addr,
                step: op.step,
                flags: op.flags,
                step_dual: op.step_dual,
                value: op.value,
            });
            index += 1;
        }
    }
    pub fn count_n_dual(&mut self, n_dual: usize) -> (u32, u32) {
        self.prepare();
        assert!(self.dual_count == 0);
        let mut index = 0;
        let count = self.ops.len();
        let mut dual_count = 0;
        let mut dual_rows = 0;
        while index < count {
            let op = &self.ops[index];
            let chunk_id = MemHelpers::mem_step_to_chunk(op.step);
            for n in 0..n_dual {
                if index + 1 < self.ops.len() {
                    let op2 = &self.ops[index + 1];
                    let chunk_id_2 = MemHelpers::mem_step_to_chunk(op2.step);
                    if op.addr == op2.addr
                        && chunk_id == chunk_id_2
                        && !Self::flags_to_write(op2.flags)
                    {
                        dual_count += 1;
                        if n == 0 {
                            dual_count += 1;
                            dual_rows += 1;
                        }
                        index += 1;
                    }
                }
            }
            index += 1;
        }
        (dual_rows, dual_count)
    }
    pub fn save_to_file(&mut self, num_rows: usize, file_name: &str) {
        self.prepare();
        println!("[MemDebug] writing information {file_name} .....");
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
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
    pub fn dump_to_file(&mut self, num_rows: usize, file_name: &str) {
        println!("[MemDebug] writing information {file_name} .....");
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        for (i, op) in self.ops.iter().enumerate() {
            if (i % num_rows) == 0 {
                writeln!(writer, " ======= INSTANCE {} ==========", i / num_rows).unwrap();
            };

            writeln!(
                writer,
                "#@# 0x{:08X}|{}|8|{}|{}|{}",
                op.addr,
                op.step,
                if Self::flags_to_write(op.flags) { "W" } else { "R" },
                op.value as u32,
                (op.value >> 32) as u32
            )
            .unwrap();
            if op.step_dual > 0 {
                writeln!(
                    writer,
                    "#@# 0x{:08X}|{}|8|R|{}|{}",
                    op.addr,
                    op.step_dual,
                    op.value as u32,
                    (op.value >> 32) as u32
                )
                .unwrap();
            }
        }
        println!("[MemDebug] done");
    }
    pub fn info_instances(&self, rows: usize) {
        let count = self.ops.len();
        assert!(count > 0);
        let instances = ((count - 1) / rows) + 1;
        println!("instances: {instances}");
        for instance in 0..instances {
            println!("### instance {instance} ###");
            let first_row = instance * rows;
            let row_0 = &self.ops[first_row];
            println!(
                "ROW {:8}: 0x{:08X} {:12} {} {:12}",
                0,
                row_0.addr,
                row_0.step,
                if Self::flags_to_write(row_0.flags) { "W" } else { "R" },
                row_0.step_dual
            );
            let last_row = (instance + 1) * rows - 1;
            let last_row_with_data = std::cmp::min(last_row, count - 1);

            let row = &self.ops[last_row_with_data];
            println!(
                "ROW {:8}: 0x{:08X} {:12} {} {:12}",
                last_row_with_data % rows,
                row.addr,
                row.step,
                if Self::flags_to_write(row.flags) { "W" } else { "R" },
                row.step_dual
            );
            if last_row != last_row_with_data {
                let padding = last_row - last_row_with_data;
                println!(
                    "PADDING: {padding} ({:.2}% used)",
                    ((rows - padding) as f32 * 100.0) / rows as f32
                );
            }
        }
    }
    pub fn info_chunks(&self, rows: usize) {
        let count = self.ops.len();
        assert!(count > 0);
        let max_chunk_id = MemHelpers::mem_step_to_chunk(self.max_step);
        let chunk_count = usize::from(max_chunk_id) + 1;
        let instances = ((count - 1) / rows) + 1;
        println!("instances: {instances}");
        for instance in 0..instances {
            println!("### instance {instance} ###");
            let mut chunk_info: Vec<ChunkInfo> = vec![ChunkInfo::default(); chunk_count];
            let first_row = instance * rows;
            let last_row = std::cmp::min((instance + 1) * rows - 1, count - 1);
            for i_row in first_row..=last_row {
                let row = &self.ops[i_row];

                for step in [row.step, row.step_dual] {
                    if step == 0 {
                        break;
                    }
                    let chunk_id = usize::from(MemHelpers::mem_step_to_chunk(step));
                    let addr = row.addr;
                    let chunk = &mut chunk_info[chunk_id];
                    if chunk.from_addr > addr {
                        chunk.from_addr = addr;
                    }
                    if chunk.to_addr < addr {
                        chunk.to_addr = addr;
                    }
                    chunk.count += 1;
                    if row.step_dual == step {
                        chunk.dual_count += 1;
                    }
                }
            }
            for i_row in first_row..=last_row {
                let row = &self.ops[i_row];
                let addr = row.addr;
                let chunk_id = usize::from(MemHelpers::mem_step_to_chunk(row.step));
                assert!(
                    row.step_dual == 0 || MemHelpers::mem_step_to_chunk(row.step_dual) == chunk_id
                );
                let chunk = &mut chunk_info[chunk_id];
                if chunk.to_addr == addr {
                    if row.step_dual == 0 {
                        chunk.to_count += 1;
                        chunk.to_single_count += 2;
                    } else {
                        chunk.to_single_count += 2;
                        chunk.to_count += 1;
                    }
                }
            }
            for (chunk_id, chunk) in chunk_info.iter().enumerate().take(chunk_count) {
                println!(
                    "====: CHUNK_{}: (0x{:08X},{})  (0x{:08X},{},{}) count:{} dual_count:{}",
                    chunk_id,
                    chunk.from_addr,
                    chunk.from_skip,
                    chunk.to_addr,
                    chunk.to_count,
                    chunk.to_single_count,
                    chunk.count,
                    chunk.dual_count
                );
            }
        }
    }
}
