//! Zisk ROM
//!
//! # ROM data
//!
//! The Zisk ROM contains the result of parsing a RISC-V ELF program file data, and then keeping the
//! data that is required to execute any input data against this program using the Zisk processor.
//! This information consists on the following data:
//!
//! ## Zisk instructions
//!
//! * Created by transpiling the RISC-V instructions
//! * Every RISC-V instruction can generate a different number of Zisk instructions: 1 (in most of
//!   the cases), 2, 3 or 4 (e.g. in instruction containing some atomic operations).
//! * For this reason, Zisk instructions addresses are normally spaced 4 units (e.g. 4096, 4100,
//!   4104...) leaving room for up to 3 additional Zisk instructions if needed to fulfill the
//!   original RISC-V instruction they represent.
//! * This way, RISC-V jumps can conveniently be mapped to Zisk jumps by multiplying their relative
//!   offsets by 4.
//! * The Zisk instructions are stored in a map using the pc as the key
//!
//! ## Read-only (RO) data
//!
//! * RISC-V programs can contain some data that is required to execute the program, e.g. constants.
//! * There can be several sections of RO memory-mapped data in the same RISC-V program, so we need
//!   to store a list of them as part of the ROM.
//! * There can be none, one, or several.
//!
//! # Fetching instructions
//!
//! * During the Zisk program execution, the Zisk Emulator must fetch the Zisk instruction
//!   corresponding to the current pc for every execution step.
//! * This fetch can be expensive in terms of computational time if done directly using the map.
//! * For this reason, the original map of instructions is split into 5 different containers that
//!   allow to speed-up the process of finding the Zisk instruction that matches a specific pc
//!   address.
//! * The logic of this fetch procedure can be seen in the method `get_instruction()`.  This method
//!   searches for the Zisk instruction in 5 different containers:
//!   * If the address is < ROM_ADDR, then get it from the vector `rom_bios_instructions`, using as
//!     index `(pc-ROM_ENTRY)/4`.  Note that ZisK BIOS code is always aligned to 4 bytes, so there
//!     is no need to check the alignment here.
//!   * If the address is < `FLOAT_LIB_ROM_ADDR`, there can be 2 cases:
//!     * If the address is aligned to 4 bytes, then get it from the vector `rom_program_instructions`,
//!       using as index `(pc-ROM_ADDR)/4`
//!     * If the address is not aligned, then get it from the vector `rom_program_na_instructions`, using
//!       as index `(pc-ROM_ADDR)`
//!   * If the address is <= `ROM_ADDR_MAX`, there can be 2 cases:
//!     * If the address is aligned to 4 bytes, then get it from the vector `rom_float_instructions`,
//!       using as index `(pc-FLOAT_LIB_ROM_ADDR)/4`
//!     * If the address is not aligned, then get it from the vector `rom_float_na_instructions`,
//!       using as index `(pc-FLOAT_LIB_ROM_ADDR)`
use crate::{ZiskInst, ZiskInstBuilder, FLOAT_LIB_ROM_ADDR, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::collections::BTreeMap;
use std::error::Error;

// #[cfg(feature = "sp")]
// use crate::SRC_SP;

/// Data section with 64-bit data, used for RW sections that contain initial data
/// It is an evolution of DataSection
#[derive(Debug, Clone)]
pub struct DataSection64 {
    pub addr: u64,
    pub data: Vec<u64>,
}

/// ZisK ROM structure, including a map address to ZisK instruction
#[derive(Default, Debug, Clone)]
pub struct ZiskRom {
    /// Address to be used to build the next instruction (and to be increased afterwards)
    pub next_init_inst_addr: u64,

    /// Map of instructions that are part of the ROM; the key is the ROM address (pc)
    /// This map contains the instructions that are part of the program, i.e. address >= ROM_ADDR
    pub insts: BTreeMap<u64, ZiskInstBuilder>,

    /// RO data sections in u64
    pub ro_data_64: Vec<DataSection64>,

    /// RW data sections in u64
    pub rw_data_64: Vec<DataSection64>,

    // The following vectors are to store subsets of the ROM instructions in order to improve the
    // program execution performance while fetching the instruction for the current step pc
    // address
    /// Vector of ROM instructions with address < ROM_ADDR
    pub rom_bios_instructions: Vec<ZiskInst>,

    /// ROM program instructions with an address that is aligned to 4 bytes
    pub rom_program_instructions: Vec<ZiskInst>,

    /// ROM program instructions with an address that is not aligned to 4 bytes
    pub rom_program_na_instructions: Vec<ZiskInst>,

    /// ROM float instructions with an address that is aligned to 4 bytes
    pub rom_float_instructions: Vec<ZiskInst>,

    /// ROM float instructions with an address that is not aligned to 4 bytes
    pub rom_float_na_instructions: Vec<ZiskInst>,

    /// Maximum entry/bios ROM instruction PC
    pub max_bios_pc: u64,

    /// Maximum program ROM instruction PC
    pub max_program_pc: u64,

    /// Maximum float library ROM instruction PC
    pub max_float_pc: u64,

    /// List of instruction program counter (address) in incremental order:
    /// 0x1000, 0x1004, ..., 0x80000000, 0x80000004, ...
    pub sorted_pc_list: Vec<u64>,

    /// Minimum ROM instruction PC (first program instruction address)
    /// This is typically 0x80000000 but can be different (e.g., 0x80001000 with Go's internal linker)
    pub min_program_pc: u64, // check usage

    /// Used for tracking the instruction creation order in the ROM
    pub build_counter: u64,

    /// Latest internal instruction odd address offset
    /// Initialized to 0, then 1, 3, 5... every time a non-aligned instruction is added to the ROM
    /// Add this value to ROM_ADDR to get the actual address of the internal instruction
    pub last_internal_address_offset: u64,
}

/// ZisK ROM implementation
impl ZiskRom {
    /// Optimizes instruction lookup by organizing instructions into direct-access arrays.
    ///
    /// ## Problem it solves:
    ///
    /// Instead of using a HashMap for every instruction fetch (key is the `pc`),
    /// this creates 5 separate arrays where instructions can be accessed by direct
    /// index/PC calculations.
    ///
    /// Instructions are split into 5 categories:
    ///
    /// 1. Entry/BIOS instructions:
    ///    - Address range: `[ROM_ENTRY, ROM_ADDR)`
    ///    - 4-byte aligned instructions in the startup/BIOS area
    ///    - Accessed via: `array[(addr - ROM_ENTRY) / 4]`
    ///
    /// 2. Main program instructions:
    ///    - Address range: `[ROM_ADDR, FLOAT_LIB_ROM_ADDR)`
    ///    - 4-byte aligned instructions in the main ROM area
    ///    - Accessed via: `array[(addr - ROM_ADDR) / 4]`
    ///
    /// 3. Non-aligned program instructions:
    ///    - Any instruction NOT on a 4-byte boundary
    ///    - Accessed via: `array[addr - ROM_ADDR]`
    ///
    /// 4. Float library program instructions:
    ///    - Address range: `[FLOAT_LIB_ROM_ADDR, ROM_ADDR_MAX]`
    ///    - 4-byte aligned instructions in the float library ROM area
    ///    - Accessed via: `array[(addr - FLOAT_LIB_ROM_ADDR) / 4]`
    ///
    /// 5. Non-aligned float library instructions:
    ///    - Any instruction NOT on a 4-byte boundary
    ///    - Accessed via: `array[addr - FLOAT_LIB_ROM_ADDR]`
    ///
    /// There are two places where this optimization is used:
    ///     - When building traces for proof generation, we iterate through all instructions in address order
    ///     - When running the emulator, each iteration of emulator need to fetch an instruction based on the `pc`
    ///       Using an array vs a hashmap here will be faster due to instructions being next to each other and array cache locality.
    pub fn optimize_instruction_lookup(&mut self) -> Result<(), Box<dyn Error>> {
        // 1. Find the address ranges for each instruction category
        let mut max_bios_address = 0_u64;
        let mut min_program_address = u64::MAX;
        let mut max_program_address = 0_u64;
        //let mut min_program_na_address = u64::MAX;
        let mut max_program_na_address = 0_u64;
        //let mut min_float_address = u64::MAX;
        let mut max_float_address = 0_u64;
        //let mut min_float_na_address = u64::MAX;
        let mut max_float_na_address = 0_u64;

        // Prepare sorted pc list
        if !self.sorted_pc_list.is_empty() {
            return Err(
                 "ZiskRom::optimize_instruction_lookup() sorted_pc_list should be empty before optimization"
                     .into(),
             );
        }
        self.sorted_pc_list.reserve(self.insts.len());

        // Scan all instructions to categorize them and find ranges
        for instruction in &self.insts {
            let addr = *instruction.0;

            // Add to pc list (still unsorted)
            self.sorted_pc_list.push(addr);

            if addr < ROM_ENTRY {
                return Err(format!("Address out of range: {addr}").into());
            } else if addr < ROM_ADDR {
                // Entry/BIOS area
                // Check that all BIOS instructions are 4-byte aligned
                if addr & 0x03 != 0 {
                    return Err(format!(
                        "Non-aligned instruction in entry area at address {addr:#x}"
                    )
                    .into());
                }
                max_bios_address = std::cmp::max(max_bios_address, addr);
            } else if addr < FLOAT_LIB_ROM_ADDR {
                // Main ROM program area
                if addr & 0x03 != 0 {
                    // Non-aligned instruction in main program area
                    //min_program_na_address = std::cmp::min(min_program_na_address, addr);
                    max_program_na_address = std::cmp::max(max_program_na_address, addr);
                } else {
                    // Aligned instruction in main area
                    min_program_address = min_program_address.min(addr);
                    max_program_address = max_program_address.max(addr);
                }
            } else if addr <= ROM_ADDR_MAX {
                // Float library area
                if addr & 0x03 != 0 {
                    // Non-aligned instruction in float library area
                    //min_float_na_address = std::cmp::min(min_float_na_address, addr);
                    max_float_na_address = std::cmp::max(max_float_na_address, addr);
                } else {
                    // Aligned instruction in float library area
                    //min_float_address = min_float_address.min(addr);
                    max_float_address = max_float_address.max(addr);
                }
            } else {
                return Err(format!("Address out of range: {addr}").into());
            }
        }

        // println!("Found instruction address ranges:");
        // println!("\tentry [0x{ROM_ENTRY:x}, 0x{max_bios_address:x}]");
        // println!("\tmain aligned [0x{min_program_address:x}, 0x{max_program_address:x}]");
        // println!("\tmain non-aligned [0x{min_program_na_address:x}, 0x{max_program_na_address:x}]");
        // println!("\tfloat lib aligned [0x{min_float_address:x}, 0x{max_float_address:x}]");
        // println!(
        //     "\tfloat lib non-aligned [0x{min_float_na_address:x}, 0x{max_float_na_address:x}]"
        // );

        self.max_bios_pc = max_bios_address;
        self.max_program_pc = max_program_address.max(max_program_na_address);
        self.max_float_pc = max_float_address.max(max_float_na_address);
        self.min_program_pc =
            if min_program_address == u64::MAX { ROM_ADDR } else { min_program_address };

        let num_bios_instructions =
            if max_bios_address > 0 { (max_bios_address - ROM_ENTRY) / 4 + 1 } else { 0 };
        let num_program_instructions =
            if max_program_address > 0 { (max_program_address - ROM_ADDR) / 4 + 1 } else { 0 };
        let num_program_na_instructions =
            if max_program_na_address > 0 { (max_program_na_address - ROM_ADDR) + 1 } else { 0 };
        let num_float_instructions = if max_float_address > 0 {
            (max_float_address - FLOAT_LIB_ROM_ADDR) / 4 + 1
        } else {
            0
        };
        let num_float_na_instructions = if max_float_na_address > 0 {
            (max_float_na_address - FLOAT_LIB_ROM_ADDR) + 1
        } else {
            0
        };

        // Initialize in parallel to increase performance
        self.rom_bios_instructions =
            (0..num_bios_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();
        self.rom_program_instructions =
            (0..num_program_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();
        self.rom_program_na_instructions =
            (0..num_program_na_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();
        self.rom_float_instructions =
            (0..num_float_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();
        self.rom_float_na_instructions =
            (0..num_float_na_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();

        // Sort pc list
        self.sorted_pc_list.sort();

        // 2. Populate the arrays with instructions at their calculated indices
        for instruction in &self.insts {
            let addr = *instruction.0;

            if addr < ROM_ADDR {
                // Entry/BIOS area: divide by 4 for index (using shift for efficiency)
                self.rom_bios_instructions[((addr - ROM_ENTRY) >> 2) as usize] =
                    instruction.1.i.clone();
            } else if addr < FLOAT_LIB_ROM_ADDR {
                if addr % 4 != 0 {
                    // Non-aligned: store at offset from minimum non-aligned address
                    self.rom_program_na_instructions[(addr - ROM_ADDR) as usize] =
                        instruction.1.i.clone();
                } else {
                    // Main ROM: divide by 4 for index (using shift for efficiency)
                    self.rom_program_instructions[((addr - ROM_ADDR) >> 2) as usize] =
                        instruction.1.i.clone();
                }
            } else if addr <= ROM_ADDR_MAX {
                if addr % 4 != 0 {
                    // Non-aligned: store at offset from minimum non-aligned address
                    self.rom_float_na_instructions[(addr - FLOAT_LIB_ROM_ADDR) as usize] =
                        instruction.1.i.clone();
                } else {
                    // Float library ROM: divide by 4 for index (using shift for efficiency)
                    self.rom_float_instructions[((addr - FLOAT_LIB_ROM_ADDR) >> 2) as usize] =
                        instruction.1.i.clone();
                }
            } else {
                return Err(format!("Address out of range: {addr}").into());
            }
        }

        // 3. Link every instruction with the position they occupy in the sorted pc list
        //
        // The index is stored in two places because instructions exist in:
        // - rom.insts: The original HashMap for random access by PC
        // - rom.*_instructions arrays: The optimized arrays for fast indexed access
        for i in 0..self.sorted_pc_list.len() {
            let pc = self.sorted_pc_list[i];
            self.insts.get_mut(&pc).unwrap().i.sorted_pc_list_index = i;

            let inst = self.get_mut_instruction(pc);
            inst.sorted_pc_list_index = i;
        }

        // println!("Optimized instruction lookup: {} bios instructions, {} program instructions, {} program non-aligned instructions, {} float instructions, {} float non-aligned instructions",
        // num_bios_instructions, num_program_instructions, num_program_na_instructions, num_float_instructions, num_float_na_instructions);

        Ok(())
    }

    /// Gets the ROM instruction corresponding to the provided pc address.
    /// Depending on the range and alignment of the address, the function searches for it in the
    /// corresponding vector.
    #[inline(always)]
    pub fn get_instruction(&self, pc: u64) -> &ZiskInst {
        if pc < ROM_ENTRY {
            // pc is out of range (below ROM_ENTRY)
            panic!(
                "ZiskRom::get_instruction() pc={pc} is out of range (below ROM_ENTRY=0x{:x})",
                ROM_ENTRY
            );
        } else if pc < ROM_ADDR {
            // pc is in the ROM_ENTRY range (always aligned)
            if pc & 0x03 != 0 {
                panic!("ZiskRom::get_instruction() pc=0x{:x} is not aligned to 4 bytes, but it is in the ROM_ENTRY range", pc);
            }
            // pc is aligned to a 4-byte boundary
            let rom_index = ((pc - ROM_ENTRY) >> 2) as usize;
            if rom_index >= self.rom_bios_instructions.len() {
                panic!(
                    "ZiskRom::get_instruction() pc=0x{0:X} ({0}) is out of range rom_bios_instructions (rom_index:{1:} >= {2:})",
                    pc,
                    rom_index,
                    self.rom_bios_instructions.len()
                );
            }
            &self.rom_bios_instructions[rom_index]
        } else if pc < FLOAT_LIB_ROM_ADDR {
            // pc is in the ROM_ADDR range
            // If the address is aligned, take it from the proper vector
            if pc & 0x03 == 0 {
                // pc is aligned to a 4-byte boundary
                let rom_index = ((pc - ROM_ADDR) >> 2) as usize;
                if rom_index >= self.rom_program_instructions.len() {
                    panic!(
                        "ZiskRom::get_instruction() pc=0x{0:X} ({0}) is out of range rom_program_instructions (rom_index:{1:} >= {2:})",
                        pc,
                        rom_index,
                        self.rom_program_instructions.len()
                    );
                }
                &self.rom_program_instructions[rom_index]
                // Otherwise, take it from the non aligned vector
            } else {
                let rom_index = (pc - ROM_ADDR) as usize;
                if rom_index >= self.rom_program_na_instructions.len() {
                    panic!(
                        "ZiskRom::get_instruction() pc={} is out of range rom_program_na_instructions (rom_index:{} >= {})",
                        pc,
                        rom_index,
                        self.rom_program_na_instructions.len()
                    );
                }
                &self.rom_program_na_instructions[rom_index]
            }
        } else if pc <= ROM_ADDR_MAX {
            // pc is in the FLOAT_LIB_ROM_ADDR range
            // If the address is aligned, take it from the proper vector
            if pc & 0x03 == 0 {
                // pc is aligned to a 4-byte boundary
                let rom_index = ((pc - FLOAT_LIB_ROM_ADDR) >> 2) as usize;
                if rom_index >= self.rom_float_instructions.len() {
                    panic!(
                        "ZiskRom::get_instruction() pc=0x{0:X} ({0}) is out of range rom_float_instructions (rom_index:{1:} >= {2:})",
                        pc,
                        rom_index,
                        self.rom_float_instructions.len()
                    );
                }
                &self.rom_float_instructions[rom_index]
                // Otherwise, take it from the non aligned vector
            } else {
                let rom_index = (pc - FLOAT_LIB_ROM_ADDR) as usize;
                if rom_index >= self.rom_float_na_instructions.len() {
                    panic!(
                        "ZiskRom::get_instruction() pc={} is out of range rom_float_na_instructions (rom_index:{} >= {})",
                        pc,
                        rom_index,
                        self.rom_float_na_instructions.len()
                    );
                }
                &self.rom_float_na_instructions[rom_index]
            }
        } else {
            panic!("ZiskRom::get_instruction() pc={pc} is out of range");
        }
    }

    /// Gets the ROM instruction corresponding to the provided pc address, as a mutable reference.
    /// Depending on the range and alignment of the address, the function searches for it in the
    /// corresponding vector.
    #[inline(always)]
    pub fn get_mut_instruction(&mut self, pc: u64) -> &mut ZiskInst {
        if pc < ROM_ENTRY {
            // pc is out of range (below ROM_ENTRY)
            panic!(
                "ZiskRom::get_mut_instruction() pc={pc} is out of range (below ROM_ENTRY=0x{:x})",
                ROM_ENTRY
            );
        } else if pc < ROM_ADDR {
            // pc is in the ROM_ENTRY range (always aligned)
            if pc & 0x03 != 0 {
                panic!("ZiskRom::get_mut_instruction() pc=0x{:x} is not aligned to 4 bytes, but it is in the ROM_ENTRY range", pc);
            }
            // pc is aligned to a 4-byte boundary
            let rom_index = ((pc - ROM_ENTRY) >> 2) as usize;
            if rom_index >= self.rom_bios_instructions.len() {
                panic!(
                    "ZiskRom::get_mut_instruction() pc=0x{0:X} ({0}) is out of range rom_bios_instructions (rom_index:{1:} >= {2:})",
                    pc,
                    rom_index,
                    self.rom_bios_instructions.len()
                );
            }
            &mut self.rom_bios_instructions[rom_index]
        } else if pc < FLOAT_LIB_ROM_ADDR {
            // pc is in the ROM_ADDR range
            // If the address is aligned, take it from the proper vector
            if pc & 0x03 == 0 {
                // pc is aligned to a 4-byte boundary
                let rom_index = ((pc - ROM_ADDR) >> 2) as usize;
                if rom_index >= self.rom_program_instructions.len() {
                    panic!(
                        "ZiskRom::get_mut_instruction() pc=0x{0:X} ({0}) is out of range rom_program_instructions (rom_index:{1:} >= {2:})",
                        pc,
                        rom_index,
                        self.rom_program_instructions.len()
                    );
                }
                &mut self.rom_program_instructions[rom_index]
                // Otherwise, take it from the non aligned vector
            } else {
                let rom_index = (pc - ROM_ADDR) as usize;
                if rom_index >= self.rom_program_na_instructions.len() {
                    panic!(
                        "ZiskRom::get_mut_instruction() pc={} is out of range rom_program_na_instructions (rom_index:{} >= {})",
                        pc,
                        rom_index,
                        self.rom_program_na_instructions.len()
                    );
                }
                &mut self.rom_program_na_instructions[rom_index]
            }
        } else if pc <= ROM_ADDR_MAX {
            // pc is in the FLOAT_LIB_ROM_ADDR range
            // If the address is aligned, take it from the proper vector
            if pc & 0x03 == 0 {
                // pc is aligned to a 4-byte boundary
                let rom_index = ((pc - FLOAT_LIB_ROM_ADDR) >> 2) as usize;
                if rom_index >= self.rom_float_instructions.len() {
                    panic!(
                        "ZiskRom::get_mut_instruction() pc=0x{0:X} ({0}) is out of range rom_float_instructions (rom_index:{1:} >= {2:})",
                        pc,
                        rom_index,
                        self.rom_float_instructions.len()
                    );
                }
                &mut self.rom_float_instructions[rom_index]
                // Otherwise, take it from the non aligned vector
            } else {
                let rom_index = (pc - FLOAT_LIB_ROM_ADDR) as usize;
                if rom_index >= self.rom_float_na_instructions.len() {
                    panic!(
                        "ZiskRom::get_mut_instruction() pc={} is out of range rom_float_na_instructions (rom_index:{} >= {})",
                        pc,
                        rom_index,
                        self.rom_float_na_instructions.len()
                    );
                }
                &mut self.rom_float_na_instructions[rom_index]
            }
        } else {
            panic!("ZiskRom::get_mut_instruction() pc={pc} is out of range");
        }
    }

    /// Gets the next internal instruction address, which is the next odd address after the last one
    pub fn get_internal_address(&mut self) -> u64 {
        // Calculate the new internal instruction address offset, which is the next odd address
        // after the last one, if any
        if self.last_internal_address_offset == 0 {
            // This is the first time we need to get an internal instruction address
            self.last_internal_address_offset = 1;
        } else {
            // Next times: 3, 5, 7, etc.
            self.last_internal_address_offset += 2;
        }

        // Calculate the resulting address by adding the offset to ROM_ADDR
        let result = ROM_ADDR + self.last_internal_address_offset;

        // Check that the result is odd
        if result & 0x01 == 0 {
            panic!("ZiskRom::get_internal_address() result=0x{:x} is not odd", result);
        }

        // Check that the result is not out of range
        if result > ROM_ADDR_MAX {
            panic!("ZiskRom::get_internal_address() result=0x{:x} is out of range (ROM_ADDR_MAX=0x{:x})", result, ROM_ADDR_MAX);
        }

        result
    }
}

pub trait MemDataSection {
    fn ro_sections(&self) -> &[DataSection64];
    fn rw_sections(&self) -> &[DataSection64];
}

impl MemDataSection for ZiskRom {
    fn ro_sections(&self) -> &[DataSection64] {
        &self.ro_data_64
    }

    fn rw_sections(&self) -> &[DataSection64] {
        &self.rw_data_64
    }
}

/********/
/* TEST */
/********/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ZiskInstBuilder, ZiskRom, ROM_ADDR};

    // Helper to create a test instruction with a given opcode
    fn create_test_inst_builder(addr: u64, op: u8) -> ZiskInstBuilder {
        let mut builder = ZiskInstBuilder::new(addr);
        builder.i.op = op;
        builder
    }

    #[test]
    fn test_optimize_empty_rom() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        assert!(rom.optimize_instruction_lookup().is_ok());
        assert_eq!(rom.sorted_pc_list.len(), 0);
        assert_eq!(rom.rom_bios_instructions.len(), 0);
        assert_eq!(rom.rom_program_instructions.len(), 0);
        assert_eq!(rom.rom_program_na_instructions.len(), 0);
        assert_eq!(rom.rom_float_instructions.len(), 0);
        assert_eq!(rom.rom_float_na_instructions.len(), 0);
    }

    #[test]
    fn test_optimize_entry_instructions_only() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add some entry area instructions, but none in main area
        let entry_base = ROM_ENTRY;
        rom.insts.insert(entry_base, create_test_inst_builder(entry_base, 1));
        rom.insts.insert(entry_base + 4, create_test_inst_builder(entry_base + 4, 2));
        rom.insts.insert(entry_base + 8, create_test_inst_builder(entry_base + 8, 3));

        assert!(rom.optimize_instruction_lookup().is_ok());

        // Check arrays are correctly sized
        assert_eq!(rom.rom_bios_instructions.len(), 3);
        assert_eq!(rom.rom_program_instructions.len(), 0);
        assert_eq!(rom.rom_program_na_instructions.len(), 0);
        assert_eq!(rom.rom_float_instructions.len(), 0);
        assert_eq!(rom.rom_float_na_instructions.len(), 0);

        // Check sorted PC list
        assert_eq!(rom.sorted_pc_list, vec![entry_base, entry_base + 4, entry_base + 8]);

        // Verify instructions are at correct indices
        assert_eq!(rom.rom_bios_instructions[0].op, 1);
        assert_eq!(rom.rom_bios_instructions[1].op, 2);
        assert_eq!(rom.rom_bios_instructions[2].op, 3);

        // Check max values
        assert_eq!(rom.max_bios_pc, entry_base + 8);
        assert_eq!(rom.max_program_pc, 0);
    }

    #[test]
    fn test_optimize_main_rom_instructions() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add main ROM area instructions, but none in BIOS area
        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 10));
        rom.insts.insert(ROM_ADDR + 4, create_test_inst_builder(ROM_ADDR + 4, 11));
        rom.insts.insert(ROM_ADDR + 12, create_test_inst_builder(ROM_ADDR + 12, 12)); // Gap in addresses

        assert!(rom.optimize_instruction_lookup().is_ok());

        // Check arrays
        assert_eq!(rom.rom_bios_instructions.len(), 0);
        assert_eq!(rom.rom_program_instructions.len(), 4); // Includes the gap at ROM_ADDR + 8
        assert_eq!(rom.rom_program_na_instructions.len(), 0);
        assert_eq!(rom.rom_float_instructions.len(), 0);
        assert_eq!(rom.rom_float_na_instructions.len(), 0);

        // Check instructions are at correct indices
        assert_eq!(rom.rom_program_instructions[0].op, 10); // (ROM_ADDR - ROM_ADDR) / 4 = 0
        assert_eq!(rom.rom_program_instructions[1].op, 11); // (ROM_ADDR + 4 - ROM_ADDR) / 4 = 1
        assert_eq!(rom.rom_program_instructions[3].op, 12); // (ROM_ADDR + 12 - ROM_ADDR) / 4 = 3

        assert_eq!(rom.max_program_pc, ROM_ADDR + 12);
    }

    #[test]
    fn test_optimize_non_aligned_instructions() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add non-aligned instructions (not on 4-byte boundary)
        rom.insts.insert(ROM_ADDR + 1, create_test_inst_builder(ROM_ADDR + 1, 20));
        rom.insts.insert(ROM_ADDR + 5, create_test_inst_builder(ROM_ADDR + 5, 21));
        rom.insts.insert(ROM_ADDR + 7, create_test_inst_builder(ROM_ADDR + 7, 22));

        assert!(rom.optimize_instruction_lookup().is_ok());

        // Check arrays
        assert_eq!(rom.rom_bios_instructions.len(), 0);
        assert_eq!(rom.rom_program_instructions.len(), 0);
        assert_eq!(rom.rom_float_instructions.len(), 0);
        assert_eq!(rom.rom_float_na_instructions.len(), 0);

        assert_eq!(rom.rom_program_na_instructions.len(), 8); // (ROM_ADDR + 7) - ROM_ADDR + 1

        // Check instructions are at correct indices
        assert_eq!(rom.rom_program_na_instructions[1].op, 20); // (ROM_ADDR+1) - ROM_ADDR = 1
        assert_eq!(rom.rom_program_na_instructions[5].op, 21); // (ROM_ADDR+5) - ROM_ADDR = 5
        assert_eq!(rom.rom_program_na_instructions[7].op, 22); // (ROM_ADDR+7) - ROM_ADDR = 7
    }

    #[test]
    fn test_optimize_mixed_instructions() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Mix of all three types
        rom.insts.insert(ROM_ENTRY + 4, create_test_inst_builder(ROM_ENTRY + 4, 1));
        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 2));
        rom.insts.insert(ROM_ADDR + 3, create_test_inst_builder(ROM_ADDR + 3, 3));

        assert!(rom.optimize_instruction_lookup().is_ok());

        // All three arrays should have content
        assert!(!rom.rom_bios_instructions.is_empty());
        assert!(!rom.rom_program_instructions.is_empty());
        assert!(!rom.rom_program_na_instructions.is_empty());
        assert!(rom.rom_float_instructions.is_empty());
        assert!(rom.rom_float_na_instructions.is_empty());

        // Check sorted list has all PCs
        assert_eq!(rom.sorted_pc_list.len(), 3);
        assert_eq!(rom.sorted_pc_list, vec![ROM_ENTRY + 4, ROM_ADDR, ROM_ADDR + 3]);
    }

    #[test]
    fn test_optimize_sorted_pc_indices() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add instructions out of order
        rom.insts.insert(ROM_ADDR + 8, create_test_inst_builder(ROM_ADDR + 8, 3));
        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 1));
        rom.insts.insert(ROM_ADDR + 4, create_test_inst_builder(ROM_ADDR + 4, 2));

        assert!(rom.optimize_instruction_lookup().is_ok());

        // Check sorted order
        assert_eq!(rom.sorted_pc_list, vec![ROM_ADDR, ROM_ADDR + 4, ROM_ADDR + 8]);

        // Verify each instruction knows its position in sorted list
        assert_eq!(rom.insts.get(&ROM_ADDR).unwrap().i.sorted_pc_list_index, 0);
        assert_eq!(rom.insts.get(&(ROM_ADDR + 4)).unwrap().i.sorted_pc_list_index, 1);
        assert_eq!(rom.insts.get(&(ROM_ADDR + 8)).unwrap().i.sorted_pc_list_index, 2);

        // Also check in the arrays
        assert_eq!(rom.rom_program_instructions[0].sorted_pc_list_index, 0);
        assert_eq!(rom.rom_program_instructions[1].sorted_pc_list_index, 1);
        assert_eq!(rom.rom_program_instructions[2].sorted_pc_list_index, 2);
    }

    #[test]
    fn test_optimize_sorted_pc_indices_with_gaps() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 10));
        rom.insts.insert(ROM_ADDR + 4, create_test_inst_builder(ROM_ADDR + 4, 11));
        rom.insts.insert(ROM_ADDR + 12, create_test_inst_builder(ROM_ADDR + 12, 12));
        rom.insts.insert(ROM_ADDR + 100, create_test_inst_builder(ROM_ADDR + 100, 13));

        assert!(rom.optimize_instruction_lookup().is_ok());

        // Check sorted list has 4 instructions (no gaps in sorted list)
        assert_eq!(rom.sorted_pc_list.len(), 4);
        assert_eq!(rom.sorted_pc_list, vec![ROM_ADDR, ROM_ADDR + 4, ROM_ADDR + 12, ROM_ADDR + 100]);

        // Array has space for all addresses including gaps
        // Array size = (100 - 0) / 4 + 1 = 26 slots
        assert_eq!(rom.rom_program_instructions.len(), 26);

        // rom_program_instructions[0] is at ROM_ADDR
        assert_eq!(rom.rom_program_instructions[0].op, 10);
        assert_eq!(rom.rom_program_instructions[0].sorted_pc_list_index, 0);

        // rom_program_instructions[1] is at ROM_ADDR + 4
        assert_eq!(rom.rom_program_instructions[1].op, 11);
        assert_eq!(rom.rom_program_instructions[1].sorted_pc_list_index, 1);

        // rom_program_instructions[3] is at ROM_ADDR + 12
        assert_eq!(rom.rom_program_instructions[3].op, 12);
        assert_eq!(rom.rom_program_instructions[3].sorted_pc_list_index, 2);

        // rom_program_instructions[25] is at ROM_ADDR + 100
        assert_eq!(rom.rom_program_instructions[25].op, 13);
        assert_eq!(rom.rom_program_instructions[25].sorted_pc_list_index, 3);
    }

    #[test]
    fn test_optimize_address_below_rom_entry_err() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add instruction below ROM_ENTRY
        rom.insts.insert(ROM_ENTRY - 4, create_test_inst_builder(ROM_ENTRY - 4, 1));
        assert!(rom.optimize_instruction_lookup().is_err());
    }

    #[test]
    fn test_optimize_address_above_rom_max_err() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add instruction above ROM_ADDR_MAX.
        rom.insts.insert(ROM_ADDR_MAX + 4, create_test_inst_builder(ROM_ADDR_MAX + 4, 1));
        assert!(rom.optimize_instruction_lookup().is_err());
    }

    #[test]
    fn test_basic_optimize_preserves_instruction_data() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        let mut builder = ZiskInstBuilder::new(ROM_ADDR);
        builder.i.op = 42;
        builder.i.a_src = 1;
        builder.i.b_src = 2;
        builder.i.store = 3;

        rom.insts.insert(ROM_ADDR, builder);

        assert!(rom.optimize_instruction_lookup().is_ok());

        // Verify all fields are preserved
        let stored = &rom.rom_program_instructions[0];
        assert_eq!(stored.op, 42);
        assert_eq!(stored.a_src, 1);
        assert_eq!(stored.b_src, 2);
        assert_eq!(stored.store, 3);
    }
}
