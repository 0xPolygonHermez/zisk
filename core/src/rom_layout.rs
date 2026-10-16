//! Guest-agnostic ROM layout: the fixed BIOS blocks every ZisK ROM starts with, and the
//! normalization of RW data sections into ROM-data initialization rows.
//!
//! These helpers only touch `ZiskRom`/`ZiskInst`, so every producer of a ROM (the RISC-V and
//! WebAssembly transpilers, and `ziskasm` for the ZisK library) shares them from here.

use crate::{
    DataSection, ZiskInst, ZiskInstBuilder, ZiskRom, ARCH_ID_CSR_ADDR, ARCH_ID_ZISK, INPUT_ADDR,
    MAX_ZISK_OS_ROM_ADDR, MTVEC, OUTPUT_ADDR, RAM_ADDR, ROM_ENTRY, ROM_EXIT,
};

#[cfg(feature = "float")]
use crate::{FLOAT_LIB_ROM_ADDR, FLOAT_LIB_SP, FREG_RA, FREG_X0};

/// Value of register a7 (x17) on an ecall that exits the program.
const CAUSE_EXIT: u64 = 93;

/// ROM address of the float handler installed by [`add_end_and_lib`]: RISC-V float
/// instructions jump here to be emulated by the float library.
#[cfg(feature = "float")]
pub const FLOAT_HANDLER_ADDR: u64 = 0x1008;
#[cfg(feature = "float")]
const FLOAT_HANDLER_RETURN_ADDR: u64 = FLOAT_HANDLER_ADDR + 4 * 34; // 31 regs + set sp + set ra + jump to zisk_float

/// ROM address of the block [`add_entry_exit_jmp`] emits: right after [`add_end_and_lib`]'s entry
/// jump, end instruction and, with the `float` feature, float handler.
#[cfg(feature = "float")]
const ENTRY_EXIT_JMP_ADDR: u64 = ROM_ENTRY + 4 * 68;
#[cfg(not(feature = "float"))]
const ENTRY_EXIT_JMP_ADDR: u64 = ROM_ENTRY + 4 * 2;

/// ROM address of the ecall trap handler that [`add_entry_exit_jmp`] stores in MTVEC.
///
/// It depends on this crate's `float` feature, which selects the BIOS layout, so code that jumps
/// to the handler directly must use this constant rather than derive it from its own features.
pub const ECALL_HANDLER_ADDR: u64 = ENTRY_EXIT_JMP_ADDR + 0x54; // must match add_entry_exit_jmp's trap handler offset

/// A library routine's body for an inline zkvmcall (see `ZiskLibrary::inline_body`),
/// as a small control-flow graph: the instructions, entry first, and for each one
/// the index of the instruction it continues to when its flag is set
/// (`jmp_offset1`) and when it is not (`jmp_offset2`), `None` being the end of the
/// call site. A precompile's first target is unused: its `jmp_offset1` is a
/// parameter.
pub struct InlineBody {
    pub insts: Vec<ZiskInst>,
    pub next: Vec<[Option<usize>; 2]>,
}

