use zisk_core::{
    INPUT_ADDR, MAX_INPUT_SIZE, RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_ADDR_MAX, STACK_ADDR, STACK_SIZE,
};

/// Keeps counters for every type of memory operation (including registers).
///
/// Since RISC-V registers are mapped to memory, memory operations include register access
/// operations.
use crate::{
    emu_costs::{
        MEM_ALIGN_READ_BYTE_COST, MEM_ALIGN_READ_UNALIGNED_1_COST, MEM_ALIGN_READ_UNALIGNED_2_COST,
        MEM_ALIGN_WRITE_BYTE_COST, MEM_READ_COST, MEM_WRITE_COST,
    },
    INPUT_READ_COST, MEM_ALIGN_WRITE_UNALIGNED_1_COST, MEM_ALIGN_WRITE_UNALIGNED_2_COST,
    ROM_READ_COST,
};

/// Status code returned by `memory_read` / `memory_write` (u32):
/// - bits 0..16 — classification code (`MEM_CODE_*`): the shape of the access.
/// - bit 16 (`MEM_ACCESS_INVALID`) — the address is outside every known memory region; the caller
///   must treat it as an unauthorized access and error out.
/// - bit 17 (`MEM_ACCESS_MONITOR`) — the caller should log the access with full execution context.
pub const MEM_ACCESS_CODE_MASK: u32 = 0xFFFF;
pub const MEM_ACCESS_INVALID: u32 = 1 << 16;
pub const MEM_ACCESS_MONITOR: u32 = 1 << 17;

// Classification codes (low 16 bits of the status).
pub const MEM_CODE_ALIGNED_8B: u32 = 0;
pub const MEM_CODE_UNALIGNED_1B: u32 = 1;
pub const MEM_CODE_UNALIGNED_2B_SINGLE: u32 = 2;
pub const MEM_CODE_UNALIGNED_2B_DOUBLE: u32 = 3;
pub const MEM_CODE_UNALIGNED_4B_SINGLE: u32 = 4;
pub const MEM_CODE_UNALIGNED_4B_DOUBLE: u32 = 5;
pub const MEM_CODE_UNALIGNED_8B_DOUBLE: u32 = 6;

