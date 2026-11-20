//! Zisk ROM to C
//!
//! Generates C code that implements the Zisk ROM program.
//!
//! This is the C counterpart of `zisk_rom_2_asm`: both consume the same `ZiskRom` and produce a
//! program exposing the same symbols to the C runtime in `emulator-asm/src`, so that either one can
//! be linked into the emulator.  Everything the two backends must agree on (generation methods,
//! trace layouts, fcall context layout, precompile queries) lives in `zisk_rom_2_code`.
//!
//! The difference between the two backends is not syntax, it is register allocation.  The assembly
//! backend allocates a, b, c, flag, step and pc to fixed machine registers and decides, per
//! instruction, which of them may be overwritten (`store_a_in_c`, `store_b_in_b`, …).  Here they are
//! plain C locals and the C compiler does that job, which is the whole point of this backend.  What
//! is kept from the assembly backend is the target-neutral half of its optimizer: constant
//! propagation of a and b, and static knowledge of the flag value, which is what makes most jumps
//! resolvable at generation time.
//!
//! Control flow is expressed with labels and `goto`: one label per ROM program counter, a `goto` for
//! every static jump, and GCC's computed goto over a label-address table for dynamic jumps, mirroring
//! the `map_pc_*` branch table of the assembly backend.  This means the generated code is GCC/Clang
//! specific, which is no new restriction: the emulator is already built with gcc.
//!
//! Scope: `AsmGenerationMethod::AsmFast` and `AsmGenerationMethod::AsmMinimalTraces` are
//! implemented.  The remaining methods (ROM histogram, mem op) panic rather than silently emitting a
//! program that produces no trace.
//!
//! The minimal trace is written exactly as the assembly backend writes it, because the same
//! consumers read it: the chunk layout and the order of the mem reads are a contract, not an
//! implementation detail, so anything that reads memory here records the same words, in the same
//! order, as its assembly counterpart.  Two things are borrowed from the runtime instead of being
//! written again: the DMA operations, whose trace encoding lives in hand-written assembly and is
//! reached through the trampolines below, and `_chunk_done()` / `_realloc_trace()`.

use std::path::Path;

use crate::{
    zisk_ops::ZiskOp,
    zisk_rom_2_code::{
        AsmGenerationMethod, PrecompileResults, FCALL_FUNCTION_ID, FCALL_LENGTH, FCALL_PARAMS,
        FCALL_PARAMS_CAPACITY, FCALL_PARAMS_LENGTH, FCALL_PARAMS_SIZE, FCALL_RESULT,
        FCALL_RESULT_CAPACITY, FCALL_RESULT_GOT, FCALL_RESULT_LENGTH, FCALL_RESULT_SIZE,
    },
    ZiskInst, ZiskRom, EXTRA_PARAMS_ADDR, FLOAT_LIB_ROM_ADDR, FREE_INPUT_ADDR, INPUT_ADDR, M64,
    ROM_ADDR, ROM_ENTRY, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP, STORE_IND, STORE_MEM,
    STORE_NONE, STORE_REG, UART_ADDR,
};
use ziskos::zisklib::FCALL_INPUT_READY_ID;

/// Number of ZisK registers, i.e. the 32 RISC-V registers plus the ZisK-specific ones
const ZISK_REGS: u64 = 35;

/// Size, in 8-byte words, of the fixed part of a minimal trace chunk, i.e. everything that comes
/// before its mem reads.  See the layout in `preamble_minimal_trace()`.
const ZISK_CHUNK_HEADER: u64 = 41;

/// Registers whose value a minimal trace chunk records as part of the state it starts from: 1 to 33.
/// Register 0 is always zero, and the ZisK-specific registers above 33 are not part of that state.
const ZISK_CHUNK_START_REGS: u64 = 33;

/// Address of the counter of input bytes written so far, in the control input shared memory.
///
/// The client or the server writes it as the input data arrives, so this code only ever reads it.
/// The same literal address is used by the assembly backend, see its `mem_input_written_address`.
const INPUT_WRITTEN_ADDR: u64 = 0x7000_0010;

/// The address of a memory access, as far as the generator knows it.
///
/// What it is used for is deciding what the minimal trace has to record, which depends on the
/// alignment of the address: when the address is known at generation time, so is its alignment, and
/// the check disappears from the generated code.
enum MtAddr {
    /// Known at generation time
    Const(u64),
    /// Only known at run time; the C expression that reads it
    Dynamic(String),
}

/// What is statically known about the flag produced by an operation.
///
/// This is the C equivalent of the assembly backend's `flag_is_always_zero` / `flag_is_always_one`
/// pair, and it serves the same purpose: when the flag is known at generation time the jump that
/// follows the operation is unconditional, so it becomes a plain `goto` (or is elided altogether
/// when the target is the next instruction).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum FlagState {
    /// The flag value is only known at run time; it is held in the `flag` local
    #[default]
    Dynamic,
    /// The operation always clears the flag
    AlwaysZero,
    /// The operation always sets the flag
    AlwaysOne,
}

/// One of the a, b or c registers of a ZisK instruction, as seen by the C backend.
///
/// Unlike the assembly backend there is no machine register to track: what matters is only whether
/// the value is a compile-time constant, and the C expression that reads it.
#[derive(Default, Debug, Clone)]
struct ZiskCRegister {
    /// Register holds a constant value known at generation time
    is_constant: bool,
    /// Register constant value, only valid if is_constant == true
    constant_value: u64,
    /// C expression that evaluates to the register value, e.g. "a", "reg[3]" or "0x20UL"
    expr: String,
}

#[derive(Default, Debug, Clone)]
pub struct ZiskCContext {
    /// Program counter of the instruction being generated
    pc: u64,
    /// Program counter of the instruction that will be generated next, used to elide jumps
    next_pc: u64,
    mode: AsmGenerationMethod,
    /// True to emit comments in the generated C source code
    comments: bool,
    /// True to call the runtime's character output when the program writes to the UART address
    log_output: bool,
    /// True to call the runtime's `_print_pc()` at the end of every instruction.
    ///
    /// This is a debugging aid: it prints one line per executed instruction, which is what makes the
    /// execution comparable, step by step, against another emulator.  It is enormously slow and
    /// verbose, so it is off unless asked for.
    print_pc: bool,
    /// Which precompiles provide their results to this code
    precompile_results: PrecompileResults,
    a: ZiskCRegister,
    b: ZiskCRegister,
    c: ZiskCRegister,
    /// What is known about the flag produced by the current instruction's operation
    flag: FlagState,

    /// Target number of instructions per generated C function, or 0 to emit the whole ROM as one
    /// function.
    ///
    /// One function per ROM does not scale: the C compiler's per-function passes grow worse than
    /// quadratically with the number of basic blocks, so a real ROM never finishes compiling.
    /// Splitting the instruction stream into functions caps every pass at a size where the cost is
    /// still linear, at the price of spilling the machine state at the boundaries.
    chunk_size: u64,
    /// Lowest program counter of the chunk being generated, inclusive
    chunk_lo: u64,
    /// Highest program counter of the chunk being generated, inclusive
    chunk_hi: u64,
}

impl ZiskCContext {
    pub fn fast(&self) -> bool {
        self.mode.is_fast()
    }

    /// True if the generated program writes the minimal trace
    fn minimal_trace(&self) -> bool {
        self.mode.is_minimal_trace()
    }

    /// True if the ROM is split into several C functions instead of one
    fn chunked(&self) -> bool {
        self.chunk_size != 0
    }

    /// True if the given program counter belongs to the chunk being generated, i.e. a jump to it can
    /// be a direct goto instead of a return to the dispatcher
    fn in_chunk(&self, pc: u64) -> bool {
        if !self.chunked() {
            return true;
        }

        // An internal instruction lives at an odd program counter and is generated right after the
        // instruction that depends on it, so it is in whatever chunk that one fell into, not in the
        // chunk its own program counter would suggest.  Nothing else can reach it: odd addresses get
        // no entry in the dispatch tables, precisely because the only way in is this fall-through.
        if pc & 0x1 != 0 {
            return true;
        }

        (pc >= self.chunk_lo) && (pc <= self.chunk_hi)
    }

    /// Creates an end-of-line comment with C syntax, or nothing if comments are disabled
    fn comment(&self, c: String) -> String {
        if self.comments {
            format!(" /* {c} */")
        } else {
            String::new()
        }
    }

    fn comment_str(&self, c: &str) -> String {
        self.comment(c.to_string())
    }

    /// Creates a full-line comment, or nothing if comments are disabled
    fn full_line_comment(&self, c: String) -> String {
        if self.comments {
            format!("\t/* {c} */\n")
        } else {
            String::new()
        }
    }
}

/// Formats a u64 as a C unsigned long long literal
fn u64_lit(value: u64) -> String {
    format!("0x{value:x}ULL")
}

/// Formats an i64 offset as a C expression that can be added to a u64 address without signed
/// overflow, preserving two's complement wrap-around exactly like the assembly backend's `add`
fn offset_lit(offset: i64) -> String {
    if offset >= 0 {
        format!("0x{offset:x}ULL")
    } else {
        format!("(uint64_t)INT64_C({offset})")
    }
}

pub struct ZiskRom2C {}

impl ZiskRom2C {
    /// Saves ZisK rom into a C source file: first save to a string, then save the string to the file
    #[allow(clippy::too_many_arguments)]
    pub fn save_to_c_file(
        rom: &ZiskRom,
        file_name: &Path,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
        precompile_results: bool,
        chunk_size: u64,
        print_pc: bool,
    ) {
        // Get a string with the C data
        let mut s = String::new();
        Self::save_to_c(
            rom,
            &mut s,
            generation_method,
            log_output,
            comments,
            precompile_results,
            chunk_size,
            print_pc,
        );

        // Save to file
        let path = std::path::PathBuf::from(file_name);
        let result = std::fs::write(path, s);
        if result.is_err() {
            panic!("ZiskRom2C::save_to_c_file() failed writing to file={}", file_name.display());
        }
    }

    /// Saves ZisK rom into a C source data string
    #[allow(clippy::too_many_arguments)]
    pub fn save_to_c(
        rom: &ZiskRom,
        code: &mut String,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
        precompile_results: bool,
        chunk_size: u64,
        print_pc: bool,
    ) {
        // The ROM histogram and mem op methods are not implemented yet.  Refuse them instead of
        // emitting a program that runs but silently produces no trace.
        if !generation_method.is_fast() && !generation_method.is_minimal_trace() {
            panic!(
                "ZiskRom2C::save_to_c() generation_method={generation_method:?} is not implemented \
                 by the C backend yet; only AsmFast and AsmMinimalTraces are.  Use ZiskRom2Asm for \
                 the other methods."
            );
        }
        assert!(
            !precompile_results,
            "ZiskRom2C::save_to_c() precompile_results is not implemented by the C backend yet"
        );

        // Clear output data, just in case
        code.clear();

        // Create context
        let mut ctx = ZiskCContext {
            mode: generation_method,
            comments,
            log_output,
            precompile_results: PrecompileResults::new(precompile_results),
            chunk_size,
            print_pc,
            ..Default::default()
        };

        // Split the ROM into the functions to generate.  Without chunking there is a single function
        // covering every instruction.
        let chunks: Vec<(usize, usize)> = if ctx.chunked() {
            Self::partition_chunks(rom, chunk_size)
        } else {
            vec![(0, rom.sorted_pc_list.len().saturating_sub(1))]
        };

        Self::preamble(&mut ctx, code, rom, &chunks);

        /****************/
        /* INSTRUCTIONS */
        /****************/

        for (chunk_index, (first, last)) in chunks.iter().enumerate() {
            ctx.chunk_lo = rom.sorted_pc_list[*first];
            ctx.chunk_hi = rom.sorted_pc_list[*last];

            if ctx.chunked() {
                Self::chunk_start(&mut ctx, code, rom, chunk_index, *first, *last);
            }

            // Generate code for every instruction of this chunk, in ascending program counter order,
            // so that an instruction that continues into the next one needs no jump at all
            for k in *first..=*last {
                ctx.pc = rom.sorted_pc_list[k];

                // Skip internal, odd address instructions.  They are generated right after the
                // non-internal instruction they depend on, in order of dependency, so that we can
                // skip jumps
                if ctx.pc & 0x1 != 0 {
                    continue;
                }

                // Get the instruction from ROM
                let mut instruction = &rom.insts[&ctx.pc].i;

                // Get next instruction pc, to be used in jumps
                ctx.next_pc = Self::next_pc(rom, k, instruction);

                // Generate code for this instruction
                Self::instruction_to_c(&mut ctx, rom, instruction, code);

                // Iterate on the chain of internal instructions this instruction depends on, and
                // generate code for them as well, until there are no more internal instructions
                while instruction.next_internal_inst.is_some() {
                    let pc = instruction.next_internal_inst.unwrap();
                    ctx.pc = pc;
                    instruction = &rom.insts[&pc].i;
                    ctx.next_pc = Self::next_pc(rom, k, instruction);
                    Self::instruction_to_c(&mut ctx, rom, instruction, code);
                }
            }

            if ctx.chunked() {
                Self::chunk_end(&mut ctx, code);
            }
        }

        /***********/
        /* EMU END */
        /***********/

        if !ctx.chunked() {
            *code += "\nemu_end:\n";
            if ctx.minimal_trace() {
                // In this method the step counter is a count down to the end of the chunk, and
                // MEM_STEP is the total, which every chunk boundary has already updated
                *code += &ctx.full_line_comment(
                    "MEM_STEP was published by the last chunk boundary".to_string(),
                );
            } else {
                *code += &ctx.full_line_comment(
                    "Publish the step counter so that the caller can read it".to_string(),
                );
                *code += "\tMEM_STEP = step;\n";
            }
            *code += "\treturn;\n";
            *code += "}\n";
        } else {
            Self::dispatcher(&mut ctx, code, rom, &chunks);
        }

        /********************/
        /* ROM INITIAL DATA */
        /********************/

        Self::write_init_data(&mut ctx, code, rom);
    }

    /*******************/
    /* CHUNK PARTITION */
    /*******************/

    /// Splits the ROM into chunks of about `chunk_size` instructions, each of which becomes one C
    /// function.  Returns, for every chunk, the range of indices into `rom.sorted_pc_list` it covers.
    ///
    /// Where a chunk ends matters for performance, not for correctness: a jump that stays inside a
    /// chunk is a direct goto, while one that leaves it costs a return to the dispatcher and a call.
    /// So a boundary is placed:
    ///
    /// - after an instruction that cannot fall through (it ends the emulation, or it always jumps),
    ///   which keeps a hot fall-through edge from becoming a dispatcher round trip.  The search for
    ///   such an instruction starts at the target size and gives up after a margin, in which case the
    ///   chunk is cut at the target size anyway;
    /// - at a large gap in the program counter sequence, because each chunk dispatches its own
    ///   dynamic jumps through a table indexed by `pc - chunk_lo`, and a gap inside a chunk would
    ///   make that table enormous.  The ROM entry region and the float library both sit in their own
    ///   address range, so this is what keeps them in chunks of their own.
    fn partition_chunks(rom: &ZiskRom, chunk_size: u64) -> Vec<(usize, usize)> {
        // A gap wider than this starts a new chunk rather than being covered by a dispatch table
        const MAX_PC_GAP: u64 = 4096;
        // How far past the target size to look for an instruction that cannot fall through
        let search_margin = (chunk_size / 5).max(16);

        // Indices of the instructions that get generated, i.e. skipping the odd internal ones, which
        // are generated as part of the instruction they belong to
        let generated: Vec<usize> =
            (0..rom.sorted_pc_list.len()).filter(|k| rom.sorted_pc_list[*k] & 0x1 == 0).collect();

        let mut chunks: Vec<(usize, usize)> = Vec::new();
        let mut start = 0usize; // index into `generated`
        while start < generated.len() {
            let mut end = start; // last index of this chunk, inclusive
            let mut count = 0u64;
            loop {
                let k = generated[end];
                let pc = rom.sorted_pc_list[k];
                count += 1;

                // Stop at the last instruction of the ROM
                if end + 1 >= generated.len() {
                    break;
                }

                // Stop before a large gap in the program counter sequence
                let next = rom.sorted_pc_list[generated[end + 1]];
                if next.saturating_sub(pc) > MAX_PC_GAP {
                    break;
                }

                // Past the target size, stop as soon as the instruction cannot fall through, so that
                // no fall-through edge is cut.  `end` is a terminator and `set_pc` always jumps.
                if count >= chunk_size {
                    let inst = &rom.insts[&pc].i;
                    if inst.end || inst.set_pc {
                        break;
                    }
                    // Give up looking and cut here rather than grow without bound
                    if count >= chunk_size + search_margin {
                        break;
                    }
                }

                end += 1;
            }
            chunks.push((generated[start], generated[end]));
            start = end + 1;
        }

        chunks
    }

