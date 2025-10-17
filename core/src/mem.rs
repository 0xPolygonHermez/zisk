//! Zisk program memory
//!
//! # Memory map
//!
//! * The Zisk processor memory stores data in little-endian format.
//! * The addressable memory space is divided into several regions described in the following map:
//!
//! `|--------------- ROM_ENTRY: first BIOS instruction   (    0x1000)`
//! `|--------------- ROM_EXIT: last BIOS instruction     (    0x1004)`
//! `|`
//! `| Performs memory initialization, calls program at ROM_ADDR,`
//! `| and after returning it performs memory finalization.`
//! `| Contains ecall/system call management code.`
//! `|`
//! `|---------------`
//! `      ...`
//! `|--------------- ROM_ADDR: first program instruction (0x80000000)`
//! `|`
//! `| Contains program instructions.`
//! `| Calls ecalls/system calls when required.`
//! `|`
//! `|--- FLOAT_LIB_ROM_ADDR: first float lib instruction (0x87F00000)`
//! `|`
//! `| Contains float library instructions. 1M before ROM_ADDR_MAX.`
//! `|`
//! `|------------- FLOAT_LIB_SP: float lib stack pointer (0xaffffff0)`
//! `|`
//! `| Initial value of the float library stack pointer.`
//! `|`
//! `|--------------- INPUT_ADDR                          (0x90000000)`
//! `|`
//! `| Contains program input data.`
//! `|`
//! `|--------------- SYS_ADDR (= RAM_ADDR = REG_FIRST)   (0xa0000000)`
//! `|`
//! `| Contains system address.`
//! `| The first 256 bytes contain 32 8-byte registers`
//! `| The address UART_ADDR is used as a stdout at addr = 0xa0000200`
//! `| The first float register is at         FREG_FIRST = 0xa0001000`
//! `| The first CSR register is at             CSR_ADDR = 0xa0008000`
//! `|`
//! `|--------------- OUTPUT_ADDR                         (0xa0010000)`
//! `|`
//! `| Contains output data, which is written during`
//! `| program execution and read during memory finalization`
//! `|`
//! `|--------------- AVAILABLE_MEM_ADDR                  (0xa0030000)`
//! `|`
//! `| Contains program memory, available for normal R/W`
//! `| used during program execution.`
//! `|`
//! `|--------------- FLOAT_LIB_RAM_ADDR = 0xafff0000     (0xc0000000 - 0x10000)`
//! `|`
//! `| Contains float library memory, available for normal R/W`
//! `| used during library execution (bottom-up).`
//! `|`
//! `| Contains float library stack memory (top-down).`
//! `|`
//! `|--------------- FLOAT_LIB_SP = 0xaffffff0           (0xc0000000 - 16)`
//! `|`
//! `|--------------- END OF RAM                          (0xc0000000)`
//! `      ...`
//!
//! ## ROM_ENTRY / ROM_ADDR / ROM_EXIT
//! * The program will start executing at the first BIOS address `ROM_ENTRY`.
//! * The first instructions do the basic program setup, including writing the input data into
//!   memory, configuring the ecall (system call) program address, and configuring the program
//!   completion return address.
//! * After the program set1, the program counter jumps to `ROM_ADDR`, executing the actual program.
//! * During the execution, the program can make system calls that will jump to the configured ecall
//!   program address, and return once the task has completed. The precompiled are implemented via
//!   ecall.
//! * After the program is completed, the program counter will jump to the configured return
//!   address, where the finalization tasks will happen, including reading the output data from
//!   memory.
//! * The address before the last one will jump to `ROM_EXIT`, the last insctruction of the
//!   execution.
//! * In general, setup and finalization instructions are located in low addresses, while the actual
//!   program instructions are located in high addresses.
//!
//! ## INPUT_ADDR
//! * During the program initialization the input data for the program execution is copied in this
//!   memory region, beginning with `INPUT_ADDR`.
//! * After the data has been written by the setup process, this data can only be read by the
//!   program execution, i.e. it becomes a read-only (RO) memory region.
//!
//! ## SYS_ADDR / OUTPUT_ADDR / AVAILABLE_MEM_ADDR
//! * This memory section can be written and read by the program execution many times, i.e. it is a
//!   read-write (RW) memory region.
//! * The first RW memory region going from `SYS_ADDR` to `OUTPUT_ADDR` is reserved for the system
//!   operation.
//! * The lower addresses of this region is used to store 32 registers of 8 bytes each, i.e. 256
//!   bytes in total.  These registers are the equivalent to the RISC-V registers.
//! * Any data of exactly 1-byte length written to UART_ADDR will be sent to the standard output of
//!   the system.
//! * The second RW memory region going from `OUTPUT_ADDR` to `AVAILABLE_MEM_ADDR` is reserved to
//!   copy the output data during the program execution.
//! * The third RW memory region going from `AVAILABLE_MEM_ADDR` onwards can be used during the
//!   program execution as general purpose memory.