#[derive(Default, Debug, Clone)]
pub struct MemoryStatsItem {
    pub count: u64,
    pub cost: u64,
    pub write: bool,
    pub dirty: bool,
    pub double: bool,
    pub aligned_32: bool,
    pub width: u64,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryReadZoneStatsData {
    aligned_8b: u64,
    unaligned_1b: u64,
    unaligned_2b_single: u64,
    unaligned_2b_double: u64,
    unaligned_4b_single_32_align: u64,
    unaligned_4b_single_non_32_align: u64,
    unaligned_4b_double: u64,
    unaligned_8b_double: u64,
}
#[derive(Default, Debug, Clone)]
pub struct MemoryWriteZoneStatsData {
    aligned_8b: u64,
    unaligned_1b_clean: u64,
    unaligned_1b_dirty: u64,
    unaligned_2b_single: u64,
    unaligned_2b_double: u64,
    unaligned_4b_single_32_align_clean: u64,
    unaligned_4b_single_32_align_dirty: u64,
    unaligned_4b_single_non_32_align: u64,
    unaligned_4b_double: u64,
    unaligned_8b_double: u64,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryReadWriteZoneStatsData {
    reads: MemoryReadZoneStatsData,
    writes: MemoryWriteZoneStatsData,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryOperationsStats {
    rom: MemoryReadZoneStatsData,
    ram_stack: MemoryReadWriteZoneStatsData,
    ram_no_stack: MemoryReadWriteZoneStatsData,
    input: MemoryReadZoneStatsData,
}

impl MemoryOperationsStats {
    /// Creates a new MemoryOperations structure with all counters set to zero.
    pub fn new() -> Self {
        Self::default()
    }
    /// Accounts a memory write and returns its status code (see `MEM_ACCESS_*`). Returns
    /// `MEM_ACCESS_INVALID` if `address` falls outside every known memory region (an unauthorized
    /// access); the caller reports it with execution context instead of panicking here.
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) -> u32 {
        if (STACK_ADDR..(STACK_ADDR + STACK_SIZE)).contains(&address) {
            self.ram_stack.memory_write(address, width, value)
        } else if (RAM_ADDR..(RAM_ADDR + RAM_SIZE)).contains(&address) {
            self.ram_no_stack.memory_write(address, width, value)
        } else {
            MEM_ACCESS_INVALID
        }
    }
    /// Accounts a memory read and returns its status code (see `MEM_ACCESS_*`). Returns
    /// `MEM_ACCESS_INVALID` if `address` falls outside every known memory region (an unauthorized
    /// access); the caller reports it with execution context instead of panicking here.
    pub fn memory_read(&mut self, address: u64, width: u64) -> u32 {
        if (STACK_ADDR..(STACK_ADDR + STACK_SIZE)).contains(&address) {
            self.ram_stack.memory_read(address, width)
        } else if (RAM_ADDR..(RAM_ADDR + RAM_SIZE)).contains(&address) {
            self.ram_no_stack.memory_read(address, width)
        } else if (ROM_ADDR..=ROM_ADDR_MAX).contains(&address) {
            self.rom.memory_read(address, width)
        } else if (INPUT_ADDR..(INPUT_ADDR + MAX_INPUT_SIZE)).contains(&address) {
            self.input.memory_read(address, width)
        } else {
            MEM_ACCESS_INVALID
        }
    }
    pub fn get_ram_cost(&self) -> u64 {
        self.get_ram_aligned_cost() + self.get_ram_unaligned_cost()
    }
    pub fn get_ram_stack_cost(&self) -> u64 {
        self.get_ram_stack_aligned_cost() + self.get_ram_stack_unaligned_cost()
    }
    pub fn get_ram_no_stack_cost(&self) -> u64 {
        self.get_ram_no_stack_aligned_cost() + self.get_ram_no_stack_unaligned_cost()
    }
    pub fn get_rom_cost(&self) -> u64 {
        self.get_rom_aligned_cost() + self.get_rom_unaligned_cost()
    }
    pub fn get_input_cost(&self) -> u64 {
        self.get_input_aligned_cost() + self.get_input_unaligned_cost()
    }
    pub fn get_rom_unaligned_cost(&self) -> u64 {
        self.rom.get_unaligned_cost(ROM_READ_COST)
    }
    pub fn get_input_unaligned_cost(&self) -> u64 {
        self.input.get_unaligned_cost(INPUT_READ_COST)
    }
    pub fn get_ram_unaligned_cost(&self) -> u64 {
        self.get_ram_stack_unaligned_cost() + self.get_ram_no_stack_unaligned_cost()
    }
    pub fn get_ram_stack_unaligned_cost(&self) -> u64 {
        self.ram_stack.get_unaligned_cost(MEM_READ_COST, MEM_WRITE_COST)
    }
    pub fn get_ram_no_stack_unaligned_cost(&self) -> u64 {
        self.ram_no_stack.get_unaligned_cost(MEM_READ_COST, MEM_WRITE_COST)
    }
    pub fn get_rom_aligned_cost(&self) -> u64 {
        self.rom.get_aligned_cost(ROM_READ_COST)
    }
    pub fn get_input_aligned_cost(&self) -> u64 {
        self.input.get_aligned_cost(INPUT_READ_COST)
    }
    pub fn get_ram_aligned_cost(&self) -> u64 {
        self.get_ram_stack_aligned_cost() + self.get_ram_no_stack_aligned_cost()
    }
    pub fn get_ram_stack_aligned_cost(&self) -> u64 {
        self.ram_stack.get_aligned_cost(MEM_READ_COST, MEM_WRITE_COST)
    }
    pub fn get_ram_no_stack_aligned_cost(&self) -> u64 {
        self.ram_no_stack.get_aligned_cost(MEM_READ_COST, MEM_WRITE_COST)
    }
    pub fn get_cost(&self) -> u64 {
        self.get_aligned_cost() + self.get_unaligned_cost()
    }
    pub fn get_aligned_cost(&self) -> u64 {
        self.get_ram_aligned_cost() + self.get_rom_aligned_cost() + self.get_input_aligned_cost()
    }
    pub fn get_unaligned_cost(&self) -> u64 {
        self.get_ram_unaligned_cost()
            + self.get_rom_unaligned_cost()
            + self.get_input_unaligned_cost()
    }

    pub fn get_ram_count(&self) -> u64 {
        self.get_ram_aligned_count() + self.get_ram_unaligned_count()
    }
    pub fn get_ram_stack_count(&self) -> u64 {
        self.get_ram_stack_aligned_count() + self.get_ram_stack_unaligned_count()
    }
    pub fn get_ram_no_stack_count(&self) -> u64 {
        self.get_ram_no_stack_aligned_count() + self.get_ram_no_stack_unaligned_count()
    }
    pub fn get_rom_count(&self) -> u64 {
        self.get_rom_aligned_count() + self.get_rom_unaligned_count()
    }
    pub fn get_input_count(&self) -> u64 {
        self.get_input_aligned_count() + self.get_input_unaligned_count()
    }
    pub fn get_rom_unaligned_count(&self) -> u64 {
        self.rom.get_unaligned_count()
    }
    pub fn get_input_unaligned_count(&self) -> u64 {
        self.input.get_unaligned_count()
    }
    pub fn get_ram_unaligned_count(&self) -> u64 {
        self.ram_stack.get_unaligned_count() + self.ram_no_stack.get_unaligned_count()
    }
    pub fn get_ram_stack_unaligned_count(&self) -> u64 {
        self.ram_stack.get_unaligned_count()
    }
    pub fn get_ram_no_stack_unaligned_count(&self) -> u64 {
        self.ram_no_stack.get_unaligned_count()
    }
    pub fn get_rom_aligned_count(&self) -> u64 {
        self.rom.get_aligned_count()
    }
    pub fn get_input_aligned_count(&self) -> u64 {
        self.input.get_aligned_count()
    }
    pub fn get_ram_aligned_count(&self) -> u64 {
        self.ram_stack.get_aligned_count() + self.ram_no_stack.get_aligned_count()
    }
    pub fn get_ram_stack_aligned_count(&self) -> u64 {
        self.ram_stack.get_aligned_count()
    }
    pub fn get_ram_no_stack_aligned_count(&self) -> u64 {
        self.ram_no_stack.get_aligned_count()
    }
    pub fn get_count(&self) -> u64 {
        self.get_aligned_count() + self.get_unaligned_count()
    }
    pub fn get_aligned_count(&self) -> u64 {
        self.get_ram_aligned_count() + self.get_rom_aligned_count() + self.get_input_aligned_count()
    }
    pub fn get_unaligned_count(&self) -> u64 {
        self.get_ram_unaligned_count()
            + self.get_rom_unaligned_count()
            + self.get_input_unaligned_count()
    }
    pub fn add_delta(
        &mut self,
        reference: &MemoryOperationsStats,
        current: &MemoryOperationsStats,
    ) {
        self.rom.add_delta(&reference.rom, &current.rom);
        self.ram_stack.add_delta(&reference.ram_stack, &current.ram_stack);
        self.ram_no_stack.add_delta(&reference.ram_no_stack, &current.ram_no_stack);
        self.input.add_delta(&reference.input, &current.input);
    }
    pub fn get_detailed_items(
        &self,
        rom_init_count: u64,
        ram_init_count: u64,
    ) -> Vec<(String, u64, u64)> {
        let mut report_items = self.ram_stack.get_detailed_items(
            "RAM STACK",
            MEM_READ_COST,
            MEM_WRITE_COST,
            ram_init_count,
        );
        report_items.append(&mut self.ram_no_stack.get_detailed_items(
            "RAM NO STACK",
            MEM_READ_COST,
            MEM_WRITE_COST,
            0,
        ));
        report_items.append(&mut self.rom.get_detailed_items("ROM", ROM_READ_COST, rom_init_count));
        report_items.append(&mut self.input.get_detailed_items("INPUT", INPUT_READ_COST, 0));
        let mut items = self.ram_stack.get_items(MEM_READ_COST, MEM_WRITE_COST, ram_init_count);
        items.append(&mut self.ram_no_stack.get_items(MEM_READ_COST, MEM_WRITE_COST, 0));
        items.append(&mut self.rom.get_items(ROM_READ_COST, rom_init_count));
        items.append(&mut self.input.get_items(INPUT_READ_COST, 0));

        report_items.push(("".to_string(), 0, 0));

        const WIDTHS_SINGLE_DOUBLE: [(&str, u64); 7] = [
            ("aligned 8B", 0),
            ("unaligned 1B single", 1),
            ("unaligned 2B single", 2),
            ("unaligned 2B double", 102),
            ("unaligned 4B single", 4),
            ("unaligned 4B double", 104),
            ("unaligned 8B double", 108),
        ];

        for (label, key) in WIDTHS_SINGLE_DOUBLE.iter() {
            let (count, cost) = items.iter().fold((0, 0), |acc, item| {
                let item_key = if !item.double && item.width == 8 {
                    0
                } else {
                    item.width + item.double as u64 * 100
                };
                if item_key == *key {
                    (acc.0 + item.count, acc.1 + item.cost)
                } else {
                    acc
                }
            });

            if count > 0 {
                report_items.push((format!("TOTAL {}", label), count, cost));
            }
        }

        report_items.push(("".to_string(), 0, 0));

        const WIDTHS: [(&str, u64); 5] = [
            ("aligned 8B", 0),
            ("unaligned 1B", 1),
            ("unaligned 2B", 2),
            ("unaligned 4B", 4),
            ("unaligned 8B", 8),
        ];

        for (label, key) in WIDTHS.iter() {
            let (count, cost) = items.iter().fold((0, 0), |acc, item| {
                let item_key = if !item.double && item.width == 8 { 0 } else { item.width };
                if item_key == *key {
                    (acc.0 + item.count, acc.1 + item.cost)
                } else {
                    acc
                }
            });

            if count > 0 {
                report_items.push((format!("TOTAL {}", label), count, cost));
            }
        }

        report_items.push(("".to_string(), 0, 0));

        const READ_WRITE: [(&str, bool); 2] = [("reads", false), ("writes", true)];

        let (read, clean_write, dirty_write) =
            items.iter().fold(((0, 0), (0, 0), (0, 0)), |acc, item| {
                if item.aligned_32 {
                    if !item.write {
                        ((acc.0 .0 + item.count, acc.0 .1 + item.cost), acc.1, acc.2)
                    } else if item.dirty {
                        (acc.0, acc.1, (acc.2 .0 + item.count, acc.2 .1 + item.cost))
                    } else {
                        (acc.0, (acc.1 .0 + item.count, acc.1 .1 + item.cost), acc.2)
                    }
                } else {
                    acc
                }
            });

        if read.0 > 0 {
            report_items.push(("TOTAL unaligned 4B 32-align read".to_string(), read.0, read.1));
        }
        if clean_write.0 > 0 {
            report_items.push((
                "TOTAL unaligned 4B 32-align clean write".to_string(),
                clean_write.0,
                clean_write.1,
            ));
        }
        if dirty_write.0 > 0 {
            report_items.push((
                "TOTAL unaligned 4B 32-align dirty write".to_string(),
                dirty_write.0,
                dirty_write.1,
            ));
        }
        report_items.push(("".to_string(), 0, 0));

        for (label, key) in READ_WRITE.iter() {
            let (count, cost) = items.iter().fold((0, 0), |acc, item| {
                if item.write == *key {
                    (acc.0 + item.count, acc.1 + item.cost)
                } else {
                    acc
                }
            });

            if count > 0 {
                report_items.push((format!("TOTAL {}", label), count, cost));
            }
        }

        report_items.push(("".to_string(), 0, 0));
        let (count, cost) =
            items.iter().fold((0, 0), |acc, item| (acc.0 + item.count, acc.1 + item.cost));
        report_items.push(("TOTAL".to_string(), count, cost));
        report_items
    }
}

impl MemoryReadZoneStatsData {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn memory_read(&mut self, address: u64, width: u64) -> u32 {
        if ((address & 0x07) == 0) && (width == 8) {
            self.aligned_8b += 1;
            MEM_CODE_ALIGNED_8B
        } else {
            let offset = address & 0x07;
            match width {
                1 => {
                    self.unaligned_1b += 1;
                    MEM_CODE_UNALIGNED_1B
                }
                2 => {
                    if offset > 6 {
                        self.unaligned_2b_double += 1;
                        MEM_CODE_UNALIGNED_2B_DOUBLE
                    } else {
                        self.unaligned_2b_single += 1;
                        MEM_CODE_UNALIGNED_2B_SINGLE
                    }
                }
                4 => {
                    if offset > 4 {
                        self.unaligned_4b_double += 1;
                        MEM_CODE_UNALIGNED_4B_DOUBLE | MEM_ACCESS_MONITOR
                    } else if offset == 0 || offset == 4 {
                        self.unaligned_4b_single_32_align += 1;
                        MEM_CODE_UNALIGNED_4B_SINGLE
                    } else {
                        self.unaligned_4b_single_non_32_align += 1;
                        MEM_CODE_UNALIGNED_4B_SINGLE
                    }
                }
                8 => {
                    self.unaligned_8b_double += 1;
                    MEM_CODE_UNALIGNED_8B_DOUBLE | MEM_ACCESS_MONITOR
                }
                _ => panic!("Invalid memory read width: {}", width),
            }
        }
    }
    pub fn get_cost(&self, read_cost: u64) -> u64 {
        self.get_aligned_cost(read_cost) + self.get_unaligned_cost(read_cost)
    }
    pub fn get_aligned_cost(&self, read_cost: u64) -> u64 {
        self.aligned_8b * read_cost
    }
    pub fn get_unaligned_cost(&self, read_cost: u64) -> u64 {
        self.unaligned_1b * (MEM_ALIGN_READ_BYTE_COST + read_cost)
            + self.unaligned_2b_single * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost)
            + self.unaligned_2b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost)
            + self.unaligned_4b_single_32_align * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost)
            + self.unaligned_4b_single_non_32_align * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost)
            + self.unaligned_4b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost)
            + self.unaligned_8b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost)
    }
    pub fn get_count(&self) -> u64 {
        self.get_aligned_count() + self.get_unaligned_count()
    }
    pub fn get_aligned_count(&self) -> u64 {
        self.aligned_8b
    }
    pub fn get_unaligned_count(&self) -> u64 {
        self.unaligned_1b
            + self.unaligned_2b_single
            + self.unaligned_2b_double
            + self.unaligned_4b_single_32_align
            + self.unaligned_4b_single_non_32_align
            + self.unaligned_4b_double
            + self.unaligned_8b_double
    }
    pub fn add_delta(
        &mut self,
        reference: &MemoryReadZoneStatsData,
        current: &MemoryReadZoneStatsData,
    ) {
        self.aligned_8b += current.aligned_8b - reference.aligned_8b;
        self.unaligned_1b += current.unaligned_1b - reference.unaligned_1b;
        self.unaligned_2b_single += current.unaligned_2b_single - reference.unaligned_2b_single;
        self.unaligned_2b_double += current.unaligned_2b_double - reference.unaligned_2b_double;
        self.unaligned_4b_single_32_align +=
            current.unaligned_4b_single_32_align - reference.unaligned_4b_single_32_align;
        self.unaligned_4b_single_non_32_align +=
            current.unaligned_4b_single_non_32_align - reference.unaligned_4b_single_non_32_align;
        self.unaligned_4b_double += current.unaligned_4b_double - reference.unaligned_4b_double;
        self.unaligned_8b_double += current.unaligned_8b_double - reference.unaligned_8b_double;
    }
    pub fn get_detailed_items(
        &self,
        title: &str,
        read_cost: u64,
        init_count: u64,
    ) -> Vec<(String, u64, u64)> {
        let mut items = Vec::new();
        let aligned_8b = self.aligned_8b + init_count;
        if aligned_8b > 0 {
            items.push((format!("{} aligned 8B read", title), aligned_8b, aligned_8b * read_cost));
        }
        if self.unaligned_1b > 0 {
            items.push((
                format!("{} unaligned 1B read", title),
                self.unaligned_1b,
                self.unaligned_1b * (MEM_ALIGN_READ_BYTE_COST + read_cost),
            ));
        }
        if self.unaligned_2b_single > 0 {
            items.push((
                format!("{} unaligned 2B single read", title),
                self.unaligned_2b_single,
                self.unaligned_2b_single * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost),
            ));
        }
        if self.unaligned_2b_double > 0 {
            items.push((
                format!("{} unaligned 2B double read", title),
                self.unaligned_2b_double,
                self.unaligned_2b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost),
            ));
        }
        if self.unaligned_4b_single_32_align > 0 {
            items.push((
                format!("{} unaligned 4B single 32-align read", title),
                self.unaligned_4b_single_32_align,
                self.unaligned_4b_single_32_align * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost),
            ));
        }
        if self.unaligned_4b_single_non_32_align > 0 {
            items.push((
                format!("{} unaligned 4B single non 32-align read", title),
                self.unaligned_4b_single_non_32_align,
                self.unaligned_4b_single_non_32_align
                    * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost),
            ));
        }
        if self.unaligned_4b_double > 0 {
            items.push((
                format!("{} unaligned 4B double read", title),
                self.unaligned_4b_double,
                self.unaligned_4b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost),
            ));
        }
        if self.unaligned_8b_double > 0 {
            items.push((
                format!("{} unaligned 8B double read", title),
                self.unaligned_8b_double,
                self.unaligned_8b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost),
            ));
        }
        items
    }
    pub fn get_items(&self, read_cost: u64, init_count: u64) -> Vec<MemoryStatsItem> {
        let mut items = Vec::new();
        let aligned_8b = self.aligned_8b + init_count;
        if aligned_8b > 0 {
            items.push(MemoryStatsItem {
                count: aligned_8b,
                cost: aligned_8b * read_cost,
                width: 8,
                ..Default::default()
            });
        }
        if self.unaligned_1b > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_1b,
                cost: self.unaligned_1b * (MEM_ALIGN_READ_BYTE_COST + read_cost),
                width: 1,
                ..Default::default()
            });
        }
        if self.unaligned_2b_single > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_2b_single,
                cost: self.unaligned_2b_single * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost),
                width: 2,
                ..Default::default()
            });
        }
        if self.unaligned_2b_double > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_2b_double,
                cost: self.unaligned_2b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost),
                double: true,
                width: 2,
                ..Default::default()
            });
        }
        if self.unaligned_4b_single_32_align > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_single_32_align,
                cost: self.unaligned_4b_single_32_align
                    * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost),
                width: 4,
                aligned_32: true,
                ..Default::default()
            });
        }
        if self.unaligned_4b_single_non_32_align > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_single_non_32_align,
                cost: self.unaligned_4b_single_non_32_align
                    * (MEM_ALIGN_READ_UNALIGNED_1_COST + read_cost),
                dirty: true,
                width: 4,
                ..Default::default()
            });
        }
        if self.unaligned_4b_double > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_double,
                cost: self.unaligned_4b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost),
                double: true,
                width: 4,
                ..Default::default()
            });
        }
        if self.unaligned_8b_double > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_8b_double,
                cost: self.unaligned_8b_double * (MEM_ALIGN_READ_UNALIGNED_2_COST + 2 * read_cost),
                double: true,
                width: 8,
                ..Default::default()
            });
        }
        items
    }
}