/// Add the entry/exit jump program section to the rom instruction set.
pub fn add_entry_exit_jmp(rom: &mut ZiskRom, addr: u64) {
    //print!("add_entry_exit_jmp() rom.next_init_inst_addr={}\n", rom.next_init_inst_addr);

    // The trap handler (the ecall section) is at a fixed offset from the start of this block
    assert!(rom.next_init_inst_addr == ENTRY_EXIT_JMP_ADDR);
    let trap_handler: u64 = ECALL_HANDLER_ADDR;

    // :0000 we note the rom pc address offset from the first address for each instruction
    // Store the Zisk architecture ID into memory
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ARCH_ID_ZISK, false);
    zib.op("copyb").unwrap();
    zib.store("mem", ARCH_ID_CSR_ADDR as i64, false, false);
    zib.j(4, 4);
    zib.verbose(&format!("Set marchid: {ARCH_ID_ZISK:x}"));
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0004
    // Store the trap handler address into memory
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", trap_handler, false);
    zib.op("copyb").unwrap();
    zib.store("mem", MTVEC as i64, false, false);
    zib.j(4, 4);
    zib.verbose(&format!("Set mtvec: {trap_handler}"));
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0008
    // Store the input data address into register #10
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", INPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 10, false, false);
    zib.j(0, 4);
    zib.verbose(&format!("Set 1st Param (pInput): 0x{INPUT_ADDR:08x}"));
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :000c
    // Store the output data address into register #11
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", OUTPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 11, false, false);
    zib.j(0, 4);
    zib.verbose(&format!("Set 2nd Param (pOutput): 0x{OUTPUT_ADDR:08x}"));
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0010
    // Call to the program rom pc address, i.e. call the program
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", addr, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.store_pc("reg", 1, false);
    zib.j(0, 4);
    zib.verbose(&format!("CALL to entry: 0x{addr:08x}"));
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0014
    // Returns from the program execution.
    // Reads output data using the specific pubout operation in 32 chunks of 64 bits:
    //
    // loadw: c(reg11) = b(32), a=0
    // copyb: c(reg12)=b=0, a=0
    // copyb: c(reg13)=b=OUTPUT_ADDR, a=0
    //
    // eq: if reg12==reg11 jump to end
    // pubout: c=b.mem(reg13), a = reg12
    // add: reg13 = reg13 + 8 // Increment memory address
    // add: reg12 = reg12 + 1, jump -12 // Increment index, goto eq
    //
    // end
    //
    // Copy output data address into register #1
    // copyb: reg11 = c = b = mem(OUTPUT_ADDR,4), a=0
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 32, false);
    zib.ind_width(4);
    zib.op("copyb").unwrap();
    zib.store("reg", 11, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg11 to output data length = 32");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0018 -> copyb: copyb: c(reg12)=b=0, a=0
    // Set register #12 to zero
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 12, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg12 to 0");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :001c -> copyb: c(reg13)=b=OUTPUT_ADDR, a=0
    // Set register #13 to OUTPUT_ADDR, i.e. to the beginning of the actual data after skipping
    // the data length value
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", OUTPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 13, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg13 to OUTPUT_ADDR");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0020 -> eq: if reg12==reg11 jump to end
    // Jump to end if registers #11 and #12 are equal, to break the data copy loop
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 11, false);
    zib.src_b("reg", 12, false);
    zib.op("eq").unwrap();
    zib.store("none", 0, false, false);
    zib.j(20, 4);
    zib.verbose("If reg11==reg12 jump to end");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0024 -> copyb: c = b = mem(reg13, 8)
    // Copy the contents of memory at address set by register #13 into c, i.e. copy output data chunk
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 13, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.store("none", 0, false, false);
    zib.j(0, 4);
    zib.verbose("Set c to mem(output_data[index]), a=index");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0028 -> pubout: c = last_c = mem(reg13, 8), a = reg12 = index
    // Call the special operation pubout with this data, being a the data chunk index
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 12, false);
    zib.src_b("lastc", 0, false);
    zib.op("pubout").unwrap();
    zib.store("none", 0, false, false);
    zib.j(0, 4);
    zib.verbose("Public output, set c to output_data[index], a=index");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :002c -> add: reg13 = reg13 + 8
    // Increase the register #13, i.e. the data address, in 8 units
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 13, false);
    zib.src_b("imm", 8, false);
    zib.op("add").unwrap();
    zib.store("reg", 13, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg13 to reg13 + 8");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0030 -> add: reg12 = reg12 + 1, jump -16
    // Increase the register #12, i.e. the data chunk index, in 1 unit.
    // Jump to the beginning of the output data read loop
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 12, false);
    zib.src_b("imm", 1, false);
    zib.op("add").unwrap();
    zib.store("reg", 12, false, false);
    zib.j(4, -16);
    zib.verbose("Set reg12 to reg12 + 1");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // We read the input data boundaries of 128MB chunks to make sure we can prove large input data
    // sizes that are not continuous, i.e. when the program reads 2 input data chunks distant more
    // than 128MB, we can still prove the program by reading the input data in 128MB steps

    // :0034 -> read input[128M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[128M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0038 -> read input[256M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 2 * 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[256M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :003c -> read input[384M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 3 * 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[384M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0040 -> read input[512M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 4 * 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[512M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0044 -> read input[640M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 5 * 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[640M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0048 -> read input[768M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 6 * 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[768M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :004c -> read input[896M]
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", INPUT_ADDR + 7 * 128 * 1024 * 1024, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.j(4, 4);
    zib.verbose("Read input[896M]");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0050 jump to end (success)
    // Jump to the last instruction (ROM_EXIT) to properly finish the program execution
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ROM_EXIT, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, 0);
    zib.verbose("jump to end successfully");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0054 trap_handle -> This is the address offset we use at the beginning of the function
    // This code is executed when the program makes an ecall (system call).
    // The pc is set to this address, and after the system call, it returns to the pc next to the
    // one that made the ecall
    // If register a7==CAUSE_EXIT, jump to the exit code check (:005c); otherwise
    // return to the caller (:0058)
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 17, false);
    zib.src_b("imm", CAUSE_EXIT, false);
    zib.op("eq").unwrap();
    zib.j(8, 4);
    zib.verbose(&format!("beq r17, {CAUSE_EXIT} # Check if is exit, jump to the exit code check"));
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0058
    // Return to the instruction next to the one that made this ecall
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("reg", 1, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, 4);
    zib.verbose("ret");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :005c
    // Exit: the exit code is in a0 (register #10). 0 means success: publish the output
    // and end (:0014); anything else is a failed execution (:0060), which must not be
    // proven
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 10, false);
    zib.src_b("imm", 0, false);
    zib.op("eq").unwrap();
    zib.j(-72, 4);
    zib.verbose("beq r10, 0 # Exit code 0: jump to output, then end");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0060
    // Nonzero exit code: end the execution with an error. The halt reads a0 as its `a`
    // operand, so the error carries the exit code (a trap's halt has a = 0)
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 10, false);
    zib.src_b("imm", 0, false);
    zib.op("halt").unwrap();
    zib.j(0, 0);
    zib.end();
    zib.verbose("halt # Nonzero exit code: failed execution");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // Check resulting rom address does not exceed max
    if rom.next_init_inst_addr > MAX_ZISK_OS_ROM_ADDR {
        panic!(
            "add_entry_exit_jmp() exceeded max rom address: next_init_inst_addr={:#x} max={:#x}",
            rom.next_init_inst_addr, MAX_ZISK_OS_ROM_ADDR
        );
    }
}