use crate::{M16, M3, M32, M8, REG_FIRST, REG_LAST};
use core::fmt;

/// Fist input data memory address
pub const INPUT_ADDR: u64 = 0x90000000;
/// Maximum size of the input data
pub const MAX_INPUT_SIZE: u64 = 0x08000000; // 128M,
/// Free input data memory address = first input address
pub const FREE_INPUT_ADDR: u64 = INPUT_ADDR;
/// First global RW memory address
pub const RAM_ADDR: u64 = 0xa0000000;
/// Size of the global RW memory
pub const RAM_SIZE: u64 = 0x20000000; // 512M
/// First system RW memory address
pub const SYS_ADDR: u64 = RAM_ADDR;
/// Size of the system RW memory
pub const SYS_SIZE: u64 = 0x10000;
/// First output RW memory address
pub const OUTPUT_ADDR: u64 = SYS_ADDR + SYS_SIZE;
/// Size of the output RW memory
pub const OUTPUT_MAX_SIZE: u64 = 0x10000; // 64K
/// First general purpose RW memory address
pub const AVAILABLE_MEM_ADDR: u64 = SYS_ADDR + 0x30000;
/// Size of the general purpose RW memory address
pub const AVAILABLE_MEM_SIZE: u64 = RAM_SIZE - OUTPUT_MAX_SIZE - SYS_SIZE;
/// First BIOS instruction address, i.e. first instruction executed
pub const ROM_ENTRY: u64 = 0x1000;
/// Last BIOS instruction address, i.e. last instruction executed
pub const ROM_EXIT: u64 = 0x1004;
/// First program ROM instruction address, i.e. first RISC-V transpiled instruction
pub const ROM_ADDR: u64 = 0x80000000;
/// Maximum program ROM instruction address
pub const ROM_ADDR_MAX: u64 = ROM_ADDR + 0x08000000 - 1; // 128M
/// First float library ROM instruction address
pub const FLOAT_LIB_ROM_ADDR: u64 = ROM_ADDR + 0x08000000 - 0x100000; // 1M before ROM_ADDR_MAX = 0x87F00000
/// First float library RAM address
pub const FLOAT_LIB_RAM_ADDR: u64 = 0xc0000000 - 0x10000; // 0xbfff0000
/// Float library stack pointer address
pub const FLOAT_LIB_SP: u64 = 0xc0000000 - 16; // 0xbffffff0
/// Zisk architecture ID
pub const ARCH_ID_ZISK: u64 = 0xFFFEEEE;
/// UART memory address; single bytes written here will be copied to the standard output
pub const UART_ADDR: u64 = SYS_ADDR + 0x200;
/// Float registers first address
pub const FREG_FIRST: u64 = SYS_ADDR + 0x1000;
/// CSR memory address; contains control and status registers
pub const CSR_ADDR: u64 = SYS_ADDR + 0x8000;
/// Machine trap-vector base-address register
pub const MTVEC: u64 = CSR_ADDR + 0x305 * 8;
/// Floating-point Control and Status Register
pub const FCSR: u64 = CSR_ADDR + 0x003 * 8;
/// Architecture ID Control and Status Register
pub const ARCH_ID_CSR: u64 = 0xF12;
/// Architecture ID Control and Status Register address
pub const ARCH_ID_CSR_ADDR: u64 = CSR_ADDR + (ARCH_ID_CSR * 8);

/// Memory section data, including a buffer (a vector of bytes) and start and end program
/// memory addresses.
pub struct MemSection {
    pub start: u64,
    pub end: u64,
    pub real_end: u64,
    pub buffer: Vec<u8>,
}

/// Default constructor for MemSection structure
impl Default for MemSection {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MemSection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

/// Memory section structure implementation
impl MemSection {
    /// Memory section constructor
    pub fn new() -> MemSection {
        MemSection { start: 0, end: 0, real_end: 0, buffer: Vec::new() }
    }
    pub fn to_text(&self) -> String {
        format!(
            "start={:x} real_end={:x} end={:x} diff={:x}={} buffer.len={:x}={}",
            self.start,
            self.real_end,
            self.end,
            self.end - self.start,
            self.end - self.start,
            self.buffer.len(),
            self.buffer.len()
        )
    }
}

/// Memory structure, containing several read sections and one single write section
#[derive(Debug, Default)]
pub struct Mem {
    pub read_sections: Vec<MemSection>,
    pub write_section: MemSection,
    pub free_input: u64,
}

impl Mem {
    /// Memory structure constructor
    pub fn new() -> Mem {
        //println!("Mem::new()");
        Mem { read_sections: Vec::new(), write_section: MemSection::new(), free_input: 0 }
    }