    /// Program counter of the instruction that will be generated after the given one
    fn next_pc(rom: &ZiskRom, k: usize, instruction: &ZiskInst) -> u64 {
        if let Some(next_internal_inst) = instruction.next_internal_inst {
            // If there is an internal instruction, take it as the next instruction
            next_internal_inst
        } else if ((k + 1) < rom.sorted_pc_list.len()) && (rom.sorted_pc_list[k + 1] & 0x1 == 0) {
            rom.sorted_pc_list[k + 1]
        } else if ((k + 2) < rom.sorted_pc_list.len()) && (rom.sorted_pc_list[k + 2] & 0x1 == 0) {
            rom.sorted_pc_list[k + 2]
        } else if ((k + 3) < rom.sorted_pc_list.len()) && (rom.sorted_pc_list[k + 3] & 0x1 == 0) {
            rom.sorted_pc_list[k + 3]
        } else {
            M64
        }
    }

    /*************/
    /* PREAMBLE  */
    /*************/

    /// Emits the includes, the helpers, the globals shared with the C runtime, the configuration
    /// getters, and either the head of emu_start() or the declarations the chunk functions need
    fn preamble(
        ctx: &mut ZiskCContext,
        code: &mut String,
        rom: &ZiskRom,
        chunks: &[(usize, usize)],
    ) {
        *code += "/* Generated by ZiskRom2C.  Do not edit. */\n\n";
        *code += "#include <stdint.h>\n\n";

        // Every ROM program counter gets a label, whether or not anything jumps to it: an instruction
        // only reached by falling through from the previous one is never a goto target.  That is
        // expected in generated code, so the warning is not useful here.
        *code += "#pragma GCC diagnostic ignored \"-Wunused-label\"\n\n";

        // Guest memory is addressed by absolute address, exactly like in the assembly backend, and
        // the same address is read and written through different widths.  may_alias keeps that legal
        // under the strict aliasing rules the optimizer is allowed to assume.
        *code += "/* Guest memory access: absolute addresses, aliasing-safe at any width */\n";
        *code += "typedef uint64_t __attribute__((may_alias)) zisk_u64;\n";
        *code += "typedef uint32_t __attribute__((may_alias)) zisk_u32;\n";
        *code += "typedef uint16_t __attribute__((may_alias)) zisk_u16;\n";
        *code += "typedef uint8_t __attribute__((may_alias)) zisk_u8;\n";
        *code += "#define ZM64(addr) (*(zisk_u64 *)(uintptr_t)(addr))\n";
        *code += "#define ZM32(addr) (*(zisk_u32 *)(uintptr_t)(addr))\n";
        *code += "#define ZM16(addr) (*(zisk_u16 *)(uintptr_t)(addr))\n";
        *code += "#define ZM8(addr) (*(zisk_u8 *)(uintptr_t)(addr))\n\n";

        // The globals come first: some of the helpers below read the fcall context
        Self::preamble_globals(ctx, code);
        Self::preamble_externs(ctx, code);
        Self::preamble_helpers(code);
        if ctx.minimal_trace() {
            Self::preamble_minimal_trace(ctx, code);
        }
        Self::preamble_getters(ctx, code, rom);
        if ctx.chunked() {
            Self::preamble_chunked(ctx, code, chunks);
        } else {
            Self::preamble_emu_start(ctx, code, rom);
        }
    }

    /// Emits what the chunk functions need in scope: the machine state they hand to each other, the
    /// sentinel that ends the emulation, and one prototype per chunk so that emu_start can be
    /// generated after them
    fn preamble_chunked(ctx: &mut ZiskCContext, code: &mut String, chunks: &[(usize, usize)]) {
        *code += &ctx.full_line_comment(format!(
            "{} chunks of about {} instructions",
            chunks.len(),
            ctx.chunk_size
        ));
        *code += "/* The machine state the chunk functions hand to each other.  Each of them copies it\n";
        *code +=
            "   into locals on entry and back on exit, so that within a chunk the C compiler is\n";
        *code += "   free to keep it all in machine registers. */\n";
        *code += "typedef struct {\n";
        *code += "\tuint64_t a, b, c, flag, step, pc;\n";
        if ctx.minimal_trace() {
            *code += "\tuint64_t *mem_reads;\n";
            *code += "\tuint64_t mem_reads_size;\n";
        }
        *code += &format!("\tuint64_t reg[{ZISK_REGS}];\n");
        *code += "} ZiskState;\n\n";

        *code += &format!(
            "#define ZISK_END {} /* returned by a chunk when the emulation is over */\n\n",
            u64_lit(M64)
        );

        *code += "/* One function per chunk */\n";
        for i in 0..chunks.len() {
            *code += &format!("static uint64_t zisk_chunk_{i}(ZiskState *s);\n");
        }
        *code += "\n";
    }

