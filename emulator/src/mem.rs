use crate::MemSection;

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
        let end = start + buffer.len() as u64;
        let mem_section = MemSection { start, end, buffer: buffer.to_owned() };
        self.read_sections.push(mem_section);
        /*println!(
            "Mem::add_read_section() start={:x}={} len={} end={:x}={}",
            start,
            start,
            buffer.len(),
            end,
            end
        );*/
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
    #[inline(always)]
    pub fn read(&self, addr: u64, width: u64) -> u64 {
        // First try to read in the write section
        if (addr >= self.write_section.start) && (addr <= (self.write_section.end - width)) {
            // Calculate the read position
            let read_position: usize = (addr - self.write_section.start) as usize;

            // Read the requested data based on the provided width
            let value: u64 = match width {
                1 => self.write_section.buffer[read_position] as u64,
                2 => u16::from_le_bytes(
                    self.write_section.buffer[read_position..read_position + 2].try_into().unwrap(),
                ) as u64,
                4 => u32::from_le_bytes(
                    self.write_section.buffer[read_position..read_position + 4].try_into().unwrap(),
                ) as u64,
                8 => u64::from_le_bytes(
                    self.write_section.buffer[read_position..read_position + 8].try_into().unwrap(),
                ),
                _ => panic!("Mem::read() invalid width={}", width),
            };

            //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value, value);
            return value;
        }

        // Search for the section that contains the address using binary search (dicothomic search)
        let section = if let Ok(section) = self.read_sections.binary_search_by(|section| {
            if addr < section.start {
                std::cmp::Ordering::Greater
            } else if addr > section.end - width {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            &self.read_sections[section]
        } else {
            panic!("Section not found for addr: {} with width: {}", addr, width);
        };

        // Calculate the read position
        let read_position: usize = (addr - section.start) as usize;

        // Read the requested data based on the provided width
        match width {
            1 => section.buffer[read_position] as u64,
            2 => u16::from_le_bytes(
                section.buffer[read_position..read_position + 2].try_into().unwrap(),
            ) as u64,
            4 => u32::from_le_bytes(
                section.buffer[read_position..read_position + 4].try_into().unwrap(),
            ) as u64,
            8 => u64::from_le_bytes(
                section.buffer[read_position..read_position + 8].try_into().unwrap(),
            ),
            _ => panic!("Mem::read() invalid width={}", width),
        }
    }

    /// Write a u64 value to the memory write section, based on the provided address and width
    #[inline(always)]
    pub fn write(&mut self, addr: u64, val: u64, width: u64) {
        //println!("Mem::write() addr={:x}={} width={} value={:x}={}", addr, addr, width, val,
        // val);

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
            1 => section.buffer[write_position] = val as u8,
            2 => section.buffer[write_position..write_position + 2]
                .copy_from_slice(&(val as u16).to_le_bytes()),
            4 => section.buffer[write_position..write_position + 4]
                .copy_from_slice(&(val as u32).to_le_bytes()),
            8 => section.buffer[write_position..write_position + 8]
                .copy_from_slice(&val.to_le_bytes()),
            _ => panic!("Mem::write() invalid width={}", width),
        }

        // Log to console bytes written to UART address
        // if (addr == UART_ADDR) && (width == 1) {
        //     print!("{}", String::from(val as u8 as char));
        // }
    }
}