    /// Adds a read section to the memory structure
    pub fn add_read_section(&mut self, start: u64, buffer: &[u8]) {
        // Check that the start address is alligned to 8 bytes
        if (start & 0x07) != 0 {
            panic!("Mem::add_read_section() got a start address={start:x} not alligned to 8 bytes");
        }

        // Calculate the end address
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

        // Check that the start address is alligned to 8 bytes
        if (start & 0x07) != 0 {
            panic!(
                "Mem::add_write_section() got a start address={start:x} not alligned to 8 bytes"
            );
        }

        // Check the start address is not zero
        if start == 0 {
            panic!("Mem::add_write_section() got invalid start={start}");
        }

        // Check the write section address has not been set before this call, since one only write
        // section is allowed
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

    /// Reads a 1, 2, 4 or 8 bytes value from the memory read sections, based on the provided
    /// address and width
    #[inline(always)]
    pub fn read(&self, addr: u64, width: u64) -> u64 {
        debug_assert!(!Mem::address_is_register(addr));

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
                _ => panic!("Mem::read() invalid width={width}"),
            };

            //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value, value);
            return value;
        }

        // Search for the section that contains the address using binary search (dicothomic search).
        // Read sections are ordered by start address to allow this search.
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
            panic!("Mem::read() section not found for addr: {addr}={addr:x} with width: {width}");
        };