    /// Emits the operations that are not a single C expression, as static helpers, so that the
    /// generated instruction stream stays one statement per operation
    fn preamble_helpers(code: &mut String) {
        *code += "/* Operations that do not fit in a single C expression */\n\n";

        *code += "static inline uint64_t zisk_rol64(uint64_t v, uint64_t n) {\n";
        *code += "\tn &= 63;\n";
        *code += "\treturn n ? ((v << n) | (v >> (64 - n))) : v;\n}\n\n";

        *code += "static inline uint64_t zisk_ror64(uint64_t v, uint64_t n) {\n";
        *code += "\tn &= 63;\n";
        *code += "\treturn n ? ((v >> n) | (v << (64 - n))) : v;\n}\n\n";

        *code += "static inline uint32_t zisk_rol32(uint32_t v, uint64_t n) {\n";
        *code += "\tn &= 31;\n";
        *code += "\treturn n ? ((v << n) | (v >> (32 - n))) : v;\n}\n\n";

        *code += "static inline uint32_t zisk_ror32(uint32_t v, uint64_t n) {\n";
        *code += "\tn &= 31;\n";
        *code += "\treturn n ? ((v >> n) | (v << (32 - n))) : v;\n}\n\n";

        *code += "static inline uint64_t zisk_brev8(uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 0; i < 8; i++) {\n";
        *code += "\t\tuint64_t byte = (b >> (i * 8)) & 0xFF;\n";
        *code += "\t\tuint64_t rev = 0;\n";
        *code += "\t\tfor (int j = 0; j < 8; j++) {\n";
        *code += "\t\t\tif (byte & (1ULL << j)) rev |= 1ULL << (7 - j);\n";
        *code += "\t\t}\n";
        *code += "\t\tr |= rev << (i * 8);\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        *code += "static inline uint64_t zisk_orc_b(uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 0; i < 8; i++) {\n";
        *code += "\t\tuint64_t mask = 0xFFULL << (i * 8);\n";
        *code += "\t\tif (b & mask) r |= mask;\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        *code += "static inline uint64_t zisk_clmul(uint64_t a, uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 0; i < 64; i++) {\n";
        *code += "\t\tif ((b >> i) & 1) r ^= a << i;\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        *code += "static inline uint64_t zisk_clmul_h(uint64_t a, uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 1; i < 64; i++) {\n";
        *code += "\t\tif ((b >> i) & 1) r ^= a >> (64 - i);\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        *code += "static inline uint64_t zisk_clmul_r(uint64_t a, uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 0; i < 64; i++) {\n";
        *code += "\t\tif ((b >> i) & 1) r ^= a >> (63 - i);\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        *code += "static inline uint64_t zisk_xperm4(uint64_t a, uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 0; i < 16; i++) {\n";
        *code += "\t\tuint64_t index = (b >> (i * 4)) & 0xF;\n";
        *code += "\t\tuint64_t value = (a >> (index * 4)) & 0xF;\n";
        *code += "\t\tr |= value << (i * 4);\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        *code += "static inline uint64_t zisk_xperm8(uint64_t a, uint64_t b) {\n";
        *code += "\tuint64_t r = 0;\n";
        *code += "\tfor (int i = 0; i < 8; i++) {\n";
        *code += "\t\tuint64_t index = (b >> (i * 8)) & 0xFF;\n";
        *code += "\t\tif (index < 8) {\n";
        *code += "\t\t\tuint64_t value = (a >> (index * 8)) & 0xFF;\n";
        *code += "\t\t\tr |= value << (i * 8);\n";
        *code += "\t\t}\n";
        *code += "\t}\n";
        *code += "\treturn r;\n}\n\n";

        // DMA operations are plain byte moves over guest memory in this method: the assembly backend
        // calls its hand-written routines, which also maintain the trace, but with no trace to
        // maintain the operation is just its memory effect.
        *code +=
            "static inline void zisk_dma_memcpy(uint64_t dst, uint64_t src, uint64_t count) {\n";
        *code += "\tfor (uint64_t i = 0; i < count; i++) ZM8(dst + i) = ZM8(src + i);\n}\n\n";

        *code +=
            "static inline void zisk_dma_memset(uint64_t dst, uint64_t count, uint8_t fill) {\n";
        *code += "\tfor (uint64_t i = 0; i < count; i++) ZM8(dst + i) = fill;\n}\n\n";

        // Returns the sign extended difference of the first differing byte, or 0 if all bytes match
        *code +=
            "static inline uint64_t zisk_dma_memcmp(uint64_t a, uint64_t b, uint64_t count) {\n";
        *code += "\tfor (uint64_t i = 0; i < count; i++) {\n";
        *code += "\t\tuint8_t byte_a = ZM8(a + i);\n";
        *code += "\t\tuint8_t byte_b = ZM8(b + i);\n";
        *code +=
            "\t\tif (byte_a != byte_b) return (uint64_t)((int64_t)byte_a - (int64_t)byte_b);\n";
        *code += "\t}\n";
        *code += "\treturn 0;\n}\n\n";

        // The input copy takes its bytes from the result of the last fcall instead of from guest
        // memory.  It starts at the word the free input mechanism is about to hand out, result[got-1],
        // and consumes the words it copies, so that the next free input is the first word it did not
        // copy.  This mirrors fast_inputcpy in emulator-asm/src/dma.
        *code += "static inline uint64_t zisk_dma_inputcpy(uint64_t dst, uint64_t count) {\n";
        *code += &format!("\tuint64_t got = fcall_ctx[{FCALL_RESULT_GOT}];\n");
        *code += &format!(
            "\tconst uint8_t *src = (const uint8_t *)&fcall_ctx[{FCALL_RESULT} + got - 1];\n"
        );
        *code += "\tfor (uint64_t i = 0; i < count; i++) ZM8(dst + i) = src[i];\n";
        *code += "\tgot += (count + 7) >> 3;\n";
        *code += &format!("\tfcall_ctx[{FCALL_RESULT_GOT}] = got;\n");
        // Reading past the result yields a zero free input, as in opc_dma_inputcpy().  The assembly
        // backend reads the word that follows the result instead, which only differs once the result
        // is exhausted, i.e. only for a program that reads more input than it asked for.
        *code += &format!(
            "\tMEM_FREE_INPUT = (got > fcall_ctx[{FCALL_RESULT_SIZE}]) ? 0 : \
             fcall_ctx[{FCALL_RESULT} + got - 1];\n"
        );
        *code += "\treturn dst;\n}\n\n";
    }

    /*******************/
    /* MINIMAL TRACE   */
    /*******************/

    /// Emits everything the minimal trace needs that is not per-instruction: the chunk layout, the
    /// chunk open and close operations, the helpers that record a memory access, and the trampolines
    /// that reach the runtime's DMA routines
    fn preamble_minimal_trace(ctx: &mut ZiskCContext, code: &mut String) {
        *code += "/**********************/\n";
        *code += "/*   MINIMAL TRACE    */\n";
        *code += "/**********************/\n\n";

        *code += "/* One chunk of the minimal trace, in 8-byte words:\n";
        *code += "     [0]      start pc\n";
        *code += "     [1]      start sp\n";
        *code += "     [2]      start c\n";
        *code += "     [3]      start step\n";
        *code += "     [4..36]  start reg[1] .. reg[33]\n";
        *code += "     [37]     last c\n";
        *code += "     [38]     end\n";
        *code += "     [39]     steps executed in this chunk\n";
        *code += "     [40]     mem reads size\n";
        *code += "     [41...]  mem reads\n";
        *code +=
            "   The chunks are written back to back, so the next one starts right after the mem\n";
        *code += "   reads of this one. */\n";
        *code += &format!("#define ZISK_CHUNK_HEADER {ZISK_CHUNK_HEADER}\n\n");

        // The registers the chunk start state records: 1 to 33.  Register 0 is always zero and the
        // ZisK-specific ones above 33 are not part of the state a chunk resumes from.
        *code +=
            "/* Opens a chunk: counts it, writes the state it starts from, and returns where its\n";
        *code += "   mem reads go */\n";
        *code +=
            "static uint64_t *zisk_chunk_start(uint64_t pc, uint64_t c, const uint64_t *reg) {\n";
        *code += "\tZM64(MEM_TRACE_ADDRESS) += 1;\n";
        *code += "\tuint64_t *chunk = (uint64_t *)(uintptr_t)MEM_CHUNK_ADDRESS;\n";
        *code += "\tchunk[0] = pc;\n";
        *code += "\tchunk[1] = MEM_SP;\n";
        *code += "\tchunk[2] = c;\n";
        *code += "\tchunk[3] = MEM_STEP;\n";
        *code += "\tMEM_CHUNK_START_STEP = MEM_STEP;\n";
        *code += &format!(
            "\tfor (int i = 1; i < {}; i++) chunk[3 + i] = reg[i];\n",
            ZISK_CHUNK_START_REGS + 1
        );
        *code +=
            "\t/* All ones is what tells the consumer that this chunk is not complete yet */\n";
        *code += "\tchunk[40] = 0xFFFFFFFFFFFFFFFFULL;\n";
        *code += "\treturn chunk + ZISK_CHUNK_HEADER;\n";
        *code += "}\n\n";

        *code +=
            "/* Closes the current chunk: writes the state it ended at and its size, advances to\n";
        *code += "   the next chunk, grows the trace if it is running short, and hands the chunk to the\n";
        *code +=
            "   runtime.  steps_left is what is left of the chunk's step count down, which is 0\n";
        *code += "   for every chunk but the last one. */\n";
        *code +=
            "static void zisk_chunk_end(uint64_t c, uint64_t mem_reads_size, uint64_t steps_left) {\n";
        *code += "\tMEM_STEP += chunk_size - steps_left;\n";
        *code += "\tuint64_t *chunk = (uint64_t *)(uintptr_t)MEM_CHUNK_ADDRESS;\n";
        *code += "\tchunk[37] = c;\n";
        *code += "\tchunk[38] = MEM_END;\n";
        *code += "\tchunk[39] = MEM_STEP - MEM_CHUNK_START_STEP;\n";
        *code += "\tchunk[40] = mem_reads_size;\n";
        *code += "\tMEM_CHUNK_ADDRESS =\n";
        *code += "\t\t(uint64_t)(uintptr_t)(chunk + ZISK_CHUNK_HEADER + mem_reads_size);\n";
        *code += "\tif (MEM_CHUNK_ADDRESS >= trace_address_threshold) _realloc_trace();\n";
        *code += "\t_chunk_done();\n";
        *code += "}\n\n";

        *code +=
            "/* Records the two aligned words that an unaligned 8-byte access reads or writes */\n";
        *code += "static inline uint64_t zisk_mt_8(uint64_t *mr, uint64_t size, uint64_t addr) {\n";
        *code += "\tuint64_t prev = addr & ~(uint64_t)7;\n";
        *code += "\tmr[size] = ZM64(prev);\n";
        *code += "\tmr[size + 1] = ZM64(prev + 8);\n";
        *code += "\treturn size + 2;\n";
        *code += "}\n\n";

        *code +=
            "/* Records the aligned word, or the two of them, that a narrow (1, 2 or 4 byte)\n";
        *code += "   access at addr falls into */\n";
        *code +=
            "static inline uint64_t zisk_mt_narrow(uint64_t *mr, uint64_t size, uint64_t addr,\n";
        *code += "\t\tuint64_t width) {\n";
        *code += "\tuint64_t prev = addr & ~(uint64_t)7;\n";
        *code += "\tuint64_t next = (addr + width - 1) & ~(uint64_t)7;\n";
        *code += "\tmr[size] = ZM64(prev);\n";
        *code += "\tif (next == prev) return size + 1;\n";
        *code += "\tmr[size + 1] = ZM64(next);\n";
        *code += "\treturn size + 2;\n";
        *code += "}\n\n";

        Self::preamble_dma_trampolines(ctx, code);
    }

    /// Emits the trampolines that reach the runtime's DMA routines.
    ///
    /// A DMA operation writes a variable number of mem reads, encoded together with the operation's
    /// alignment metadata by a table the runtime owns (`emulator-asm/src/dma`).  Rather than write
    /// that encoding a second time here, where it could drift from the one the trace consumers
    /// expect, the routines themselves are called.  They take the mem reads buffer in r12/r13 and the
    /// remaining step count in r14, which the C ABI knows nothing about, so each one gets the wrapper
    /// its own documentation asks for: move the arguments where it wants them, preserve the registers
    /// the C ABI expects to survive a call, and hand back the mem reads index it advanced.
    ///
    /// The number of pushes is even on purpose: these routines save XMM registers with `movaps`,
    /// which needs the stack alignment that only follows from an even number of pushes on top of the
    /// alignment the C ABI guarantees at a call.
    fn preamble_dma_trampolines(_ctx: &mut ZiskCContext, code: &mut String) {
        // (C name, assembly routine, whether it returns a value besides the mem reads index)
        const TRAMPOLINES: [(&str, &str, bool); 4] = [
            ("zisk_dma_memcpy_mt", "direct_dma_memcpy_mtrace_with_count_check", false),
            ("zisk_dma_memcmp_mt", "direct_dma_memcmp_mtrace", true),
            ("zisk_dma_inputcpy_mt", "direct_dma_inputcpy_mtrace_with_count_check", false),
            ("zisk_dma_memset_mt", "direct_dma_xmemset_mtrace", false),
        ];

        *code += "/* The DMA operations keep the minimal trace themselves, in the runtime's assembly */\n";
        *code += "typedef struct { uint64_t value, mem_reads_size; } ZiskDmaResult;\n\n";
        for (name, routine, returns_value) in TRAMPOLINES {
            if returns_value {
                *code += &format!(
                    "/* {routine}: returns its result and the mem reads index it advanced */\n"
                );
                *code += &format!("extern ZiskDmaResult {name}(uint64_t dst, uint64_t src,\n");
            } else {
                *code += &format!("/* {routine}: returns the mem reads index it advanced */\n");
                *code += &format!("extern uint64_t {name}(uint64_t dst, uint64_t src,\n");
            }
            *code +=
                "\t\tuint64_t count, uint64_t *mem_reads, uint64_t mem_reads_size, uint64_t steps_left);\n";
        }
        *code += "\n";

        *code += "asm(\n";
        *code += "\t\".text\\n\"\n";
        for (name, routine, returns_value) in TRAMPOLINES {
            *code += &format!("\t\"{name}:\\n\"\n");
            *code += "\t\"\\tpush %rbp\\n\\tpush %rbx\\n\\tpush %r12\\n\"\n";
            *code += "\t\"\\tpush %r13\\n\\tpush %r14\\n\\tpush %r15\\n\"\n";
            *code += "\t\"\\tmov %rcx, %r12\\n\"\n";
            *code += "\t\"\\tmov %r8, %r13\\n\"\n";
            *code += "\t\"\\tmov %r9, %r14\\n\"\n";
            *code += &format!("\t\"\\tcall {routine}\\n\"\n");
            // The mem reads index comes back in r13, as the second return value when the routine
            // already uses rax for its own result
            if returns_value {
                *code += "\t\"\\tmov %r13, %rdx\\n\"\n";
            } else {
                *code += "\t\"\\tmov %r13, %rax\\n\"\n";
            }
            *code += "\t\"\\tpop %r15\\n\\tpop %r14\\n\\tpop %r13\\n\"\n";
            *code += "\t\"\\tpop %r12\\n\\tpop %rbx\\n\\tpop %rbp\\n\"\n";
            *code += "\t\"\\tret\\n\"\n";
        }
        *code += ");\n\n";
    }

    /// Emits the global variables the C runtime reads, matching the `.comm` symbols of the assembly
    /// backend one for one
    fn preamble_globals(_ctx: &mut ZiskCContext, code: &mut String) {
        *code +=
            "/* Globals shared with the C runtime (see emulator-asm/src/asm_provided.hpp) */\n";
        *code += "uint64_t MEM_STEP = 0;\n";
        *code += "uint64_t MEM_SP = 0;\n";
        *code += "uint64_t MEM_END = 0;\n";
        *code += "uint64_t MEM_ERROR = 0;\n";
        *code += "uint64_t MEM_TRACE_ADDRESS = 0;\n";
        *code += "uint64_t MEM_CHUNK_ADDRESS = 0;\n";
        *code += "uint64_t MEM_CHUNK_START_STEP = 0;\n";
        *code += "uint64_t MEM_FREE_INPUT = 0;\n";
        *code += &format!("uint64_t fcall_ctx[{FCALL_LENGTH}] = {{0}};\n\n");

        // Another process writes this counter, so the read must not be cached: without volatile the
        // compiler would be free to reuse the value loaded for an earlier check
        *code += "/* Input bytes written so far, in the control input shared memory */\n";
        *code += &format!(
            "#define ZISK_INPUT_WRITTEN (*(volatile uint64_t *)(uintptr_t){})\n\n",
            u64_lit(INPUT_WRITTEN_ADDR)
        );
    }

    /// Emits the declarations of the runtime functions the generated code calls.  These are the same
    /// symbols the assembly backend declares `.extern`.
    fn preamble_externs(ctx: &ZiskCContext, code: &mut String) {
        *code += "/* Runtime-provided functions (see emulator-asm/src/emu.c) */\n";
        if ctx.print_pc {
            *code += "extern int _print_pc(uint64_t pc, uint64_t c);\n";
        }
        *code += "extern int _print_char(uint64_t param);\n";
        *code += "extern int _opcode_keccak(uint64_t address);\n";
        *code += "extern int _opcode_poseidon2(uint64_t address);\n";
        *code += "extern int _opcode_poseidon1(uint64_t address);\n";
        *code += "extern int _opcode_sha256(uint64_t *address);\n";
        *code += "extern int _opcode_blake2(uint64_t *address);\n";
        *code += "extern int _opcode_arith256(uint64_t *address);\n";
        *code += "extern int _opcode_arith256_mod(uint64_t *address);\n";
        *code += "extern int _opcode_arith384_mod(uint64_t *address);\n";
        *code += "extern int _opcode_secp256k1_add(uint64_t *address);\n";
        *code += "extern int _opcode_secp256k1_dbl(uint64_t *address);\n";
        *code += "extern int _opcode_secp256r1_add(uint64_t *address);\n";
        *code += "extern int _opcode_secp256r1_dbl(uint64_t *address);\n";
        *code += "extern int _opcode_bn254_curve_add(uint64_t *address);\n";
        *code += "extern int _opcode_bn254_curve_dbl(uint64_t *address);\n";
        *code += "extern int _opcode_bn254_complex_add(uint64_t *address);\n";
        *code += "extern int _opcode_bn254_complex_sub(uint64_t *address);\n";
        *code += "extern int _opcode_bn254_complex_mul(uint64_t *address);\n";
        *code += "extern int _opcode_bls12_381_curve_add(uint64_t *address);\n";
        *code += "extern int _opcode_bls12_381_curve_dbl(uint64_t *address);\n";
        *code += "extern int _opcode_bls12_381_complex_add(uint64_t *address);\n";
        *code += "extern int _opcode_bls12_381_complex_sub(uint64_t *address);\n";
        *code += "extern int _opcode_bls12_381_complex_mul(uint64_t *address);\n";
        *code += "extern uint64_t _opcode_add256(uint64_t *address);\n";
        *code += "extern int _opcode_fcall(void *ctx);\n";
        *code += "extern int _wait_for_input_avail(uint64_t required_input_bytes);\n";
        if ctx.minimal_trace() {
            *code += "extern void _chunk_done(void);\n";
            *code += "extern void _realloc_trace(void);\n";
        }
        *code += "\n";

        if ctx.minimal_trace() {
            // The runtime owns these: it decides how long a chunk is, when to stop, and where the
            // trace lives, and it may move the threshold when it grows the trace.  They are read on
            // every chunk boundary, so they are read from memory rather than cached in a local.
            *code += "/* Runtime-provided configuration (see emulator-asm/src/globals.c) */\n";
            *code += "extern uint64_t chunk_size;\n";
            *code += "extern uint64_t max_steps;\n";
            *code += "extern uint64_t trace_address;\n";
            *code += "extern uint64_t trace_address_threshold;\n\n";
        }
    }

    /// Emits the configuration getters the C main program queries to check that the generated code
    /// and the runtime agree
    fn preamble_getters(ctx: &mut ZiskCContext, code: &mut String, rom: &ZiskRom) {
        *code +=
            &format!("uint64_t get_rom_length(void) {{ return 0x{:08x}ULL; }}\n", rom.insts.len());
        // These are the values the C main program's GenMethod enum uses, which are not the ZisK
        // discriminants: see emulator-asm/src/globals.hpp
        let gen_method: u64 = match ctx.mode {
            AsmGenerationMethod::AsmFast => 0,
            AsmGenerationMethod::AsmMinimalTraces => 1,
            AsmGenerationMethod::AsmRomHistogram => 2,
            AsmGenerationMethod::AsmMemOp => 7,
        };
        *code += &format!("uint64_t get_gen_method(void) {{ return {gen_method}ULL; }}\n");
        *code += &format!(
            "uint64_t get_precompile_results(void) {{ return {}ULL; }}\n\n",
            u64::from(ctx.precompile_results.enabled())
        );
    }

    /// Emits the head of emu_start(): the locals that replace the assembly backend's fixed
    /// registers, the branch table, and the initialization of the state the runtime observes
    fn preamble_emu_start(ctx: &mut ZiskCContext, code: &mut String, rom: &ZiskRom) {
        *code += "void emu_start(void) {\n";

        // These are the assembly backend's fixed registers.  As plain locals whose address is never
        // taken, the C compiler is free to keep them in machine registers, and to spill only the
        // ones that are live across a call to the runtime.
        *code += &ctx.full_line_comment(
            "ZisK machine state: locals, so that the C compiler allocates them".to_string(),
        );
        *code += "\tuint64_t a = 0, b = 0, c = 0, flag = 0;\n";
        *code += "\tuint64_t step = 0, pc = 0, addr = 0;\n";
        *code += &format!("\tuint64_t reg[{ZISK_REGS}] = {{0}};\n");
        if ctx.minimal_trace() {
            *code += &ctx.full_line_comment(
                "Where the mem reads of the chunk being written go, and how many there are so far"
                    .to_string(),
            );
            *code += "\tuint64_t *mem_reads = 0;\n";
            *code += "\tuint64_t mem_reads_size = 0;\n";
        }
        *code += "\n";

        Self::branch_table(ctx, code, rom);
        Self::emit_state_init(ctx, code);

        // Silence "set but not used" for the locals a ROM may never happen to need
        *code += "\t(void)a; (void)b; (void)flag; (void)addr; (void)pc;\n";

        if ctx.minimal_trace() {
            // Open the first chunk at the first instruction of the ROM.  Everything below is
            // generated in ascending program counter order, so that is where execution starts.
            let first_pc = rom.sorted_pc_list.first().copied().unwrap_or(ROM_ENTRY);
            *code += &format!(
                "\tpc = {};{}\n",
                u64_lit(first_pc),
                ctx.comment_str("start at the first instruction of the ROM")
            );
            Self::emit_first_chunk_start(ctx, code, "pc", "c", "reg", "");
        }
    }

    /// Emits the opening of the first chunk of the minimal trace.  The chunk counter is reset here
    /// rather than in `zisk_chunk_start()`, which increments it.
    fn emit_first_chunk_start(
        ctx: &mut ZiskCContext,
        code: &mut String,
        pc: &str,
        c: &str,
        reg: &str,
        state: &str,
    ) {
        *code += &ctx.full_line_comment("Open the first chunk of the minimal trace".to_string());
        *code += &format!("\tZM64(MEM_TRACE_ADDRESS) = 0;{}\n", ctx.comment_str("no chunks yet"));
        *code += &format!("\t{state}mem_reads = zisk_chunk_start({pc}, {c}, {reg});\n");
        *code += &format!("\t{state}mem_reads_size = 0;\n");
        *code += &format!(
            "\t{state}step = chunk_size;{}\n",
            ctx.comment_str("step counts down to the end of the chunk")
        );
    }

    /// Emits the initialization of the state the C runtime observes.  Shared by both modes: the
    /// runtime cannot tell whether the ROM was generated as one function or many.
    fn emit_state_init(ctx: &mut ZiskCContext, code: &mut String) {
        *code += &ctx.full_line_comment("ASM memory initialization".to_string());
        *code += "\tMEM_END = 0;\n";
        *code += "\tMEM_ERROR = 0;\n";
        *code += "\tMEM_STEP = 0;\n";
        *code += "\tMEM_SP = 0;\n";
        *code += "\tMEM_FREE_INPUT = 0;\n";
        *code += &ctx.full_line_comment("fcall_context initialization".to_string());
        *code += &format!("\tfcall_ctx[{FCALL_PARAMS_CAPACITY}] = {FCALL_PARAMS_LENGTH};\n");
        *code += &format!("\tfcall_ctx[{FCALL_PARAMS_SIZE}] = 0;\n");
        *code += &format!("\tfcall_ctx[{FCALL_RESULT_CAPACITY}] = {FCALL_RESULT_LENGTH};\n");
        *code += &format!("\tfcall_ctx[{FCALL_RESULT_SIZE}] = 0;\n");
        *code += &format!("\tfcall_ctx[{FCALL_RESULT_GOT}] = 0;\n");

        if ctx.minimal_trace() {
            // The runtime keeps a 32-byte header at the start of the trace (version, exit code,
            // allocated size, used size) that it writes itself, so the chunks start after it: first
            // the number of chunks, then the chunks themselves.
            *code += &ctx.full_line_comment("Minimal trace initialization".to_string());
            *code += &format!(
                "\tMEM_TRACE_ADDRESS = trace_address + 0x20;{}\n",
                ctx.comment_str("past the runtime's header")
            );
            *code += &format!(
                "\tMEM_CHUNK_ADDRESS = MEM_TRACE_ADDRESS + 8;{}\n",
                ctx.comment_str("past the number of chunks")
            );
            *code += "\tMEM_CHUNK_START_STEP = 0;\n";
        }
    }

    /*******************/
    /* CHUNK FUNCTIONS */
    /*******************/

    /// Emits the head of a chunk function: the machine state copied into locals, and the dispatch that
    /// enters the chunk at whatever program counter it was called with.
    ///
    /// The same table serves the entry dispatch and every dynamic jump that stays inside the chunk, so
    /// an indirect jump to a nearby address remains a single indirect branch rather than a return to
    /// the top-level dispatcher.
    fn chunk_start(
        ctx: &mut ZiskCContext,
        code: &mut String,
        rom: &ZiskRom,
        chunk_index: usize,
        first: usize,
        last: usize,
    ) {
        *code += &format!("\n/* chunk {chunk_index} */\n");
        *code += &ctx.full_line_comment(format!(
            "program counters 0x{:x} to 0x{:x}, {} instructions",
            ctx.chunk_lo,
            ctx.chunk_hi,
            last - first + 1
        ));
        *code += &format!("static uint64_t zisk_chunk_{chunk_index}(ZiskState *s) {{\n");

        // Copy the machine state into locals.  Within the chunk everything below is a local whose
        // address is never taken, so the C compiler allocates it as it sees fit; the copy back in
        // chunk_end() is the price of the boundary.
        *code += &ctx.full_line_comment("machine state into locals".to_string());
        *code += "\tuint64_t a = s->a, b = s->b, c = s->c, flag = s->flag;\n";
        *code += "\tuint64_t step = s->step, pc = s->pc, addr = 0, ret;\n";
        if ctx.minimal_trace() {
            *code += "\tuint64_t *mem_reads = s->mem_reads;\n";
            *code += "\tuint64_t mem_reads_size = s->mem_reads_size;\n";
        }
        *code += &format!("\tuint64_t reg[{ZISK_REGS}];\n");
        *code += &format!("\tfor (int i = 0; i < {ZISK_REGS}; i++) reg[i] = s->reg[i];\n");
        *code += "\t(void)a; (void)b; (void)flag; (void)addr;\n\n";

        // Dispatch table for this chunk, indexed by (pc - chunk_lo) / 2.  Program counters step by at
        // least 2 bytes, and the odd ones are internal instructions that cannot be jumped to.
        *code += &ctx.full_line_comment(
            "dispatch table for this chunk, indexed by (pc - chunk_lo) / 2".to_string(),
        );
        *code += "\tstatic void *const local_map[] = {\n";
        let mut entries: u64 = 0;
        let mut pc = ctx.chunk_lo;
        while pc <= ctx.chunk_hi {
            // Only even program counters that are real instructions of this chunk are reachable
            let is_instruction = (pc & 0x1 == 0)
                && rom
                    .sorted_pc_list
                    .binary_search(&pc)
                    .map(|k| k >= first && k <= last)
                    .unwrap_or(false);
            if is_instruction {
                *code += &format!("\t\t&&pc_{pc:x},\n");
            } else {
                // Not an instruction: behave like the single function mode, which sends a jump to an
                // unknown program counter to the end of the emulation
                *code += "\t\t&&zisk_unknown_pc,\n";
            }
            entries += 1;
            pc += 2;
        }
        *code += "\t};\n";
        *code += &ctx.full_line_comment(format!("{entries} entries"));

        *code += &format!(
            "\tgoto *local_map[(pc - {}) >> 1];{}\n",
            u64_lit(ctx.chunk_lo),
            ctx.comment_str("enter at the requested pc")
        );
    }

    /// Emits the tail of a chunk function: the exits, the copy of the machine state back, and the
    /// program counter the top-level dispatcher continues from
    fn chunk_end(ctx: &mut ZiskCContext, code: &mut String) {
        *code += "\n";
        *code += &ctx
            .full_line_comment("a jump to a pc that is not an instruction ends here".to_string());
        *code += "zisk_unknown_pc:\n";
        *code += "\tret = ZISK_END;\n";
        *code += "\tgoto zisk_spill;\n\n";

        *code += &ctx.full_line_comment("the emulation is over".to_string());
        *code += "zisk_end:\n";
        *code += "\tret = ZISK_END;\n";
        *code += "\tgoto zisk_spill;\n\n";

        *code += &ctx.full_line_comment("control left this chunk: hand the pc back".to_string());
        *code += "zisk_leave:\n";
        *code += "\tret = pc;\n\n";

        *code += "zisk_spill:\n";
        *code += "\ts->a = a; s->b = b; s->c = c; s->flag = flag;\n";
        *code += "\ts->step = step; s->pc = pc;\n";
        if ctx.minimal_trace() {
            *code += "\ts->mem_reads = mem_reads; s->mem_reads_size = mem_reads_size;\n";
        }
        *code += &format!("\tfor (int i = 0; i < {ZISK_REGS}; i++) s->reg[i] = reg[i];\n");
        *code += "\treturn ret;\n";
        *code += "}\n";
    }

    /// Emits the top-level dispatcher and emu_start().
    ///
    /// The dispatcher maps a program counter to the chunk that contains it.  Chunks cover contiguous
    /// program counter ranges, but the ROM as a whole does not: the entry region and the float library
    /// live in address ranges of their own.  So the chunks are grouped into clusters of adjacent
    /// ranges, and each cluster gets its own table; the dispatcher picks the cluster with a range
    /// check.  In practice there are only a handful of clusters.
    fn dispatcher(
        ctx: &mut ZiskCContext,
        code: &mut String,
        rom: &ZiskRom,
        chunks: &[(usize, usize)],
    ) {
        // Group chunks whose program counter ranges are adjacent into clusters
        struct Cluster {
            lo: u64,
            hi: u64,
            first_chunk: usize,
        }
        let mut clusters: Vec<Cluster> = Vec::new();
        for (i, (first, last)) in chunks.iter().enumerate() {
            let lo = rom.sorted_pc_list[*first];
            let hi = rom.sorted_pc_list[*last];
            match clusters.last_mut() {
                Some(cluster) if lo.saturating_sub(cluster.hi) <= 4096 => cluster.hi = hi,
                _ => clusters.push(Cluster { lo, hi, first_chunk: i }),
            }
        }

        *code += "\n/* Top-level dispatch */\n";
        *code += "typedef uint64_t (*zisk_chunk_fn)(ZiskState *);\n";
        *code += "static const zisk_chunk_fn zisk_chunks[] = {\n";
        for i in 0..chunks.len() {
            *code += &format!("\tzisk_chunk_{i},\n");
        }
        *code += "};\n\n";

        // One table per cluster, mapping a program counter to the chunk that contains it
        for (ci, cluster) in clusters.iter().enumerate() {
            *code +=
                &format!("/* cluster {}: pc 0x{:x} to 0x{:x} */\n", ci, cluster.lo, cluster.hi);
            *code += &format!("static const uint32_t zisk_chunk_of_pc_{ci}[] = {{\n");
            let mut chunk = cluster.first_chunk;
            let mut pc = cluster.lo;
            while pc <= cluster.hi {
                // Advance to the chunk that contains this program counter
                while chunk + 1 < chunks.len() && pc > rom.sorted_pc_list[chunks[chunk].1] {
                    chunk += 1;
                }
                *code += &format!("\t{chunk},\n");
                pc += 2;
            }
            *code += "};\n\n";
        }

        *code += "void emu_start(void) {\n";
        *code += "\tZiskState state;\n";
        *code += "\tZiskState *s = &state;\n";
        *code += &ctx.full_line_comment("ZisK machine state starts at zero".to_string());
        *code += "\ts->a = 0; s->b = 0; s->c = 0; s->flag = 0; s->step = 0; s->pc = 0;\n";
        *code += &format!("\tfor (int i = 0; i < {ZISK_REGS}; i++) s->reg[i] = 0;\n");
        Self::emit_state_init(ctx, code);

        // The first instruction of the ROM is where execution starts
        let first_pc = rom.sorted_pc_list.first().copied().unwrap_or(ROM_ENTRY);
        *code += &format!(
            "\n\tuint64_t pc = {};{}\n",
            u64_lit(first_pc),
            ctx.comment_str("start at the first instruction of the ROM")
        );
        if ctx.minimal_trace() {
            Self::emit_first_chunk_start(ctx, code, "pc", "s->c", "s->reg", "s->");
        }
        *code += "\twhile (pc != ZISK_END) {\n";
        *code += "\t\tuint32_t chunk;\n";
        for (ci, cluster) in clusters.iter().enumerate() {
            *code += &format!(
                "\t\t{}if (pc >= {} && pc <= {}) chunk = zisk_chunk_of_pc_{ci}[(pc - {}) >> 1];\n",
                if ci == 0 { "" } else { "else " },
                u64_lit(cluster.lo),
                u64_lit(cluster.hi),
                u64_lit(cluster.lo)
            );
        }
        *code += &ctx.full_line_comment(
            "\ta pc outside the ROM ends the emulation, like the single function mode".to_string(),
        );
        *code += "\t\telse break;\n";
        *code += "\t\ts->pc = pc;\n";
        *code += "\t\tpc = zisk_chunks[chunk](s);\n";
        *code += "\t}\n\n";

        if ctx.minimal_trace() {
            // In this method the step counter is a count down to the end of the chunk, and MEM_STEP
            // is the total, which every chunk boundary has already updated
            *code += &ctx
                .full_line_comment("MEM_STEP was published by the last chunk boundary".to_string());
        } else {
            *code += &ctx.full_line_comment(
                "Publish the step counter so that the caller can read it".to_string(),
            );
            *code += "\tMEM_STEP = s->step;\n";
        }
        *code += "}\n";
    }

    /// Emits the dynamic jump table: one label address per ROM byte address, so that a dynamic jump
    /// can index it with `pc - ROM_ADDR`.  This mirrors the `map_pc_*` table of the assembly
    /// backend, including its padding of unreachable addresses with the end label.
    fn branch_table(ctx: &mut ZiskCContext, code: &mut String, rom: &ZiskRom) {
        *code += &ctx.full_line_comment(
            "Dynamic jump table, indexed by pc - ROM_ADDR (one entry per ROM byte address)"
                .to_string(),
        );
        *code += "\tstatic void *const zisk_map_pc[] = {\n";

        let mut previous_key: u64 = ROM_ENTRY;
        let mut entries: u64 = 0;
        for key in &rom.sorted_pc_list {
            // Odd, internal addresses cannot be jumped to, so they get no entry
            if key & 0x1 != 0 {
                continue;
            }

            // The table is indexed from ROM_ADDR, so it only covers program addresses.  Addresses
            // below ROM_ADDR (the ROM entry region) are only ever reached by static jumps.
            if *key < ROM_ADDR {
                previous_key = *key;
                continue;
            }

            // Add the missing ROM_ADDR entry and padding up to the first key, so the table resolves
            // when .text starts above ROM_ADDR, e.g. Go ELFs
            if previous_key < ROM_ADDR
                && (*key != (previous_key + 1))
                && (*key != FLOAT_LIB_ROM_ADDR)
            {
                for _ in ROM_ADDR..*key {
                    *code += "\t\t&&emu_end,\n";
                    entries += 1;
                }
            }

            // Fill the gaps between consecutive, valid keys, in order to keep the distance between
            // entries constant and allow indexing with pc - ROM_ADDR
            if (previous_key >= ROM_ADDR)
                && (*key != (previous_key + 1))
                && (*key != FLOAT_LIB_ROM_ADDR)
            {
                for _ in previous_key + 1..*key {
                    *code += "\t\t&&emu_end,\n";
                    entries += 1;
                }
            }

            *code += &format!("\t\t&&pc_{key:x},\n");
            entries += 1;

            previous_key = *key;
        }

        *code += "\t};\n";
        *code += &ctx.full_line_comment(format!("{entries} entries"));
        *code += "\t(void)zisk_map_pc;\n\n";
    }

    /*****************/
    /* INSTRUCTIONS  */
    /*****************/

    /// Generates the C code of one ZisK instruction: read a and b, apply the operation, store c,
    /// count the step, and jump
    fn instruction_to_c(
        ctx: &mut ZiskCContext,
        rom: &ZiskRom,
        instruction: &ZiskInst,
        code: &mut String,
    ) {
        // Instruction label and, if enabled, the instruction in human readable form
        *code += "\n";
        let mut instruction_comment = instruction.to_text();
        instruction_comment.remove(0);
        *code += &format!("pc_{:x}:{}\n", ctx.pc, ctx.comment(instruction_comment));

        // Reset the per-instruction state
        ctx.a = ZiskCRegister::default();
        ctx.b = ZiskCRegister::default();
        ctx.c = ZiskCRegister::default();
        ctx.a.expr = "a".to_string();
        ctx.b.expr = "b".to_string();
        ctx.c.expr = "c".to_string();
        ctx.flag = FlagState::Dynamic;

        Self::source_a(ctx, instruction, code);
        Self::source_b(ctx, instruction, code);
        Self::operation_to_c(ctx, instruction, code);
        Self::store_c(ctx, instruction, code);

        // Debugging: hand this instruction to the runtime's tracer, at the same point of the
        // instruction where the assembly backend calls it, so that the two traces are comparable
        if ctx.print_pc {
            *code += &format!(
                "\t_print_pc({}, c);{}\n",
                u64_lit(ctx.pc),
                ctx.comment_str("trace this instruction")
            );
        }

        // Count the step.  When the minimal trace is generated the step counter counts down to the
        // end of the chunk instead of up, exactly like the assembly backend's: the total is kept in
        // MEM_STEP, which the chunk boundaries update.
        if ctx.minimal_trace() {
            *code += &format!("\tstep--;{}\n", ctx.comment_str("decrement step count down"));
        } else {
            *code += &format!("\tstep++;{}\n", ctx.comment_str("increment step"));
        }

        // The end instruction stops the emulation
        if instruction.end {
            *code += &format!("\tMEM_END = 1;{}\n", ctx.comment_str("end = 1"));
            *code += &format!("\tpc = {};{}\n", u64_lit(ctx.pc), ctx.comment_str("pc = this pc"));
            if ctx.minimal_trace() {
                // Close the chunk this instruction ends, which is short of a full chunk of steps
                *code += &format!(
                    "\tzisk_chunk_end(c, mem_reads_size, step);{}\n",
                    ctx.comment_str("close the last chunk")
                );
            }
            *code += if ctx.chunked() { "\tgoto zisk_end;\n" } else { "\tgoto emu_end;\n" };
            return;
        }

        // The chunk ends every chunk_size steps: close it, and open the next one at the instruction
        // this one is about to jump to
        if ctx.minimal_trace() {
            Self::minimal_trace_chunk_boundary(ctx, instruction, code);
        }

        Self::set_pc(ctx, instruction, code, rom);
    }

    /// Emits the end of a minimal trace chunk and the start of the next one, for the step at which
    /// the chunk's step count down reaches zero.
    ///
    /// The chunk that starts here starts at the instruction this one jumps to, so its program counter
    /// has to be computed before the chunk is opened.  That computation is then made again by the
    /// `set_pc()` that follows, which is what actually jumps: this is the same shape as the assembly
    /// backend's, where the count down path computes the program counter and then rejoins the common
    /// jump code.
    fn minimal_trace_chunk_boundary(
        ctx: &mut ZiskCContext,
        instruction: &ZiskInst,
        code: &mut String,
    ) {
        *code += &format!("\tif (step == 0) {{{}\n", ctx.comment_str("the chunk is over"));
        Self::pc_value(ctx, instruction, code);
        *code += &format!(
            "\t\tzisk_chunk_end(c, mem_reads_size, 0);{}\n",
            ctx.comment_str("close this chunk")
        );
        // The next chunk is opened before the step count is checked, and so before the emulation may
        // stop, exactly as the assembly backend's chunk_end_and_start does: what the consumer sees
        // when the emulation runs out of steps is part of the contract, not an accident
        *code += "\t\tmem_reads = zisk_chunk_start(pc, c, reg);\n";
        *code += "\t\tmem_reads_size = 0;\n";
        *code += "\t\tstep = chunk_size;\n";
        *code += &format!(
            "\t\tif (MEM_STEP >= max_steps) goto {};{}\n",
            if ctx.chunked() { "zisk_end" } else { "emu_end" },
            ctx.comment_str("out of steps")
        );
        *code += "\t}\n";
    }

    /// Emits the program counter this instruction continues at, without jumping to it, indented for
    /// the inside of the chunk boundary's block.
    ///
    /// This is the value part of `set_pc()`, and it says nothing about where control goes; it is what
    /// the chunk boundary needs, as the program counter a chunk starts at is part of its trace.
    fn pc_value(ctx: &mut ZiskCContext, instruction: &ZiskInst, code: &mut String) {
        if instruction.set_pc {
            if ctx.c.is_constant {
                let new_pc = (ctx.c.constant_value as i64 + instruction.jmp_offset1) as u64;
                *code += &format!("\t\tpc = {};\n", u64_lit(new_pc));
            } else if instruction.jmp_offset1 != 0 {
                *code += &format!(
                    "\t\tpc = {} + {};\n",
                    ctx.c.expr,
                    offset_lit(instruction.jmp_offset1)
                );
            } else {
                *code += &format!("\t\tpc = {};\n", ctx.c.expr);
            }
            return;
        }

        let pc_if_set = (ctx.pc as i64 + instruction.jmp_offset1) as u64;
        let pc_if_clear = (ctx.pc as i64 + instruction.jmp_offset2) as u64;
        match ctx.flag {
            FlagState::AlwaysZero => *code += &format!("\t\tpc = {};\n", u64_lit(pc_if_clear)),
            FlagState::AlwaysOne => *code += &format!("\t\tpc = {};\n", u64_lit(pc_if_set)),
            FlagState::Dynamic => {
                if pc_if_set == pc_if_clear {
                    *code += &format!("\t\tpc = {};\n", u64_lit(pc_if_set));
                } else {
                    *code += &format!(
                        "\t\tpc = flag ? {} : {};\n",
                        u64_lit(pc_if_set),
                        u64_lit(pc_if_clear)
                    );
                }
            }
        }
    }

    /*************/
    /* MEM READS */
    /*************/

    /// Records the 8-byte read of `value` at `addr` in the mem reads of the current chunk.
    ///
    /// An aligned read records the value that was read.  An unaligned one records instead the two
    /// aligned words it spans, because that is what the consumer needs to reconstruct it: it works in
    /// aligned words, and the value read is not one.
    fn mt_read_8(ctx: &mut ZiskCContext, code: &mut String, addr: &MtAddr, value: &str) {
        match addr {
            MtAddr::Const(address) if address % 8 == 0 => {
                *code += &format!(
                    "\tmem_reads[mem_reads_size++] = {value};{}\n",
                    ctx.comment_str("mem read, aligned")
                );
            }
            MtAddr::Const(address) => {
                *code += &format!(
                    "\tmem_reads_size = zisk_mt_8(mem_reads, mem_reads_size, {});{}\n",
                    u64_lit(*address),
                    ctx.comment_str("mem read, unaligned")
                );
            }
            MtAddr::Dynamic(expr) => {
                *code += &format!(
                    "\tif (({expr} & 7) == 0) mem_reads[mem_reads_size++] = {value};{}\n",
                    ctx.comment_str("mem read, aligned")
                );
                *code += &format!(
                    "\telse mem_reads_size = zisk_mt_8(mem_reads, mem_reads_size, {expr});{}\n",
                    ctx.comment_str("mem read, unaligned")
                );
            }
        }
    }

    /// Records what the mem reads need about an 8-byte write at `addr`.
    ///
    /// An aligned write records nothing: the value written is known from the trace itself.  An
    /// unaligned one records the two aligned words it spans, as they hold the bytes the write does
    /// not overwrite.
    fn mt_write_8(ctx: &mut ZiskCContext, code: &mut String, addr: &MtAddr) {
        match addr {
            MtAddr::Const(address) if address % 8 == 0 => {
                *code += &ctx.full_line_comment("aligned write: nothing to record".to_string());
            }
            MtAddr::Const(address) => {
                *code += &format!(
                    "\tmem_reads_size = zisk_mt_8(mem_reads, mem_reads_size, {});{}\n",
                    u64_lit(*address),
                    ctx.comment_str("write, unaligned")
                );
            }
            MtAddr::Dynamic(expr) => {
                *code += &format!(
                    "\tif (({expr} & 7) != 0)\n\t\tmem_reads_size = \
                     zisk_mt_8(mem_reads, mem_reads_size, {expr});{}\n",
                    ctx.comment_str("write, unaligned")
                );
            }
        }
    }

    /// Records the aligned words a narrow (1, 2 or 4 byte) access at `addr` falls into.
    ///
    /// Reads and writes record the same thing here: a narrow access is never a whole aligned word, so
    /// either way what the consumer needs is the word, or the two words, that contain it.
    fn mt_narrow(ctx: &mut ZiskCContext, code: &mut String, addr: &MtAddr, width: u64) {
        assert!(
            (width == 1) || (width == 2) || (width == 4),
            "ZiskRom2C::mt_narrow() width={width} pc={}",
            ctx.pc
        );

        match addr {
            MtAddr::Const(address) => {
                // The words the access falls into are known, so only the ones it really spans are
                // recorded
                let prev = address & !7;
                let next = (address + width - 1) & !7;
                *code += &format!(
                    "\tmem_reads[mem_reads_size++] = ZM64({});{}\n",
                    u64_lit(prev),
                    ctx.comment_str("narrow access: first word")
                );
                if next != prev {
                    *code += &format!(
                        "\tmem_reads[mem_reads_size++] = ZM64({});{}\n",
                        u64_lit(next),
                        ctx.comment_str("narrow access: second word")
                    );
                }
            }
            // A single byte always falls into one word, so there is nothing to decide at run time
            MtAddr::Dynamic(expr) if width == 1 => {
                *code += &format!(
                    "\tmem_reads[mem_reads_size++] = ZM64({expr} & ~(uint64_t)7);{}\n",
                    ctx.comment_str("byte access: its word")
                );
            }
            MtAddr::Dynamic(expr) => {
                *code += &format!(
                    "\tmem_reads_size = \
                     zisk_mt_narrow(mem_reads, mem_reads_size, {expr}, {width});{}\n",
                    ctx.comment_str("narrow access: the word, or two, it spans")
                );
            }
        }
    }

    /************/
    /* A SOURCE */
    /************/

    fn source_a(ctx: &mut ZiskCContext, instruction: &ZiskInst, code: &mut String) {
        match instruction.a_src {
            SRC_C => {
                // c is overwritten by the operation, so a must be read into its own local first
                *code += &format!("\ta = c;{}\n", ctx.comment_str("a=SRC_C"));
                ctx.a.expr = "a".to_string();
            }
            SRC_REG => {
                *code +=
                    &ctx.full_line_comment(format!("a=SRC_REG reg={}", instruction.a_offset_imm0));
                assert!(instruction.a_offset_imm0 < ZISK_REGS);
                ctx.a.expr = format!("reg[{}]", instruction.a_offset_imm0);
            }
            SRC_MEM => {
                *code += &ctx.full_line_comment("a=SRC_MEM".to_string());
                let address = if instruction.a_use_sp_imm1 != 0 {
                    *code += &format!(
                        "\taddr = {} + MEM_SP;{}\n",
                        u64_lit(instruction.a_offset_imm0),
                        ctx.comment_str("address = a_offset_imm0 + sp")
                    );
                    *code += &format!("\ta = ZM64(addr);{}\n", ctx.comment_str("a = mem[address]"));
                    MtAddr::Dynamic("addr".to_string())
                } else {
                    *code += &format!(
                        "\ta = ZM64({});{}\n",
                        u64_lit(instruction.a_offset_imm0),
                        ctx.comment_str("a = mem[a_offset_imm0]")
                    );
                    MtAddr::Const(instruction.a_offset_imm0)
                };
                ctx.a.expr = "a".to_string();

                if ctx.minimal_trace() {
                    Self::mt_read_8(ctx, code, &address, "a");
                }
            }
            SRC_IMM => {
                ctx.a.is_constant = true;
                ctx.a.constant_value =
                    instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32);
                ctx.a.expr = u64_lit(ctx.a.constant_value);
                *code += &ctx.full_line_comment(format!("a=SRC_IMM {}", ctx.a.expr));
            }
            SRC_STEP => {
                // NOTE: this mirrors the assembly backend, which reads the MEM_STEP variable here
                // rather than its step register.  MEM_STEP is not the step count of the instruction
                // being executed: it is only written at the end of the emulation, or at every chunk
                // boundary when the minimal trace is generated.  Reading it is what the assembly
                // backend does, so both backends read the same value.
                *code += &format!("\ta = MEM_STEP;{}\n", ctx.comment_str("a=SRC_STEP"));
                ctx.a.expr = "a".to_string();
            }
            _ => {
                panic!("ZiskRom2C::source_a() Invalid a_src={} pc={}", instruction.a_src, ctx.pc)
            }
        }
    }

    /************/
    /* B SOURCE */
    /************/

    fn source_b(ctx: &mut ZiskCContext, instruction: &ZiskInst, code: &mut String) {
        match instruction.b_src {
            SRC_C => {
                *code += &format!("\tb = c;{}\n", ctx.comment_str("b=SRC_C"));
                ctx.b.expr = "b".to_string();
            }
            SRC_REG => {
                *code +=
                    &ctx.full_line_comment(format!("b=SRC_REG reg={}", instruction.b_offset_imm0));
                assert!(instruction.b_offset_imm0 < ZISK_REGS);
                ctx.b.expr = format!("reg[{}]", instruction.b_offset_imm0);
            }
            SRC_MEM => {
                let b_is_free_input = (instruction.b_offset_imm0 == FREE_INPUT_ADDR)
                    && (instruction.b_use_sp_imm1 == 0);

                let address = if b_is_free_input {
                    // The free input is not read from guest memory but from the variable the fcall
                    // operations write
                    *code += &format!(
                        "\tb = MEM_FREE_INPUT;{}\n",
                        ctx.comment_str("b=SRC_MEM free input")
                    );
                    // The trace records it as a read of the free input address, which is aligned, so
                    // what gets recorded is the value the fcall produced
                    MtAddr::Const(instruction.b_offset_imm0)
                } else {
                    *code += &ctx.full_line_comment("b=SRC_MEM".to_string());
                    if instruction.b_use_sp_imm1 != 0 {
                        *code += &format!(
                            "\taddr = {} + MEM_SP;{}\n",
                            u64_lit(instruction.b_offset_imm0),
                            ctx.comment_str("address = b_offset_imm0 + sp")
                        );
                        *code +=
                            &format!("\tb = ZM64(addr);{}\n", ctx.comment_str("b = mem[address]"));
                        MtAddr::Dynamic("addr".to_string())
                    } else {
                        *code += &format!(
                            "\tb = ZM64({});{}\n",
                            u64_lit(instruction.b_offset_imm0),
                            ctx.comment_str("b = mem[b_offset_imm0]")
                        );
                        MtAddr::Const(instruction.b_offset_imm0)
                    }
                };
                ctx.b.expr = "b".to_string();

                if ctx.minimal_trace() {
                    Self::mt_read_8(ctx, code, &address, "b");
                }
            }
            SRC_IMM => {
                ctx.b.is_constant = true;
                ctx.b.constant_value =
                    instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32);
                ctx.b.expr = u64_lit(ctx.b.constant_value);
                *code += &ctx.full_line_comment(format!("b=SRC_IMM {}", ctx.b.expr));
            }
            SRC_IND => {
                *code +=
                    &ctx.full_line_comment(format!("b=SRC_IND width={}", instruction.ind_width));

                // The indirect address is a plus an immediate offset, and optionally sp
                let mut address =
                    format!("{} + {}", ctx.a.expr, u64_lit(instruction.b_offset_imm0));
                if instruction.b_use_sp_imm1 != 0 {
                    address += " + MEM_SP";
                }
                *code +=
                    &format!("\taddr = {address};{}\n", ctx.comment_str("address = a + offset"));

                // Narrow reads are zero extended, like the assembly backend's movzx / 32-bit mov;
                // sign extension, when needed, is a separate ZisK operation
                let read = match instruction.ind_width {
                    8 => "ZM64(addr)",
                    4 => "(uint64_t)ZM32(addr)",
                    2 => "(uint64_t)ZM16(addr)",
                    1 => "(uint64_t)ZM8(addr)",
                    _ => panic!(
                        "ZiskRom2C::source_b() Invalid ind_width={} pc={}",
                        instruction.ind_width, ctx.pc
                    ),
                };
                *code += &format!("\tb = {read};{}\n", ctx.comment_str("b = mem[address]"));
                ctx.b.expr = "b".to_string();

                if ctx.minimal_trace() {
                    let address = MtAddr::Dynamic("addr".to_string());
                    if instruction.ind_width == 8 {
                        Self::mt_read_8(ctx, code, &address, "b");
                    } else {
                        Self::mt_narrow(ctx, code, &address, instruction.ind_width);
                    }
                }
            }
            _ => {
                panic!("ZiskRom2C::source_b() Invalid b_src={} pc={}", instruction.b_src, ctx.pc)
            }
        }
    }

    /***********/
    /* STORE C */
    /***********/

    fn store_c(ctx: &mut ZiskCContext, instruction: &ZiskInst, code: &mut String) {
        // The value stored is either c or, for the instructions that link a call, the return address
        let value = if instruction.store_pc {
            u64_lit((ctx.pc as i64 + instruction.jmp_offset2) as u64)
        } else {
            ctx.c.expr.clone()
        };
        let value_name = if instruction.store_pc { "pc + jmp_offset2" } else { "c" };

        match instruction.store {
            STORE_NONE => {
                *code += &ctx.full_line_comment("STORE_NONE".to_string());
            }
            STORE_REG => {
                assert!(instruction.store_offset >= 0);
                assert!(instruction.store_offset < ZISK_REGS as i64);
                *code +=
                    &ctx.full_line_comment(format!("STORE_REG reg={}", instruction.store_offset));
                *code += &format!(
                    "\treg[{}] = {value};{}\n",
                    instruction.store_offset,
                    ctx.comment(format!("reg = {value_name}"))
                );
            }
            STORE_MEM => {
                *code += &ctx.full_line_comment("STORE_MEM".to_string());
                let mut address = if instruction.store_use_sp {
                    format!("{} + MEM_SP", offset_lit(instruction.store_offset))
                } else {
                    offset_lit(instruction.store_offset)
                };

                // What the trace records about a write is what the write is about to overwrite, so it
                // is recorded first.  The address is put in a local so that it is not computed twice.
                if ctx.minimal_trace() {
                    if instruction.store_use_sp {
                        *code += &format!(
                            "\taddr = {address};{}\n",
                            ctx.comment_str("address = store_offset + sp")
                        );
                        address = "addr".to_string();
                        Self::mt_write_8(ctx, code, &MtAddr::Dynamic(address.clone()));
                    } else {
                        Self::mt_write_8(
                            ctx,
                            code,
                            &MtAddr::Const(instruction.store_offset as u64),
                        );
                    }
                }

                *code += &format!(
                    "\tZM64({address}) = {value};{}\n",
                    ctx.comment(format!("mem[store_offset] = {value_name}"))
                );
            }
            STORE_IND => {
                *code +=
                    &ctx.full_line_comment(format!("STORE_IND width={}", instruction.ind_width));

                let mut address =
                    format!("{} + {}", ctx.a.expr, offset_lit(instruction.store_offset));
                if instruction.store_use_sp {
                    address += " + MEM_SP";
                }
                *code +=
                    &format!("\taddr = {address};{}\n", ctx.comment_str("address = a + offset"));

                // What the trace records about a write is what the write is about to overwrite, so it
                // is recorded first
                if ctx.minimal_trace() {
                    // When a is a constant and sp is not involved, so is the address, and its
                    // alignment is decided here instead of at run time
                    let address = if ctx.a.is_constant && !instruction.store_use_sp {
                        MtAddr::Const(
                            (ctx.a.constant_value as i64 + instruction.store_offset) as u64,
                        )
                    } else {
                        MtAddr::Dynamic("addr".to_string())
                    };
                    if instruction.ind_width == 8 {
                        Self::mt_write_8(ctx, code, &address);
                    } else {
                        Self::mt_narrow(ctx, code, &address, instruction.ind_width);
                    }
                }

                match instruction.ind_width {
                    8 => {
                        *code += &format!(
                            "\tZM64(addr) = {value};{}\n",
                            ctx.comment(format!("mem[address] = {value_name}"))
                        );
                    }
                    4 => {
                        *code += &format!(
                            "\tZM32(addr) = (uint32_t)({value});{}\n",
                            ctx.comment(format!("mem[address] = {value_name}"))
                        );
                    }
                    2 => {
                        *code += &format!(
                            "\tZM16(addr) = (uint16_t)({value});{}\n",
                            ctx.comment(format!("mem[address] = {value_name}"))
                        );
                    }
                    1 => {
                        *code += &format!(
                            "\tZM8(addr) = (uint8_t)({value});{}\n",
                            ctx.comment(format!("mem[address] = {value_name}"))
                        );

                        // A single byte written to the UART address is the program's console output
                        if ctx.log_output {
                            *code += &format!(
                                "\tif (addr == {}) _print_char((uint8_t)({value}));{}\n",
                                u64_lit(UART_ADDR),
                                ctx.comment_str("if address = UART then print char")
                            );
                        }
                    }
                    _ => panic!(
                        "ZiskRom2C::store_c() Invalid ind_width={} pc={}",
                        instruction.ind_width, ctx.pc
                    ),
                }
            }
            _ => {
                panic!("ZiskRom2C::store_c() Invalid store={} pc={}", instruction.store, ctx.pc)
            }
        }
    }

    /**************/
    /* OPERATIONS */
    /**************/

    /// Generates the C code of the instruction's operation: op(a, b) -> (c, flag).
    ///
    /// The semantics implemented here are those of the `op_*` functions in `ops_core`, which are the
    /// reference the Rust emulator uses.
    fn operation_to_c(ctx: &mut ZiskCContext, instruction: &ZiskInst, code: &mut String) {
        let zisk_op = ZiskOp::try_from_code(instruction.op).unwrap();
        let a = ctx.a.expr.clone();
        let b = ctx.b.expr.clone();

        // Most operations leave the flag clear; the ones that do not say so explicitly below
        ctx.flag = FlagState::AlwaysZero;

        // Emits `c = <expr>;`
        let set_c = |code: &mut String, expr: String, comment: &str| {
            *code += &format!("\tc = {expr};{}\n", ctx.comment_str(comment));
        };

        match zisk_op {
            /* Internal */
            ZiskOp::Flag => {
                set_c(code, "0".to_string(), "flag: c = 0");
                ctx.c.is_constant = true;
                ctx.c.constant_value = 0;
                ctx.flag = FlagState::AlwaysOne;
            }
            ZiskOp::CopyB => {
                set_c(code, b, "copyb: c = b");
                ctx.c.is_constant = ctx.b.is_constant;
                ctx.c.constant_value = ctx.b.constant_value;
            }
            ZiskOp::Halt => {
                *code += &format!("\tMEM_ERROR = 1;{}\n", ctx.comment_str("halt: error = 1"));
                set_c(code, "0".to_string(), "halt: c = 0");
                ctx.c.is_constant = true;
                ctx.c.constant_value = 0;
                ctx.flag = FlagState::AlwaysOne;
            }
            ZiskOp::PubOut => {
                set_c(code, b, "pubout: c = b");
                ctx.c.is_constant = ctx.b.is_constant;
                ctx.c.constant_value = ctx.b.constant_value;
            }

            /* Sign extension */
            ZiskOp::SignExtendB => {
                set_c(code, format!("(uint64_t)(int64_t)(int8_t)({b})"), "signextend_b")
            }
            ZiskOp::SignExtendH => {
                set_c(code, format!("(uint64_t)(int64_t)(int16_t)({b})"), "signextend_h")
            }
            ZiskOp::SignExtendW => {
                set_c(code, format!("(uint64_t)(int64_t)(int32_t)({b})"), "signextend_w")
            }

            /* Arithmetic */
            ZiskOp::Add => set_c(code, format!("{a} + {b}"), "add"),
            ZiskOp::AddW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) + (uint32_t)({b}))"),
                "add_w",
            ),
            ZiskOp::Sub => set_c(code, format!("{a} - {b}"), "sub"),
            ZiskOp::SubW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) - (uint32_t)({b}))"),
                "sub_w",
            ),