impl MemoryWriteZoneStatsData {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) -> u32 {
        // If the memory is alligned to 8 bytes, i.e. last 3 bits are zero, then increase the
        // aligned memory read counter

        if ((address & 0x07) == 0) && (width == 8) {
            self.aligned_8b += 1;
            MEM_CODE_ALIGNED_8B
        } else {
            let offset = address & 0x07;
            match width {
                1 => {
                    if value & 0xFFFF_FFFF_FFFF_FF00 == 0 {
                        self.unaligned_1b_clean += 1
                    } else {
                        self.unaligned_1b_dirty += 1
                    }
                    MEM_CODE_UNALIGNED_1B
                }
                2 => {
                    if offset > 6 {
                        self.unaligned_2b_double += 1;
                        MEM_CODE_UNALIGNED_2B_DOUBLE
                    } else {
                        self.unaligned_2b_single += 1;
                        MEM_CODE_UNALIGNED_2B_SINGLE
                    }
                }
                4 => {
                    if offset > 4 {
                        self.unaligned_4b_double += 1;
                        MEM_CODE_UNALIGNED_4B_DOUBLE | MEM_ACCESS_MONITOR
                    } else {
                        if offset & 0x03 == 0 {
                            if (value & 0xFFFF_FFFF_0000_0000) == 0 {
                                self.unaligned_4b_single_32_align_clean += 1;
                            } else {
                                self.unaligned_4b_single_32_align_dirty += 1;
                            }
                        } else {
                            self.unaligned_4b_single_non_32_align += 1;
                        }
                        MEM_CODE_UNALIGNED_4B_SINGLE
                    }
                }
                8 => {
                    self.unaligned_8b_double += 1;
                    MEM_CODE_UNALIGNED_8B_DOUBLE | MEM_ACCESS_MONITOR
                }
                _ => panic!("Invalid memory write width: {}", width),
            }
        }
    }
    pub fn get_cost(&self, read_cost: u64, write_cost: u64) -> u64 {
        self.get_aligned_cost(write_cost) + self.get_unaligned_cost(read_cost, write_cost)
    }
    pub fn get_aligned_cost(&self, write_cost: u64) -> u64 {
        self.aligned_8b * write_cost
    }
    pub fn get_unaligned_cost(&self, read_cost: u64, write_cost: u64) -> u64 {
        let read_write_cost = read_cost + write_cost;
        self.unaligned_1b_clean * (MEM_ALIGN_WRITE_BYTE_COST + read_write_cost)
            + self.unaligned_1b_dirty * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost)
            + self.unaligned_2b_single * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost)
            + self.unaligned_2b_double * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost)
            + self.unaligned_4b_single_32_align_clean
                * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost)
            + self.unaligned_4b_single_32_align_dirty
                * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost)
            + self.unaligned_4b_single_non_32_align
                * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost)
            + self.unaligned_4b_double * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost)
            + self.unaligned_8b_double * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost)
    }
    pub fn get_count(&self) -> u64 {
        self.get_aligned_count() + self.get_unaligned_count()
    }
    pub fn get_aligned_count(&self) -> u64 {
        self.aligned_8b
    }
    pub fn get_unaligned_count(&self) -> u64 {
        self.unaligned_1b_clean
            + self.unaligned_1b_dirty
            + self.unaligned_2b_single
            + self.unaligned_2b_double
            + self.unaligned_4b_single_32_align_clean
            + self.unaligned_4b_single_32_align_dirty
            + self.unaligned_4b_single_non_32_align
            + self.unaligned_4b_double
            + self.unaligned_8b_double
    }
    pub fn add_delta(
        &mut self,
        reference: &MemoryWriteZoneStatsData,
        current: &MemoryWriteZoneStatsData,
    ) {
        self.aligned_8b += current.aligned_8b - reference.aligned_8b;
        self.unaligned_1b_clean += current.unaligned_1b_clean - reference.unaligned_1b_clean;
        self.unaligned_1b_dirty += current.unaligned_1b_dirty - reference.unaligned_1b_dirty;
        self.unaligned_2b_single += current.unaligned_2b_single - reference.unaligned_2b_single;
        self.unaligned_2b_double += current.unaligned_2b_double - reference.unaligned_2b_double;
        self.unaligned_4b_single_32_align_clean += current.unaligned_4b_single_32_align_clean
            - reference.unaligned_4b_single_32_align_clean;
        self.unaligned_4b_single_32_align_dirty += current.unaligned_4b_single_32_align_dirty
            - reference.unaligned_4b_single_32_align_dirty;
        self.unaligned_4b_single_non_32_align +=
            current.unaligned_4b_single_non_32_align - reference.unaligned_4b_single_non_32_align;
        self.unaligned_4b_double += current.unaligned_4b_double - reference.unaligned_4b_double;
        self.unaligned_8b_double += current.unaligned_8b_double - reference.unaligned_8b_double;
    }
    pub fn get_detailed_items(
        &self,
        title: &str,
        read_cost: u64,
        write_cost: u64,
        init_count: u64,
    ) -> Vec<(String, u64, u64)> {
        let mut items = Vec::new();
        let read_write_cost = read_cost + write_cost;
        let aligned_8b = self.aligned_8b + init_count;
        if aligned_8b > 0 {
            items.push((
                format!("{} aligned 8B write", title),
                aligned_8b,
                aligned_8b * write_cost,
            ));
        }
        if self.unaligned_1b_clean > 0 {
            items.push((
                format!("{} unaligned 1B clean write", title),
                self.unaligned_1b_clean,
                self.unaligned_1b_clean * (MEM_ALIGN_WRITE_BYTE_COST + read_write_cost),
            ));
        }
        if self.unaligned_1b_dirty > 0 {
            items.push((
                format!("{} unaligned 1B dirty write", title),
                self.unaligned_1b_dirty,
                self.unaligned_1b_dirty * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
            ));
        }
        if self.unaligned_2b_single > 0 {
            items.push((
                format!("{} unaligned 2B single write", title),
                self.unaligned_2b_single,
                self.unaligned_2b_single * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
            ));
        }
        if self.unaligned_2b_double > 0 {
            items.push((
                format!("{} unaligned 2B double write", title),
                self.unaligned_2b_double,
                self.unaligned_2b_double * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost),
            ));
        }
        if self.unaligned_4b_single_32_align_clean > 0 {
            items.push((
                format!("{} unaligned 4B single 32-align clean write", title),
                self.unaligned_4b_single_32_align_clean,
                self.unaligned_4b_single_32_align_clean
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
            ));
        }
        if self.unaligned_4b_single_32_align_dirty > 0 {
            items.push((
                format!("{} unaligned 4B single 32-align dirty write", title),
                self.unaligned_4b_single_32_align_dirty,
                self.unaligned_4b_single_32_align_dirty
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
            ));
        }
        if self.unaligned_4b_single_non_32_align > 0 {
            items.push((
                format!("{} unaligned 4B single non 32-align write", title),
                self.unaligned_4b_single_non_32_align,
                self.unaligned_4b_single_non_32_align
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
            ));
        }
        if self.unaligned_4b_double > 0 {
            items.push((
                format!("{} unaligned 4B double write", title),
                self.unaligned_4b_double,
                self.unaligned_4b_double * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost),
            ));
        }
        if self.unaligned_8b_double > 0 {
            items.push((
                format!("{} unaligned 8B double write", title),
                self.unaligned_8b_double,
                self.unaligned_8b_double * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost),
            ));
        }
        items
    }
    pub fn get_items(
        &self,
        read_cost: u64,
        write_cost: u64,
        init_count: u64,
    ) -> Vec<MemoryStatsItem> {
        let read_write_cost = read_cost + write_cost;
        let aligned_8b = self.aligned_8b + init_count;
        let mut items = Vec::new();
        if aligned_8b > 0 {
            items.push(MemoryStatsItem {
                count: aligned_8b,
                cost: aligned_8b * write_cost,
                write: true,
                width: 8,
                ..Default::default()
            });
        }
        if self.unaligned_1b_clean > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_1b_clean,
                cost: self.unaligned_1b_clean * (MEM_ALIGN_WRITE_BYTE_COST + read_write_cost),
                write: true,
                width: 1,
                ..Default::default()
            });
        }
        if self.unaligned_1b_dirty > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_1b_dirty,
                cost: self.unaligned_1b_dirty
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
                write: true,
                dirty: true,
                width: 1,
                ..Default::default()
            });
        }
        if self.unaligned_2b_single > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_2b_single,
                cost: self.unaligned_2b_single
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
                write: true,
                width: 2,
                ..Default::default()
            });
        }
        if self.unaligned_2b_double > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_2b_double,
                cost: self.unaligned_2b_double
                    * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost),
                write: true,
                dirty: false,
                width: 2,
                ..Default::default()
            });
        }
        if self.unaligned_4b_single_32_align_clean > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_single_32_align_clean,
                cost: self.unaligned_4b_single_32_align_clean
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
                write: true,
                width: 4,
                aligned_32: true,
                ..Default::default()
            });
        }
        if self.unaligned_4b_single_32_align_dirty > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_single_32_align_dirty,
                cost: self.unaligned_4b_single_32_align_dirty
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
                write: true,
                dirty: true,
                width: 4,
                aligned_32: true,
                ..Default::default()
            });
        }
        if self.unaligned_4b_single_non_32_align > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_single_non_32_align,
                cost: self.unaligned_4b_single_non_32_align
                    * (MEM_ALIGN_WRITE_UNALIGNED_1_COST + read_write_cost),
                write: true,
                dirty: false,
                width: 4,
                aligned_32: false,
                ..Default::default()
            });
        }
        if self.unaligned_4b_double > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_4b_double,
                cost: self.unaligned_4b_double
                    * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost),
                write: true,
                double: true,
                width: 4,
                ..Default::default()
            });
        }
        if self.unaligned_8b_double > 0 {
            items.push(MemoryStatsItem {
                count: self.unaligned_8b_double,
                cost: self.unaligned_8b_double
                    * (MEM_ALIGN_WRITE_UNALIGNED_2_COST + 2 * read_write_cost),
                write: true,
                double: true,
                width: 8,
                ..Default::default()
            });
        }
        items
    }
}