        // Calculate the buffer relative read position
        let read_position: usize = (addr - section.start) as usize;
        if addr == INPUT_ADDR && width == 8 {
            // increment of pointer is done by the fcall_get
            return self.free_input;
        }

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
            _ => panic!("Mem::read() invalid width={width}"),
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
        let addr_req_1 = addr & 0xFFFF_FFFF_FFFF_FFF8; // Aligned address of the first 8-bytes chunk
        let addr_req_2 = (addr + width - 1) & 0xFFFF_FFFF_FFFF_FFF8; // Aligned address of the second 8-bytes chunk, if needed
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
                _ => panic!("Mem::read() invalid width={width}"),
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
            } else if (addr + width) > section.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            &self.read_sections[section]
        } else {
            println!("sections: {:?}", self.read_sections);
            panic!("Mem::read() section not found for addr: {addr} with width: {width}");
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
            _ => panic!(
                "Mem::read() invalid addr:0x{addr:X} read_position:{read_position} width:{width}"
            ),
        };

        // If is a single not aligned operation, return the aligned address value
        if is_single_not_aligned {
            let mut additional_data: Vec<u64> = Vec::new();

            assert!(addr_req_1 >= section.start);
            let read_position_req: usize = (addr_req_1 - section.start) as usize;
            let value_req = u64::from_le_bytes(
                section.buffer[read_position_req..read_position_req + 8].try_into().unwrap(),
            );
            additional_data.push(value_req);

            return (value, additional_data);
        }

        // If is a double not aligned operation, return the aligned address value and the next
        // one
        if is_double_not_aligned {
            let mut additional_data: Vec<u64> = Vec::new();

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

            return (value, additional_data);
        }

        //println!("Mem::read() addr={:x} width={} value={:x}={}", addr, width, value, value);

        (value, Vec::new())
    }

    /// Write a u64 value to the memory write section, based on the provided address and width
    #[inline(always)]
    pub fn write(&mut self, addr: u64, val: u64, width: u64) {
        debug_assert!(!Mem::address_is_register(addr));

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
        debug_assert!(!Mem::address_is_register(addr));

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
            _ => panic!("Mem::write_silent() invalid width={width}"),
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
        let addr_req_1 = addr & 0xFFFF_FFFF_FFFF_FFF8; // Aligned address of the first 8-bytes chunk
        let addr_req_2 = (addr + width - 1) & 0xFFFF_FFFF_FFFF_FFF8; // Aligned address of the second 8-bytes chunk, if needed
        let is_full_aligned = ((addr & 0x03) == 0) && (width == 8);
        let is_single_not_aligned = !is_full_aligned && (addr_req_1 == addr_req_2);
        let is_double_not_aligned = !is_full_aligned && !is_single_not_aligned;

        // Declare an empty vector
        let mut additional_data: Vec<u64> = Vec::new();

        // If is a single not aligned operation, return the aligned address value
        if is_single_not_aligned {
            assert!(
                addr_req_1 >= section.start,
                "addr_req_1: 0x{:X} 0x{:X}]",
                addr_req_1,
                section.start
            );
            let read_position_req: usize = (addr_req_1 - section.start) as usize;
            let value_req = u64::from_le_bytes(
                section.buffer[read_position_req..read_position_req + 8].try_into().unwrap(),
            );
            additional_data.push(value_req);
        }

        // If is a double not aligned operation, return the aligned address value and the next
        // one
        if is_double_not_aligned {
            assert!(
                addr_req_1 >= section.start,
                "addr_req_1(d): 0x{:X} 0x{:X}]",
                addr_req_1,
                section.start
            );
            let read_position_req_1: usize = (addr_req_1 - section.start) as usize;
            let value_req_1 = u64::from_le_bytes(
                section.buffer[read_position_req_1..read_position_req_1 + 8].try_into().unwrap(),
            );
            additional_data.push(value_req_1);

            assert!(
                addr_req_2 >= section.start,
                "addr_req_2(d): 0x{:X} 0x{:X}]",
                addr_req_2,
                section.start
            );
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
            _ => panic!("Mem::write_silent() invalid width={width}"),
        }

        additional_data
    }

    #[inline(always)]
    pub fn address_is_register(address: u64) -> bool {
        ((address & 0x7) == 0) && (REG_FIRST..=REG_LAST).contains(&address)
    }

    #[inline(always)]
    pub fn address_to_register_index(address: u64) -> usize {
        debug_assert!(Mem::address_is_register(address));
        ((address - REG_FIRST) >> 3) as usize
    }

    /// Returns true if the address and width are fully aligned
    #[inline(always)]
    pub fn is_full_aligned(address: u64, width: u64) -> bool {
        ((address & 0x03) == 0) && (width == 8)
    }

    /// Returns true if the address and width are single non aligned
    #[inline(always)]
    pub fn is_single_not_aligned(address: u64, width: u64) -> bool {
        if Self::is_full_aligned(address, width) {
            return true;
        }
        let (address_required_1, address_required_2) = Self::required_addresses(address, width);
        address_required_1 == address_required_2
    }

    /// Returns true if the address and width are double non aligned
    #[inline(always)]
    pub fn is_double_not_aligned(address: u64, width: u64) -> bool {
        if Self::is_full_aligned(address, width) {
            return true;
        }
        let (address_required_1, address_required_2) = Self::required_addresses(address, width);
        address_required_1 != address_required_2
    }

    /// Aligned addresses of the first and second 8-bytes chunks
    /// They can be equal if the required data fits into one single chunk of 8 bytes, or if it is
    /// a fully aligned data
    #[inline(always)]
    pub fn required_addresses(address: u64, width: u64) -> (u64, u64) {
        (address & 0xFFFF_FFFF_FFFF_FFF8, (address + width - 1) & 0xFFFF_FFFF_FFFF_FFF8)
    }

    /// Get single not aligned data from the raw data
    #[inline(always)]
    pub fn get_single_not_aligned_data(address: u64, width: u64, raw_data: u64) -> u64 {
        debug_assert!(width < 8);
        let offset = address & M3;
        let raw_data = raw_data >> (8 * offset);
        match width {
            1 => raw_data & M8,
            2 => raw_data & M16,
            4 => raw_data & M32,
            _ => panic!("Mem::get_single_not_aligned_data() invalid width={width}"),
        }
    }

    /// Get double not aligned data from the raw data
    #[inline(always)]
    pub fn get_double_not_aligned_data(
        address: u64,
        width: u64,
        raw_data_1: u64,
        raw_data_2: u64,
    ) -> u64 {
        //println!("Mem::get_double_not_aligned_data() address={:x} width={} raw_data_1={:x}
        // raw_data_2={:x}", address, width, raw_data_1, raw_data_2);
        debug_assert!(width <= 8);
        let offset = address & M3;
        let raw_data = ((raw_data_1 as u128 + ((raw_data_2 as u128) << 64)) >> (8 * offset)) as u64;
        match width {
            1 => raw_data & M8,
            2 => raw_data & M16,
            4 => raw_data & M32,
            8 => raw_data,
            _ => panic!("Mem::get_double_not_aligned_data() invalid width={width}"),
        }
    }

    //pub fn get_non_aligned_data_from_required(address: u64, width: u8,)
}
