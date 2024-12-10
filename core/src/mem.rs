//! Zisk program memory
//!
//! # Memory map
//!
//! * The Zisk processor memory stores data in little-endian format.
//! * The addressable memory space is divided into several regions described in the following map:
//!
//! `|--------------- ROM_ENTRY: first BIOS instruction   (    0x1000)`  
//! `|`  
//! `| Performs memory initialization, calls program at ROM_ADDR,`  
//! `| and after returning it performs memory finalization.`  
//! `| Contains ecall/system call management code.`  
//! `|`  
//! `|--------------- ROM_EXIT: last BIOS instruction     (0x10000000)`  
//! `      ...`  
//! `|--------------- ROM_ADDR: first program instruction (0x80000000)`  
//! `|`  
//! `| Contains program instructions.`  
//! `| Calls ecalls/system calls when required.`  
//! `|`  
//! `|--------------- INPUT_ADDR                          (0x90000000)`  
//! `|`  
//! `| Contains program input data.`  
//! `|`  
//! `|--------------- SYS_ADDR (= RAM_ADDR = REG_FIRST)   (0xa0000000)`  
//! `|`  
//! `| Contains system address.`  
//! `| The first 256 bytes contain 32 8-byte registers`  
//! `| The address UART_ADDR is used as a standard output`  
//! `|`  
//! `|--------------- OUTPUT_ADDR                         (0xa0010000)`  
//! `|`  
//! `| Contains output data, which is written during`  
//! `| program execution and read during memory finalization`  
//! `|`  
//! `|--------------- AVAILABLE_MEM_ADDR                  (0xa0020000)`  
//! `|`  
//! `| Contains program memory, available for normal R/W`  
//! `| use during program execution.`  
//! `|`  
//! `|---------------                                     (0xb0000000)`  
//! `      ...`  
//!
//! ## ROM_ENTRY / ROM_ADDR / ROM_EXIT
//! * The program will start executing at the first BIOS address `ROM_ENTRY`.
//! * The first instructions do the basic program setup, including writing the input data into
//!   memory, configuring the ecall (system call) program address, and configuring the program
//!   completion return address.
//! * After the program setup, the program counter jumps to `ROM_ADDR`, executing the actual
//!   program.
//! * During the execution, the program can make system calls that will jump to the configured ecall
//!   program address, and return once the task has completed. The precompiled are implemented via
//!   ecall.
//! * After the program is completed, the program counter will jump to the configured return
//!   address, where the finalization tasks will happen, inluding reading the output data from
//!   memory.
//! * The address before the last one will jump to `ROM_EXIT`, the last insctruction of the
//!   execution.
//! * In general, setup and finalization instructions are located in low addresses, while the actual
//!   program insctuctions are located in high addresses.
//!
//! ## INPUT_ADDR
//! * During the program initialization the input data for the program execution is copied in this
//!   memory region, beginning with `INPUT_ADDR`.
//! * After the data has been written by the setup process, this data can only be read by the
//!   program execution, i.e. it becomes a read-only (RO) memory region.
//!
//! ## SYS_ADDR / OUPUT_ADDR / AVAILABLE_MEM_ADDR
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
//!   program execution a general purpose memory.

use crate::{REG_FIRST, REG_LAST, UART_ADDR};

/// Fist input data memory address
pub const INPUT_ADDR: u64 = 0x90000000;
/// Maximum size of the input data
pub const MAX_INPUT_SIZE: u64 = 0x10000000; // 256M,
/// First globa RW memory address
pub const RAM_ADDR: u64 = 0xa0000000;
/// Size of the global RW memory
pub const RAM_SIZE: u64 = 0x10000000; // 256M
/// First system RW memory address
pub const SYS_ADDR: u64 = RAM_ADDR;
/// Size of the system RW memory
pub const SYS_SIZE: u64 = 0x10000;
/// First output RW memory address
pub const OUTPUT_ADDR: u64 = SYS_ADDR + SYS_SIZE;
/// Size of the output RW memory
pub const OUTPUT_MAX_SIZE: u64 = 0x10000; // 64K
/// First general purpose RW memory address
pub const AVAILABLE_MEM_ADDR: u64 = OUTPUT_ADDR + OUTPUT_MAX_SIZE;
/// Size of the general purpose RW memory address
pub const AVAILABLE_MEM_SIZE: u64 = RAM_SIZE - OUTPUT_MAX_SIZE - SYS_SIZE;
/// First BIOS instruction address, i.e. first instruction executed
pub const ROM_ENTRY: u64 = 0x1000;
/// Last BIOS instruction address, i.e. last instruction executed
pub const ROM_EXIT: u64 = 0x10000000;
/// First program ROM instruction address, i.e. first RISC-V transpiled instruction
pub const ROM_ADDR: u64 = 0x80000000;
/// Maximum program ROM instruction address
pub const ROM_ADDR_MAX: u64 = INPUT_ADDR - 1;
/// Zisk architecture ID
pub const ARCH_ID_ZISK: u64 = 0xFFFEEEE;
/// UART memory address; single bytes written here will be copied to the standard output
pub const UART_ADDR: u64 = SYS_ADDR + 512;

/// Memory section data, including a buffer (a vector of bytes) and start and end program
/// memory addresses.
#[derive(Default)]
pub struct MemSection {
    pub start: u64,
    pub end: u64,
    pub buffer: Vec<u8>,
}

/// Memory structure, containing several read sections and one single write section
#[derive(Default)]
pub struct Mem {
    pub read_sections: Vec<MemSection>,
    pub write_section: MemSection,
}

impl Mem {
    /// Adds a read section to the memory structure
    pub fn add_read_section(&mut self, start: u64, buffer: &[u8]) {
        // Check that the start address is alligned to 8 bytes
        if (start & 0x07) != 0 {
            panic!(
                "Mem::add_read_section() got a start address={:x} not alligned to 8 bytes",
                start
            );
        }

        // Calculate the end address
        let end = start + buffer.len() as u64;

        // Create a mem section with this data
        let mut mem_section = MemSection { start, end, buffer: buffer.to_owned() };

        // Add zero-value bytes until the end address is alligned to 8 bytes
        while (mem_section.end) % 8 != 0 {
            mem_section.buffer.push(0);
            mem_section.end += 1;
        }

        // Push the new read section to the read sections list
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

        // Check that the start address is alligned to 8 bytes
        if (start & 0x07) != 0 {
            panic!(
                "Mem::add_write_section() got a start address={:x} not alligned to 8 bytes",
                start
            );
        }

        // Check the start address is not zero
        if start == 0 {
            panic!("Mem::add_write_section() got invalid start={}", start);
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
                _ => panic!("Mem::read() invalid width={}", width),
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
            panic!("Mem::read() section not found for addr: {} with width: {}", addr, width);
        };

        // Calculate the buffer relative read position
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

        // Get a reference to the write section
        let section = &mut self.write_section;

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
        }
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
}
