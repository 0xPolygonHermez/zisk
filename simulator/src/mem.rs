use crate::MemSection;
use riscv2zisk::{read_u16_le, read_u32_le, read_u64_le, write_u16_le, write_u32_le, write_u64_le};

/// Memory structure, containing several read sections and one single write section
pub struct Mem {
    pub read_sections: Vec<MemSection>,
    pub write_section: MemSection,
}

/// Default constructor for Mem structure
impl Default for Mem {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory structure implementation
impl Mem {
    /// Memory structue constructor
    pub fn new() -> Mem {
        Mem { read_sections: Vec::new(), write_section: MemSection::new() }
    }

    /// Adds a read section to the memory structure
    pub fn add_read_section(&mut self, start: u64, buffer: &[u8]) {
        let mem_section =
            MemSection { start, end: start + buffer.len() as u64, buffer: buffer.to_owned() };
        self.read_sections.push(mem_section);
    }

    /// Adds a write section to the memory structure, which cannot be written twice
    pub fn add_write_section(&mut self, start: u64, size: u64) {
        //println!("Mem::add_write_section() start={} size={}", start, size);

        // Check the start address is not zero
        if start == 0 {
            panic!("add_write_section() got invalid start={}", start);
        }

        // Check the write section address has been set before this call
        if self.write_section.start != 0 {
            panic!(
                "add_write_section() only one write section allowed, write_section.start={}",
                self.write_section.start
            );
        }

        // Create an empty vector of size bytes
        let mem: Vec<u8> = vec![0; size as usize];

        // Store as the write section
        self.write_section.start = start;
        self.write_section.end = start + mem.len() as u64;
        self.write_section.buffer = mem;
    }

    /// Read a u64 value from the memory read sections, based on the provided address and width
    pub fn read(&self, addr: u64, width: u64) -> u64 {
        // First try to read in the write section
        if (addr >= self.write_section.start) && (addr <= (self.write_section.end - width)) {
            // Calculate the read position
            let read_position: usize = (addr - self.write_section.start) as usize;

            // Read the requested data based on the provided width
            let value: u64 = match width {
                1 => self.write_section.buffer[read_position] as u64,
                2 => read_u16_le(&self.write_section.buffer, read_position) as u64,
                4 => read_u32_le(&self.write_section.buffer, read_position) as u64,
                8 => read_u64_le(&self.write_section.buffer, read_position),
                _ => panic!("Mem::read() invalid width={}", width),
            };

            //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value, value);
            return value;
        }

        // For all read sections
        for i in 0..self.read_sections.len() {
            // Get a section reference
            let section = &self.read_sections[i];

            // If the provided address and size are between this section address range, then we
            // found the section
            if (addr >= section.start) && (addr <= (section.end - width)) {
                // Calculate the read position
                let read_position: usize = (addr - section.start) as usize;

                // Read the requested data based on the provided width
                let value: u64 = match width {
                    1 => section.buffer[read_position] as u64,
                    2 => read_u16_le(&section.buffer, read_position) as u64,
                    4 => read_u32_le(&section.buffer, read_position) as u64,
                    8 => read_u64_le(&section.buffer, read_position),
                    _ => panic!("Mem::read() invalid width={}", width),
                };

                //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value,
                // value);
                return value;
            }
        }
        panic!("Read out of Range: 0x{:08}", addr);
    }

    /// Write a u64 value to the memory write section, based on the provided address and width
    pub fn write(&mut self, addr: u64, val: u64, width: u64) {
        //println!("Mem::write() addr={:x} width={} value={:x}={}", addr, width, val, val);

        // Get a reference to the write section
        let section = &mut self.write_section;

        // Check that the address and width fall into this section address range
        if (addr < section.start) || ((addr + width) > section.end) {
            panic!("Mem::write() invalid addr={}", addr);
        }

        // Calculate the write position
        let write_position: usize = (addr - section.start) as usize;

        // Write the value based on the provided width
        match width {
            1 => section.buffer[write_position] = (val & 0xFF) as u8,
            2 => write_u16_le(&mut section.buffer, write_position, (val & 0xFFFF) as u16),
            4 => write_u32_le(&mut section.buffer, write_position, (val & 0xFFFFFFFF) as u32),
            8 => write_u64_le(&mut section.buffer, write_position, val),
            _ => panic!("Mem::write() invalid width={}", width),
        }
    }
}