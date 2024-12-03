use crate::UART_ADDR;

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
        //println!("Mem::new()");
        Mem { read_sections: Vec::new(), write_section: MemSection::new() }
    }

    /// Adds a read section to the memory structure
    pub fn add_read_section(&mut self, start: u64, buffer: &[u8]) {
        let end = start + buffer.len() as u64;

        // If there exists a read section next to this one, reuse it
        for existing_section in self.read_sections.iter_mut() {
            if existing_section.real_end == start {
                // Sanity check
                assert!(existing_section.real_end <= existing_section.end);
                assert!((existing_section.end - existing_section.real_end) < 8);

                // Pop tail zeros until end matches real_end
                while existing_section.real_end > existing_section.end {
                    existing_section.buffer.pop();
                    existing_section.end -= 1;
                }

                // Append buffer
                existing_section.buffer.extend(buffer);
                existing_section.real_end += buffer.len() as u64;
                existing_section.end = existing_section.real_end;

                // Append zeros until end is multiple of 8, so that we can read non-alligned reads
                while (existing_section.end & 0x07) != 0 {
                    existing_section.buffer.push(0);
                    existing_section.end += 1;
                }

                /*println!(
                    "Mem::add_read_section() start={:x} len={} existing section={}",
                    start,
                    buffer.len(),
                    existing_section.to_text()
                );*/

                return;
            }
        }

        // Create a new memory section
        let mut new_section = MemSection { start, end, real_end: end, buffer: buffer.to_owned() };

        // Append zeros until end is multiple of 8, so that we can read non-alligned reads
        while (new_section.end & 0x07) != 0 {
            new_section.buffer.push(0);
            new_section.end += 1;
        }

        //println!("Mem::add_read_section() new section={}", new_section.to_text());

        // Add the new section to the read sections
        self.read_sections.push(new_section);
    }

    /// Adds a write section to the memory structure, which cannot be written twice
    pub fn add_write_section(&mut self, start: u64, size: u64) {
        //println!("Mem::add_write_section() start={:x}={} size={:x}={}", start, start, size,
        // size);

        // Check the start address is not zero
        if start == 0 {
            panic!("Mem::add_write_section() got invalid start={}", start);
        }

        // Check the write section address has been set before this call
        if self.write_section.start != 0 {
            panic!(
                "Mem::add_write_section() only one write section allowed, write_section.start={}",
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
            panic!("Mem::read() section not found for addr: {} with width: {}", addr, width);
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

    /*
    Possible alignment situations:
    - Full aligned = address is aligned to 8 bytes (last 3 bits are zero) and width is 8
    - Single not aligned = not full aligned, and the data fits into one aligned slice of 8 bytes
    - Double not aligned = not full aligned, and the data needs 2 aligned slices of 8 bytes

    Data required for each situation:
    - full_aligned + RD = value
    - full_aligned + WR = value, full_value
    - single_not_aligned + RD = value, full_value  TODO: We can save the value space, optimization
    - single_not_aligned + WR = value, previous_full_value
    - double_not_aligned + RD = value, full_values_0, full_values_1
    - double_not_aligned + WR = value, previous_full_values_0, previous_full_values_1

    read_required() returns read value, and a vector of additional data required to prove it
    */

    /// Read a u64 value from the memory read sections, based on the provided address and width
    #[inline(always)]
    pub fn read_required(&self, addr: u64, width: u64) -> (u64, Vec<u64>) {
        // Calculate how aligned this operation is
        let addr_req_1 = addr >> 3; // Aligned address of the first 8-bytes chunk
        let addr_req_2 = (addr + width - 1) >> 3; // Aligned address of the second 8-bytes chunk, if needed
        let is_full_aligned = ((addr & 0x03) == 0) && (width == 8);
        let is_single_not_aligned = !is_full_aligned && (addr_req_1 == addr_req_2);
        let is_double_not_aligned = !is_full_aligned && !is_single_not_aligned;

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

            // If is a single not aligned operation, return the aligned address value
            if is_single_not_aligned {
                let mut additional_data: Vec<u64> = Vec::new();

                assert!(addr_req_1 >= self.write_section.start);
                let read_position_req: usize = (addr_req_1 - self.write_section.start) as usize;
                let value_req = u64::from_le_bytes(
                    self.write_section.buffer[read_position_req..read_position_req + 8]
                        .try_into()
                        .unwrap(),
                );
                additional_data.push(value_req);

                return (value, additional_data);
            }

            // If is a double not aligned operation, return the aligned address value and the next
            // one
            if is_double_not_aligned {
                let mut additional_data: Vec<u64> = Vec::new();

                assert!(addr_req_1 >= self.write_section.start);
                let read_position_req_1: usize = (addr_req_1 - self.write_section.start) as usize;
                let value_req_1 = u64::from_le_bytes(
                    self.write_section.buffer[read_position_req_1..read_position_req_1 + 8]
                        .try_into()
                        .unwrap(),
                );
                additional_data.push(value_req_1);

                assert!(addr_req_2 >= self.write_section.start);
                let read_position_req_2: usize = (addr_req_2 - self.write_section.start) as usize;
                let value_req_2 = u64::from_le_bytes(
                    self.write_section.buffer[read_position_req_2..read_position_req_2 + 8]
                        .try_into()
                        .unwrap(),
                );
                additional_data.push(value_req_2);

                return (value, additional_data);
            }

            //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value, value);
            return (value, Vec::new());
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
            panic!("Mem::read() section not found for addr: {} with width: {}", addr, width);
        };

        // Calculate the read position
        let read_position: usize = (addr - section.start) as usize;

        // Read the requested data based on the provided width
        let value: u64 = match width {
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
        };

        // If is a single not aligned operation, return the aligned address value
        if is_single_not_aligned {
            let mut additional_data: Vec<u64> = Vec::new();

            assert!(addr_req_1 >= self.write_section.start);
            let read_position_req: usize = (addr_req_1 - self.write_section.start) as usize;
            let value_req = u64::from_le_bytes(
                self.write_section.buffer[read_position_req..read_position_req + 8]
                    .try_into()
                    .unwrap(),
            );
            additional_data.push(value_req);

            return (value, additional_data);
        }

        // If is a double not aligned operation, return the aligned address value and the next
        // one
        if is_double_not_aligned {
            let mut additional_data: Vec<u64> = Vec::new();

            assert!(addr_req_1 >= self.write_section.start);
            let read_position_req_1: usize = (addr_req_1 - self.write_section.start) as usize;
            let value_req_1 = u64::from_le_bytes(
                self.write_section.buffer[read_position_req_1..read_position_req_1 + 8]
                    .try_into()
                    .unwrap(),
            );
            additional_data.push(value_req_1);

            assert!(addr_req_2 >= self.write_section.start);
            let read_position_req_2: usize = (addr_req_2 - self.write_section.start) as usize;
            let value_req_2 = u64::from_le_bytes(
                self.write_section.buffer[read_position_req_2..read_position_req_2 + 8]
                    .try_into()
                    .unwrap(),
            );
            additional_data.push(value_req_2);

            return (value, additional_data);
        }

        //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value, value);

        (value, Vec::new())
    }

    /// Write a u64 value to the memory write section, based on the provided address and width
    #[inline(always)]
    pub fn write(&mut self, addr: u64, val: u64, width: u64) {
        // Call write_silent to perform the real work
        self.write_silent(addr, val, width);

        // Log to console bytes written to UART address
        if (addr == UART_ADDR) && (width == 1) {
            print!("{}", String::from(val as u8 as char));
        }
    }

    /// Write a u64 value to the memory write section, based on the provided address and width
    #[inline(always)]
    pub fn write_silent(&mut self, addr: u64, val: u64, width: u64) {
        //println!("Mem::write() addr={:x}={} width={} value={:x}={}", addr, addr, width, val,
        // val);

        // Search for the section that contains the address using binary search (dicothomic search)
        let section = if let Ok(section) = self.read_sections.binary_search_by(|section| {
            if addr < section.start {
                std::cmp::Ordering::Greater
            } else if addr > (section.end - width) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            &mut self.read_sections[section]
        } else {
            /*panic!(
                "Mem::write_silent() section not found for addr={:x}={} with width: {}",
                addr, addr, width
            );*/
            &mut self.write_section
        };

        // Check that the address and width fall into this section address range
        if (addr < section.start) || ((addr + width) > section.end) {
            panic!(
                "Mem::write_silent() invalid addr={}={:x} write section start={:x} end={:x}",
                addr, addr, section.start, section.end
            );
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
            _ => panic!("Mem::write_silent() invalid width={}", width),
        };
    }

    /// Write a u64 value to the memory write section, based on the provided address and width
    #[inline(always)]
    pub fn write_silent_required(&mut self, addr: u64, val: u64, width: u64) -> Vec<u64> {
        //println!("Mem::write() addr={:x}={} width={} value={:x}={}", addr, addr, width, val,
        // val);

        // Search for the section that contains the address using binary search (dicothomic search)
        let section = if let Ok(section) = self.read_sections.binary_search_by(|section| {
            if addr < section.start {
                std::cmp::Ordering::Greater
            } else if addr > (section.end - width) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            &mut self.read_sections[section]
        } else {
            /*panic!(
                "Mem::write_silent() section not found for addr={:x}={} with width: {}",
                addr, addr, width
            );*/
            &mut self.write_section
        };

        // Check that the address and width fall into this section address range
        if (addr < section.start) || ((addr + width) > section.end) {
            panic!(
                "Mem::write_silent() invalid addr={}={:x} write section start={:x} end={:x}",
                addr, addr, section.start, section.end
            );
        }

        // Calculate how aligned this operation is
        let addr_req_1 = addr >> 3; // Aligned address of the first 8-bytes chunk
        let addr_req_2 = (addr + width - 1) >> 3; // Aligned address of the second 8-bytes chunk, if needed
        let is_full_aligned = ((addr & 0x03) == 0) && (width == 8);
        let is_single_not_aligned = !is_full_aligned && (addr_req_1 == addr_req_2);
        let is_double_not_aligned = !is_full_aligned && !is_single_not_aligned;

        // Declare an empty vector
        let mut additional_data: Vec<u64> = Vec::new();

        // If is a single not aligned operation, return the aligned address value
        if is_single_not_aligned {
            assert!(addr_req_1 >= section.start);
            let read_position_req: usize = (addr_req_1 - section.start) as usize;
            let value_req = u64::from_le_bytes(
                section.buffer[read_position_req..read_position_req + 8].try_into().unwrap(),
            );
            additional_data.push(value_req);
        }

        // If is a double not aligned operation, return the aligned address value and the next
        // one
        if is_double_not_aligned {
            assert!(addr_req_1 >= section.start);
            let read_position_req_1: usize = (addr_req_1 - section.start) as usize;
            let value_req_1 = u64::from_le_bytes(
                section.buffer[read_position_req_1..read_position_req_1 + 8].try_into().unwrap(),
            );
            additional_data.push(value_req_1);

            assert!(addr_req_2 >= section.start);
            let read_position_req_2: usize = (addr_req_2 - section.start) as usize;
            let value_req_2 = u64::from_le_bytes(
                section.buffer[read_position_req_2..read_position_req_2 + 8].try_into().unwrap(),
            );
            additional_data.push(value_req_2);
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
            _ => panic!("Mem::write_silent() invalid width={}", width),
        }

        additional_data
    }
}