impl MemoryReadWriteZoneStatsData {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) -> u32 {
        self.writes.memory_write(address, width, value)
    }
    pub fn memory_read(&mut self, address: u64, width: u64) -> u32 {
        self.reads.memory_read(address, width)
    }
    pub fn get_cost(&self, read_cost: u64, write_cost: u64) -> u64 {
        self.reads.get_cost(read_cost) + self.writes.get_cost(read_cost, write_cost)
    }
    pub fn get_aligned_cost(&self, read_cost: u64, write_cost: u64) -> u64 {
        self.reads.get_aligned_cost(read_cost) + self.writes.get_aligned_cost(write_cost)
    }
    pub fn get_unaligned_cost(&self, read_cost: u64, write_cost: u64) -> u64 {
        self.reads.get_unaligned_cost(read_cost)
            + self.writes.get_unaligned_cost(read_cost, write_cost)
    }
    pub fn get_count(&self) -> u64 {
        self.reads.get_count() + self.writes.get_count()
    }
    pub fn get_aligned_count(&self) -> u64 {
        self.reads.get_aligned_count() + self.writes.get_aligned_count()
    }
    pub fn get_unaligned_count(&self) -> u64 {
        self.reads.get_unaligned_count() + self.writes.get_unaligned_count()
    }
    pub fn add_delta(
        &mut self,
        reference: &MemoryReadWriteZoneStatsData,
        current: &MemoryReadWriteZoneStatsData,
    ) {
        self.reads.add_delta(&reference.reads, &current.reads);
        self.writes.add_delta(&reference.writes, &current.writes);
    }
    pub fn get_detailed_items(
        &self,
        title: &str,
        read_cost: u64,
        write_cost: u64,
        init_count: u64,
    ) -> Vec<(String, u64, u64)> {
        let mut items = self.reads.get_detailed_items(title, read_cost, 0);
        items.append(&mut self.writes.get_detailed_items(title, read_cost, write_cost, init_count));
        items
    }
    pub fn get_items(
        &self,
        read_cost: u64,
        write_cost: u64,
        init_count: u64,
    ) -> Vec<MemoryStatsItem> {
        let mut read_items = self.reads.get_items(read_cost, 0);
        read_items.append(&mut self.writes.get_items(read_cost, write_cost, init_count));
        read_items
    }
}