            /* Bitwise */
            ZiskOp::And => set_c(code, format!("{a} & {b}"), "and"),
            ZiskOp::Or => set_c(code, format!("{a} | {b}"), "or"),
            ZiskOp::Xor => set_c(code, format!("{a} ^ {b}"), "xor"),
            ZiskOp::Andn => set_c(code, format!("{a} & ~({b})"), "andn"),
            ZiskOp::Orn => set_c(code, format!("{a} | ~({b})"), "orn"),
            ZiskOp::Xnor => set_c(code, format!("~({a} ^ {b})"), "xnor"),

            /* Shifts */
            ZiskOp::Sll => set_c(code, format!("{a} << ({b} & 0x3f)"), "sll"),
            ZiskOp::Srl => set_c(code, format!("{a} >> ({b} & 0x3f)"), "srl"),
            ZiskOp::Sra => {
                set_c(code, format!("(uint64_t)((int64_t)({a}) >> ({b} & 0x3f))"), "sra")
            }
            ZiskOp::SllW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) << ({b} & 0x1f))"),
                "sll_w",
            ),
            ZiskOp::SrlW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) >> ({b} & 0x1f))"),
                "srl_w",
            ),
            ZiskOp::SraW => set_c(
                code,
                format!("(uint64_t)(int64_t)((int32_t)({a}) >> ({b} & 0x1f))"),
                "sra_w",
            ),
            ZiskOp::SllUW => set_c(code, format!("({a} & 0xFFFFFFFFULL) << ({b} & 0x3f)"), "sll_u_w"),

            /* Comparisons: c is the flag */
            ZiskOp::Eq => Self::compare(ctx, code, format!("{a} == {b}"), "eq"),
            ZiskOp::EqW => Self::compare(
                ctx,
                code,
                format!("(int32_t)({a}) == (int32_t)({b})"),
                "eq_w",
            ),
            ZiskOp::Ltu => Self::compare(ctx, code, format!("{a} < {b}"), "ltu"),
            ZiskOp::Lt => {
                Self::compare(ctx, code, format!("(int64_t)({a}) < (int64_t)({b})"), "lt")
            }
            ZiskOp::LtuW => {
                Self::compare(ctx, code, format!("(uint32_t)({a}) < (uint32_t)({b})"), "ltu_w")
            }
            ZiskOp::LtW => {
                Self::compare(ctx, code, format!("(int32_t)({a}) < (int32_t)({b})"), "lt_w")
            }
            ZiskOp::Leu => Self::compare(ctx, code, format!("{a} <= {b}"), "leu"),
            ZiskOp::Le => {
                Self::compare(ctx, code, format!("(int64_t)({a}) <= (int64_t)({b})"), "le")
            }
            ZiskOp::LeuW => {
                Self::compare(ctx, code, format!("(uint32_t)({a}) <= (uint32_t)({b})"), "leu_w")
            }
            ZiskOp::LeW => {
                Self::compare(ctx, code, format!("(int32_t)({a}) <= (int32_t)({b})"), "le_w")
            }

            /* Minimum and maximum */
            ZiskOp::Minu => set_c(code, format!("({a} < {b}) ? {a} : {b}"), "minu"),
            ZiskOp::Min => set_c(
                code,
                format!("((int64_t)({a}) < (int64_t)({b})) ? {a} : {b}"),
                "min",
            ),
            ZiskOp::Maxu => set_c(code, format!("({a} > {b}) ? {a} : {b}"), "maxu"),
            ZiskOp::Max => set_c(
                code,
                format!("((int64_t)({a}) > (int64_t)({b})) ? {a} : {b}"),
                "max",
            ),
            ZiskOp::MinuW => set_c(
                code,
                format!(
                    "(uint64_t)(int64_t)(int32_t)(((uint32_t)({a}) < (uint32_t)({b})) ? {a} : {b})"
                ),
                "minu_w",
            ),
            ZiskOp::MinW => set_c(
                code,
                format!(
                    "(uint64_t)(int64_t)(int32_t)(((int32_t)({a}) < (int32_t)({b})) ? {a} : {b})"
                ),
                "min_w",
            ),
            ZiskOp::MaxuW => set_c(
                code,
                format!(
                    "(uint64_t)(int64_t)(int32_t)(((uint32_t)({a}) > (uint32_t)({b})) ? {a} : {b})"
                ),
                "maxu_w",
            ),
            ZiskOp::MaxW => set_c(
                code,
                format!(
                    "(uint64_t)(int64_t)(int32_t)(((int32_t)({a}) > (int32_t)({b})) ? {a} : {b})"
                ),
                "max_w",
            ),

            /* Multiplication */
            ZiskOp::Mulu => set_c(code, format!("{a} * {b}"), "mulu"),
            ZiskOp::Mul => set_c(
                code,
                format!("(uint64_t)((int64_t)({a}) * (int64_t)({b}))"),
                "mul",
            ),
            ZiskOp::MulW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) * (uint32_t)({b}))"),
                "mul_w",
            ),
            ZiskOp::Muluh => set_c(
                code,
                format!("(uint64_t)(((unsigned __int128)({a}) * (unsigned __int128)({b})) >> 64)"),
                "muluh",
            ),
            ZiskOp::Mulh => set_c(
                code,
                format!(
                    "(uint64_t)(((__int128)(int64_t)({a}) * (__int128)(int64_t)({b})) >> 64)"
                ),
                "mulh",
            ),
            ZiskOp::Mulsuh => set_c(
                code,
                format!(
                    "(uint64_t)(((__int128)(int64_t)({a}) * (__int128)(unsigned __int128)({b})) >> 64)"
                ),
                "mulsuh",
            ),

            /* Division and remainder: the flag reports the division by zero case */
            ZiskOp::Divu => Self::divide(
                ctx,
                code,
                format!("{b} == 0"),
                u64_lit(M64),
                format!("{a} / {b}"),
                "divu",
            ),
            ZiskOp::Remu => Self::divide(
                ctx,
                code,
                format!("{b} == 0"),
                a.clone(),
                format!("{a} % {b}"),
                "remu",
            ),
            ZiskOp::Div => Self::divide(
                ctx,
                code,
                format!("{b} == 0"),
                u64_lit(M64),
                format!(
                    "(({a} == 0x8000000000000000ULL) && ((int64_t)({b}) == -1)) ? \
                     0x8000000000000000ULL : \
                     (uint64_t)((int64_t)({a}) / (int64_t)({b}))"
                ),
                "div",
            ),
            ZiskOp::Rem => Self::divide(
                ctx,
                code,
                format!("{b} == 0"),
                a.clone(),
                format!(
                    "(({a} == 0x8000000000000000ULL) && ((int64_t)({b}) == -1)) ? 0ULL : \
                     (uint64_t)((int64_t)({a}) % (int64_t)({b}))"
                ),
                "rem",
            ),
            ZiskOp::DivuW => Self::divide(
                ctx,
                code,
                format!("(uint32_t)({b}) == 0"),
                u64_lit(M64),
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) / (uint32_t)({b}))"),
                "divu_w",
            ),
            ZiskOp::RemuW => Self::divide(
                ctx,
                code,
                format!("(uint32_t)({b}) == 0"),
                format!("(uint64_t)(int64_t)(int32_t)({a})"),
                format!("(uint64_t)(int64_t)(int32_t)((uint32_t)({a}) % (uint32_t)({b}))"),
                "remu_w",
            ),
            ZiskOp::DivW => Self::divide(
                ctx,
                code,
                format!("(int32_t)({b}) == 0"),
                u64_lit(M64),
                format!(
                    "(((uint32_t)({a}) == 0x80000000UL) && ((int32_t)({b}) == -1)) ? \
                     0xFFFFFFFF80000000ULL : \
                     (uint64_t)(int64_t)(int32_t)((int32_t)({a}) / (int32_t)({b}))"
                ),
                "div_w",
            ),
            ZiskOp::RemW => Self::divide(
                ctx,
                code,
                format!("(int32_t)({b}) == 0"),
                format!("(uint64_t)(int64_t)(int32_t)({a})"),
                format!(
                    "(((uint32_t)({a}) == 0x80000000UL) && ((int32_t)({b}) == -1)) ? 0ULL : \
                     (uint64_t)(int64_t)((int32_t)({a}) % (int32_t)({b}))"
                ),
                "rem_w",
            ),

            /* Bit manipulation */
            ZiskOp::Rev8 => set_c(code, format!("__builtin_bswap64({b})"), "rev8"),
            ZiskOp::Brev8 => set_c(code, format!("zisk_brev8({b})"), "brev8"),
            ZiskOp::OrcB => set_c(code, format!("zisk_orc_b({b})"), "orc_b"),
            ZiskOp::Pack => {
                set_c(code, format!("({a} & 0xFFFFFFFFULL) | (({b} & 0xFFFFFFFFULL) << 32)"), "pack")
            }
            ZiskOp::PackH => {
                set_c(code, format!("({a} & 0xFFULL) | (({b} & 0xFFULL) << 8)"), "pack_h")
            }
            ZiskOp::PackW => set_c(
                code,
                format!(
                    "(uint64_t)(int64_t)(int32_t)(uint32_t)(({a} & 0xFFFFULL) | \
                     (({b} & 0xFFFFULL) << 16))"
                ),
                "pack_w",
            ),
            ZiskOp::Rol => set_c(code, format!("zisk_rol64({a}, {b})"), "rol"),
            ZiskOp::Ror => set_c(code, format!("zisk_ror64({a}, {b})"), "ror"),
            ZiskOp::RolW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)zisk_rol32((uint32_t)({a}), {b})"),
                "rol_w",
            ),
            ZiskOp::RorW => set_c(
                code,
                format!("(uint64_t)(int64_t)(int32_t)zisk_ror32((uint32_t)({a}), {b})"),
                "ror_w",
            ),
            // The builtins are undefined for a zero argument, so the zero case is spelled out
            ZiskOp::Clz => set_c(
                code,
                format!("(({b}) == 0) ? 64ULL : (uint64_t)__builtin_clzll({b})"),
                "clz",
            ),
            ZiskOp::ClzW => set_c(
                code,
                format!("((uint32_t)({b}) == 0) ? 32ULL : (uint64_t)__builtin_clz((uint32_t)({b}))"),
                "clz_w",
            ),
            ZiskOp::Ctz => set_c(
                code,
                format!("(({b}) == 0) ? 64ULL : (uint64_t)__builtin_ctzll({b})"),
                "ctz",
            ),
            ZiskOp::CtzW => set_c(
                code,
                format!("((uint32_t)({b}) == 0) ? 32ULL : (uint64_t)__builtin_ctz((uint32_t)({b}))"),
                "ctz_w",
            ),
            ZiskOp::Cpop => set_c(code, format!("(uint64_t)__builtin_popcountll({b})"), "cpop"),
            ZiskOp::CpopW => {
                set_c(code, format!("(uint64_t)__builtin_popcount((uint32_t)({b}))"), "cpop_w")
            }
            ZiskOp::Bclr => set_c(code, format!("{a} & ~(1ULL << ({b} & 0x3F))"), "bclr"),
            ZiskOp::Bext => set_c(code, format!("({a} >> ({b} & 0x3F)) & 1ULL"), "bext"),
            ZiskOp::Binv => set_c(code, format!("{a} ^ (1ULL << ({b} & 0x3F))"), "binv"),
            ZiskOp::Bset => set_c(code, format!("{a} | (1ULL << ({b} & 0x3F))"), "bset"),

            /* Address generation */
            ZiskOp::AddUW => set_c(code, format!("{b} + ({a} & 0xFFFFFFFFULL)"), "add_u_w"),
            ZiskOp::Sh1add => set_c(code, format!("{b} + ({a} << 1)"), "sh1add"),
            ZiskOp::Sh2add => set_c(code, format!("{b} + ({a} << 2)"), "sh2add"),
            ZiskOp::Sh3add => set_c(code, format!("{b} + ({a} << 3)"), "sh3add"),
            ZiskOp::Sh1addUW => {
                set_c(code, format!("{b} + (({a} & 0xFFFFFFFFULL) << 1)"), "sh1add_u_w")
            }
            ZiskOp::Sh2addUW => {
                set_c(code, format!("{b} + (({a} & 0xFFFFFFFFULL) << 2)"), "sh2add_u_w")
            }
            ZiskOp::Sh3addUW => {
                set_c(code, format!("{b} + (({a} & 0xFFFFFFFFULL) << 3)"), "sh3add_u_w")
            }

            /* Carry-less multiplication and permutations */
            ZiskOp::Clmul => set_c(code, format!("zisk_clmul({a}, {b})"), "clmul"),
            ZiskOp::ClmulH => set_c(code, format!("zisk_clmul_h({a}, {b})"), "clmul_h"),
            ZiskOp::ClmulR => set_c(code, format!("zisk_clmul_r({a}, {b})"), "clmul_r"),
            ZiskOp::Xperm4 => set_c(code, format!("zisk_xperm4({a}, {b})"), "xperm4"),
            ZiskOp::Xperm8 => set_c(code, format!("zisk_xperm8({a}, {b})"), "xperm8"),

            /* Conditional zero */
            ZiskOp::CzeroEqz => set_c(code, format!("({b} == 0) ? 0ULL : {a}"), "czero_eqz"),
            ZiskOp::CzeroNez => set_c(code, format!("({b} != 0) ? 0ULL : {a}"), "czero_nez"),

            /* Precompiles: the runtime does the work, addressed by b */
            ZiskOp::Keccak => Self::precompile(ctx, code, zisk_op, "_opcode_keccak", &b, false, false),
            ZiskOp::Poseidon2 => Self::precompile(ctx, code, zisk_op, "_opcode_poseidon2", &b, false, false),
            ZiskOp::Poseidon1 => Self::precompile(ctx, code, zisk_op, "_opcode_poseidon1", &b, false, false),
            ZiskOp::Sha256 => Self::precompile(ctx, code, zisk_op, "_opcode_sha256", &b, true, false),
            ZiskOp::Blake2 => Self::precompile(ctx, code, zisk_op, "_opcode_blake2", &b, true, false),
            ZiskOp::Arith256 => Self::precompile(ctx, code, zisk_op, "_opcode_arith256", &b, true, false),
            ZiskOp::Arith256Mod => {
                Self::precompile(ctx, code, zisk_op, "_opcode_arith256_mod", &b, true, false)
            }
            ZiskOp::Arith384Mod => {
                Self::precompile(ctx, code, zisk_op, "_opcode_arith384_mod", &b, true, false)
            }
            ZiskOp::Secp256k1Add => {
                Self::precompile(ctx, code, zisk_op, "_opcode_secp256k1_add", &b, true, false)
            }
            ZiskOp::Secp256k1Dbl => {
                Self::precompile(ctx, code, zisk_op, "_opcode_secp256k1_dbl", &b, true, false)
            }
            ZiskOp::Secp256r1Add => {
                Self::precompile(ctx, code, zisk_op, "_opcode_secp256r1_add", &b, true, false)
            }
            ZiskOp::Secp256r1Dbl => {
                Self::precompile(ctx, code, zisk_op, "_opcode_secp256r1_dbl", &b, true, false)
            }
            ZiskOp::Bn254CurveAdd => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bn254_curve_add", &b, true, false)
            }
            ZiskOp::Bn254CurveDbl => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bn254_curve_dbl", &b, true, false)
            }
            ZiskOp::Bn254ComplexAdd => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bn254_complex_add", &b, true, false)
            }
            ZiskOp::Bn254ComplexSub => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bn254_complex_sub", &b, true, false)
            }
            ZiskOp::Bn254ComplexMul => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bn254_complex_mul", &b, true, false)
            }
            ZiskOp::Bls12_381CurveAdd => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bls12_381_curve_add", &b, true, false)
            }
            ZiskOp::Bls12_381CurveDbl => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bls12_381_curve_dbl", &b, true, false)
            }
            ZiskOp::Bls12_381ComplexAdd => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bls12_381_complex_add", &b, true, false)
            }
            ZiskOp::Bls12_381ComplexSub => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bls12_381_complex_sub", &b, true, false)
            }
            ZiskOp::Bls12_381ComplexMul => {
                Self::precompile(ctx, code, zisk_op, "_opcode_bls12_381_complex_mul", &b, true, false)
            }
            // add256 is the only precompile that returns the c value instead of leaving it zero
            ZiskOp::Add256 => Self::precompile(ctx, code, zisk_op, "_opcode_add256", &b, true, true),

            /* Free input calls */
            ZiskOp::FcallParam => Self::fcall_param(ctx, code, &b),
            ZiskOp::Fcall => Self::fcall(ctx, code, &b),
            ZiskOp::FcallGet => Self::fcall_get(ctx, code, &b),

            /* Direct memory access */
            ZiskOp::DmaMemCpy | ZiskOp::DmaXMemCpy => {
                let count = Self::dma_count(instruction, zisk_op == ZiskOp::DmaXMemCpy);
                if ctx.minimal_trace() {
                    *code += &format!(
                        "\tmem_reads_size = zisk_dma_memcpy_mt({a}, {b}, {count}, mem_reads, \
                         mem_reads_size, step);{}\n",
                        ctx.comment_str("dma_memcpy(dst = a, src = b, count)")
                    );
                } else {
                    *code += &format!(
                        "\tzisk_dma_memcpy({a}, {b}, {count});{}\n",
                        ctx.comment_str("dma_memcpy(dst = a, src = b, count)")
                    );
                }
                set_c(code, a.clone(), "dma_memcpy: c = dst");
            }
            ZiskOp::DmaMemCmp | ZiskOp::DmaXMemCmp => {
                let count = Self::dma_count(instruction, zisk_op == ZiskOp::DmaXMemCmp);
                if ctx.minimal_trace() {
                    // The comparison result and the mem reads it wrote come back together
                    *code += &format!(
                        "\t{{{}\n",
                        ctx.comment_str("dma_memcmp(a, b, count) -> c, mem reads")
                    );
                    *code += &format!(
                        "\t\tZiskDmaResult result = zisk_dma_memcmp_mt({a}, {b}, {count}, \
                         mem_reads, mem_reads_size, step);\n"
                    );
                    *code += "\t\tc = result.value;\n";
                    *code += "\t\tmem_reads_size = result.mem_reads_size;\n";
                    *code += "\t}\n";
                } else {
                    set_c(
                        code,
                        format!("zisk_dma_memcmp({a}, {b}, {count})"),
                        "dma_memcmp: c = result",
                    );
                }
            }
            ZiskOp::DmaXMemSet => {
                // The fill byte travels in the instruction's extended argument
                let fill = (instruction.jmp_offset1 as u64) & 0xFF;
                if ctx.minimal_trace() {
                    *code += &format!(
                        "\tmem_reads_size = zisk_dma_memset_mt({a}, {fill}, {b}, mem_reads, \
                         mem_reads_size, step);{}\n",
                        ctx.comment_str("dma_memset(dst = a, fill, count = b)")
                    );
                } else {
                    *code += &format!(
                        "\tzisk_dma_memset({a}, {b}, {fill});{}\n",
                        ctx.comment_str("dma_memset(dst = a, count = b, fill)")
                    );
                }
                set_c(code, a.clone(), "dma_memset: c = dst");
            }

            // Unlike the other DMA operations, the count is b: there is no extended variant
            ZiskOp::DmaInputCpy => {
                if ctx.minimal_trace() {
                    // The source is the fcall result, which the runtime's routine finds by itself, so
                    // there is no source argument to pass
                    *code += &format!(
                        "\tmem_reads_size = zisk_dma_inputcpy_mt({a}, 0, {b}, mem_reads, \
                         mem_reads_size, step);{}\n",
                        ctx.comment_str("dma_inputcpy(dst = a, count = b)")
                    );
                    set_c(code, a.clone(), "dma_inputcpy: c = dst");
                } else {
                    set_c(code, format!("zisk_dma_inputcpy({a}, {b})"), "dma_inputcpy: c = dst");
                }
            }
            ZiskOp::Profile => {
                panic!("ZiskRom2C::operation_to_c() Internal opcode Profile, pc={}", ctx.pc)
            }
        }
    }

    /// Emits a comparison operation, whose c value is its flag
    fn compare(ctx: &mut ZiskCContext, code: &mut String, condition: String, name: &str) {
        *code += &format!(
            "\tflag = ({condition}) ? 1 : 0;{}\n",
            ctx.comment(format!("{name}: flag = a ? b"))
        );
        *code += &format!("\tc = flag;{}\n", ctx.comment(format!("{name}: c = flag")));
        ctx.flag = FlagState::Dynamic;
    }

    /// Emits a division or remainder operation, whose flag reports the division by zero case that
    /// the operation answers with a fixed value instead of trapping
    fn divide(
        ctx: &mut ZiskCContext,
        code: &mut String,
        zero_condition: String,
        zero_value: String,
        value: String,
        name: &str,
    ) {
        *code += &format!(
            "\tif ({zero_condition}) {{ c = {zero_value}; flag = 1; }}{}\n",
            ctx.comment(format!("{name}: division by zero"))
        );
        *code += &format!("\telse {{ c = {value}; flag = 0; }}{}\n", ctx.comment_str(name));
        ctx.flag = FlagState::Dynamic;
    }

    /// The words a precompile's operands occupy, as the minimal trace records them: how many words
    /// the operand block at b holds, and, for each of those words that is a pointer, how many words it
    /// points at.  A zero means that word is not a pointer.
    ///
    /// This is the shape the trace consumer expects, so it is the shape the assembly backend writes:
    /// see the `precompiled_save_mem_reads()` calls in `zisk_rom_2_asm`, which this mirrors one for
    /// one.
    fn precompile_operands(zisk_op: ZiskOp) -> (u64, &'static [u64]) {
        match zisk_op {
            ZiskOp::Keccak => (25, &[]),
            ZiskOp::Poseidon2 | ZiskOp::Poseidon1 => (16, &[]),
            ZiskOp::Sha256 => (2, &[4, 8]),
            ZiskOp::Blake2 => (3, &[0, 16, 16]),
            ZiskOp::Arith256 => (5, &[4, 4, 4]),
            ZiskOp::Arith256Mod => (5, &[4, 4, 4, 4]),
            ZiskOp::Arith384Mod => (5, &[6, 6, 6, 6]),
            ZiskOp::Secp256k1Add | ZiskOp::Secp256r1Add => (2, &[8, 8]),
            ZiskOp::Secp256k1Dbl | ZiskOp::Secp256r1Dbl => (8, &[]),
            ZiskOp::Bn254CurveAdd => (2, &[8, 8]),
            ZiskOp::Bn254CurveDbl => (8, &[]),
            ZiskOp::Bn254ComplexAdd | ZiskOp::Bn254ComplexSub | ZiskOp::Bn254ComplexMul => {
                (2, &[8, 8])
            }
            ZiskOp::Bls12_381CurveAdd => (2, &[12, 12]),
            ZiskOp::Bls12_381CurveDbl => (12, &[]),
            ZiskOp::Bls12_381ComplexAdd
            | ZiskOp::Bls12_381ComplexSub
            | ZiskOp::Bls12_381ComplexMul => (2, &[12, 12]),
            ZiskOp::Add256 => (4, &[4, 4]),
            _ => panic!("ZiskRom2C::precompile_operands() {zisk_op:?} is not a precompile"),
        }
    }

    /// Records everything a precompile reads: the operand block at b, and whatever the pointers in it
    /// point at.  It has to be done before the call, as the precompile writes its result over some of
    /// those same words.
    fn precompile_mem_reads(ctx: &mut ZiskCContext, code: &mut String, zisk_op: ZiskOp, b: &str) {
        let (params, loads) = Self::precompile_operands(zisk_op);
        assert!(
            loads.len() as u64 <= params,
            "ZiskRom2C::precompile_mem_reads() more pointers than operands, pc={}",
            ctx.pc
        );

        *code += &format!("\t{{{}\n", ctx.comment_str("precompile operands into the mem reads"));
        *code += &format!("\t\tconst uint64_t params = {b};\n");
        *code += &format!("\t\tfor (uint64_t i = 0; i < {params}; i++)\n");
        *code += "\t\t\tmem_reads[mem_reads_size + i] = ZM64(params + i * 8);\n";
        *code += &format!("\t\tmem_reads_size += {params};\n");
        for (i, words) in loads.iter().enumerate() {
            if *words == 0 {
                continue;
            }
            *code += &format!("\t\t{{{}\n", ctx.comment(format!("operand {i} points at {words}")));
            *code += &format!("\t\t\tconst uint64_t data = ZM64(params + {});\n", i * 8);
            *code += &format!("\t\t\tfor (uint64_t j = 0; j < {words}; j++)\n");
            *code += "\t\t\t\tmem_reads[mem_reads_size + j] = ZM64(data + j * 8);\n";
            *code += &format!("\t\t\tmem_reads_size += {words};\n");
            *code += "\t\t}\n";
        }
        *code += "\t}\n";
    }

    /// Emits a call to a runtime precompile.  The precompile reads and writes guest memory itself,
    /// addressed by b; c is zero unless the precompile returns it.
    fn precompile(
        ctx: &mut ZiskCContext,
        code: &mut String,
        zisk_op: ZiskOp,
        function: &str,
        b: &str,
        takes_pointer: bool,
        returns_c: bool,
    ) {
        let argument =
            if takes_pointer { format!("(uint64_t *)(uintptr_t)({b})") } else { b.to_string() };

        if ctx.minimal_trace() {
            Self::precompile_mem_reads(ctx, code, zisk_op, b);
        }

        if returns_c {
            *code += &format!(
                "\tc = {function}({argument});{}\n",
                ctx.comment_str("precompile: c = result")
            );
            if ctx.minimal_trace() {
                // A precompile that produces a c value records it: unlike the ones that write their
                // result to memory, there is nothing in the mem reads it could be derived from
                *code += &format!(
                    "\tmem_reads[mem_reads_size++] = c;{}\n",
                    ctx.comment_str("precompile result into the mem reads")
                );
            }
        } else {
            *code += &format!("\t{function}({argument});{}\n", ctx.comment_str("precompile"));
            *code += &format!("\tc = 0;{}\n", ctx.comment_str("precompile: c = 0"));
            ctx.c.is_constant = true;
            ctx.c.constant_value = 0;
        }
        ctx.flag = FlagState::AlwaysZero;
    }

    /// Emits the fcall_param operation, which appends one parameter, or a block of them, to the
    /// fcall context
    fn fcall_param(ctx: &mut ZiskCContext, code: &mut String, b: &str) {
        assert!(ctx.a.is_constant, "ZiskRom2C::fcall_param() a must be constant, pc={}", ctx.pc);
        assert!(
            ctx.a.constant_value <= 32,
            "ZiskRom2C::fcall_param() a must be <= 32, pc={}",
            ctx.pc
        );
        *code += &ctx.full_line_comment("fcall_param".to_string());

        // c takes the b value, and is what gets appended
        *code += &format!("\tc = {b};{}\n", ctx.comment_str("fcall_param: c = b"));

        if ctx.a.constant_value == 1 {
            // A single parameter, taken from c itself
            *code += &format!(
                "\tfcall_ctx[{FCALL_PARAMS} + fcall_ctx[{FCALL_PARAMS_SIZE}]] = c;{}\n",
                ctx.comment_str("ctx.params[size] = c")
            );
            *code += &format!(
                "\tfcall_ctx[{FCALL_PARAMS_SIZE}]++;{}\n",
                ctx.comment_str("inc ctx.params_size")
            );
        } else {
            // A block of parameters, read from the memory c points at
            *code += &format!("\tfor (uint64_t i = 0; i < {}; i++) {{\n", ctx.a.constant_value);
            *code += &format!(
                "\t\tfcall_ctx[{FCALL_PARAMS} + fcall_ctx[{FCALL_PARAMS_SIZE}] + i] = \
                 ZM64(c + i * 8);\n"
            );
            *code += "\t}\n";
            *code += &format!(
                "\tfcall_ctx[{FCALL_PARAMS_SIZE}] += {};{}\n",
                ctx.a.constant_value,
                ctx.comment_str("ctx.params_size += count")
            );
        }

        ctx.c.is_constant = ctx.b.is_constant;
        ctx.c.constant_value = ctx.b.constant_value;
        ctx.flag = FlagState::AlwaysZero;
    }

    /// Emits the fcall operation, which calls the runtime with the accumulated parameters and makes
    /// the first result word available as the next free input.
    ///
    /// Note that c takes the b value, not the result: the result reaches the program as the free
    /// input of a later instruction, not as this instruction's c.
    fn fcall(ctx: &mut ZiskCContext, code: &mut String, b: &str) {
        *code += &ctx.full_line_comment("fcall".to_string());
        *code += &format!("\tc = {b};{}\n", ctx.comment_str("fcall: c = b"));

        // One function id is not a call into the runtime's fcall proxy at all, but a request to wait
        // for input data, so it is generated as such and produces no result
        if ctx.a.is_constant && (ctx.a.constant_value == FCALL_INPUT_READY_ID as u64) {
            Self::wait_for_input_ready(ctx, code);
        } else {
            Self::fcall_runtime(ctx, code);
        }

        // Both paths leave the parameters consumed and the first result word as the free input, which
        // for the input ready case means no result at all
        *code += &format!(
            "\tfcall_ctx[{FCALL_PARAMS_SIZE}] = 0;{}\n",
            ctx.comment_str("ctx.params_size = 0")
        );
        *code += &format!(
            "\tfcall_ctx[{FCALL_RESULT_GOT}] = 1;{}\n",
            ctx.comment_str("ctx.result_got = 1")
        );

        ctx.c.is_constant = ctx.b.is_constant;
        ctx.c.constant_value = ctx.b.constant_value;
        ctx.flag = FlagState::AlwaysZero;
    }

    /// Emits the ordinary fcall: hand the accumulated parameters to the runtime and take its first
    /// result word, if it produced any, as the free input
    fn fcall_runtime(ctx: &mut ZiskCContext, code: &mut String) {
        *code += &format!(
            "\tfcall_ctx[{FCALL_FUNCTION_ID}] = {};{}\n",
            ctx.a.expr,
            ctx.comment_str("ctx.function_id = a")
        );
        *code += &format!("\t_opcode_fcall(fcall_ctx);{}\n", ctx.comment_str("call the runtime"));
        *code += &format!(
            "\tMEM_FREE_INPUT = fcall_ctx[{FCALL_RESULT_SIZE}] ? fcall_ctx[{FCALL_RESULT}] : 0;{}\n",
            ctx.comment_str("free_input = ctx.result_size ? ctx.result[0] : 0")
        );
    }

    /// Emits the FCALL_INPUT_READY_ID case of fcall, which waits until the input data the program is
    /// about to read has been written to the input region.
    ///
    /// The single parameter is the address of the last byte the program requires.  The runtime counts
    /// input in bytes from INPUT_ADDR, so the address becomes a count; it is rounded down to the u64
    /// that contains the byte, because a u64 only ever arrives whole.  The runtime is called only when
    /// the data is not there yet, as that call blocks; a non zero return means the emulation is over
    /// (the other side exited or asked for a reset), which ends it here exactly like the assembly
    /// backend's jump to its end label.
    fn wait_for_input_ready(ctx: &mut ZiskCContext, code: &mut String) {
        assert!(ctx.a.is_constant, "ZiskRom2C::wait_for_input_ready() a must be constant");

        *code +=
            &ctx.full_line_comment("fcall: wait for the input data to be available".to_string());

        // A block, so that several of these can live in the same C function
        *code += "\t{\n";
        *code += &format!(
            "\t\tuint64_t required_bytes = (fcall_ctx[{FCALL_PARAMS}] - {}) & ~(uint64_t)0x7;{}\n",
            u64_lit(INPUT_ADDR),
            ctx.comment_str("params[0] = address of the last required byte")
        );
        *code += &format!(
            "\t\tif (required_bytes > ZISK_INPUT_WRITTEN) {{{}\n",
            ctx.comment_str("not written yet: block in the runtime")
        );
        *code += &format!(
            "\t\t\tif (_wait_for_input_avail(required_bytes) != 0) goto {};{}\n",
            if ctx.chunked() { "zisk_end" } else { "emu_end" },
            ctx.comment_str("the emulation is over")
        );
        *code += "\t\t}\n";
        *code += "\t}\n";

        // The input ready fcall produces no result
        *code += &format!("\tMEM_FREE_INPUT = 0;{}\n", ctx.comment_str("free_input = 0"));
    }

    /// Emits the fcall_get operation, which advances through the results of the last fcall.
    ///
    /// Like fcall(), c takes the b value.  The read is not bounds checked, matching the assembly
    /// backend: the result capacity is what bounds it.
    fn fcall_get(ctx: &mut ZiskCContext, code: &mut String, b: &str) {
        *code += &ctx.full_line_comment("fcall_get".to_string());
        *code += &format!("\tc = {b};{}\n", ctx.comment_str("fcall_get: c = b"));
        *code += &format!(
            "\tMEM_FREE_INPUT = fcall_ctx[{FCALL_RESULT} + fcall_ctx[{FCALL_RESULT_GOT}]];{}\n",
            ctx.comment_str("free_input = ctx.result[result_got]")
        );
        *code += &format!(
            "\tfcall_ctx[{FCALL_RESULT_GOT}]++;{}\n",
            ctx.comment_str("inc ctx.result_got")
        );

        ctx.c.is_constant = ctx.b.is_constant;
        ctx.c.constant_value = ctx.b.constant_value;
        ctx.flag = FlagState::AlwaysZero;
    }

    /// The byte count of a DMA operation: an instruction argument for the extended variants, or a
    /// well-known memory location for the rest
    fn dma_count(instruction: &ZiskInst, extended: bool) -> String {
        if extended {
            u64_lit(instruction.jmp_offset1 as u64)
        } else {
            format!("ZM64({})", u64_lit(EXTRA_PARAMS_ADDR))
        }
    }

    /**********/
    /* SET PC */
    /**********/

    /// Emits the program counter update and the jump to the next instruction.
    ///
    /// This is where the flag knowledge pays off: when the flag is statically known, or when both
    /// jump offsets agree, there is a single possible successor, so the jump is unconditional and
    /// disappears entirely when that successor is the instruction that follows.
    fn set_pc(ctx: &mut ZiskCContext, instruction: &ZiskInst, code: &mut String, rom: &ZiskRom) {
        if instruction.set_pc {
            // The new pc is the c value, i.e. a computed jump
            *code += &ctx.full_line_comment("set pc".to_string());
            if ctx.c.is_constant {
                let new_pc = (ctx.c.constant_value as i64 + instruction.jmp_offset1) as u64;
                *code += &format!(
                    "\tpc = {};{}\n",
                    u64_lit(new_pc),
                    ctx.comment_str("pc = c(const) + jmp_offset1")
                );
                Self::jump_to(ctx, code, rom, new_pc, "jump to static pc c=const");
            } else {
                if instruction.jmp_offset1 != 0 {
                    *code += &format!(
                        "\tpc = {} + {};{}\n",
                        ctx.c.expr,
                        offset_lit(instruction.jmp_offset1),
                        ctx.comment_str("pc = c + jmp_offset1")
                    );
                } else {
                    *code += &format!("\tpc = {};{}\n", ctx.c.expr, ctx.comment_str("pc = c"));
                }
                Self::jump_to_dynamic_pc(ctx, code);
            }
        } else if ctx.flag == FlagState::AlwaysZero {
            let new_pc = (ctx.pc as i64 + instruction.jmp_offset2) as u64;
            Self::jump_to_known_pc(ctx, code, rom, new_pc, "flag=0: pc += jmp_offset2");
        } else if ctx.flag == FlagState::AlwaysOne {
            let new_pc = (ctx.pc as i64 + instruction.jmp_offset1) as u64;
            Self::jump_to_known_pc(ctx, code, rom, new_pc, "flag=1: pc += jmp_offset1");
        } else if instruction.jmp_offset1 == instruction.jmp_offset2 {
            // The flag is dynamic but both successors are the same, so it does not matter
            let new_pc = (ctx.pc as i64 + instruction.jmp_offset1) as u64;
            Self::jump_to_known_pc(ctx, code, rom, new_pc, "jmp_offset1 == jmp_offset2");
        } else {
            // A real conditional branch: one successor per flag value
            let pc_if_set = (ctx.pc as i64 + instruction.jmp_offset1) as u64;
            let pc_if_clear = (ctx.pc as i64 + instruction.jmp_offset2) as u64;

            // Branch on the less likely successor and let the other one fall through when it is the
            // instruction that follows.  Only when that instruction is in this chunk: the last
            // instruction of a chunk has nothing to fall through to, so both successors have to be
            // jumps, or the flag=0 path would run into the chunk's exit labels and end the emulation
            if pc_if_clear == ctx.next_pc && ctx.in_chunk(pc_if_clear) {
                *code += &format!("\tif (flag) {{{}\n", ctx.comment_str("flag=1: jump"));
                Self::indented_jump(ctx, code, rom, pc_if_set, "flag=1: pc += jmp_offset1");
                *code += "\t}\n";
            } else if pc_if_set == ctx.next_pc && ctx.in_chunk(pc_if_set) {
                *code += &format!("\tif (!flag) {{{}\n", ctx.comment_str("flag=0: jump"));
                Self::indented_jump(ctx, code, rom, pc_if_clear, "flag=0: pc += jmp_offset2");
                *code += "\t}\n";
            } else {
                *code += &format!("\tif (flag) {{{}\n", ctx.comment_str("flag=1: jump"));
                Self::indented_jump(ctx, code, rom, pc_if_set, "flag=1: pc += jmp_offset1");
                *code += "\t} else {\n";
                Self::indented_jump(ctx, code, rom, pc_if_clear, "flag=0: pc += jmp_offset2");
                *code += "\t}\n";
            }
        }
    }

    /// Emits the jump to a program counter known at generation time, eliding it when the target is
    /// the instruction that follows
    fn jump_to_known_pc(
        ctx: &mut ZiskCContext,
        code: &mut String,
        rom: &ZiskRom,
        new_pc: u64,
        comment: &str,
    ) {
        // Falling through to the next instruction needs no jump at all, but only when that
        // instruction is in this chunk: otherwise control has to go back to the dispatcher
        if new_pc == ctx.next_pc && ctx.in_chunk(new_pc) {
            *code += &ctx.full_line_comment(format!("{comment} (falls through)"));
            return;
        }
        *code += &format!("\tpc = {};{}\n", u64_lit(new_pc), ctx.comment_str(comment));
        Self::jump_to(ctx, code, rom, new_pc, comment);
    }

    /// Emits a jump to a program counter known at generation time: a direct goto when the target is
    /// a real instruction of this chunk, a return to the dispatcher when it is in another chunk, and
    /// a dynamic jump when it is not an instruction at all
    fn jump_to(
        ctx: &mut ZiskCContext,
        code: &mut String,
        rom: &ZiskRom,
        new_pc: u64,
        comment: &str,
    ) {
        if rom.sorted_pc_list.binary_search(&new_pc).is_ok() {
            if ctx.in_chunk(new_pc) {
                *code += &format!("\tgoto pc_{new_pc:x};{}\n", ctx.comment_str(comment));
            } else {
                *code += &format!("\tgoto zisk_leave;{}\n", ctx.comment_str("leaves this chunk"));
            }
        } else {
            // The target is not an instruction of this ROM, so let the branch table decide, exactly
            // like the assembly backend does
            Self::jump_to_dynamic_pc(ctx, code);
        }
    }

    /// Same as jump_to(), one level of indentation deeper, for the inside of a conditional
    fn indented_jump(
        ctx: &mut ZiskCContext,
        code: &mut String,
        rom: &ZiskRom,
        new_pc: u64,
        comment: &str,
    ) {
        *code += &format!("\t\tpc = {};{}\n", u64_lit(new_pc), ctx.comment_str(comment));
        if rom.sorted_pc_list.binary_search(&new_pc).is_ok() {
            if ctx.in_chunk(new_pc) {
                *code += &format!("\t\tgoto pc_{new_pc:x};\n");
            } else {
                *code += "\t\tgoto zisk_leave;\n";
            }
        } else if ctx.chunked() {
            *code += "\t\tgoto zisk_unknown_pc;\n";
        } else {
            *code += &format!("\t\tgoto *zisk_map_pc[pc - {}];\n", u64_lit(ROM_ADDR));
        }
    }

    /// Emits a jump to a program counter only known at run time.
    ///
    /// Without chunking there is one table covering the whole ROM.  With chunking, a target inside
    /// this chunk is still a single indirect branch through the chunk's own table, and only a target
    /// elsewhere costs a return to the dispatcher.
    fn jump_to_dynamic_pc(ctx: &mut ZiskCContext, code: &mut String) {
        if !ctx.chunked() {
            *code += &format!(
                "\tgoto *zisk_map_pc[pc - {}];{}\n",
                u64_lit(ROM_ADDR),
                ctx.comment_str("jump to dynamic pc")
            );
            return;
        }

        *code += &format!(
            "\tif (pc >= {} && pc <= {}) goto *local_map[(pc - {}) >> 1];{}\n",
            u64_lit(ctx.chunk_lo),
            u64_lit(ctx.chunk_hi),
            u64_lit(ctx.chunk_lo),
            ctx.comment_str("dynamic pc inside this chunk")
        );
        *code += &format!("\tgoto zisk_leave;{}\n", ctx.comment_str("dynamic pc elsewhere"));
    }

    /*********************/
    /* ROM INITIAL DATA  */
    /*********************/

    /// Emits the two functions the C main program calls before starting the emulation, to lay the
    /// ROM's initial data into guest memory
    fn write_init_data(ctx: &mut ZiskCContext, code: &mut String, rom: &ZiskRom) {
        *code += "\n/* Read-only ROM data */\n";
        *code += "void write_ro_init_data(void) {\n";
        for section in &rom.ro_data_64 {
            for (j, value) in section.data.iter().enumerate() {
                *code += &format!(
                    "\tZM64({}) = {};\n",
                    u64_lit(section.addr + (j as u64) * 8),
                    u64_lit(*value)
                );
            }
        }
        *code += "}\n";

        *code += "\n/* Read-write ROM data */\n";
        *code += "void write_rw_init_data(void) {\n";
        for section in &rom.rw_data_64 {
            for (j, value) in section.data.iter().enumerate() {
                *code += &format!(
                    "\tZM64({}) = {};\n",
                    u64_lit(section.addr + (j as u64) * 8),
                    u64_lit(*value)
                );
            }
        }
        *code += "}\n";

        *code += &ctx.full_line_comment("end of generated code".to_string());
    }
}