/// Add the end jump program section to the rom instruction set.
pub fn add_end_and_lib(rom: &mut ZiskRom) {
    //print!("add_entry_exit_jmp() rom.next_init_inst_addr={}\n", rom.next_init_inst_addr);

    // :0000 we jump to the third instruction, leaving room for the end instruction
    assert!(rom.next_init_inst_addr == ROM_ENTRY);
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    let jump = (ENTRY_EXIT_JMP_ADDR - ROM_ENTRY) as i64;
    zib.j(jump, jump);
    #[cfg(feature = "float")]
    zib.verbose("Jump over end instruction and float handler");
    #[cfg(not(feature = "float"))]
    zib.verbose("Jump over end instruction");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    // :0004 END: all programs should exit here, regardless of the execution result
    // This is the last instruction to be executed.  The emulator must stop after the instruction
    // end flag is found to be true
    assert!(rom.next_init_inst_addr == ROM_EXIT);
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.end();
    zib.j(0, 0);
    zib.verbose("end");
    zib.build(rom);
    rom.next_init_inst_addr += 4;

    #[cfg(feature = "float")]
    {
        // Float handler
        // RISC-V float instructions are handled here
        // The instruction to be handled is in register FREG_INST
        // The return address is in register FREG_RA
        // We must save integer registers before calling the zisk_float function
        assert!(rom.next_init_inst_addr == FLOAT_HANDLER_ADDR);
        for i in 1..32 {
            let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
            zib.src_a("imm", 0, false);
            zib.src_b("reg", i, false);
            zib.op("copyb").unwrap();
            zib.store("mem", FREG_X0 as i64 + (i * 8) as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("Float: save r{i} into freg_x{i}"));
            zib.build(rom);
            rom.next_init_inst_addr += 4;
        }

        // Set sp to the top of the float library stack
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", FLOAT_LIB_SP, false);
        zib.op("copyb").unwrap();
        zib.store("reg", 2, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Float: save FLOAT_LIB_SP={FLOAT_LIB_SP:x} into reg[2]"));
        zib.build(rom);
        rom.next_init_inst_addr += 4;

        // Set the return address to the FLOAT_HANDLER_RETURN_ADDR
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", FLOAT_HANDLER_RETURN_ADDR, false);
        zib.op("copyb").unwrap();
        zib.store("reg", 1, false, false);
        zib.j(4, 4);
        zib.verbose(&format!(
            "Float: save FLOAT_HANDLER_RETURN_ADDR={FLOAT_HANDLER_RETURN_ADDR:x} into reg[1]"
        ));
        zib.build(rom);
        rom.next_init_inst_addr += 4;

        // Jump back to the zisk_float function address
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", FLOAT_LIB_ROM_ADDR, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.j(0, 4);
        zib.verbose(&format!("Float: jump to FLOAT_LIB_ROM_ADDR={FLOAT_LIB_ROM_ADDR:x}"));
        zib.build(rom);
        rom.next_init_inst_addr += 4;

        // We must retrieve integer registers after calling the zisk_float function
        assert!(rom.next_init_inst_addr == FLOAT_HANDLER_RETURN_ADDR);
        for i in 1..32 {
            let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
            zib.src_a("imm", 0, false);
            zib.src_b("mem", FREG_X0 + (i * 8), false);
            zib.op("copyb").unwrap();
            zib.store("reg", i as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("Float: restore r{i} from freg_x{i}"));
            zib.build(rom);
            rom.next_init_inst_addr += 4;
        }

        // Jump back to the address previously stored in FREG_RA
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", 0, false);
        zib.src_b("mem", FREG_RA, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.j(0, 4);
        zib.verbose("Float: jump to FREG_RA");
        zib.build(rom);
        rom.next_init_inst_addr += 4;
    }

    assert!(rom.next_init_inst_addr == ENTRY_EXIT_JMP_ADDR);

    // Check resulting rom address does not exceed max
    if rom.next_init_inst_addr > MAX_ZISK_OS_ROM_ADDR {
        panic!(
            "add_end_and_lib() exceeded max rom address: next_init_inst_addr={:#x} max={:#x}",
            rom.next_init_inst_addr, MAX_ZISK_OS_ROM_ADDR
        );
    }
}

/// A ROM data row initializes 32 bytes (4 u64), so RW sections must be sized in
/// whole 32-byte blocks.
const ROM_DATA_BLOCK: usize = 32;
/// Minimum internal all-zero region worth carving out of a section (1 KiB = 32
/// blocks). RAM reads as zero, so carved regions are simply left uninitialized.
const MIN_CARVE_ZEROS: usize = 1024;

/// Compile-time guarantee that RAM starts on a 32-byte block boundary. The section
/// normalization clamps the downward expansion to `RAM_ADDR`, which only preserves
/// block alignment (and the length-multiple-of-32 invariant) if `RAM_ADDR` is itself
/// 32-byte aligned. Fails the build if someone ever changes `RAM_ADDR` otherwise.
const _: () = assert!(
    RAM_ADDR % ROM_DATA_BLOCK as u64 == 0,
    "RAM_ADDR must be aligned to the 32-byte ROM data block size"
);

/// Appends the block range `[start_block, end_block)` of `section` as its own
/// `DataSection` (copied once), skipping empty ranges. Block indices are 32-byte
/// units, so the resulting address and length stay 32-byte aligned.
fn push_block_range(
    out: &mut Vec<DataSection>,
    section: &DataSection,
    start_block: usize,
    end_block: usize,
) {
    if end_block > start_block {
        let bytes = &section.data[start_block * ROM_DATA_BLOCK..end_block * ROM_DATA_BLOCK];
        out.push(DataSection {
            addr: section.addr + (start_block * ROM_DATA_BLOCK) as u64,
            data: bytes.to_vec(),
        });
    }
}

/// Normalizes RW data sections for ROM-data initialization. In one pass it:
/// 1. trims leading/trailing zeros and expands every section to whole 32-byte blocks
///    aligned on absolute 32-byte boundaries (dropping fully-zero sections),
/// 2. merges sections whose expanded ranges overlap or touch, so no address is
///    initialized twice (which the memory argument forbids), and
/// 3. carves out every internal all-zero region of >= 1 KiB (block-aligned),
///    splitting the sections around them so those zeros are not initialized.
///
/// Output invariants: every section is non-empty, both its address and length are
/// multiples of 32 bytes, and no two sections overlap. Sections are returned sorted
/// by address.
///
/// Aligning to absolute 32-byte boundaries (rather than to each section's own start)
/// keeps every section on the same block grid, so merging two sections always yields
/// a length that is still a multiple of 32.
///
/// The downward expansion is clamped to `RAM_ADDR` (the lowest writable RAM address):
/// it must never cross below it, since that region is the stack guard. `RAM_ADDR` is
/// 32-byte aligned (checked at compile time above), so clamping preserves alignment.
pub fn normalize_rw_data_sections(sections: Vec<DataSection>) -> Vec<DataSection> {
    const BLOCK: u64 = ROM_DATA_BLOCK as u64;

    // ---- Phase 1: trim + expand to absolute 32-byte block bounds -----------
    let mut expanded: Vec<DataSection> = sections
        .into_iter()
        .filter_map(|section| {
            let data = section.data;
            let len = data.len() as u64;

            // Locate the first and last non-zero bytes; drop fully-zero sections.
            let first = data.iter().position(|&b| b != 0)? as u64;
            let last = data.iter().rposition(|&b| b != 0).unwrap() as u64;

            // Expand [first_addr, last_addr] outwards to absolute 32-byte blocks.
            // `end_addr` is exclusive; both bounds are block-aligned so the length is
            // a multiple of 32. `start_addr` may fall below `section.addr` and
            // `end_addr` above its end when the section is not block-aligned; the
            // out-of-section bytes are zero-filled.
            let first_addr = section.addr + first;
            let last_addr = section.addr + last;
            // Clamp the downward expansion so it never crosses below RAM start.
            let start_addr = (first_addr & !(BLOCK - 1)).max(RAM_ADDR);
            let end_addr = (last_addr & !(BLOCK - 1)) + BLOCK;
            // A whole section below RAM start would be clamped to nothing; drop it
            // (such sections are already rejected by the RAM-bounds check upstream).
            if start_addr >= end_addr {
                return None;
            }

            let mut out = vec![0u8; (end_addr - start_addr) as usize];
            // Copy the bytes that actually exist ([section.addr, section.addr+len))
            // into their place within the expanded block range.
            let copy_from = start_addr.max(section.addr);
            let copy_to = end_addr.min(section.addr + len);
            let dst = (copy_from - start_addr) as usize;
            let src = (copy_from - section.addr) as usize;
            let n = (copy_to - copy_from) as usize;
            out[dst..dst + n].copy_from_slice(&data[src..src + n]);

            Some(DataSection { addr: start_addr, data: out })
        })
        .collect();

    // ---- Phase 2: merge sections that overlap or touch once expanded -------
    expanded.sort_by_key(|s| s.addr);
    let mut merged: Vec<DataSection> = Vec::with_capacity(expanded.len());
    for section in expanded {
        if let Some(prev) = merged.last_mut() {
            let prev_end = prev.addr + prev.data.len() as u64;
            if section.addr <= prev_end {
                // Overlap or touch: fold `section` into `prev`. Both are block-aligned
                // so the combined length stays a multiple of 32.
                let prev_orig_len = prev.data.len();
                let sec_end = section.addr + section.data.len() as u64;
                if sec_end > prev_end {
                    prev.data.resize((sec_end - prev.addr) as usize, 0);
                }
                let off = (section.addr - prev.addr) as usize;
                for (k, &b) in section.data.iter().enumerate() {
                    let idx = off + k;
                    if idx < prev_orig_len {
                        // In the overlap keep whatever is non-zero: real data never
                        // overlaps across sections, so at most one side is non-zero.
                        if b != 0 {
                            prev.data[idx] = b;
                        }
                    } else {
                        prev.data[idx] = b;
                    }
                }
                continue;
            }
        }
        merged.push(section);
    }

    // ---- Phase 3: carve out internal all-zero regions of >= MIN_CARVE_ZEROS ----
    // RAM reads as zero, so a large internal zero run needs no initialization: split
    // the section around it. Runs are internal by construction (phase 1 makes the
    // first and last block non-zero), so every emitted piece is non-empty.
    let min_blocks = MIN_CARVE_ZEROS / ROM_DATA_BLOCK;
    let block_is_zero = |data: &[u8], b: usize| {
        data[b * ROM_DATA_BLOCK..(b + 1) * ROM_DATA_BLOCK].iter().all(|&x| x == 0)
    };

    let mut result: Vec<DataSection> = Vec::with_capacity(merged.len());
    for section in merged {
        let nblocks = section.data.len() / ROM_DATA_BLOCK;
        let mut piece_start = 0usize; // first block of the current kept piece
        let mut carved = false;
        let mut b = 0;
        while b < nblocks {
            if !block_is_zero(&section.data, b) {
                b += 1;
                continue;
            }
            let run_start = b;
            while b < nblocks && block_is_zero(&section.data, b) {
                b += 1;
            }
            // Carve only large enough runs; smaller ones stay inside the piece.
            if b - run_start >= min_blocks {
                push_block_range(&mut result, &section, piece_start, run_start);
                piece_start = b;
                carved = true;
            }
        }
        if carved {
            push_block_range(&mut result, &section, piece_start, nblocks);
        } else {
            result.push(section); // untouched: no copy
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The BIOS blocks line up with ENTRY_EXIT_JMP_ADDR (the asserts in both functions), and the
    /// address stored in MTVEC is ECALL_HANDLER_ADDR, with or without the `float` feature.
    #[test]
    fn mtvec_is_ecall_handler_addr() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };
        add_end_and_lib(&mut rom);
        add_entry_exit_jmp(&mut rom, crate::ROM_ADDR);
        let set_mtvec = &rom.insts[&(ENTRY_EXIT_JMP_ADDR + 4)].i;
        assert_eq!(set_mtvec.store_offset, MTVEC as i64);
        assert_eq!(set_mtvec.b_offset_imm0, ECALL_HANDLER_ADDR);
    }

    // Fixtures live inside real RAM so the RAM_ADDR clamp is exercised realistically.
    const BASE: u64 = RAM_ADDR;

    fn ds(addr: u64, data: Vec<u8>) -> DataSection {
        DataSection { addr, data }
    }

    /// Every output section must satisfy the normalization invariants.
    fn assert_invariants(out: &[DataSection]) {
        for s in out {
            assert!(!s.data.is_empty(), "empty section at 0x{:x}", s.addr);
            assert_eq!(s.addr % 32, 0, "section addr 0x{:x} not 32-aligned", s.addr);
            assert_eq!(s.data.len() % 32, 0, "section len {} not multiple of 32", s.data.len());
        }
        // No overlaps (output is sorted by address).
        for w in out.windows(2) {
            assert!(
                w[0].addr + w[0].data.len() as u64 <= w[1].addr,
                "sections overlap: 0x{:x}+{} > 0x{:x}",
                w[0].addr,
                w[0].data.len(),
                w[1].addr
            );
        }
    }

    #[test]
    fn trims_and_expands_to_absolute_32_blocks() {
        // Single non-zero byte at offset 40 in a 64-byte section at a 32-aligned base.
        let mut data = vec![0u8; 64];
        data[40] = 0xAB;
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1);
        // Byte 40 lives in the block [BASE+0x20, BASE+0x40).
        assert_eq!(out[0].addr, BASE + 0x20);
        assert_eq!(out[0].data.len(), 32);
        assert_eq!(out[0].data[8], 0xAB);
    }

    #[test]
    fn drops_fully_zero_sections() {
        let out = normalize_rw_data_sections(vec![ds(BASE, vec![0u8; 128])]);
        assert!(out.is_empty());
    }

    #[test]
    fn aligns_section_with_unaligned_base() {
        // Base not 32-aligned; the output must still be 32-aligned.
        let out = normalize_rw_data_sections(vec![ds(BASE + 8, vec![0x11])]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data[8], 0x11); // byte at BASE+8 -> offset 8
    }

    #[test]
    fn merges_sections_that_collide_after_expansion() {
        // Two 1-byte sections 16 bytes apart both expand to [BASE, BASE+0x20).
        let a = ds(BASE, vec![0x01]);
        let b = ds(BASE + 0x10, vec![0x02]);
        let out = normalize_rw_data_sections(vec![a, b]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1, "colliding sections must merge");
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data.len(), 32);
        assert_eq!(out[0].data[0], 0x01); // preserved from A
        assert_eq!(out[0].data[16], 0x02); // preserved from B
    }

    #[test]
    fn does_not_merge_far_sections() {
        let a = ds(BASE, vec![0x01]);
        let b = ds(BASE + 0x1000, vec![0x02]);
        let out = normalize_rw_data_sections(vec![a, b]);
        assert_invariants(&out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[1].addr, BASE + 0x1000);
    }

    #[test]
    fn carves_large_internal_zero_run() {
        // block0 non-zero, then exactly 32 zero blocks (1 KiB), then a non-zero block.
        let mut data = vec![0u8; 32 + 1024 + 32];
        data[0] = 0x01;
        data[1056] = 0x02;
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        assert_eq!(out.len(), 2, "the 1 KiB zero hole must be carved out");
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data.len(), 32);
        assert_eq!(out[0].data[0], 0x01);
        assert_eq!(out[1].addr, BASE + 1056);
        assert_eq!(out[1].data.len(), 32);
        assert_eq!(out[1].data[0], 0x02);
    }

    #[test]
    fn keeps_small_internal_zero_run() {
        // Only 16 zero blocks (512 bytes) between the ends: below the 1 KiB threshold.
        let mut data = vec![0u8; 32 + 512 + 32];
        data[0] = 0x01;
        data[544] = 0x02;
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1, "sub-1 KiB zero holes stay initialized");
        assert_eq!(out[0].data.len(), 32 + 512 + 32);
    }

    #[test]
    fn never_expands_below_ram_start() {
        // A section based just above RAM start with a non-32-aligned base: the
        // downward expansion floors to RAM_ADDR and never below it.
        let out = normalize_rw_data_sections(vec![ds(BASE + 0x10, vec![0x22])]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1);
        assert!(out[0].addr >= RAM_ADDR, "must not expand below RAM start");
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data[0x10], 0x22);

        // A section entirely below RAM start is dropped rather than initialized.
        let out = normalize_rw_data_sections(vec![ds(RAM_ADDR - 0x1000, vec![0x33])]);
        assert!(out.is_empty());
    }

    #[test]
    fn carves_all_large_zero_runs() {
        // One section with 20 large (1 KiB) zero runs separated by non-zero blocks:
        // 21 non-zero blocks, 20 runs. Every run is carved out.
        let run = 1024usize;
        let nz = 32usize;
        let runs = 20usize;
        let mut data = vec![0u8; nz + runs * (run + nz)];
        // mark each non-zero block's first byte
        for i in 0..=runs {
            data[i * (run + nz)] = 1;
        }
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        // 20 carves split the section into 21 single-block pieces.
        assert_eq!(out.len(), 21);
        for piece in &out {
            assert_eq!(piece.data.len(), 32);
        }
    }
}
