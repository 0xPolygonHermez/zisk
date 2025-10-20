//! Zisk ROM to ASM
//!
//! Generates i86_64 assembly code that implements the Zisk ROM program
use std::path::Path;

use crate::{
    zisk_ops::ZiskOp, AsmGenerationMethod, ZiskInst, ZiskRom, FLOAT_LIB_ROM_ADDR, FREE_INPUT_ADDR,
    M64, P2_32, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG,
    SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE, STORE_REG,
};

// Regs rax, rcx, rdx, rdi, rsi, rsp, and r8-r11 are caller-save, not saved across function calls.
// Reg rax is used to store a function’s return value.
// Regs rbx, rbp, and r12-r15 are callee-save, saved across function calls.

// The caller uses registers to pass the first 6 arguments to the callee.
// Given the arguments in left-to-right order, the order of registers used is:
// rdi, rsi, rdx, rcx, r8, and r9.
// Any remaining arguments are passed on the stack in reverse order so that they can be popped off
// the stack in order.

const REG_A: &str = "rbx";
const REG_A_W: &str = "ebx";
const REG_B: &str = "rax";
const REG_B_W: &str = "eax";
const REG_B_H: &str = "ax";
const REG_B_B: &str = "al";
const REG_C: &str = "r15";
const REG_C_W: &str = "r15d";
const REG_C_H: &str = "r15w";
const REG_C_B: &str = "r15b";
const REG_FLAG: &str = "rdx";
const REG_STEP: &str = "r14";
const REG_VALUE: &str = "r9";
const REG_VALUE_W: &str = "r9d";
const REG_VALUE_H: &str = "r9w";
const REG_VALUE_B: &str = "r9b";
const REG_ADDRESS: &str = "r10";
const REG_MEM_READS_ADDRESS: &str = "r12";
const REG_MEM_READS_SIZE: &str = "r13";
const REG_AUX: &str = "r11";
const REG_PC: &str = "r8";
const REG_ACTIVE_CHUNK: &str = "rbp"; // Used only in Zip
const REG_CHUNK_PLAYER_ADDRESS: &str = "rbp"; // Used only in chunk player

// not used:
//   - rbp (frame pointer, must be restored before calling other functions),
//   - rcx (overwritten during syscall)
//   - rdi
//   - rsi
//   - rsp

// Only used to calculate histogram position for every rom pc
const TRACE_ADDR_NUMBER: u64 = 0xc0000020;

// Fcall context offsets of the different fields
const FCALL_FUNCTION_ID: u64 = 0;
const FCALL_PARAMS_CAPACITY: u64 = 1;
const FCALL_PARAMS_SIZE: u64 = 2;
const FCALL_PARAMS: u64 = 3;
const FCALL_RESULT_CAPACITY: u64 = 35;
const FCALL_RESULT_SIZE: u64 = 36;
const FCALL_RESULT: u64 = 37;
const FCALL_RESULT_GOT: u64 = 69;

const XMM_MAPPED_REGS: [u64; 16] = [1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];
//const XMM_MAPPED_REGS: [u64; 0] = []; // Used for debugging

const F_MEM_CLEAR_WRITE_BYTE: u64 = 1 << 37;
const F_MEM_WRITE_SHIFT: u64 = 36;
const F_MEM_WRITE: u64 = 1 << F_MEM_WRITE_SHIFT;
const F_MEM_WIDTH_SHIFT: u64 = 32;

#[derive(Default, Debug, Clone)]
pub struct ZiskAsmRegister {
    is_constant: bool,   // register is a constant value known at compilation time
    constant_value: u64, // register constant value, only valid if is_constant==true
    is_saved: bool,      // register has been saved to memory/register
    string_value: String, // register string value: a constant value (e.g. "0x3f") or a register
                         // (e.g. "rax")
}

#[derive(Default, Debug, Clone)]
pub struct ZiskAsmContext {
    pc: u64,
    next_pc: u64,
    flag_is_always_one: bool,
    flag_is_always_zero: bool,
    jump_to_dynamic_pc: bool,
    jump_to_static_pc: String,
    log_output: bool,
    call_chunk_done: bool,
    mode: AsmGenerationMethod,
    a: ZiskAsmRegister,
    b: ZiskAsmRegister,
    c: ZiskAsmRegister,

    address_is_constant: bool,   // true if address is a constant value
    address_constant_value: u64, // address constant value, only valid if address_is_constant==true

    // This is the address of the entrypoint.
    min_program_pc: u64,

    // Force in which register a or b must be stored
    store_a_in_c: bool,
    store_a_in_a: bool,
    store_b_in_c: bool,
    store_b_in_b: bool,

    // Memory variables
    mem_step: String,
    mem_sp: String,
    mem_end: String,
    mem_error: String,
    mem_trace_address: String,
    mem_chunk_address: String,
    mem_chunk_start_step: String,
    fcall_ctx: String,
    mem_chunk_id: String,   // 0, 1, 2, 3, 4...
    mem_chunk_mask: String, // Module 8 of the chunks we want to activate, e.g. 0x03
    mem_rsp: String,        // Backup of rsp register value from caller

    comments: bool, // true if we want to generate comments in the assembly source code
    boc: String,    // begin of comment: '/*', ';', '#', etc.
    eoc: String,    // end of comment: '*/', '', etc

    ptr: String, // "ptr ", ""

                 //assert_rsp_counter: u64,
}

impl ZiskAsmContext {
    pub fn fast(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmFast
    }
    pub fn minimal_trace(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmMinimalTraces
    }
    pub fn rom_histogram(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmRomHistogram
    }
    pub fn main_trace(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmMainTrace
    }
    pub fn chunks(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmChunks
    }
    pub fn zip(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmZip
    }
    pub fn mem_op(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmMemOp
    }
    pub fn chunk_player_mt_collect_mem(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmChunkPlayerMTCollectMem
    }
    pub fn mem_reads(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmMemReads
    }
    pub fn chunk_player_mem_reads_collect_main(&self) -> bool {
        self.mode == AsmGenerationMethod::AsmChunkPlayerMemReadsCollectMain
    }
    pub fn jump_to_unaligned_pc(&self) -> bool {
        //ctx.chunk_player_mem_reads_collect_main() || ctx.chunk_player_mt_collect_mem()
        true
    }

    // Creates a comment with the specified prefix and sufix, i.e. with the requested syntax
    pub fn comment(&self, c: String) -> String {
        let mut s = String::new();
        if self.comments {
            s = format!("{}{}{}", self.boc, c, self.eoc);
        }
        s
    }

    // Creates a comment from a str
    pub fn comment_str(&self, c: &str) -> String {
        self.comment(c.to_string())
    }

    // Creates a full-line comment
    pub fn full_line_comment(&self, c: String) -> String {
        let mut s = String::new();
        if self.comments {
            s = format!("\t{}{}{}\n", self.boc, c, self.eoc);
        }
        s
    }

    pub fn op_is_precompiled(&self, zisk_op: &ZiskOp) -> bool {
        matches!(
            zisk_op,
            ZiskOp::Keccak
                | ZiskOp::Sha256
                | ZiskOp::Arith256
                | ZiskOp::Arith256Mod
                | ZiskOp::Secp256k1Add
                | ZiskOp::Secp256k1Dbl
                | ZiskOp::Bn254CurveAdd
                | ZiskOp::Bn254CurveDbl
                | ZiskOp::Bn254ComplexAdd
                | ZiskOp::Bn254ComplexSub
                | ZiskOp::Bn254ComplexMul
                | ZiskOp::Arith384Mod
                | ZiskOp::Bls12_381CurveAdd
                | ZiskOp::Bls12_381CurveDbl
                | ZiskOp::Bls12_381ComplexAdd
                | ZiskOp::Bls12_381ComplexSub
                | ZiskOp::Bls12_381ComplexMul
                | ZiskOp::Add256
        )
    }
}

// One-pass (single emulation) memory trace, used to count, plan and collect.
// If ZisK instruction contains at least one memory operation:
//   [32b] header (from higher bits to lower bits)
//     [1b] read_a
//       0 = no reg a mem op
//       1 = one reg a mem op
//     [3b] read_b
//       0 = no reg b mem op
//       1 = one reg b mem op of width 1
//       2 = one reg b mem op of width 2
//       3 = one reg b mem op of width 4
//       4 = one reg b mem op of width 8
//     [3b] write
//       0 = no write op
//       1 = one write c mem op of width 1
//       2 = one write c mem op of width 2
//       3 = one write c mem op of width 4
//       4 = one write c mem op of width 8
//       5 = one precompiled mem op of contiguous addresses
//       6 = one precompiled mem op of non-contiguous addresses
//     [25b] relative step: lower bits of step
// If header.read_a == 1:
//   [32b] a mem address
// If header.read_b == 1, 2, 3 or 4:
//   [32b] b mem address
// If header.write == 1, 2, 3 or 4
//   [32b] c mem address
//   [64b] c write value
// If header.write == 5
//   [32b] prec_cont_count = prec_read_count + prec_write_count<<16
//   [32b] prec_const_address
//   [64b x prec_write_count] prec_cont_write_data
// If header.write == 6
//   [32b] prec_non_cont_count = prec_read_count + prec_write_count<<16
//   [32b x prec_read_count] prec_non_cont_read_address = precompiled read addresses
//   [32b x prec_write_count] prec_non_const_write_address = precompiled write addresses
//   [64b x prec_write_count] prec_non_const_write_data = precompiled write data
// If not aligned to 64b
//   [32b] padding zeros

// pub struct ZiskAsmMemTraceContext {
//     // Header mask
//     header_mask: u32,

//     // a, b and c
//     a_mem_address_offset: u64,
//     b_mem_address_offset: u64,
//     c_mem_address_offset: u64,
//     c_write_value_offset: u64,

//     // Precompiled with continuous data
//     prec_cont_count_offset: u64,
//     prec_cont_address_offset: u64,
//     prec_cont_write_data_offset: u64,

//     // Precompiled with non-continuous data
//     prec_non_cont_count_offset: u64,
//     prec_non_cont_read_address_offset: u64,
//     prec_non_cont_write_address_offset: u64,
//     prec_non_cont_write_data_offset: u64,
// }
// const MEM_TRACE_HEADER_READ_A: u32 = 1u32 << 31;
// const MEM_TRACE_HEADER_READ_B_1: u32 = 1u32 << 27;
// const MEM_TRACE_HEADER_READ_B_2: u32 = 2u32 << 27;
// const MEM_TRACE_HEADER_READ_B_4: u32 = 3u32 << 27;
// const MEM_TRACE_HEADER_READ_B_8: u32 = 4u32 << 27;
// const MEM_TRACE_HEADER_WRITE_C_1: u32 = 1u32 << 24;
// const MEM_TRACE_HEADER_WRITE_C_2: u32 = 2u32 << 24;
// const MEM_TRACE_HEADER_WRITE_C_4: u32 = 3u32 << 24;
// const MEM_TRACE_HEADER_WRITE_C_8: u32 = 4u32 << 24;
// const MEM_TRACE_HEADER_WRITE_PREC_CONT: u32 = 5u32 << 24;
// const MEM_TRACE_HEADER_WRITE_PREC_NON_CONT: u32 = 6u32 << 24;

// impl ZiskAsmMemTraceContext {
//     pub fn reset(&mut self) {
//         self.header_mask = 0;
//         self.a_mem_address_offset = 0;
//         self.b_mem_address_offset = 0;
//         self.c_mem_address_offset = 0;
//         self.c_write_value_offset = 0;

//         self.prec_cont_count_offset = 0;
//         self.prec_cont_address_offset = 0;
//         self.prec_cont_write_data_offset = 0;

//         self.prec_non_cont_count_offset = 0;
//         self.prec_non_cont_read_address_offset = 0;
//         self.prec_non_cont_write_address_offset = 0;
//         self.prec_non_cont_write_data_offset = 0;
//     }
//     pub fn configure(&mut self, instruction: &ZiskInst) {
//         let mut offset: u64 = 4;

//         if instruction.a_src == SRC_MEM {
//             self.header_mask |= TRACE_CONTEXT_HEADER_READ_A;
//             self.a_mem_address_offset = offset;
//             offset += 4;
//         }
//         if instruction.b_src == SRC_MEM {
//             self.header_mask |= TRACE_CONTEXT_HEADER_READ_B_8;
//             self.b_mem_address_offset = offset;
//             offset += 4;
//         }
//         if instruction.b_src == SRC_IND {
//             match instruction.ind_width {
//                 1 => self.header_mask |= TRACE_CONTEXT_HEADER_READ_B_1,
//                 2 => self.header_mask |= TRACE_CONTEXT_HEADER_READ_B_2,
//                 4 => self.header_mask |= TRACE_CONTEXT_HEADER_READ_B_4,
//                 8 => self.header_mask |= TRACE_CONTEXT_HEADER_READ_B_8,
//                 _ => {
//                     panic!(
//                         "ZiskAsmMemTraceContext::configure() invalid instruction.ind_width={}",
//                         instruction.ind_width
//                     );
//                 }
//             }
//             self.b_mem_address_offset = offset;
//             offset += 4;
//         }
//         if instruction.store == STORE_MEM {
//             self.header_mask |= TRACE_CONTEXT_HEADER_WRITE_C_8;
//             self.c_mem_address_offset = offset;
//             offset += 4;
//         }
//         if instruction.store == STORE_IND {
//             match instruction.ind_width {
//                 1 => self.header_mask |= TRACE_CONTEXT_HEADER_WRITE_C_1,
//                 2 => self.header_mask |= TRACE_CONTEXT_HEADER_WRITE_C_2,
//                 4 => self.header_mask |= TRACE_CONTEXT_HEADER_WRITE_C_4,
//                 8 => self.header_mask |= TRACE_CONTEXT_HEADER_WRITE_C_4,
//                 _ => {
//                     panic!(
//                         "ZiskAsmMemTraceContext::configure() invalid instruction.ind_width={}",
//                         instruction.ind_width
//                     );
//                 }
//             }
//             self.c_mem_address_offset = offset;
//             offset += 4;
//             self.c_write_value_offset = offset;
//             offset += 8;
//         }
//         if instruction.op == ZiskOp::Keccak.code() || instruction.op == ZiskOp::Sha256.code() {
//             self.header_mask |= TRACE_CONTEXT_HEADER_WRITE_PREC_CONT;
//             self.prec_cont_count_offset = offset;
//             offset += 4;
//             self.prec_cont_address_offset = offset;
//             offset += 4;
//             self.prec_cont_write_data_offset = offset;
//             offset += 4;
//         }
//     }
// }

pub struct ZiskRom2Asm {}

impl ZiskRom2Asm {
    /// Saves ZisK rom into an i64-64 assembly file: first save to a string, then
    /// save the string to the file
    pub fn save_to_asm_file(
        rom: &ZiskRom,
        file_name: &Path,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
    ) {
        // Get a string with the ASM data
        let mut s = String::new();
        Self::save_to_asm(rom, &mut s, generation_method, log_output, comments);

        // Save to file
        let path = std::path::PathBuf::from(file_name);
        let result = std::fs::write(path, s);
        if result.is_err() {
            panic!(
                "ZiskRom2Asm::save_to_asm_file() failed writing to file={}",
                file_name.display()
            );
        }
    }

    /// Saves ZisK rom into an i86-64 assembly data string
    pub fn save_to_asm(
        rom: &ZiskRom,
        code: &mut String,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
    ) {
        // Clear output data, just in case
        code.clear();

        // Store less usual code branches in distant memory to improve cache hits
        let mut unusual_code: String = String::new();

        // Create context
        let mut ctx = ZiskAsmContext {
            log_output,
            call_chunk_done: true,
            mode: generation_method,
            comments,
            boc: "/* ".to_string(),
            eoc: " */".to_string(),
            min_program_pc: rom.min_program_pc,
            ..Default::default()
        };

        ctx.ptr = "ptr ".to_string();
        ctx.mem_step = format!("qword {}[MEM_STEP]", ctx.ptr);
        ctx.mem_sp = format!("qword {}[MEM_SP]", ctx.ptr);
        ctx.mem_end = format!("qword {}[MEM_END]", ctx.ptr);
        ctx.mem_error = format!("qword {}[MEM_ERROR]", ctx.ptr);
        ctx.mem_trace_address = format!("qword {}[MEM_TRACE_ADDRESS]", ctx.ptr);
        ctx.mem_chunk_address = format!("qword {}[MEM_CHUNK_ADDRESS]", ctx.ptr);
        ctx.mem_chunk_start_step = format!("qword {}[MEM_CHUNK_START_STEP]", ctx.ptr);
        ctx.fcall_ctx = "fcall_ctx".to_string();
        ctx.mem_chunk_id = format!("qword {}[MEM_CHUNK_ID]", ctx.ptr);
        ctx.mem_chunk_mask = format!("qword {}[chunk_mask]", ctx.ptr);
        ctx.mem_rsp = format!("qword {}[MEM_RSP]", ctx.ptr);

        // Preamble
        *code += ".intel_syntax noprefix\n";
        *code += ".code64\n";

        // Global variables
        *code += ".section .data\n";
        *code += ".align 8\n";
        *code += ".comm MEM_STEP, 8, 8\n";
        *code += ".comm MEM_SP, 8, 8\n";
        *code += ".comm MEM_END, 8, 8\n";
        *code += ".comm MEM_ERROR, 8, 8\n";
        *code += ".comm MEM_TRACE_ADDRESS, 8, 8\n";
        *code += ".comm MEM_CHUNK_ADDRESS, 8, 8\n";
        *code += ".comm MEM_CHUNK_START_STEP, 8, 8\n";
        *code += ".comm MEM_RSP, 8, 8\n";
        if ctx.zip() {
            *code += ".comm MEM_CHUNK_ID, 8, 8\n";
        }

        // Allocate space for the registers
        for r in 0u64..35u64 {
            if !XMM_MAPPED_REGS.contains(&r) {
                *code += &format!(".comm reg_{r}, 8, 8\n");
            }
        }

        if ctx.main_trace() {
            for i in 0..3 {
                *code += &format!(".comm reg_steps_{i}, 8, 8\n");
            }
            for i in 0..3 {
                *code += &format!(".comm reg_prev_steps_{i}, 8, 8\n");
            }
            for i in 0..3 {
                *code += &format!(".comm reg_step_ranges_{i}, 8, 8\n");
            }
            for i in 0..35 {
                *code += &format!(".comm first_step_uses_{i}, 8, 8\n");
            }
        }

        // fcall_context =
        //     function_id
        //     params_max_size
        //     params_size
        //     params[32]
        //     result_max_size
        //     result_size
        //     result[32]
        //     result_got
        *code += ".comm fcall_ctx, 8*70, 8\n";

        // for k in 0..keys.len() {
        //     let pc = keys[k];
        //     let instruction = &rom.insts[&pc].i;
        //     *s += &format!("pc_{}_log: .ascii \"PCLOG={}\\n\"\n", pc, instruction.to_text());
        //     *s += &format!(".set pc_{}_log_len, (. - pc_{}_log)\n", pc, pc);
        // }

        *code += ".section .text\n";
        *code += ".extern print_abcflag\n";
        *code += ".extern print_char\n";
        *code += ".extern print_step\n";
        *code += ".extern opcode_keccak\n";
        *code += ".extern opcode_sha256\n";
        *code += ".extern opcode_arith256\n";
        *code += ".extern opcode_arith256_mod\n";
        *code += ".extern opcode_secp256k1_add\n";
        *code += ".extern opcode_secp256k1_dbl\n";
        *code += ".extern opcode_fcall\n";
        *code += ".extern opcode_bn254_curve_add\n";
        *code += ".extern opcode_bn254_curve_dbl\n";
        *code += ".extern opcode_bn254_complex_add\n";
        *code += ".extern opcode_bn254_complex_sub\n";
        *code += ".extern opcode_bn254_complex_mul\n";
        *code += ".extern opcode_arith384_mod\n";
        *code += ".extern opcode_bls12_381_curve_add\n";
        *code += ".extern opcode_bls12_381_curve_dbl\n";
        *code += ".extern opcode_bls12_381_complex_add\n";
        *code += ".extern opcode_bls12_381_complex_sub\n";
        *code += ".extern opcode_bls12_381_complex_mul\n";
        *code += ".extern opcode_add256\n";
        *code += ".extern chunk_done\n";
        *code += ".extern print_fcall_ctx\n";
        *code += ".extern print_pc\n";
        *code += ".extern realloc_trace\n\n";

        if ctx.minimal_trace()
            || ctx.main_trace()
            || ctx.zip()
            || ctx.mem_op()
            || ctx.chunk_player_mt_collect_mem()
            || ctx.mem_reads()
            || ctx.chunk_player_mem_reads_collect_main()
        {
            *code += ".extern max_steps\n";
            *code += ".extern chunk_size\n";
            *code += ".extern trace_address\n\n";
            *code += ".extern trace_address_threshold\n\n";
        }

        if ctx.zip() {
            *code += ".extern chunk_mask\n";
        }

        if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
            *code += ".extern chunk_player_address\n";
        }

        if ctx.chunks()
            || ctx.minimal_trace()
            || ctx.main_trace()
            || ctx.zip()
            || ctx.mem_op()
            || ctx.mem_reads()
        {
            // Chunk start
            *code += "chunk_start:\n";
            Self::chunk_start(&mut ctx, code, "start");
            *code += "\tret\n\n";

            // Chunk end
            *code += "chunk_end:\n";
            Self::chunk_end(&mut ctx, code, "end");
            *code += "\tret\n\n";

            // Chunk end and start
            *code += "chunk_end_and_start:\n";
            Self::chunk_end(&mut ctx, code, "end_and_start");
            *code += "\n";
            Self::chunk_start(&mut ctx, code, "end_and_start");
            *code += "\tret\n\n";
        }

        // Functions to let C know about ASM generation
        *code += ".global get_max_bios_pc\n";
        *code += "get_max_bios_pc:\n";
        *code += &format!("\tmov rax, 0x{:08x}\n", rom.max_bios_pc);
        *code += "\tret\n\n";

        *code += ".global get_max_program_pc\n";
        *code += "get_max_program_pc:\n";
        *code += &format!("\tmov rax, 0x{:08x}\n", rom.max_program_pc);
        *code += "\tret\n\n";

        *code += ".global get_gen_method\n";
        *code += "get_gen_method:\n";
        if ctx.fast() {
            *code += "\tmov rax, 0\n";
        } else if ctx.minimal_trace() {
            *code += "\tmov rax, 1\n";
        } else if ctx.rom_histogram() {
            *code += "\tmov rax, 2\n";
        } else if ctx.main_trace() {
            *code += "\tmov rax, 3\n";
        } else if ctx.chunks() {
            *code += "\tmov rax, 4\n";
        } else if ctx.zip() {
            *code += "\tmov rax, 6\n";
        } else if ctx.mem_op() {
            *code += "\tmov rax, 7\n";
        } else if ctx.chunk_player_mt_collect_mem() {
            *code += "\tmov rax, 8\n";
        } else if ctx.mem_reads() {
            *code += "\tmov rax, 9\n";
        } else if ctx.chunk_player_mem_reads_collect_main() {
            *code += "\tmov rax, 10\n";
        } else {
            panic!("ZiskRom2Asm::save_to_asm() invalid generation method");
        }
        *code += "\tret\n\n";

        // Externally callable function label
        *code += ".global emulator_start\n";
        *code += "emulator_start:\n";

        Self::push_external_registers(&mut ctx, code);

        if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main() {
            *code += &format!("\n{}\n", ctx.comment_str("ZisK registers initialization"));
            *code += &format!("\txor {}, {} {}\n", REG_A, REG_A, ctx.comment_str("a = 0"));
            *code += &format!("\txor {}, {} {}\n", REG_B, REG_B, ctx.comment_str("b = 0"));
            *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
            *code += &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
            *code += &format!("\txor {}, {} {}\n", REG_PC, REG_PC, ctx.comment_str("pc = 0"));
            *code += &format!("\txor {}, {} {}\n", REG_STEP, REG_STEP, ctx.comment_str("step = 0"));

            // Initialize registers to zero
            *code += &ctx.full_line_comment("Set RISC-V registers to zero".to_string());

            for r in 0u64..35u64 {
                if !XMM_MAPPED_REGS.contains(&r) {
                    *code += &format!("\tmov qword {}[reg_{}], 0\n", ctx.ptr, r);
                }
            }
            for r in 0..16 {
                *code += &format!("\tpxor xmm{r}, xmm{r}\n");
            }
        }

        *code += &format!("\n{}\n", ctx.comment_str("ASM memory initialization"));
        *code += &format!("\tmov {}, 0 {}\n", ctx.mem_end, ctx.comment_str("end = 0"));
        *code += &format!("\tmov {}, 0 {}\n", ctx.mem_error, ctx.comment_str("error = 0"));
        if ctx.fast()
            || ctx.minimal_trace()
            || ctx.rom_histogram()
            || ctx.main_trace()
            || ctx.chunks()
            || ctx.zip()
            || ctx.mem_op()
            || ctx.mem_reads()
        {
            *code += &format!("\tmov {}, 0 {}\n", ctx.mem_step, ctx.comment_str("step = 0"));
            *code += &format!("\tmov {}, 0 {}\n", ctx.mem_sp, ctx.comment_str("sp = 0"));
            if ctx.zip() {
                *code +=
                    &format!("\tmov {}, 0 {}\n", ctx.mem_chunk_id, ctx.comment_str("chunk_id = 0"));
                *code += &format!(
                    "\txor {}, {} {}\n",
                    REG_ACTIVE_CHUNK,
                    REG_ACTIVE_CHUNK,
                    ctx.comment_str("active_chunk = 0")
                );
            }
        }
        if ctx.minimal_trace()
            || ctx.main_trace()
            || ctx.zip()
            || ctx.mem_op()
            || ctx.chunk_player_mt_collect_mem()
            || ctx.mem_reads()
            || ctx.chunk_player_mem_reads_collect_main()
        {
            *code += &format!(
                "\tmov {}, qword {}[trace_address] {}\n",
                REG_VALUE,
                ctx.ptr,
                ctx.comment_str("value = trace address")
            );
            *code += &format!("\tadd {}, 0x20 {}\n", REG_VALUE, ctx.comment_str("value += 32"));
            *code += &format!(
                "\tmov {}, {} {}\n",
                ctx.mem_trace_address,
                REG_VALUE,
                ctx.comment_str("trace_address = value")
            );
            if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main() {
                // Skip number of chunks
                *code += &format!("\tadd {}, 8 {}\n", REG_VALUE, ctx.comment_str("value+=8"));
            }
            *code += &format!(
                "\tmov {}, {} {}\n\n",
                ctx.mem_chunk_address,
                REG_VALUE,
                ctx.comment_str("chunk_address = value = TRACE_ADDR+8")
            );
            *code += &format!(
                "\tmov {}, 0 {}\n",
                ctx.mem_chunk_start_step,
                ctx.comment_str("chunk_start_step = 0")
            );
        }

        *code += &ctx.full_line_comment("fcall_context initialization".to_string());
        *code += &format!(
            "\tlea {}, {} {}\n",
            REG_ADDRESS,
            ctx.fcall_ctx,
            ctx.comment_str("address = fcall context")
        );
        for i in 0..70 {
            if (i == FCALL_PARAMS_CAPACITY) || (i == FCALL_RESULT_CAPACITY) {
                *code += &format!("\tmov qword {}[{} + {}*8], 32\n", ctx.ptr, REG_ADDRESS, i);
            } else {
                *code += &format!("\tmov qword {}[{} + {}*8], 0\n", ctx.ptr, REG_ADDRESS, i);
            }
        }

        if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
            Self::chunk_player_start(&mut ctx, code);
        }

        /********/
        /* LOOP */
        /********/

        // For all program addresses in the vector, create an assembly set of instructions with an
        // instruction label pc_<pc_in_hex>
        for k in 0..rom.sorted_pc_list.len() {
            // Get pc
            ctx.pc = rom.sorted_pc_list[k];

            // Call chunk_start the first time, for the first chunk
            if (ctx.minimal_trace()
                || ctx.main_trace()
                || ctx.zip()
                || ctx.mem_op()
                || ctx.mem_reads())
                && (k == 0)
            {
                // Update pc
                *code += &format!(
                    "\tmov {}, 0x{:08x} {}\n",
                    REG_PC,
                    ctx.pc,
                    ctx.comment_str("value = pc")
                );

                // Reset number of chunks
                *code += &ctx.full_line_comment(
                    "Reset number of chunks to 0 (first position in trace)".to_string(),
                );
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_ADDRESS,
                    ctx.mem_trace_address,
                    ctx.comment_str("address = trace_addr")
                );
                *code += &format!(
                    "\tmov qword {} [{}], 0 {}\n",
                    ctx.ptr,
                    REG_ADDRESS,
                    ctx.comment_str("number of chunks = 0")
                );

                // Call chunk start
                *code += &format!(
                    "\tcall chunk_start {}\n",
                    ctx.comment_str("Call chunk_start the first time")
                );
            }

            ctx.next_pc =
                if (k + 1) < rom.sorted_pc_list.len() { rom.sorted_pc_list[k + 1] } else { M64 };
            let instruction = &rom.insts[&ctx.pc].i;

            // Instruction label
            *code += "\n";
            let mut instruction_comment = instruction.to_text();
            instruction_comment.remove(0);
            *code += &format!("pc_{:x}: {}\n", ctx.pc, ctx.comment(instruction_comment));

            // Self::push_internal_registers(&mut ctx, code, false);
            // *code += &format!("\tmov rdi, {}\n", ctx.pc);
            // *code += &format!("\tmov rsi, {}\n", REG_C);
            // if ctx.chunk_player_mt_collect_mem() {
            //     *code += &format!("\tmov rdx, {}\n", REG_CHUNK_PLAYER_ADDRESS);
            // } else {
            //     *code += &format!("\tmov rdx, {}\n", REG_MEM_READS_SIZE);
            //     *code += &format!("\tshl rdx, 3\n");
            //     *code += &format!("\tadd rdx, {}\n", REG_MEM_READS_ADDRESS);
            // }
            // *code += &format!("\tcall _print_pc\n");
            // Self::pop_internal_registers(&mut ctx, code, false);

            //println!("ZiskRom2Asm::save_to_asm() instruction={}", instruction.to_text());

            // Log instruction pc
            // *s += &format!("\tlea rdi, instruction_format\n");
            // *s += &format!("\tmov rsi, {}\n", ctx.pc);
            // *s += &format!("\tmov rax, 0\n");
            // *s += &format!("\tcall printf\n");

            // *s += "\tmov rax, 1\n";
            // *s += "\tmov rdi, 1\n";
            // *s += &format!("\tlea rsi, pc_{}_log\n", ctx.pc);
            // *s += &format!("\tmov rdx, pc_{}_log_len\n", ctx.pc);
            // *s += "\tsyscall\n\n";

            // Update the rom histogram
            if ctx.rom_histogram() {
                let address = Self::get_rom_histogram_trace_address(rom, ctx.pc);
                *code += &ctx.full_line_comment("rom histogram".to_string());
                *code += &format!("\tmov {REG_ADDRESS}, 0x{address:08x}\n");
                *code += &format!("\tinc qword {}[{}]\n", ctx.ptr, REG_ADDRESS);
            }

            // Set special storage destinations for a and b registers, based on operations, in order
            // to save instructions
            let zisk_op = ZiskOp::try_from_code(instruction.op).unwrap();
            ctx.store_a_in_c = false;
            ctx.store_a_in_a = false;
            ctx.store_b_in_c = false;
            ctx.store_b_in_b = false;
            ctx.address_is_constant = false;

            // Store opcode in main trace
            if ctx.chunk_player_mem_reads_collect_main() {
                *code += &ctx.full_line_comment("Main[0] = op".to_string());
                // Copy read data into mem_reads_address
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_VALUE,
                    instruction.op,
                    ctx.comment_str("value = op")
                );
                *code += &format!(
                    "\tmov [{} + {}*8], {} {}\n",
                    REG_MEM_READS_ADDRESS,
                    REG_MEM_READS_SIZE,
                    REG_VALUE,
                    ctx.comment_str("mem_reads[@+size*8] = op")
                );
            }

            match zisk_op {
                ZiskOp::CopyB
                | ZiskOp::PubOut
                | ZiskOp::FcallParam
                | ZiskOp::Fcall
                | ZiskOp::FcallGet => ctx.store_b_in_c = true,
                ZiskOp::Xor
                | ZiskOp::And
                | ZiskOp::Or
                | ZiskOp::Sll
                | ZiskOp::Srl
                | ZiskOp::Sra
                | ZiskOp::Sub
                | ZiskOp::Min
                | ZiskOp::Minu
                | ZiskOp::Max
                | ZiskOp::Maxu => ctx.store_a_in_c = true,
                ZiskOp::MinuW | ZiskOp::MinW | ZiskOp::MaxuW | ZiskOp::MaxW => {
                    ctx.store_a_in_c = true;
                    ctx.store_b_in_b = true;
                }
                ZiskOp::SignExtendB | ZiskOp::SignExtendH | ZiskOp::SignExtendW | ZiskOp::AddW => {
                    ctx.store_b_in_b = true
                }
                ZiskOp::SubW
                | ZiskOp::Eq
                | ZiskOp::Ltu
                | ZiskOp::Lt
                | ZiskOp::LtuW
                | ZiskOp::LtW
                | ZiskOp::Leu
                | ZiskOp::Le
                | ZiskOp::LeuW
                | ZiskOp::LeW => ctx.store_a_in_a = true,
                ZiskOp::Mulu
                | ZiskOp::Muluh
                | ZiskOp::Mulsuh
                | ZiskOp::Mul
                | ZiskOp::Mulh
                | ZiskOp::MulW
                | ZiskOp::Div
                | ZiskOp::Rem
                | ZiskOp::DivuW
                | ZiskOp::RemuW
                | ZiskOp::DivW
                | ZiskOp::RemW => {
                    ctx.store_a_in_a = true;
                    ctx.store_b_in_b = true;
                }
                ZiskOp::Divu | ZiskOp::Remu => {
                    ctx.store_b_in_b = true;
                }
                ZiskOp::Add => {
                    if (instruction.a_src == SRC_IMM)
                        && (instruction.a_offset_imm0 == 0)
                        && (instruction.a_use_sp_imm1 == 0)
                    {
                        ctx.store_b_in_c = true;
                    } else {
                        ctx.store_a_in_c = true;
                    }
                }
                _ => {}
            };

            // Make sure we don't store two registers in the same register
            assert!(!(ctx.store_a_in_c && ctx.store_b_in_c));
            assert!(!(ctx.store_a_in_c && ctx.store_a_in_a));
            assert!(!(ctx.store_b_in_c && ctx.store_b_in_b));

            // Set register b content: only SRC_C
            // This is required because in case a must be stored in c, it would overwrite the
            // previouse value of c
            ctx.b.is_constant = false;
            ctx.b.is_saved = false;
            ctx.b.string_value = REG_B.to_string();
            if instruction.b_src == SRC_C {
                *code += &ctx.full_line_comment("b=SRC_C".to_string());
                if ctx.store_b_in_c {
                    // No need to copy c to b, since we need b to be stored in c
                    ctx.b.is_saved = false;
                } else {
                    *code += &format!("\tmov {}, {} {}\n", REG_B, REG_C, ctx.comment_str("b = c"));
                    ctx.b.is_saved = true;
                }
                if ctx.main_trace() {
                    Self::clear_reg_step_ranges(&mut ctx, code, 1);
                }
            }

            /************/
            /* A SOURCE */
            /************/

            // Set register a content based on instruction a_src
            ctx.a.is_constant = false;
            ctx.a.is_saved = false;
            ctx.a.string_value = REG_A.to_string();
            match instruction.a_src {
                SRC_C => {
                    *code += &ctx.full_line_comment("a=SRC_C".to_string());
                    if ctx.store_a_in_c {
                        // No need to copy c to a, since we need a to be stored in c
                        ctx.a.is_saved = false;
                    } else {
                        *code +=
                            &format!("\tmov {}, {} {}\n", REG_A, REG_C, ctx.comment_str("a = c"));
                        ctx.a.is_saved = true;
                    }
                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }
                }
                SRC_REG => {
                    *code += &ctx
                        .full_line_comment(format!("a=SRC_REG reg={}", instruction.a_offset_imm0));

                    assert!(instruction.a_offset_imm0 <= 34);

                    // Read from memory and store in the proper register: a or c
                    let dest_reg = if ctx.store_a_in_c { REG_C } else { REG_A };
                    let dest_desc = if ctx.store_a_in_c { "c" } else { "a" };
                    Self::read_riscv_reg(
                        &mut ctx,
                        code,
                        instruction.a_offset_imm0,
                        dest_reg,
                        dest_desc,
                    );

                    if ctx.main_trace() {
                        Self::trace_reg_access(&mut ctx, code, instruction.a_offset_imm0, 0);
                    }
                }
                SRC_MEM => {
                    *code += &ctx.full_line_comment("a=SRC_MEM".to_string());

                    // Calculate memory address
                    if !ctx.chunk_player_mem_reads_collect_main() {
                        *code += &format!(
                            "\tmov {}, 0x{:x} {}\n",
                            REG_ADDRESS,
                            instruction.a_offset_imm0,
                            ctx.comment_str("address = a_offset_imm0")
                        );
                        if instruction.a_use_sp_imm1 != 0 {
                            *code += &format!(
                                "\tadd {}, {} {}\n",
                                REG_ADDRESS,
                                ctx.mem_sp,
                                ctx.comment_str("address += sp")
                            );
                            ctx.address_is_constant = false;
                        } else {
                            ctx.address_is_constant = true;
                            ctx.address_constant_value = instruction.a_offset_imm0;
                        }
                    }

                    // Read value from memory and store in the proper register: a or c
                    if !ctx.chunk_player_mt_collect_mem()
                        && !ctx.chunk_player_mem_reads_collect_main()
                    {
                        *code += &format!(
                            "\tmov {}, [{}] {}\n",
                            if ctx.store_a_in_c { REG_C } else { REG_A },
                            REG_ADDRESS,
                            ctx.comment(format!(
                                "{} = mem[address]",
                                if ctx.store_a_in_c { "c" } else { "a" }
                            ))
                        );
                    }

                    // Generate mem reads
                    if ctx.minimal_trace() || ctx.zip() {
                        // If zip, check if chunk is active
                        if ctx.zip() {
                            *code += &format!(
                                "\ttest {}, 1 {}\n",
                                REG_ACTIVE_CHUNK,
                                ctx.comment_str("active_chunk == 1 ?")
                            );
                            *code += &format!("\tjnz pc_{:x}_a_active_chunk\n", ctx.pc);
                            *code += &format!("\tjmp pc_{:x}_a_address_check_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_a_active_chunk:\n", ctx.pc);
                        }
                        // If address is constant
                        if instruction.a_use_sp_imm1 == 0 {
                            // If address is constant and aligned
                            if (instruction.a_offset_imm0 & 0x7) == 0 {
                                Self::a_src_mem_aligned(&mut ctx, code);
                            } else {
                                Self::a_src_mem_not_aligned(&mut ctx, code);
                            }
                        }
                        // If address is dynamic
                        else {
                            // Check if address is aligned, i.e. it is a multiple of 8, or not,
                            // and insert code accordingly
                            *code += &format!(
                                "\ttest {}, 0x7 {}\n",
                                REG_ADDRESS,
                                ctx.comment_str("address &= 7")
                            );
                            *code += &format!("\tjnz pc_{:x}_a_address_not_aligned\n", ctx.pc);
                            Self::a_src_mem_aligned(&mut ctx, code);
                            unusual_code += &format!("pc_{:x}_a_address_not_aligned:\n", ctx.pc);
                            Self::a_src_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code +=
                                &format!("\tjmp pc_{:x}_a_address_check_done\n", ctx.pc);
                        }
                        *code += &format!("pc_{:x}_a_address_check_done:\n", ctx.pc);
                    }
                    if ctx.mem_reads() {
                        Self::a_src_mem_aligned(&mut ctx, code);
                    }

                    // Consume mem reads
                    if ctx.chunk_player_mt_collect_mem() {
                        let reg_to_store_value = if ctx.store_a_in_c { REG_C } else { REG_A };
                        let reg_to_store_value_desc = if ctx.store_a_in_c { "c" } else { "a" };
                        // If address is constant
                        if instruction.a_use_sp_imm1 == 0 {
                            // If address is constant and aligned
                            if (instruction.a_offset_imm0 & 0x7) == 0 {
                                Self::chunk_player_mem_read_aligned(
                                    &mut ctx,
                                    code,
                                    reg_to_store_value,
                                    reg_to_store_value_desc,
                                    0,
                                );
                            } else {
                                Self::chunk_player_a_src_mem_not_aligned(&mut ctx, code);
                            }
                        }
                        // If address is dynamic
                        else {
                            // Check if address is aligned, i.e. it is a multiple of 8, or not,
                            // and insert code accordingly
                            *code += &format!(
                                "\ttest {}, 0x7 {}\n",
                                REG_ADDRESS,
                                ctx.comment_str("address &= 7")
                            );
                            *code += &format!("\tjnz pc_{:x}_a_address_not_aligned\n", ctx.pc);
                            Self::chunk_player_mem_read_aligned(
                                &mut ctx,
                                code,
                                reg_to_store_value,
                                reg_to_store_value_desc,
                                0,
                            );
                            unusual_code += &format!("pc_{:x}_a_address_not_aligned:\n", ctx.pc);
                            Self::chunk_player_a_src_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code +=
                                &format!("\tjmp pc_{:x}_a_address_check_done\n", ctx.pc);
                        }
                        *code += &format!("pc_{:x}_a_address_check_done:\n", ctx.pc);
                    }
                    if ctx.chunk_player_mem_reads_collect_main() {
                        // Read value from mem reads and store in the proper register: a or c
                        *code += &format!(
                            "\tmov {}, [{}] {}\n",
                            if ctx.store_a_in_c { REG_C } else { REG_A },
                            REG_CHUNK_PLAYER_ADDRESS,
                            ctx.comment(format!(
                                "{} = mem_reads[address]",
                                if ctx.store_a_in_c { "c" } else { "a" }
                            ))
                        );

                        // Increment chunk player address
                        *code += &format!(
                            "\tadd {}, 8 {}\n",
                            REG_CHUNK_PLAYER_ADDRESS,
                            ctx.comment_str("chunk_address += 8")
                        );
                    }

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }

                    if ctx.mem_op() {
                        Self::a_src_mem_op(&mut ctx, code);
                    }

                    ctx.a.is_saved = true;
                }
                SRC_IMM => {
                    *code += &ctx.full_line_comment("a=SRC_IMM".to_string());
                    ctx.a.is_constant = true;
                    ctx.a.constant_value =
                        instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32);
                    ctx.a.string_value = format!("0x{:x}", ctx.a.constant_value);
                    if ctx.store_a_in_c {
                        *code += &format!(
                            "\tmov {}, {} {}\n",
                            REG_C,
                            ctx.a.string_value,
                            ctx.comment_str("c = constant")
                        );
                        ctx.a.is_saved = false;
                    } else if ctx.store_a_in_a {
                        *code += &format!(
                            "\tmov {}, {} {}\n",
                            REG_A,
                            ctx.a.string_value,
                            ctx.comment_str("a = constant")
                        );
                        ctx.a.is_saved = true;
                    } else {
                        ctx.a.is_saved = false;
                    }
                    // DEBUG: Used only to get register traces:
                    //*s += &format!("\tmov {}, {} {}\n", REG_A, ctx.a.string_value, ctx.commit_str("a = a_value"));

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }
                }
                SRC_STEP => {
                    *code += &ctx.full_line_comment("a=SRC_STEP".to_string());
                    let store_a_reg = if ctx.store_a_in_c { REG_C } else { REG_A };
                    let store_a_reg_name = if ctx.store_a_in_c { "c" } else { "a" };
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        store_a_reg,
                        ctx.mem_step,
                        ctx.comment(format!("{store_a_reg_name} = step"))
                    );
                    if ctx.minimal_trace()
                        || ctx.zip()
                        || ctx.chunk_player_mt_collect_mem()
                        || ctx.mem_reads()
                        || ctx.chunk_player_mem_reads_collect_main()
                    {
                        *code += &format!(
                            "\tadd {}, chunk_size {}\n",
                            store_a_reg,
                            ctx.comment(format!("{store_a_reg_name} += chunk_size"))
                        );
                        *code += &format!(
                            "\tsub {}, {} {}\n",
                            store_a_reg,
                            REG_STEP,
                            ctx.comment(format!("{store_a_reg_name} -= step_count_down"))
                        );
                    }
                    ctx.a.is_saved = !ctx.store_a_in_c;

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }
                }
                _ => {
                    panic!("ZiskRom::source_a() Invalid a_src={} pc={}", instruction.a_src, ctx.pc)
                }
            }

            // Copy a value to main trace
            if ctx.main_trace() {
                *code += &ctx.full_line_comment("Main[1]=a".to_string());
                if ctx.store_a_in_c {
                    *code += &format!(
                        "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 1*8], {REG_C}\n"
                    );
                } else if ctx.a.is_constant && !ctx.store_a_in_a {
                    *code += &format!("\tmov {}, 0x{:x}\n", REG_A, ctx.a.constant_value);
                    *code += &format!(
                        "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 1*8], {REG_A}\n"
                    );
                } else {
                    *code += &format!(
                        "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 1*8], {REG_A}\n"
                    );
                }
            }
            if ctx.chunk_player_mem_reads_collect_main() {
                *code += &ctx.full_line_comment("Main[1] = a".to_string());
                if ctx.store_a_in_c {
                    *code += &format!(
                        "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 1*8], {REG_C}\n"
                    );
                } else if ctx.a.is_constant && !ctx.store_a_in_a {
                    *code += &format!("\tmov {}, 0x{:x}\n", REG_A, ctx.a.constant_value);
                    *code += &format!(
                        "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 1*8], {REG_A}\n"
                    );
                } else {
                    *code += &format!(
                        "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 1*8], {REG_A}\n"
                    );
                }
            }

            // Copy rom_index<<32 + addr1 to main trace
            // where addr1 = b_offset_imm0 + REG_A(if b=SRC_IND)
            if ctx.main_trace() {
                *code += &ctx.full_line_comment("Main[0]=rom_index<<32+addr1".to_string());
                let rom_index = instruction.sorted_pc_list_index as u64;
                assert!(rom_index <= 0xffffffff);
                // if instruction.b_offset_imm0 > 0xffffffff {
                //     println!("instruction.b_offset_imm0={}", instruction.b_offset_imm0);
                // }
                // assert!(instruction.b_offset_imm0 <= 0xffffffff);
                if (instruction.b_src != SRC_IND) || ctx.a.is_constant {
                    // In this case the value to store is constant
                    let addr1 = (instruction.b_offset_imm0 as i64
                        + if instruction.b_src == SRC_IND {
                            ctx.a.constant_value as i64
                        } else {
                            0
                        }) as u64;
                    assert!(addr1 <= 0xffffffff);
                    let value = (rom_index << 32) + addr1;
                    *code += &format!("\tmov {REG_VALUE}, 0x{value:x}\n");
                } else {
                    // In this case the value to store is not constant
                    assert!(instruction.b_src == SRC_IND);
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_VALUE,
                        if ctx.store_a_in_c { REG_C } else { REG_A },
                        ctx.comment_str("value = a")
                    );
                    if instruction.b_offset_imm0 as i64 >= 0 {
                        *code += &format!(
                            "\tmov {}, 0x{:x} {}\n",
                            REG_AUX,
                            instruction.b_offset_imm0 + ((rom_index & 0xffffffff) << 32),
                            ctx.comment_str("aux = rom_index<<32 + b_offset_imm0")
                        );
                        *code += &format!(
                            "\tadd {}, {} {}\n",
                            REG_VALUE,
                            REG_AUX,
                            ctx.comment_str("value += aux")
                        );
                    } else {
                        *code += &format!(
                            "\tmov {}, 0x{:x} {}\n",
                            REG_AUX,
                            -(instruction.b_offset_imm0 as i64),
                            ctx.comment_str("aux = -b_offset_imm0")
                        );
                        *code += &format!(
                            "\tsub {}, {} {}\n",
                            REG_VALUE,
                            REG_AUX,
                            ctx.comment_str("value = -b_offset_imm0")
                        );
                        *code += &format!(
                            "\tmov {}, 0x{:x} {}\n",
                            REG_AUX,
                            (rom_index & 0xffffffff) << 32,
                            ctx.comment_str("aux += rom_index<<32")
                        );
                        *code += &format!(
                            "\tadd {}, {} {}\n",
                            REG_VALUE,
                            REG_AUX,
                            ctx.comment_str("value += aux = rom_index<<32 + b_offset_imm0")
                        );
                    }
                }
                *code += &format!(
                    "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 0*8], {REG_VALUE}\n"
                );
            }

            /************/
            /* B SOURCE */
            /************/

            // Set register b content: all except SRC_C
            match instruction.b_src {
                SRC_C => {}
                SRC_REG => {
                    *code += &ctx
                        .full_line_comment(format!("b=SRC_REG reg={}", instruction.b_offset_imm0));

                    assert!(instruction.b_offset_imm0 <= 34);

                    // Read from memory and store in the proper register: b or c
                    let dest_reg = if ctx.store_b_in_c { REG_C } else { REG_B };
                    let dest_desc = if ctx.store_b_in_c { "c" } else { "b" };
                    Self::read_riscv_reg(
                        &mut ctx,
                        code,
                        instruction.b_offset_imm0,
                        dest_reg,
                        dest_desc,
                    );

                    if ctx.main_trace() {
                        Self::trace_reg_access(&mut ctx, code, instruction.b_offset_imm0, 1);
                    }
                }
                SRC_MEM => {
                    *code += &ctx.full_line_comment("b=SRC_MEM".to_string());

                    if !ctx.chunk_player_mem_reads_collect_main() {
                        // Calculate memory address
                        *code += &format!(
                            "\tmov {}, 0x{:x} {}\n",
                            REG_ADDRESS,
                            instruction.b_offset_imm0,
                            ctx.comment_str("address = b_offset_imm0")
                        );
                        if instruction.b_use_sp_imm1 != 0 {
                            *code += &format!(
                                "\tadd {}, {} {}\n",
                                REG_ADDRESS,
                                ctx.mem_sp,
                                ctx.comment_str("address += sp")
                            );
                            ctx.address_is_constant = false;
                        } else {
                            ctx.address_is_constant = true;
                            ctx.address_constant_value = instruction.b_offset_imm0;
                        }
                    }

                    // Read value from memory and store in the proper register: b or c
                    if !ctx.chunk_player_mt_collect_mem()
                        && !ctx.chunk_player_mem_reads_collect_main()
                    {
                        *code += &format!(
                            "\tmov {}, [{}] {}\n",
                            if ctx.store_b_in_c { REG_C } else { REG_B },
                            REG_ADDRESS,
                            ctx.comment(format!(
                                "{} = mem[address]",
                                if ctx.store_b_in_c { "c" } else { "b" }
                            ))
                        );
                    }

                    // Generate mem reads
                    if ctx.minimal_trace() || ctx.zip() {
                        // If zip, check if chunk is active
                        if ctx.zip() {
                            *code += &format!(
                                "\ttest {}, 1 {}\n",
                                REG_ACTIVE_CHUNK,
                                ctx.comment_str("active_chunk == 1 ?")
                            );
                            *code += &format!("\tjnz pc_{:x}_b_active_chunk\n", ctx.pc);
                            *code += &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_b_active_chunk:\n", ctx.pc);
                        }
                        // If address is constant
                        if instruction.b_use_sp_imm1 == 0 {
                            // If address is constant and aligned
                            if (instruction.b_offset_imm0 & 0x7) == 0 {
                                Self::b_src_mem_aligned(&mut ctx, code);
                            } else {
                                Self::b_src_mem_not_aligned(&mut ctx, code);
                            }
                        }
                        // If address is dynamic
                        else {
                            // Check if address is aligned, i.e. it is a multiple of 8
                            *code += &format!(
                                "\ttest {}, 0x7 {}\n",
                                REG_ADDRESS,
                                ctx.comment_str("address &= 7")
                            );
                            *code += &format!("\tjnz pc_{:x}_b_address_not_aligned\n", ctx.pc);
                            Self::b_src_mem_aligned(&mut ctx, code);
                            unusual_code += &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);
                            Self::b_src_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code +=
                                &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);
                        }
                        *code += &format!("pc_{:x}_b_address_check_done:\n", ctx.pc);
                    }
                    if ctx.mem_reads() {
                        Self::b_src_mem_aligned(&mut ctx, code);
                    }

                    // Consume mem reads
                    if ctx.chunk_player_mt_collect_mem() {
                        let reg_to_store_value = if ctx.store_b_in_c { REG_C } else { REG_B };
                        let reg_to_store_value_desc = if ctx.store_b_in_c { "c" } else { "b" };

                        // If address is constant
                        if instruction.b_use_sp_imm1 == 0 {
                            // If address is constant and aligned
                            if (instruction.b_offset_imm0 & 0x7) == 0 {
                                Self::chunk_player_mem_read_aligned(
                                    &mut ctx,
                                    code,
                                    reg_to_store_value,
                                    reg_to_store_value_desc,
                                    1,
                                );
                            } else {
                                Self::chunk_player_b_src_mem_not_aligned(&mut ctx, code);
                            }
                        }
                        // If address is dynamic
                        else {
                            // Check if address is aligned, i.e. it is a multiple of 8, or not,
                            // and insert code accordingly
                            *code += &format!(
                                "\ttest {}, 0x7 {}\n",
                                REG_ADDRESS,
                                ctx.comment_str("address &= 7")
                            );
                            *code += &format!("\tjnz pc_{:x}_b_address_not_aligned\n", ctx.pc);
                            Self::chunk_player_mem_read_aligned(
                                &mut ctx,
                                code,
                                reg_to_store_value,
                                reg_to_store_value_desc,
                                1,
                            );
                            unusual_code += &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);
                            Self::chunk_player_b_src_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code +=
                                &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);
                        }
                        *code += &format!("pc_{:x}_b_address_check_done:\n", ctx.pc);
                    }
                    if ctx.chunk_player_mem_reads_collect_main() {
                        // Read value from mem reads and store in the proper register: b or c
                        *code += &format!(
                            "\tmov {}, [{}] {}\n",
                            if ctx.store_b_in_c { REG_C } else { REG_B },
                            REG_CHUNK_PLAYER_ADDRESS,
                            ctx.comment(format!(
                                "{} = mem_reads[address]",
                                if ctx.store_a_in_c { "c" } else { "b" }
                            ))
                        );

                        // Increment chunk player address
                        *code += &format!(
                            "\tadd {}, 8 {}\n",
                            REG_CHUNK_PLAYER_ADDRESS,
                            ctx.comment_str("chunk_address += 8")
                        );
                    }

                    ctx.b.is_saved = !ctx.store_b_in_c;

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 1);
                    }

                    if ctx.mem_op() {
                        Self::b_src_mem_op(&mut ctx, code);
                    }
                }
                SRC_IMM => {
                    *code += &ctx.full_line_comment("b=SRC_IMM".to_string());
                    ctx.b.is_constant = true;
                    ctx.b.constant_value =
                        instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32);
                    ctx.b.string_value = format!("0x{:x}", ctx.b.constant_value);
                    if ctx.store_b_in_c {
                        *code += &format!(
                            "\tmov {}, {} {}\n",
                            REG_C,
                            ctx.b.string_value,
                            ctx.comment_str("c = constant")
                        );
                        ctx.b.is_saved = false;
                    } else if ctx.store_b_in_b {
                        *code += &format!(
                            "\tmov {}, {} {}\n",
                            REG_B,
                            ctx.b.string_value,
                            ctx.comment_str("b = constant")
                        );
                        ctx.b.is_saved = true;
                    } else {
                        ctx.b.is_saved = false;
                    }
                    // DEBUG: Used only to get register traces:
                    //*s += &format!("\tmov {}, {} {}\n", REG_B, ctx.b.string_value, ctx.commit_str("b = b_value"));

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 1);
                    }
                }
                SRC_IND => {
                    *code += &ctx
                        .full_line_comment(format!("b=SRC_IND width={}", instruction.ind_width));

                    // Make sure register a is stored in REG_A
                    // However, since b's source is an indirection, a's source is normally a register
                    if ctx.a.is_constant && !ctx.a.is_saved {
                        *code += &format!("\tmov {}, {}\n", REG_A, ctx.a.string_value);
                        ctx.a.is_saved = true;
                    }

                    // Use REG_A if a's value is not needed beyond the b indirection, in which case
                    // we can overwirte it to build the address to read from the b value,
                    // or REG_ADDRESS otherwise to preserve the value of a
                    let mut reg_address: &str = REG_A;
                    if instruction.op == ZiskOp::CopyB.code()
                        || instruction.op == ZiskOp::SignExtendB.code()
                        || instruction.op == ZiskOp::SignExtendH.code()
                        || instruction.op == ZiskOp::SignExtendH.code()
                    {
                    } else {
                        *code += &format!(
                            "\tmov {}, {} {}\n",
                            REG_ADDRESS,
                            ctx.a.string_value,
                            ctx.comment_str("address = a")
                        );
                        reg_address = REG_ADDRESS;
                    }

                    // Calculate memory address
                    if !ctx.chunk_player_mem_reads_collect_main() {
                        if instruction.b_offset_imm0 != 0 {
                            *code += &format!(
                                "\tadd {}, 0x{:x} {}\n",
                                reg_address,
                                instruction.b_offset_imm0,
                                ctx.comment_str("address += b_offset_imm0")
                            );
                        }
                        if instruction.b_use_sp_imm1 != 0 {
                            *code += &format!(
                                "\tadd {}, {} {}\n",
                                reg_address,
                                ctx.mem_sp,
                                ctx.comment_str("address += sp")
                            );
                        }
                    }

                    // Read from memory and store in the proper register: b or c
                    if !ctx.chunk_player_mt_collect_mem()
                        && !ctx.chunk_player_mem_reads_collect_main()
                    {
                        match instruction.ind_width {
                            8 => {
                                // Read 8-bytes value from address
                                *code += &format!(
                                    "\tmov {}, qword {}[{}] {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    ctx.ptr,
                                    reg_address,
                                    ctx.comment(format!(
                                        "{} = mem[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );
                            }
                            4 => {
                                // Read 4-bytes value from address
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    if ctx.store_b_in_c { REG_C_W } else { REG_B_W },
                                    reg_address,
                                    ctx.comment(format!(
                                        "{} = mem[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );
                            }
                            2 => {
                                // Read 2-bytes value from address
                                *code += &format!(
                                    "\tmovzx {}, word {}[{}] {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    ctx.ptr,
                                    reg_address,
                                    ctx.comment(format!(
                                        "{} = mem[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );
                            }
                            1 => {
                                // Read 1-bytes value from address
                                *code += &format!(
                                    "\tmovzx {}, byte {}[{}] {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    ctx.ptr,
                                    reg_address,
                                    ctx.comment(format!(
                                        "{} = mem[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                    }

                    // Generate mem reads
                    if ctx.minimal_trace() || ctx.zip() {
                        // If zip, check if chunk is active
                        if ctx.zip() {
                            *code += &format!(
                                "\ttest {}, 1 {}\n",
                                REG_ACTIVE_CHUNK,
                                ctx.comment_str("active_chunk == 1 ?")
                            );
                            *code += &format!("\tjnz pc_{:x}_b_active_chunk\n", ctx.pc);
                            *code += &format!("\tjmp pc_{:x}_b_ind_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_b_active_chunk:\n", ctx.pc);
                        }
                        match instruction.ind_width {
                            8 => {
                                // // Check if address is aligned, i.e. it is a multiple of 8
                                *code += &format!(
                                    "\ttest {}, 0x7 {}\n",
                                    reg_address,
                                    ctx.comment_str("address &= 7")
                                );
                                *code += &format!("\tjnz pc_{:x}_b_address_not_aligned\n", ctx.pc);

                                // b register memory address is fully alligned

                                // Copy read data into mem_reads_address and increment it
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    ctx.comment_str("mem_reads[@+size*8] = b")
                                );

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size++")
                                );

                                // b memory address is not aligned

                                unusual_code +=
                                    &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);

                                // Calculate previous aligned address
                                unusual_code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = previous aligned address")
                                );

                                // Store previous aligned address value in mem_reads, and advance address
                                unusual_code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = mem[prev_address]")
                                );
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8] = prev_b")
                                );

                                // Calculate next aligned address
                                unusual_code += &format!(
                                    "\tadd {}, 8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = next aligned address")
                                );

                                // Store next aligned address value in mem_reads, and advance it
                                unusual_code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = mem[next_address]")
                                );
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8 + 8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8+8] = next_b")
                                );

                                // Increment chunk.steps.mem_reads_size twice
                                unusual_code += &format!(
                                    "\tadd {}, 2 {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size += 2")
                                );

                                // Jump to check done
                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);

                                // Check done
                                *code += &format!("pc_{:x}_b_address_check_done:\n", ctx.pc);
                            }
                            4 | 2 => {
                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = previous aligned address")
                                );

                                // Store previous aligned address value in mem_reads, advancing address
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = mem[prev_address]")
                                );
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8] = prev_b")
                                );

                                // Calculate next aligned address, keeping a copy of previous aligned
                                // address in value
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = copy of prev_address")
                                );
                                let address_increment = instruction.ind_width - 1;
                                *code += &format!(
                                    "\tadd {}, {} {}\n",
                                    reg_address,
                                    address_increment,
                                    ctx.comment(format!("address += {address_increment}"))
                                );
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = next aligned address")
                                );
                                *code += &format!(
                                    "\tcmp {}, {} {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("prev_address = next_address ?")
                                );
                                *code +=
                                    &format!("\tjnz pc_{:x}_b_ind_different_address\n", ctx.pc);

                                // Same address
                                ///////////////

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size++")
                                );

                                // Different address
                                ////////////////////

                                unusual_code +=
                                    &format!("pc_{:x}_b_ind_different_address:\n", ctx.pc);

                                // Store next aligned address value in mem_reads
                                unusual_code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = mem[next_address]")
                                );

                                // Copy read data into mem_reads_address and advance it
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8 + 8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8+8] = next_b")
                                );

                                // Increment chunk.steps.mem_reads_size
                                unusual_code += &format!(
                                    "\tadd {}, 2 {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size += 2")
                                );

                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_b_ind_address_done\n", ctx.pc);

                                // Done
                                ///////

                                *code += &format!("pc_{:x}_b_ind_address_done:\n", ctx.pc);
                            }
                            1 => {
                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = previous aligned address")
                                );

                                // Store previous aligned address value in mem_reads, and increment address
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = mem[prev_address")
                                );
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8] = prev_b")
                                );

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size++")
                                );
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                        if ctx.zip() {
                            *code += &format!("pc_{:x}_b_ind_done:\n", ctx.pc);
                        }
                    }
                    if ctx.mem_reads() {
                        Self::b_src_mem_aligned(&mut ctx, code);
                    }
                    ctx.b.is_saved = !ctx.store_b_in_c;

                    // Consume mem reads
                    if ctx.chunk_player_mt_collect_mem() {
                        match instruction.ind_width {
                            8 => {
                                // // Check if address is aligned, i.e. it is a multiple of 8
                                *code += &format!(
                                    "\ttest {}, 0x7 {}\n",
                                    reg_address,
                                    ctx.comment_str("address &= 7")
                                );
                                *code += &format!("\tjnz pc_{:x}_b_address_not_aligned\n", ctx.pc);

                                // b memory address is alligned
                                ///////////////////////////////

                                // Read value from memory and store in the proper register: a or c
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment(format!(
                                        "{} = mt[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );

                                // Increment chunk player address
                                *code += &format!(
                                    "\tadd {}, 8 {}\n",
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("chunk_address += 8")
                                );

                                // b memory address is not aligned
                                //////////////////////////////////

                                unusual_code +=
                                    &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);

                                Self::chunk_player_b_src_mem_not_aligned(
                                    &mut ctx,
                                    &mut unusual_code,
                                );

                                // Jump to check done
                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);

                                // Check done
                                *code += &format!("pc_{:x}_b_address_check_done:\n", ctx.pc);
                            }
                            4 | 2 => {
                                // Copy address into aux
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_AUX,
                                    reg_address,
                                    ctx.comment_str("aux = address")
                                );

                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = previous aligned address")
                                );

                                // Calculate next aligned address, keeping a copy of previous aligned
                                // address in value
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("value = copy of prev_address")
                                );
                                let address_increment = instruction.ind_width - 1;
                                *code += &format!(
                                    "\tadd {}, {} {}\n",
                                    reg_address,
                                    address_increment,
                                    ctx.comment(format!("address += {address_increment}"))
                                );
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    reg_address,
                                    ctx.comment_str("address = next aligned address")
                                );
                                *code += &format!(
                                    "\tcmp {}, {} {}\n",
                                    REG_VALUE,
                                    reg_address,
                                    ctx.comment_str("prev_address = next_address ?")
                                );
                                *code +=
                                    &format!("\tjnz pc_{:x}_b_ind_different_address\n", ctx.pc);

                                // Same address
                                ///////////////

                                // Read value from memory and store in the proper register: a or c
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("value = mt[address]")
                                );

                                // Increment chunk player address
                                *code += &format!(
                                    "\tadd {}, 8 {}\n",
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("chunk_address += 8")
                                );

                                // Get bits to shift
                                *code += &format!(
                                    "\tmov rcx, {} {}\n",
                                    REG_AUX,
                                    ctx.comment_str("rcx = aux = address")
                                );
                                *code += &format!(
                                    "\tand rcx, 0x7 {}\n",
                                    ctx.comment_str("rcx = misaligned bytes")
                                );
                                *code += &format!(
                                    "\tshl rcx, 0x3 {}\n",
                                    ctx.comment_str("rcx = misaligned bits")
                                );

                                // Shif it right the number of address misaligned bits
                                *code += &format!(
                                    "\tshr {}, cl {}\n",
                                    REG_VALUE,
                                    ctx.comment_str("value >> bits")
                                );

                                // Copy into destination register
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    REG_VALUE,
                                    ctx.comment(format!(
                                        "{} = mt[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );

                                // Take the lowest bytes
                                if instruction.ind_width == 2 {
                                    *code += &format!(
                                        "\tand {}, {} {}\n",
                                        if ctx.store_b_in_c { REG_C } else { REG_B },
                                        "0xFFFF",
                                        ctx.comment_str("take lowest bytes")
                                    );
                                } else {
                                    *code += &format!(
                                        "\tmov {}, 0xFFFFFFFF {}\n",
                                        REG_AUX,
                                        ctx.comment_str("aux = 4-B mask")
                                    );
                                    *code += &format!(
                                        "\tand {}, {} {}\n",
                                        if ctx.store_b_in_c { REG_C } else { REG_B },
                                        REG_AUX,
                                        ctx.comment_str("take lowest bytes")
                                    );
                                }

                                // Different address
                                ////////////////////

                                unusual_code +=
                                    &format!("pc_{:x}_b_ind_different_address:\n", ctx.pc);

                                // Get bits to shift
                                unusual_code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_ADDRESS,
                                    REG_AUX,
                                    ctx.comment_str("address = aux")
                                );

                                Self::chunk_player_b_src_mem_not_aligned(
                                    &mut ctx,
                                    &mut unusual_code,
                                );

                                // Take the lowest bytes
                                if instruction.ind_width == 2 {
                                    unusual_code += &format!(
                                        "\tand {}, {} {}\n",
                                        if ctx.store_b_in_c { REG_C } else { REG_B },
                                        "0xFFFF",
                                        ctx.comment_str("take lowest bytes")
                                    );
                                } else {
                                    unusual_code += &format!(
                                        "\tmov {}, 0xFFFFFFFF {}\n",
                                        REG_AUX,
                                        ctx.comment_str("aux = 4-B mask")
                                    );
                                    unusual_code += &format!(
                                        "\tand {}, {} {}\n",
                                        if ctx.store_b_in_c { REG_C } else { REG_B },
                                        REG_AUX,
                                        ctx.comment_str("take lowest bytes")
                                    );
                                }

                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_b_ind_address_done\n", ctx.pc);

                                // Done
                                *code += &format!("pc_{:x}_b_ind_address_done:\n", ctx.pc);
                            }
                            1 => {
                                // // Check if address is aligned, i.e. it is a multiple of 8
                                *code += &format!(
                                    "\tand {}, 0x7 {}\n",
                                    reg_address,
                                    ctx.comment_str("address &= 7")
                                );
                                *code += &format!("\tjnz pc_{:x}_b_address_not_aligned\n", ctx.pc);

                                // b register memory address is alligned
                                ////////////////////////////////////////

                                // Read value from memory and store in the proper register: a or c
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment(format!(
                                        "{} = mt[address]",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );

                                // Increment chunk player address
                                *code += &format!(
                                    "\tadd {}, 8 {}\n",
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("chunk_address += 8")
                                );

                                // Take the lowest byte
                                *code += &format!(
                                    "\tand {}, 0xFF {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    ctx.comment(format!(
                                        "{} &= 0xF",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );

                                // b memory address is not aligned
                                //////////////////////////////////

                                unusual_code +=
                                    &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);

                                // Read value from memory
                                unusual_code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("value = mt[address]")
                                );

                                // Increment chunk player address
                                unusual_code += &format!(
                                    "\tadd {}, 8 {}\n",
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("chunk_address += 8")
                                );

                                // Shift right
                                unusual_code += &format!(
                                    "\tmov rcx, {} {}\n",
                                    reg_address,
                                    ctx.comment_str("rcx = address")
                                );
                                unusual_code +=
                                    &format!("\tshl rcx, 3 {}\n", ctx.comment_str("rcx *= 8"));
                                unusual_code += &format!(
                                    "\tshr {}, cl {}\n",
                                    REG_VALUE,
                                    ctx.comment_str("value >> bits")
                                );

                                // Take the lowest byte
                                unusual_code += &format!(
                                    "\tmov {}, {} {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    REG_VALUE,
                                    ctx.comment(format!(
                                        "{} = c",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );
                                unusual_code += &format!(
                                    "\tand {}, 0xFF {}\n",
                                    if ctx.store_b_in_c { REG_C } else { REG_B },
                                    ctx.comment(format!(
                                        "{} &= 0xFF",
                                        if ctx.store_b_in_c { "c" } else { "b" }
                                    ))
                                );

                                // Jump to check done
                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);

                                // Check done
                                *code += &format!("pc_{:x}_b_address_check_done:\n", ctx.pc);
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                    }
                    if ctx.chunk_player_mem_reads_collect_main() {
                        // Read value from mem reads and store in the proper register: a or c
                        *code += &format!(
                            "\tmov {}, [{}] {}\n",
                            if ctx.store_b_in_c { REG_C } else { REG_B },
                            REG_CHUNK_PLAYER_ADDRESS,
                            ctx.comment(format!(
                                "{} = mem_reads[address]",
                                if ctx.store_a_in_c { "c" } else { "b" }
                            ))
                        );

                        // Increment chunk player address
                        *code += &format!(
                            "\tadd {}, 8 {}\n",
                            REG_CHUNK_PLAYER_ADDRESS,
                            ctx.comment_str("chunk_address += 8")
                        );
                    }

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 1);
                    }

                    if ctx.mem_op() {
                        Self::b_src_ind_mem_op(&mut ctx, code, reg_address, instruction.ind_width);
                    }
                }
                _ => panic!(
                    "ZiskRom2Asm::save_to_asm() Invalid b_src={} pc={}",
                    instruction.b_src, ctx.pc
                ),
            }

            // Copy b value to main trace
            if ctx.main_trace() {
                *code += &ctx.full_line_comment("Main[2]=b".to_string());
                if ctx.store_b_in_c {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_C,
                        ctx.comment_str("b = c")
                    );
                } else if ctx.b.is_constant && !ctx.store_b_in_b {
                    *code += &format!(
                        "\tmov {}, 0x{:x} {}\n",
                        REG_B,
                        ctx.b.constant_value,
                        ctx.comment_str("value = b_const")
                    );
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_B,
                        ctx.comment_str("b = const")
                    );
                } else {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_B,
                        ctx.comment_str("b")
                    );
                }
            }

            // Copy b value to main trace
            if ctx.chunk_player_mem_reads_collect_main() {
                *code += &ctx.full_line_comment("Main[2] = b".to_string());
                if ctx.store_b_in_c {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_C,
                        ctx.comment_str("b = c")
                    );
                } else if ctx.b.is_constant && !ctx.store_b_in_b {
                    *code += &format!(
                        "\tmov {}, 0x{:x} {}\n",
                        REG_B,
                        ctx.b.constant_value,
                        ctx.comment_str("value = b_const")
                    );
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_B,
                        ctx.comment_str("b = const")
                    );
                } else {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_B,
                        ctx.comment_str("b")
                    );
                }
            }

            // Increment chunk.steps.mem_reads_size 3 times: op, a and b
            if ctx.chunk_player_mem_reads_collect_main() {
                *code += &format!(
                    "\t add {}, 3 {}\n",
                    REG_MEM_READS_SIZE,
                    ctx.comment_str("mem_reads_size += 3")
                );
            }

            /*************/
            /* Operation */
            /*************/

            // Execute operation, storing result is registers c and flag
            Self::operation_to_asm(&mut ctx, instruction.op, code, &mut unusual_code);

            // At this point, REG_C must contain the value of c
            assert!(ctx.c.is_saved);

            // Copy c value to main trace
            if ctx.main_trace() {
                *code += &ctx.full_line_comment("Main[3]=c".to_string());
                *code += &format!(
                    "\tmov [{REG_MEM_READS_ADDRESS} + {REG_MEM_READS_SIZE}*8 + 3*8], {REG_C}\n"
                );
            }

            /***********/
            /* STORE C */
            /***********/

            // Store register c
            match instruction.store {
                STORE_NONE => {
                    *code += &ctx.full_line_comment("STORE_NONE".to_string());

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 2);
                    }
                }
                STORE_REG => {
                    assert!(instruction.store_offset >= 0);
                    assert!(instruction.store_offset <= 34);

                    // Copy previous reg value to main trace
                    if ctx.main_trace() {
                        *code += &ctx.full_line_comment("Main[4]=prev_reg".to_string());
                        Self::read_riscv_reg(
                            &mut ctx,
                            code,
                            instruction.store_offset as u64,
                            REG_VALUE,
                            "value",
                        );

                        *code += &format!(
                            "\tmov [{} + {}*8 + 4*8], {} {}\n",
                            REG_MEM_READS_ADDRESS,
                            REG_MEM_READS_SIZE,
                            REG_VALUE,
                            ctx.comment_str("main[@+size*8+4*8] = prev_reg")
                        );
                    }

                    *code += &ctx
                        .full_line_comment(format!("STORE_REG reg={}", instruction.store_offset));

                    // Store in mem[address]
                    if instruction.store_ra {
                        let value = (ctx.pc as i64 + instruction.jmp_offset2) as u64;
                        Self::write_riscv_reg_constant(
                            &mut ctx,
                            code,
                            instruction.store_offset as u64,
                            value,
                            "pc + jmp_offset2",
                        );
                    } else {
                        Self::write_riscv_reg(
                            &mut ctx,
                            code,
                            instruction.store_offset as u64,
                            REG_C,
                            "c",
                        );
                    }

                    if ctx.main_trace() {
                        Self::trace_reg_access(&mut ctx, code, instruction.store_offset as u64, 2);
                    }
                }
                STORE_MEM => {
                    *code += &ctx.full_line_comment("STORE_MEM".to_string());

                    // Calculate memory address and store it in REG_ADDRESS
                    if !ctx.chunk_player_mem_reads_collect_main() {
                        *code += &format!(
                            "\tmov {}, 0x{:x} {}\n",
                            REG_ADDRESS,
                            instruction.store_offset,
                            ctx.comment_str("address = i.store_offset")
                        );
                        if instruction.store_use_sp {
                            *code += &format!(
                                "\tadd {}, {} {}\n",
                                REG_ADDRESS,
                                ctx.mem_sp,
                                ctx.comment_str("address += sp")
                            );
                            ctx.address_is_constant = false;
                        } else {
                            ctx.address_is_constant = true;
                            ctx.address_constant_value = instruction.store_offset as u64;
                        }
                    }

                    // Generate mem reads
                    if ctx.minimal_trace() || ctx.zip() {
                        if !instruction.store_use_sp {
                            if (instruction.store_offset & 0x7) != 0 {
                                // If zip, check if chunk is active
                                if ctx.zip() {
                                    *code += &format!(
                                        "\ttest {}, 1 {}\n",
                                        REG_ACTIVE_CHUNK,
                                        ctx.comment_str("active_chunk == 1 ?")
                                    );
                                    *code += &format!("\tjnz pc_{:x}_c_active_chunk\n", ctx.pc);
                                    *code += &format!("\tjmp pc_{:x}_c_address_done\n", ctx.pc);
                                    *code += &format!("pc_{:x}_c_active_chunk:\n", ctx.pc);
                                }
                                Self::c_store_mem_not_aligned(&mut ctx, code);
                            }
                        } else {
                            // If zip, check if chunk is active
                            if ctx.zip() {
                                *code += &format!(
                                    "\ttest {}, 1 {}\n",
                                    REG_ACTIVE_CHUNK,
                                    ctx.comment_str("active_chunk == 1 ?")
                                );
                                *code += &format!("\tjnz pc_{:x}_c_active_chunk\n", ctx.pc);
                                *code += &format!("\tjmp pc_{:x}_c_address_done\n", ctx.pc);
                                *code += &format!("pc_{:x}_c_active_chunk:\n", ctx.pc);
                            }
                            *code += &format!(
                                "\ttest {}, 0x7 {}\n",
                                REG_ADDRESS,
                                ctx.comment_str("address &= 7")
                            );
                            *code += &format!("\tjnz pc_{:x}_c_address_not_aligned\n", ctx.pc);
                            unusual_code += &format!("pc_{:x}_c_address_not_aligned:\n", ctx.pc);
                            Self::c_store_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code += &format!("\tjmp pc_{:x}_c_address_done\n", ctx.pc);
                        }
                        *code += &format!("pc_{:x}_c_address_done:\n", ctx.pc);
                    }

                    // Consume mem reads
                    if ctx.chunk_player_mt_collect_mem() {
                        if !instruction.store_use_sp {
                            if (instruction.store_offset & 0x7) != 0 {
                                // Increment chunk player address
                                *code += &format!(
                                    "\tadd {}, 16 {}\n",
                                    REG_CHUNK_PLAYER_ADDRESS,
                                    ctx.comment_str("chunk_address += 16")
                                );
                            }
                        } else {
                            *code += &format!(
                                "\ttest {}, 0x7 {}\n",
                                REG_ADDRESS,
                                ctx.comment_str("address &= 7")
                            );
                            *code += &format!("\tjnz pc_{:x}_c_address_not_aligned\n", ctx.pc);
                            unusual_code += &format!("pc_{:x}_c_address_not_aligned:\n", ctx.pc);
                            // Increment chunk player address
                            unusual_code += &format!(
                                "\tadd {}, 16 {}\n",
                                REG_CHUNK_PLAYER_ADDRESS,
                                ctx.comment_str("chunk_address += 16")
                            );
                            unusual_code += &format!("\tjmp pc_{:x}_c_address_done\n", ctx.pc);
                        }
                        *code += &format!("pc_{:x}_c_address_done:\n", ctx.pc);
                    }

                    // Store mem[address] = value
                    if !ctx.chunk_player_mt_collect_mem()
                        && !ctx.chunk_player_mem_reads_collect_main()
                    {
                        if instruction.store_ra {
                            *code += &format!(
                                "\tmov {}, 0x{:x} {}\n",
                                REG_VALUE,
                                (ctx.pc as i64 + instruction.jmp_offset2) as u64,
                                ctx.comment_str("value = pc + jmp_offset2")
                            );
                            *code += &format!(
                                "\tmov [{}], {} {}\n",
                                REG_ADDRESS,
                                REG_VALUE,
                                ctx.comment_str("mem[address] = value")
                            );
                        } else {
                            *code += &format!(
                                "\tmov [{}], {} {}\n",
                                REG_ADDRESS,
                                REG_C,
                                ctx.comment_str("mem[address] = c")
                            );
                        }
                    }

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 2);
                    }

                    if ctx.mem_op() {
                        Self::c_store_mem_mem_op(&mut ctx, code);
                    }
                }
                STORE_IND => {
                    *code += &ctx
                        .full_line_comment(format!("STORE_IND width={}", instruction.ind_width));

                    // Calculate memory address and store it in REG_ADDRESS
                    if !ctx.chunk_player_mem_reads_collect_main() {
                        *code += &format!(
                            "\tmov {}, {} {}\n",
                            REG_ADDRESS,
                            ctx.a.string_value,
                            ctx.comment_str("address = a")
                        );
                        if instruction.store_offset != 0 {
                            *code += &format!(
                                "\tadd {}, 0x{:x} {}\n",
                                REG_ADDRESS,
                                instruction.store_offset as u64,
                                ctx.comment_str("address += i.store_offset")
                            );
                        }
                        if instruction.store_use_sp {
                            *code += &format!(
                                "\tadd {}, {} {}\n",
                                REG_ADDRESS,
                                ctx.mem_sp,
                                ctx.comment_str("address += sp")
                            );
                        }
                    }

                    let address_is_constant = ctx.a.is_constant && !instruction.store_use_sp;
                    let address_constant_value = if address_is_constant {
                        (ctx.a.constant_value as i64 + instruction.store_offset) as u64
                    } else {
                        0
                    };
                    ctx.address_is_constant = address_is_constant;
                    ctx.address_constant_value = address_constant_value;
                    let address_is_aligned =
                        address_is_constant && ((address_constant_value & 0x7) == 0);

                    // Generate mem_reads
                    if ctx.minimal_trace() || ctx.zip() {
                        // If zip, check if chunk is active
                        if ctx.zip() {
                            *code += &format!(
                                "\ttest {}, 1 {}\n",
                                REG_ACTIVE_CHUNK,
                                ctx.comment_str("active_chunk == 1 ?")
                            );
                            *code += &format!("\tjnz pc_{:x}_c_active_chunk\n", ctx.pc);
                            *code += &format!("\tjmp pc_{:x}_c_active_chunk_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_c_active_chunk:\n", ctx.pc);
                        }
                        match instruction.ind_width {
                            8 => {
                                // Check if address is aligned, i.e. it is a multiple of 8
                                if address_is_constant {
                                    if !address_is_aligned {
                                        Self::c_store_ind_8_not_aligned(&mut ctx, code);
                                    }
                                } else {
                                    *code += &format!(
                                        "\ttest {}, 0x7 {}\n",
                                        REG_ADDRESS,
                                        ctx.comment_str("address &= 7")
                                    );
                                    *code +=
                                        &format!("\tjnz pc_{:x}_c_address_not_aligned\n", ctx.pc);
                                    unusual_code +=
                                        &format!("pc_{:x}_c_address_not_aligned:\n", ctx.pc);
                                    Self::c_store_ind_8_not_aligned(&mut ctx, &mut unusual_code);
                                    unusual_code +=
                                        &format!("\tjmp pc_{:x}_c_address_done\n", ctx.pc);
                                    *code += &format!("pc_{:x}_c_address_done:\n", ctx.pc);
                                }
                            }
                            4 | 2 => {
                                // Get a copy of the address to preserve it: aux = address
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_AUX,
                                    REG_ADDRESS,
                                    ctx.comment_str("aux = address")
                                );

                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    REG_AUX,
                                    ctx.comment_str("address = previous aligned address")
                                );

                                // Store previous aligned address value in mem_reads, advancing address
                                *code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    REG_AUX,
                                    ctx.comment_str("value = mem[prev_address]")
                                );
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8] = prev_c")
                                );

                                // Calculate next aligned address, keeping a copy of previous aligned
                                // address in value
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_VALUE,
                                    REG_AUX,
                                    ctx.comment_str("value = copy of prev_address")
                                );
                                let address_increment = instruction.ind_width - 1;
                                *code += &format!(
                                    "\tadd {}, {} {}\n",
                                    REG_AUX,
                                    address_increment,
                                    ctx.comment(format!("address += {address_increment}"))
                                );
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    REG_AUX,
                                    ctx.comment_str("address = next aligned address")
                                );
                                *code += &format!(
                                    "\tcmp {}, {} {}\n",
                                    REG_VALUE,
                                    REG_AUX,
                                    ctx.comment_str("prev_address = next_address ?")
                                );
                                *code +=
                                    &format!("\tjnz pc_{:x}_c_ind_different_address\n", ctx.pc);

                                // Same address

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size++")
                                );

                                // Different address

                                unusual_code +=
                                    &format!("pc_{:x}_c_ind_different_address:\n", ctx.pc);

                                // Store next aligned address value in mem_reads
                                unusual_code += &format!(
                                    "\tmov {}, [{}] {}\n",
                                    REG_VALUE,
                                    REG_AUX,
                                    ctx.comment_str("value = mem[next_address]")
                                );

                                // Copy read data into mem_reads_address and advance it
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8 + 8], {} {}\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE,
                                    ctx.comment_str("mem_reads[@+size*8+8] = next_c")
                                );

                                // Increment chunk.steps.mem_reads_size
                                unusual_code += &format!(
                                    "\tadd {}, 2 {}\n",
                                    REG_MEM_READS_SIZE,
                                    ctx.comment_str("mem_reads_size += 2")
                                );

                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_c_ind_address_done\n", ctx.pc);

                                // Done

                                *code += &format!("pc_{:x}_c_ind_address_done:\n", ctx.pc);
                            }
                            1 => {
                                // Since 1 byte always fits into one alligned 8B chunk, we always
                                // store the chunk in mem_reads

                                if address_is_constant && address_is_aligned {
                                    // Store  aligned address value in mem_reads, and increment address
                                    *code += &format!(
                                        "\tmov {}, [{}] {}\n",
                                        REG_VALUE,
                                        REG_ADDRESS,
                                        ctx.comment_str("value = mem[address]")
                                    );
                                    *code += &format!(
                                        "\tmov [{} + {}*8], {} {}\n",
                                        REG_MEM_READS_ADDRESS,
                                        REG_MEM_READS_SIZE,
                                        REG_VALUE,
                                        ctx.comment_str("mem_reads[@+size*8] = prev_c")
                                    );

                                    // Increment chunk.steps.mem_reads_size
                                    *code += &format!(
                                        "\tinc {} {}\n",
                                        REG_MEM_READS_SIZE,
                                        ctx.comment_str("mem_reads_size++")
                                    );
                                } else {
                                    // Get a copy of the address to preserve it
                                    *code += &format!(
                                        "\tmov {}, {} {}\n",
                                        REG_AUX,
                                        REG_ADDRESS,
                                        ctx.comment_str("aux = address")
                                    );

                                    // Calculate previous aligned address
                                    *code += &format!(
                                        "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                        REG_AUX,
                                        ctx.comment_str("address = previous aligned address")
                                    );

                                    // Store previous aligned address value in mem_reads, and increment address
                                    *code += &format!(
                                        "\tmov {}, [{}] {}\n",
                                        REG_VALUE,
                                        REG_AUX,
                                        ctx.comment_str("value = mem[prev_address]")
                                    );
                                    *code += &format!(
                                        "\tmov [{} + {}*8], {} {}\n",
                                        REG_MEM_READS_ADDRESS,
                                        REG_MEM_READS_SIZE,
                                        REG_VALUE,
                                        ctx.comment_str("mem_reads[@+size*8] = prev_c")
                                    );

                                    // Increment chunk.steps.mem_reads_size
                                    *code += &format!(
                                        "\tinc {} {}\n",
                                        REG_MEM_READS_SIZE,
                                        ctx.comment_str("mem_reads_size++")
                                    );
                                }
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                        if ctx.zip() {
                            *code += &format!("pc_{:x}_c_active_chunk_done:\n", ctx.pc);
                        }
                    }

                    // Consume mem_reads
                    if ctx.chunk_player_mt_collect_mem() {
                        match instruction.ind_width {
                            8 => {
                                // Check if address is aligned, i.e. it is a multiple of 8
                                if address_is_constant {
                                    if address_is_aligned {
                                        Self::chunk_player_mem_write(&mut ctx, code, 8, REG_C, 0);
                                    } else {
                                        Self::chunk_player_mem_write(&mut ctx, code, 8, REG_C, 2);
                                    }
                                } else {
                                    *code += &format!(
                                        "\ttest {}, 0x7 {}\n",
                                        REG_ADDRESS,
                                        ctx.comment_str("address &= 7")
                                    );
                                    *code +=
                                        &format!("\tjnz pc_{:x}_c_address_not_aligned\n", ctx.pc);
                                    unusual_code +=
                                        &format!("pc_{:x}_c_address_not_aligned:\n", ctx.pc);
                                    Self::chunk_player_mem_write(
                                        &mut ctx,
                                        &mut unusual_code,
                                        8,
                                        REG_C,
                                        2,
                                    );
                                    unusual_code +=
                                        &format!("\tjmp pc_{:x}_c_address_done\n", ctx.pc);
                                    Self::chunk_player_mem_write(&mut ctx, code, 8, REG_C, 0);
                                    *code += &format!("pc_{:x}_c_address_done:\n", ctx.pc);
                                }
                            }
                            4 | 2 => {
                                // Get a copy of the address to preserve it: aux = address
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_AUX,
                                    REG_ADDRESS,
                                    ctx.comment_str("aux = address")
                                );

                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    REG_AUX,
                                    ctx.comment_str("address = previous aligned address")
                                );

                                // Calculate next aligned address, keeping a copy of previous aligned
                                // address in value
                                *code += &format!(
                                    "\tmov {}, {} {}\n",
                                    REG_VALUE,
                                    REG_AUX,
                                    ctx.comment_str("value = copy of prev_address")
                                );
                                let address_increment = instruction.ind_width - 1;
                                *code += &format!(
                                    "\tadd {}, {} {}\n",
                                    REG_AUX,
                                    address_increment,
                                    ctx.comment(format!("address += {address_increment}"))
                                );
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
                                    REG_AUX,
                                    ctx.comment_str("address = next aligned address")
                                );
                                *code += &format!(
                                    "\tcmp {}, {} {}\n",
                                    REG_VALUE,
                                    REG_AUX,
                                    ctx.comment_str("prev_address = next_address ?")
                                );
                                *code +=
                                    &format!("\tjnz pc_{:x}_c_ind_different_address\n", ctx.pc);

                                // Same address
                                ///////////////

                                Self::chunk_player_mem_write(
                                    &mut ctx,
                                    code,
                                    instruction.ind_width,
                                    REG_C,
                                    1,
                                );

                                // Different address
                                ////////////////////

                                unusual_code +=
                                    &format!("pc_{:x}_c_ind_different_address:\n", ctx.pc);

                                Self::chunk_player_mem_write(
                                    &mut ctx,
                                    &mut unusual_code,
                                    instruction.ind_width,
                                    REG_C,
                                    2,
                                );

                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_c_ind_address_done\n", ctx.pc);

                                // Done
                                ///////

                                *code += &format!("pc_{:x}_c_ind_address_done:\n", ctx.pc);
                            }
                            1 => {
                                // Since 1 byte always fits into one alligned 8B chunk, we always
                                // store the chunk in mem_reads

                                Self::chunk_player_mem_write(&mut ctx, code, 1, REG_C, 1);
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                        if ctx.zip() {
                            *code += &format!("pc_{:x}_c_active_chunk_done:\n", ctx.pc);
                        }
                    }

                    // Store mem[address] = value
                    if !ctx.chunk_player_mt_collect_mem()
                        && !ctx.chunk_player_mem_reads_collect_main()
                    {
                        match instruction.ind_width {
                            8 => {
                                if instruction.store_ra {
                                    *code += &format!(
                                        "\tmov qword {}[{}], {} {}\n",
                                        ctx.ptr,
                                        REG_ADDRESS,
                                        (ctx.pc as i64 + instruction.jmp_offset2) as u64,
                                        ctx.comment_str("width=8: mem[address] = pc + jmp_offset2")
                                    );
                                } else {
                                    *code += &format!(
                                        "\tmov [{}], {} {}\n",
                                        REG_ADDRESS,
                                        REG_C,
                                        ctx.comment_str("width=8: mem[address] = c")
                                    );
                                }
                            }
                            4 => {
                                if instruction.store_ra {
                                    *code += &format!(
                                        "\tmov dword {}[{}], {} {}\n",
                                        ctx.ptr,
                                        REG_ADDRESS,
                                        (ctx.pc as i64 + instruction.jmp_offset2) as u64,
                                        ctx.comment_str("width=4: mem[address] = pc + jmp_offset2")
                                    );
                                } else {
                                    *code += &format!(
                                        "\tmov [{}], {} {}\n",
                                        REG_ADDRESS,
                                        REG_C_W,
                                        ctx.comment_str("width=4: mem[address] = c")
                                    );
                                }
                            }
                            2 => {
                                if instruction.store_ra {
                                    *code += &format!(
                                        "\tmov word {}[{}], {} {}\n",
                                        ctx.ptr,
                                        REG_ADDRESS,
                                        (ctx.pc as i64 + instruction.jmp_offset2) as u64,
                                        ctx.comment_str("width=2: mem[address] = pc + jmp_offset2")
                                    );
                                } else {
                                    *code += &format!(
                                        "\tmov [{}], {} {}\n",
                                        REG_ADDRESS,
                                        REG_C_H,
                                        ctx.comment_str("width=2: mem[address] = c")
                                    );
                                }
                            }
                            1 => {
                                if instruction.store_ra {
                                    *code += &format!(
                                        "\tmov word {}[{}], {} {}\n",
                                        ctx.ptr,
                                        REG_ADDRESS,
                                        (ctx.pc as i64 + instruction.jmp_offset2) as u64,
                                        ctx.comment_str("width=1: mem[address] = pc + jmp_offset2")
                                    );
                                } else {
                                    *code += &format!(
                                        "\tmov [{}], {} {}\n",
                                        REG_ADDRESS,
                                        REG_C_B,
                                        ctx.comment_str("width=1: mem[address] = c")
                                    );
                                }
                                if ctx.log_output {
                                    *code += &format!(
                                        "\tmov {}, 0xa0000200 {}\n",
                                        REG_FLAG,
                                        ctx.comment_str("width=1: aux = UART")
                                    );
                                    *code += &format!(
                                        "\tcmp {}, {} {}\n",
                                        REG_ADDRESS,
                                        REG_FLAG,
                                        ctx.comment_str(
                                            "width=1: if address = USART then print char"
                                        )
                                    );
                                    *code += &format!(
                                        "\tjne pc_{:x}_store_c_not_uart {}\n",
                                        ctx.pc,
                                        ctx.comment_str("width=1: continue")
                                    );
                                    if instruction.store_ra {
                                        *code += &format!(
                                            "\tmov dil, 0x{:x} {}\n",
                                            (ctx.pc as i64 + instruction.jmp_offset2) as u64 as u8,
                                            ctx.comment_str("width=1: rdi = value")
                                        );
                                    } else {
                                        *code += &format!(
                                            "\tmov dil, {} {}\n",
                                            REG_C_B,
                                            ctx.comment_str("width=1: rdi = c")
                                        );
                                    }
                                    Self::push_internal_registers(&mut ctx, code, false);
                                    //Self::assert_rsp_is_aligned(&mut ctx, code);
                                    *code += "\tcall _print_char\n";
                                    Self::pop_internal_registers(&mut ctx, code, false);
                                    //Self::assert_rsp_is_aligned(&mut ctx, code);
                                    *code += &format!("pc_{:x}_store_c_not_uart:\n", ctx.pc);
                                }
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                    }

                    if ctx.main_trace() {
                        Self::clear_reg_step_ranges(&mut ctx, code, 2);
                    }

                    if ctx.mem_op() {
                        Self::c_store_ind_mem_op(&mut ctx, code, instruction.ind_width);
                    }
                }
                _ => panic!(
                    "ZiskRom2Asm::save_to_asm() Invalid store={} pc={}",
                    instruction.store, ctx.pc
                ),
            }

            // if ctx.c.is_constant && !ctx.c.string_value.eq(REG_C) {
            //     *s += &format!(
            //         "\tmov {}, {} ; STORE: make sure c=value */\n",
            //         REG_C, ctx.c.string_value
            //     );
            // }

            // Used only to get traces of registers a, b, c and flag/step
            // *s += &format!("\tpush {}\n", REG_FLAG);
            // *s += &format!("\tpush {}\n", REG_FLAG);
            // *s += &format!("\tpush {}\n", REG_C);
            // *s += &format!("\tpush {}\n", REG_B);
            // *s += &format!("\tpush {}\n", REG_A);
            // *s += &format!("\tmov rdi, {}\n", REG_A);
            // *s += &format!("\tmov rsi, {}\n", REG_B);
            // *s += &format!("\tmov rdx, {}\n", REG_C);
            // // if ctx.flag_is_always_one {
            // //     *s += &format!("\tmov rcx, 1\n");
            // // } else if ctx.flag_is_always_zero {4
            // //     *s += &format!("\tmov rcx, 0\n");
            // // } else {
            // //     *s += &format!("\tmov rcx, {}\n", REG_FLAG);
            // // }
            // *s += &format!("\tmov rcx, {}\n", MEM_STEP);
            // *s += &format!("\tmov rax, 0\n"); // NEW
            // *s += &format!("\tcall _print_abcflag\n");
            // *s += &format!("\tpop {}\n", REG_A);
            // *s += &format!("\tpop {}\n", REG_B);
            // *s += &format!("\tpop {}\n", REG_C);
            // *s += &format!("\tpop {}\n", REG_FLAG);
            // *s += &format!("\tpop {}\n", REG_FLAG);

            // Used for debugging
            // Self::push_internal_registers(&mut ctx, code, false);
            // *code += &format!("\tmov rdi, {}\n", ctx.pc);
            // *code += &format!("\tmov rsi, {}\n", REG_C);
            // *code += &format!("\tcall _print_pc\n");
            // Self::pop_internal_registers(&mut ctx, code, false);

            if ctx.main_trace() {
                *code += &ctx.full_line_comment(
                    "Main[5] = prev_reg_mem[0] + (prev_reg_mem[1] & 0xfffff ) << 40".to_string(),
                );
                *code += &format!("\tmov {}, qword {}[reg_prev_steps_1]\n", REG_VALUE, ctx.ptr);
                *code += &format!("\tshl {REG_VALUE}, 40\n"); // 64-40=24 bits
                *code += &format!("\tmov {}, qword {}[reg_prev_steps_0]\n", REG_AUX, ctx.ptr);
                *code += &format!("\tadd {REG_VALUE}, {REG_AUX}\n");
                *code += &format!(
                    "\tmov [{} + {}*8 + 5*8], {} {}\n",
                    REG_MEM_READS_ADDRESS,
                    REG_MEM_READS_SIZE,
                    REG_VALUE,
                    ctx.comment_str("main[@+size*8+5*8] = value")
                );

                *code += &ctx.full_line_comment("Main[6] = prev_reg_mem[2] + (prev_reg_mem[1] & 0xfffff00000 ) << 21 + flag<<24".to_string());
                *code += &format!("\tmov {}, qword {}[reg_prev_steps_1]\n", REG_VALUE, ctx.ptr);
                *code += &format!("\tmov {REG_AUX}, 0xfffff00000\n");
                *code += &format!("\tand {REG_VALUE}, {REG_AUX}\n");
                *code += &format!("\tshl {REG_VALUE}, 21\n");
                *code += &format!("\tmov {}, qword {}[reg_prev_steps_2]\n", REG_AUX, ctx.ptr);
                *code += &format!("\tadd {REG_VALUE}, {REG_AUX}\n");
                if ctx.flag_is_always_one {
                    *code += &format!("\tmov {REG_AUX}, 0x10000000000\n");
                    *code += &format!("\tadd {REG_VALUE}, {REG_AUX}\n");
                } else if ctx.flag_is_always_zero {
                    // Nothing to add
                } else {
                    *code += &format!("\tmov {REG_AUX}, {REG_FLAG}\n");
                    *code += &format!("\tshl {REG_AUX}, 24\n");
                    *code += &format!("\tadd {REG_VALUE}, {REG_AUX}\n");
                }
                *code += &format!(
                    "\tmov [{} + {}*8 + 6*8], {} {}\n",
                    REG_MEM_READS_ADDRESS,
                    REG_MEM_READS_SIZE,
                    REG_VALUE,
                    ctx.comment_str("main[@+size*8+6*8] = value")
                );

                // Increment chunk.steps.mem_reads_size in 7 u64 slots
                *code += &format!(
                    "\tadd {}, 7 {}\n",
                    REG_MEM_READS_SIZE,
                    ctx.comment_str("mem_reads_size += 7")
                );
            }

            /********/
            /* STEP */
            /********/

            // Decrement step counter
            *code += &ctx.full_line_comment("STEP".to_string());
            if ctx.fast() || ctx.rom_histogram() {
                *code += &format!("\tinc {} {}\n", REG_STEP, ctx.comment_str("increment step"));
            }
            if ctx.chunks()
                || ctx.minimal_trace()
                || ctx.main_trace()
                || ctx.zip()
                || ctx.mem_op()
                || ctx.chunk_player_mt_collect_mem()
                || ctx.mem_reads()
                || ctx.chunk_player_mem_reads_collect_main()
            {
                *code += &format!(
                    "\tdec {} {}\n",
                    REG_STEP,
                    ctx.comment_str("decrement step count down")
                );
                if instruction.end {
                    *code += &format!("\tmov {}, 1 {}\n", ctx.mem_end, ctx.comment_str("end = 1"));
                    *code += &format!(
                        "\tmov {}, 0x{:08x} {}\n",
                        REG_PC,
                        ctx.pc,
                        ctx.comment_str("value = pc")
                    );
                    if ctx.chunk_player_mt_collect_mem()
                        || ctx.chunk_player_mem_reads_collect_main()
                    {
                        *code += "\tjz execute_end\n";
                    } else {
                        *code += "\tcall chunk_end\n";
                    }
                } else if ctx.chunk_player_mt_collect_mem()
                    || ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += "\tjz execute_end\n";
                    Self::set_pc(&mut ctx, instruction, code, "nz", rom);
                } else {
                    *code += &format!("\tjz pc_{:x}_step_zero\n", ctx.pc);
                    unusual_code += &format!("pc_{:x}_step_zero:\n", ctx.pc);
                    Self::set_pc(&mut ctx, instruction, &mut unusual_code, "z", rom);
                    unusual_code += "\tcall chunk_end_and_start\n";
                    unusual_code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_VALUE,
                        ctx.mem_step,
                        ctx.comment_str("value = step")
                    );
                    unusual_code += &format!(
                        "\tcmp {}, qword ptr [max_steps] {}\n",
                        REG_VALUE,
                        ctx.comment_str("step ?= max_steps")
                    );
                    unusual_code += "\tjae execute_end\n";
                    unusual_code += &format!("\tjmp pc_{:x}_step_done\n", ctx.pc);
                    Self::set_pc(&mut ctx, instruction, code, "nz", rom);
                    *code += &format!("pc_{:x}_step_done:\n", ctx.pc);
                }
            }
            if ctx.fast() || ctx.rom_histogram() {
                if instruction.end {
                    *code += &format!("\tmov {}, 1 {}\n", ctx.mem_end, ctx.comment_str("end = 1"));
                }
                Self::set_pc(&mut ctx, instruction, code, "nz", rom);
            }

            // Used only to get logs of step
            // *s += &format!("\tmov {}, {} ; value = step */\n", REG_VALUE, MEM_STEP);
            // *s += &format!("\tand {}, 0xfffff ; value = step */\n", REG_VALUE);
            // *s += &format!("\tcmp {}, 0 ; value = step */\n", REG_VALUE);
            // *s += &format!("\tjne  pc_{:x}_inc_step_done ; value = step */\n", ctx.pc);
            // *s += &format!("\tpush {}\n", REG_VALUE);
            // *s += &format!("\tmov rdi, {}\n", MEM_STEP);

            // *s += "\tpush rax\n";
            // *s += "\tpush rcx\n";
            // *s += "\tpush rdx\n";
            // // *s += "\tpush rdi\n";
            // // *s += "\tpush rsi\n";
            // // *s += "\tpush rsp\n";
            // // *s += "\tpush r8\n";
            // *s += "\tpush r9\n";
            // *s += "\tpush r10\n";
            // //*s += "\tpush r11\n";
            // *s += &format!("\tcall _print_step\n");

            // //*s += "\tpop r11\n";
            // *s += "\tpop r10\n";
            // *s += "\tpop r9\n";
            // // *s += "\tpop r8\n";
            // // *s += "\tpop rsp\n";
            // // *s += "\tpop rsi\n";
            // // *s += "\tpop rdi\n";
            // *s += "\tpop rdx\n";
            // *s += "\tpop rcx\n";
            // *s += "\tpop rax\n";

            // *s += &format!("\tpop {}\n", REG_VALUE);
            // *s += &format!("pc_{:x}_inc_step_done:\n", ctx.pc);

            // If step % K == 0 then store data
            // *s += &format!("\tmov {}, {} ; copy step into value */\n", REG_VALUE, MEM_STEP);
            // *s += &format!("\tand {}, 0xffff ; value &= k */\n", REG_VALUE);
            // *s += &format!(
            //     "\tjnz pc_{:x}_no_store_data ; skip if storing is not required */\n",
            //     ctx.pc
            // );
            // *s += &format!("\t; Store data */\n");
            // *s += &format!("pc_{:x}_no_store_data:\n", ctx.pc);

            // Jump to new pc, if not the next one
            if instruction.end {
                *code += "\tjmp execute_end\n";
            } else if !ctx.jump_to_static_pc.is_empty() {
                *code += ctx.jump_to_static_pc.as_str();
            } else if ctx.jump_to_dynamic_pc {
                Self::jumpt_to_dynamic_pc(&mut ctx, code);
            }
        }

        *code += "\n";

        *code += "execute_end:\n";

        // Update step memory variable with the content of the step register, to make it accessible
        // to the caller
        if ctx.fast() || ctx.rom_histogram() || ctx.main_trace() {
            *code += &format!(
                "\tmov {}, {} {}\n",
                ctx.mem_step,
                REG_STEP,
                ctx.comment_str("update step variable")
            );
        }

        if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
            Self::chunk_player_end(&mut ctx, code);
        }

        Self::pop_external_registers(&mut ctx, code);

        // Used only to get the last log of step
        // *s += &format!("\tpush {}\n", REG_VALUE);
        // *s += &format!("\tmov rdi, {}\n", MEM_STEP);
        // *s += "\tcall _print_step\n";
        // *s += &format!("\tpop {}\n", REG_VALUE);

        // *s += "\tmov rax, 60\n";
        // *s += "\tmov rdi, 0\n";
        // *s += "\tsyscall\n\n";

        *code += "\tret\n\n";

        /****************/
        /* UNUSUAL CODE */
        /****************/
        *code += unusual_code.as_str();

        /**********************/
        /* READ_ONLY ROM DATA */
        /**********************/

        *code += "\n";
        *code += ".global write_ro_data\n";
        *code += "write_ro_data:\n";

        Self::push_external_registers(&mut ctx, code);

        // Create a new read section for every RO data entry of the rom
        //let mut total_ro_data_len: usize = 0;
        for i in 0..rom.ro_data.len() {
            let mut address = rom.ro_data[i].from;
            let ro_data_len = rom.ro_data[i].data.len();
            //total_ro_data_len += ro_data_len;
            // println!(
            //     "ZiskRom2Asm::save_to_asm() ro_data[{}] len={} total_len={} address={:x}",
            //     i, ro_data_len, total_ro_data_len, address
            // );
            let mut written_bytes = 0;
            while written_bytes + 8 <= ro_data_len {
                let value = u64::from_le_bytes(
                    rom.ro_data[i].data[written_bytes..written_bytes + 8].try_into().unwrap(),
                );
                *code += &format!("\tmov {}, 0x{:x}\n", REG_ADDRESS, address);
                *code += &format!("\tmov {}, 0x{:x}\n", REG_VALUE, value);
                *code += &format!("\tmov qword {}[{}], {}\n", ctx.ptr, REG_ADDRESS, REG_VALUE);
                address += 8;
                written_bytes += 8;
            }
            while written_bytes + 4 <= ro_data_len {
                let value = u32::from_le_bytes(
                    rom.ro_data[i].data[written_bytes..written_bytes + 4].try_into().unwrap(),
                );
                *code += &format!("\tmov {}, 0x{:x}\n", REG_ADDRESS, address);
                *code += &format!("\tmov {}, 0x{:x}\n", REG_VALUE, value);
                *code += &format!("\tmov dword {}[{}], {}\n", ctx.ptr, REG_ADDRESS, REG_VALUE_W);
                address += 4;
                written_bytes += 4;
            }
            while written_bytes + 2 <= ro_data_len {
                let value = u16::from_le_bytes(
                    rom.ro_data[i].data[written_bytes..written_bytes + 2].try_into().unwrap(),
                );
                *code += &format!("\tmov {}, 0x{:x}\n", REG_ADDRESS, address);
                *code += &format!("\tmov {}, 0x{:x}\n", REG_VALUE, value);
                *code += &format!("\tmov word {}[{}], {}\n", ctx.ptr, REG_ADDRESS, REG_VALUE_H);
                address += 2;
                written_bytes += 2;
            }
            while written_bytes < ro_data_len {
                let value = rom.ro_data[i].data[written_bytes];
                *code += &format!("\tmov {}, 0x{:x}\n", REG_ADDRESS, address);
                *code += &format!("\tmov {}, 0x{:x}\n", REG_VALUE, value);
                *code += &format!("\tmov byte {}[{}], {}\n", ctx.ptr, REG_ADDRESS, REG_VALUE_B);
                address += 1;
                written_bytes += 1;
            }
        }

        Self::pop_external_registers(&mut ctx, code);
        *code += "\tret\n\n";

        /*****************/
        /* BRANCH TABLES */
        /*****************/

        // For all program addresses in the vector, create an assembly set of instructions with a
        // map label
        *code += "\n";
        *code += ".section .rodata\n";
        *code += ".align 64\n";

        // Safety check: Ensure the minimum program address label exists
        //
        // This is defensive programming for rare cases where min_program_pc has no valid
        // instruction (non-NOP padding, data in text section).
        // In practice with NOP padding, this check never triggers and the entrypoint
        // is the min_program_pc
        if rom.min_program_pc >= ROM_ADDR && !rom.sorted_pc_list.contains(&rom.min_program_pc) {
            *code +=
                &format!("map_pc_{:x}: \t.quad pc_{:x}\n", rom.min_program_pc, rom.min_program_pc);
        }

        // Init previous key to the first ROM entry
        let mut previous_key: u64 = ROM_ENTRY;
        for key in &rom.sorted_pc_list {
            if ctx.jump_to_unaligned_pc() {
                // When in chunk player mode, we need to resume the chunk at any address,
                // including internal addresses not aligned to 4B.  We need to fill all the gaps
                // between alligned addresses to make the distance between addresses constant and
                // allow jumping to the proper branch using pc - ROM_ADDR as an increment
                //
                // 4N
                //   4N + 1
                //   4N + 2   <--  We want to be able to dynamically start a chunk at this pc
                //   4N + 3
                // 4(N+1)
                //   ...
                if (*key > ROM_ADDR) && (*key != (previous_key + 1) && (*key != FLOAT_LIB_ROM_ADDR))
                {
                    for _ in previous_key + 1..*key {
                        *code += "\t.quad 0\n";
                    }
                }
            } else if (key & 0x3) != 0 {
                // Skip internal, unaligned instructions, since we never jump directly to them,
                // except when in chunk player mode, since we need to resume a chunk at any pc
                //
                // 4N
                // 4(N+1)  <--  We only need to dynamically jump to an alligned pc
                // 4(N+2)
                //   ...
                continue;
            }

            // Map fixed-length pc labels to real variable-length instruction labels
            // This is used to implement dynamic jumps, i.e. to jump to an address that is not
            // a constant in the instruction, but dynamically built as part of the emulation

            // Only use labels in boundary pc addresses
            // match key {
            //     0x1000 | 0x80000000 => {
            //         *code += &format!("\nmap_pc_{:x}: \t.quad pc_{:x}", key, key)
            //     }
            //     _ => *code += &format!(", pc_{:x}", key),
            // }

            // Use labels always
            *code += &format!("map_pc_{key:x}: \t.quad pc_{key:x}\n");

            // Update previous key
            previous_key = *key;
        }
        *code += "\n";

        #[cfg(debug_assertions)]
        {
            let mut lines = code.lines();
            let mut map_label_lines_counter = 0u64;
            let mut pc_label_lines_counter = 0u64;
            let mut comment_lines_counter = 0u64;
            let mut code_lines_counter = 0u64;

            loop {
                let line_option = lines.next();
                if line_option.is_none() {
                    break;
                }
                let line = line_option.unwrap();
                if line.is_empty() {
                    continue;
                }
                if line.starts_with("map_pc_") {
                    map_label_lines_counter += 1;
                    continue;
                }
                if line.starts_with("pc_") {
                    pc_label_lines_counter += 1;
                    continue;
                }
                if line.starts_with("\t/*") {
                    comment_lines_counter += 1;
                    continue;
                }
                code_lines_counter += 1;
            }

            println!(
                "ZiskRom2Asm::save_to_asm() {} bytes, {} instructions, {:02} bytes/inst, {} map lines, {} label lines, {} comment lines, {} code lines, {:02} code lines/inst",
                code.len(),
                rom.sorted_pc_list.len(),
                code.len() as f64 / rom.sorted_pc_list.len() as f64,
                map_label_lines_counter,
                pc_label_lines_counter,
                comment_lines_counter,
                code_lines_counter,
                code_lines_counter as f64 / rom.sorted_pc_list.len() as f64,
            );
        }
    }

    fn operation_to_asm(
        ctx: &mut ZiskAsmContext,
        opcode: u8,
        code: &mut String,
        unusual_code: &mut String,
    ) {
        // Set flags to false, by default
        ctx.flag_is_always_one = false;
        ctx.flag_is_always_zero = false;

        // Prepare c context
        ctx.c.is_constant = false;
        ctx.c.constant_value = 0;
        ctx.c.is_saved = false;
        ctx.c.string_value = REG_C.to_string();

        let zisk_op = ZiskOp::try_from_code(opcode).unwrap();
        match zisk_op {
            ZiskOp::Flag => {
                *code +=
                    &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("Flag: c = 0"));
                ctx.c.is_constant = true;
                ctx.c.constant_value = 0;
                ctx.c.string_value = "0".to_string();
                ctx.c.is_saved = true;
                ctx.flag_is_always_one = true;
            }
            ZiskOp::CopyB => {
                assert!(ctx.store_b_in_c);
                ctx.c.is_constant = ctx.b.is_constant;
                ctx.c.constant_value = ctx.b.constant_value;
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SignExtendB => {
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tmovsx {}, {} {}\n",
                    REG_C,
                    REG_B_B,
                    ctx.comment_str("SignExtendW: sign extend b(8b) to c(64b)")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SignExtendH => {
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tmovsx {}, {} {}\n",
                    REG_C,
                    REG_B_H,
                    ctx.comment_str("SignExtendW: sign extend b(16b) to c(64b)")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SignExtendW => {
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tmovsxd {}, {} {}\n",
                    REG_C,
                    REG_B_W,
                    ctx.comment_str("SignExtendW: sign extend b(32b) to c(64b)")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Add => {
                if ctx.a.is_constant && (ctx.a.constant_value == 0) {
                    assert!(ctx.store_b_in_c);
                    *code += &ctx.full_line_comment("Add: c = a(0) + b = b".to_string());
                } else if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    assert!(ctx.store_a_in_c);
                    *code += &ctx.full_line_comment("Add: c = a + b(0) = a".to_string());
                } else {
                    assert!(ctx.store_a_in_c);
                    *code += &ctx.full_line_comment("Add: c = a".to_string());
                    *code += &format!(
                        "\tadd {}, {} {}\n",
                        REG_C,
                        ctx.b.string_value,
                        ctx.comment_str("Add: c = c + b = a + b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::AddW => {
                assert!(ctx.store_b_in_b);
                // DEBUG: Used only to preserve b value
                // s +=
                //     &format!("\tmov {}, {} {}\n", REG_VALUE, ctx.b.string_value, ctx.comment_str("AddW: value = b"));
                if ctx.a.is_constant && (ctx.a.constant_value == 0) {
                    *code += &ctx.full_line_comment("AddW: ignoring a since a = 0".to_string());
                } else {
                    *code += &format!(
                        "\tadd {}, {} {}\n",
                        REG_B,
                        ctx.a.string_value,
                        ctx.comment_str("AddW: b += a")
                    );
                }
                *code += &format!("\tcdqe {}\n", ctx.comment_str("AddW: trunk b"));
                *code +=
                    &format!("\tmov {}, {} {}\n", REG_C, REG_B, ctx.comment_str("AddW: c = b"));
                ctx.c.is_saved = true;
                // DEBUG: Used only to preserve b value
                //s += &format!("\tmov {}, {} {}\n", REG_B, REG_VALUE, ctx.comment_str("AddW: b = value"));
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sub => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += &ctx.full_line_comment("Sub: ignoring b since b = 0".to_string());
                } else {
                    *code += &format!(
                        "\tsub {}, {} {}\n",
                        REG_C,
                        ctx.b.string_value,
                        ctx.comment_str("Sub: c = c - b = a - b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SubW => {
                assert!(ctx.store_a_in_a);
                // DEBUG: Used only to preserve b value
                // s += &format!(
                //     "\tmov {}, {} {}\n",
                //     REG_ADDRESS, ctx.a.string_value,
                //     ctx.commit_str("SubW: address = a")
                // );
                // s +=
                //     &format!("\tmov {}, {} {}\n", REG_VALUE, ctx.b.string_value, ctx.comment_str("SubW: value = b"));
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += &ctx.full_line_comment("SubW: ignoring b since b = 0".to_string());
                } else {
                    *code += &format!(
                        "\tsub {}, {} {}\n",
                        REG_A,
                        ctx.b.string_value,
                        ctx.comment_str("SubW: a -= b")
                    );
                }
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_B,
                    REG_A,
                    ctx.comment_str("SubW: b = a = a - b")
                );
                *code += &format!("\tcdqe {}\n", ctx.comment_str("SubW: trunk b"));
                *code +=
                    &format!("\tmov {}, {} {}\n", REG_C, REG_B, ctx.comment_str("SubW: c = b"));
                ctx.c.is_saved = true;
                // DEBUG: Used only to preserver a,b values
                // s += &format!("\tmov {}, {} {}\n", REG_A, REG_ADDRESS, ctx.comment_str("SubW: a = address"));
                // s += &format!("\tmov {}, {} {}\n", REG_B, REG_VALUE, ctx.comment_str("SubW: b = value"));
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sll => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tshl {}, 0x{:x} {}\n",
                        REG_C,
                        ctx.b.constant_value & 0x3f,
                        ctx.comment_str("Sll: c = a << b")
                    );
                } else {
                    *code += &format!("\tmov rcx, {} {}\n", REG_B, ctx.comment_str("Sll: c = b"));
                    *code += &format!(
                        "\tshl {}, cl {}\n",
                        REG_C,
                        ctx.comment_str("Sll: c(value) = a << b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SllW => {
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE,
                    ctx.a.string_value,
                    ctx.comment_str("SllW: value = a")
                );
                *code += &format!(
                    "\tmov rcx, {} {}\n",
                    ctx.b.string_value,
                    ctx.comment_str("SllW: c = b")
                );
                *code += &format!(
                    "\tshl {}, cl {}\n",
                    REG_VALUE_W,
                    ctx.comment_str("SllW: value = a << b")
                );
                *code += &format!(
                    "\tmovsxd {}, {} {}\n",
                    REG_C,
                    REG_VALUE_W,
                    ctx.comment_str("SllW: sign extend to quad value -> c")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sra => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tmov rcx, {} {}\n",
                    ctx.b.string_value,
                    ctx.comment_str("Sra: rcx = b")
                );
                *code +=
                    &format!("\tsar {}, cl {}\n", REG_C, ctx.comment_str("Sra: c = c >> b(cl)"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Srl => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tshr {}, 0x{:x} {}\n",
                        REG_C,
                        ctx.b.constant_value & 0x3f,
                        ctx.comment_str("Srl: c = a >> b")
                    );
                } else {
                    *code += &format!(
                        "\tmov rcx, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("Srl: b = value ")
                    );
                    *code += &format!(
                        "\tshr {}, cl {}\n",
                        REG_C,
                        ctx.comment_str("Srl: c(value) = a >> b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SraW => {
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_VALUE,
                        ctx.a.string_value,
                        ctx.comment_str("SraW: c = a")
                    );
                    *code += &format!(
                        "\tsar {}, 0x{:x} {}\n",
                        REG_VALUE_W,
                        ctx.b.constant_value & 0x3f,
                        ctx.comment_str("SraW: c = a >> b")
                    );
                    *code += &format!(
                        "\tmovsxd {}, {} {}\n",
                        REG_C,
                        REG_VALUE_W,
                        ctx.comment_str("SraW: sign extend to quad")
                    );
                } else {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_VALUE,
                        ctx.a.string_value,
                        ctx.comment_str("SraW: c(value) = a")
                    );
                    *code +=
                        &format!("\tmov rcx, {} {}\n", REG_B, ctx.comment_str("SraW: rcx = b"));
                    *code += &format!(
                        "\tsar {}, cl {}\n",
                        REG_VALUE_W,
                        ctx.comment_str("SraW: c(value) = a >> b")
                    );
                    *code += &format!(
                        "\tmovsxd {}, {} {}\n",
                        REG_C,
                        REG_VALUE_W,
                        ctx.comment_str("SraW: sign extend to quad")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SrlW => {
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_VALUE,
                        ctx.a.string_value,
                        ctx.comment_str("SrlW: c = a")
                    );
                    *code += &format!(
                        "\tshr {}, 0x{:x} {}\n",
                        REG_VALUE_W,
                        ctx.b.constant_value & 0x3f,
                        ctx.comment_str("SrlW: c = a >> b")
                    );
                    *code += &format!(
                        "\tmovsxd {}, {} {}\n",
                        REG_C,
                        REG_VALUE_W,
                        ctx.comment_str("SrlW: sign extend to quad")
                    );
                } else {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_VALUE,
                        ctx.a.string_value,
                        ctx.comment_str("SrlW: c = a")
                    );
                    *code += &format!(
                        "\tmov rcx, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("SrlW: b = value")
                    );
                    *code += &format!(
                        "\tshr {}, cl {}\n",
                        REG_VALUE_W,
                        ctx.comment_str("SrlW: c(value) = a >> b")
                    );
                    *code += &format!(
                        "\tmovsxd {}, {} {}\n",
                        REG_C,
                        REG_VALUE_W,
                        ctx.comment_str("SlrW: sign extend to quad")
                    );
                }
                ctx.c.is_saved = true;
                //s += &format!("\tmov {}, {} {}\n", REG_C, REG_VALUE, ctx.comment_str("SrlW: c = value"));
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Eq => {
                assert!(ctx.store_a_in_a);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    ctx.b.string_value,
                    ctx.comment_str("Eq: a == b ?")
                );
                *code += &format!("\tje pc_{:x}_equal_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_equal_done\n", ctx.pc);
                *code += &format!("pc_{:x}_equal_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_equal_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::EqW => {
                // Make sure a is in REG_A to compare it against b (constant, expression or reg)
                if ctx.a.is_constant {
                    *code += &format!(
                        "\tmov {}, 0x{:x} {}\n",
                        REG_A,
                        ctx.a.constant_value & 0xffffffff,
                        ctx.comment_str("EqW: a = constant")
                    );
                }
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} {}\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff,
                        ctx.comment_str("EqW: a == b ?")
                    );
                } else {
                    *code += &format!(
                        "\tcmp {}, {} {}\n",
                        REG_A_W,
                        REG_B_W,
                        ctx.comment_str("EqW: a == b ?")
                    );
                }
                *code += &format!("\tje pc_{:x}_equal_w_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_equal_w_done\n", ctx.pc);
                *code += &format!("pc_{:x}_equal_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_equal_w_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Ltu => {
                assert!(ctx.store_a_in_a);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    ctx.b.string_value,
                    ctx.comment_str("Ltu: a == b ?")
                );
                *code += &format!("\tjb pc_{:x}_ltu_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_ltu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_ltu_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_ltu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Lt => {
                assert!(ctx.store_a_in_a);
                // If b is constant and too big, move it to its register
                if ctx.b.is_constant && (ctx.b.constant_value >= P2_32) {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_B,
                        ctx.b.string_value,
                        ctx.comment_str("Lt: b = constant")
                    );
                    ctx.b.is_constant = false;
                    ctx.b.string_value = REG_B.to_string();
                }
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    ctx.b.string_value,
                    ctx.comment_str("Lt: a == b ?")
                );
                *code += &format!("\tjl pc_{:x}_lt_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_lt_done\n", ctx.pc);
                *code += &format!("pc_{:x}_lt_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_lt_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LtuW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} {}\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff,
                        ctx.comment_str("LtuW: a == b ?")
                    );
                } else {
                    *code += &format!(
                        "\tcmp {}, {} {}\n",
                        REG_A_W,
                        REG_B_W,
                        ctx.comment_str("LtuW: a == b ?")
                    );
                }
                *code += &format!("\tjb pc_{:x}_ltuw_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_ltuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_ltuw_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_ltuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LtW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} {}\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff,
                        ctx.comment_str("LtW: a == b ?")
                    );
                } else {
                    *code += &format!(
                        "\tcmp {}, {} {}\n",
                        REG_A_W,
                        REG_B_W,
                        ctx.comment_str("LtW: a == b")
                    );
                }
                *code += &format!("\tjl pc_{:x}_ltw_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_ltw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_ltw_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_ltw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Leu => {
                assert!(ctx.store_a_in_a);
                // If b is constant and too big, move it to its register
                if ctx.b.is_constant && (ctx.b.constant_value >= P2_32) {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_B,
                        ctx.b.string_value,
                        ctx.comment_str("Leu: b = const_value")
                    );
                    ctx.b.is_constant = false;
                    ctx.b.string_value = REG_B.to_string();
                }
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    ctx.b.string_value,
                    ctx.comment_str("Leu: a == b ?")
                );
                *code += &format!("\tpc_{:x}_jbe leu_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tpc_{:x}_jmp leu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_leu_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_leu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Le => {
                assert!(ctx.store_a_in_a);
                // If b is constant and too big, move it to its register
                if ctx.b.is_constant && (ctx.b.constant_value >= P2_32) {
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_B,
                        ctx.b.string_value,
                        ctx.comment_str("Le: b = const_value")
                    );
                    ctx.b.is_constant = false;
                    ctx.b.string_value = REG_B.to_string();
                }
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    ctx.b.string_value,
                    ctx.comment_str("Le: a == b ?")
                );
                *code += &format!("\tjle pc_{:x}_lte_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_lte_done\n", ctx.pc);
                *code += &format!("pc_{:x}_lte_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_lte_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LeuW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} {}\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff,
                        ctx.comment_str("LeuW: a == b ?")
                    );
                } else {
                    *code += &format!(
                        "\tcmp {}, {} {}\n",
                        REG_A_W,
                        REG_B_W,
                        ctx.comment_str("LeuW: a == b ?")
                    );
                }
                *code += &format!("\tjbe pc_{:x}_leuw_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_leuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_leuw_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_leuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LeW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} {}\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff,
                        ctx.comment_str("LeW: a == b ?")
                    );
                } else {
                    *code += &format!(
                        "\tcmp {}, {} {}\n",
                        REG_A_W,
                        REG_B_W,
                        ctx.comment_str("LeW: a == b ?")
                    );
                }
                *code += &format!("\tjle pc_{:x}_lew_true\n", ctx.pc);
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                *code +=
                    &format!("\txor {}, {} {}\n", REG_FLAG, REG_FLAG, ctx.comment_str("flag = 0"));
                *code += &format!("\tjmp pc_{:x}_lew_done\n", ctx.pc);
                *code += &format!("pc_{:x}_lew_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 {}\n", REG_C, ctx.comment_str("c = 1"));
                *code += &format!("\tmov {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag = 1"));
                *code += &format!("pc_{:x}_lew_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::And => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0xffffffffffffffff) {
                    *code += &ctx.full_line_comment("And: ignoring b since b = f's".to_string());
                } else {
                    *code += &format!(
                        "\tand {}, {} {}\n",
                        REG_C,
                        ctx.b.string_value,
                        ctx.comment_str("And: c = c AND b = a AND b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Or => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += &ctx.full_line_comment("Or: ignoring b since b = 0".to_string());
                } else {
                    *code += &format!(
                        "\tor {}, {} {}\n",
                        REG_C,
                        ctx.b.string_value,
                        ctx.comment_str("Or: c = c OR b = a OR b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Xor => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += &ctx.full_line_comment("Xor: ignoring b since b = 0".to_string());
                } else {
                    *code += &format!(
                        "\txor {}, {} {}\n",
                        REG_C,
                        ctx.b.string_value,
                        ctx.comment_str("Xor: c = c XOR b = a XOR b")
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mulu => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code +=
                    &format!("\tmul {} {}\n", REG_A, ctx.comment_str("Mulu: rax*reg -> rdx:rax"));
                *code +=
                    &format!("\tmov {}, rax {}\n", REG_C, ctx.comment_str("Mulu: c = result(rax)"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Muluh => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code +=
                    &format!("\tmul {} {}\n", REG_A, ctx.comment_str("Muluh: rax*reg -> rdx:rax"));
                *code += &format!(
                    "\tmov {}, rdx {}\n",
                    REG_C,
                    ctx.comment_str("Muluh: c = high result(rdx)")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mulsuh => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\tmov rsi, {} {}\n", REG_B, ctx.comment_str("Mulsuh: rsi = b"));
                *code += &format!("\tmov rax, {} {}\n", REG_A, ctx.comment_str("Mulsuh: rax = a"));
                *code +=
                    &format!("\tmov {}, rax {}\n", REG_VALUE, ctx.comment_str("Mulsuh: value = a"));
                *code += &format!(
                    "\tsar {}, 63 {}\n",
                    REG_VALUE,
                    ctx.comment_str("Mulsuh: value = a>>63 = a_bit_63")
                );
                *code += &format!("\tmov rdx, 0 {}\n", ctx.comment_str("Mulsuh: rdx=0, rdx:rax=a"));
                *code +=
                    &format!("\tmul rsi {}\n", ctx.comment_str("Mulsuh: rdx:rax = a*b (unsigned)"));
                *code += &format!("\tmov rcx, rax {}\n", ctx.comment_str("Mulsuh: rax = a"));
                *code += &format!(
                    "\tmov rax, {} {}\n",
                    REG_VALUE,
                    ctx.comment_str("Mulsuh: rax = a_bit_63")
                );
                *code += &format!(
                    "\timul rax, rsi {}\n",
                    ctx.comment_str("Mulsuh: rax = rax*b = a_bit_63*b")
                );
                *code +=
                    &format!("\tadd rdx, rax {}\n", ctx.comment_str("Mulsuh: rdx=rdx+a_bit_63*b"));
                *code += &format!(
                    "\tmov {}, rdx {}\n",
                    REG_C,
                    ctx.comment_str("Mulsuh: c = high result(rdx)")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mul => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code +=
                    &format!("\timul {} {}\n", REG_A, ctx.comment_str("Mul: rax*reg -> rdx:rax"));
                *code +=
                    &format!("\tmov {}, rax {}\n", REG_C, ctx.comment_str("Mul: c = result(rax)"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mulh => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code +=
                    &format!("\timul {} {}\n", REG_A, ctx.comment_str("Mulh: rax*reg -> rdx:rax"));
                *code += &format!(
                    "\tmov {}, rdx {}\n",
                    REG_C,
                    ctx.comment_str("Mulh: c = high result(rdx)")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MulW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code +=
                    &format!("\tmul {} {}\n", REG_A_W, ctx.comment_str("MulW: rax*reg -> rdx:rax"));
                *code += &format!(
                    "\tmovsxd {}, {} {}\n",
                    REG_C,
                    REG_B_W,
                    ctx.comment_str("MulW: sign extend to quad")
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Divu => {
                assert!(ctx.store_b_in_b);
                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder
                // If b==0 return 0xffffffffffffffff
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B,
                    ctx.comment_str("Divu: if b == 0 return f's")
                );
                *code += &format!(
                    "\tjne pc_{:x}_divu_b_is_not_zero {}\n",
                    ctx.pc,
                    ctx.comment_str("Divu: if b is not zero, divide")
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff {}\n",
                    REG_C,
                    ctx.comment_str("Divu: set result to f's")
                );
                *code += &format!("\tje pc_{:x}_divu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_divu_b_is_not_zero:\n", ctx.pc);

                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE,
                    REG_B,
                    ctx.comment_str("Divu: value = b backup")
                );
                *code += &format!("\tmov rdx, 0 {}\n", ctx.comment_str("Divu: rdx = 0"));
                *code += &format!(
                    "\tmov rax, {} {}\n",
                    ctx.a.string_value,
                    ctx.comment_str("Divu: rax = a")
                );
                *code += &format!(
                    "\tdiv {} {}\n",
                    REG_VALUE,
                    ctx.comment_str("Divu: rdx:rax / value(b backup) -> rax (rdx remainder)")
                );
                *code += &format!(
                    "\tmov {}, rax {}\n",
                    REG_C,
                    ctx.comment_str("Divu: c = quotient(rax)")
                );
                *code += &format!("pc_{:x}_divu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Remu => {
                assert!(ctx.store_b_in_b);
                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder
                // If b==0 return a
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B,
                    ctx.comment_str("Remu: if b == 0 return a")
                );
                *code += &format!(
                    "\tjne pc_{:x}_remu_b_is_not_zero {}\n",
                    ctx.pc,
                    ctx.comment_str("Remu: if b is not zero, divide")
                );
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    ctx.a.string_value,
                    ctx.comment_str("Remu: set result to f's")
                );
                *code += &format!("\tje pc_{:x}_remu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_remu_b_is_not_zero:\n", ctx.pc);

                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE,
                    REG_B,
                    ctx.comment_str("Remu: value = b backup")
                );
                *code += &format!("\tmov rdx, 0 {}\n", ctx.comment_str("Remu: rdx = 0"));
                *code += &format!(
                    "\tmov rax, {} {}\n",
                    ctx.a.string_value,
                    ctx.comment_str("Remu: rax = a")
                );
                *code += &format!(
                    "\tdiv {} {}\n",
                    REG_VALUE,
                    ctx.comment_str("Remu: rdx:rax / value(b backup) -> rax (rdx remainder)")
                );
                *code += &format!(
                    "\tmov {}, rdx {}\n",
                    REG_C,
                    ctx.comment_str("Remu: c = remainder(rdx)")
                );
                *code += &format!("pc_{:x}_remu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Div => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
                // If a=0x8000000000000000 (MIN_I64) and b=0xFFFFFFFFFFFFFFFF (-1) the result should
                // be -MIN_I64, which cannot be represented with 64 bits (overflow)
                // and it returns c=a.

                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder

                // Check divide by zero:
                // If b==0 return 0xffffffffffffffff
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B,
                    ctx.comment_str("Div: if b == 0 return f's")
                );
                *code += &format!(
                    "\tje pc_{:x}_div_by_zero {}\n",
                    ctx.pc,
                    ctx.comment_str("Div: if b is zero, jump")
                );
                *unusual_code += &format!("pc_{:x}_div_by_zero:\n", ctx.pc);
                *unusual_code += &format!(
                    "\tmov {}, 0xffffffffffffffff {}\n",
                    REG_C,
                    ctx.comment_str("Div: set result to f's")
                );

                *unusual_code += &format!("\tjmp pc_{:x}_div_done\n", ctx.pc);

                // Check underflow:
                // If a==0x8000000000000000 && b==0xffffffffffffffff then c=a
                *code += &format!(
                    "\tmov {}, 0x8000000000000000 {}\n",
                    REG_VALUE,
                    ctx.comment_str("Div: value == 0x8000000000000000")
                );
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    REG_VALUE,
                    ctx.comment_str("Div: if a == value(0x8000000000000000), then check b")
                );
                *code += &format!(
                    "\tjne pc_{:x}_div_divide {}\n",
                    ctx.pc,
                    ctx.comment_str("Div: if a is not 0x8000000000000000, then divide")
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff {}\n",
                    REG_VALUE,
                    ctx.comment_str("Div: value == 0xffffffffffffffff")
                );
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_B,
                    REG_VALUE,
                    ctx.comment_str("Div: if b == 0xffffffffffffffff, then return a")
                );
                *code += &format!(
                    "\tjne pc_{:x}_div_divide {}\n",
                    ctx.pc,
                    ctx.comment_str("Div: if b is not 0xffffffffffffffff, divide")
                );
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    REG_A,
                    ctx.comment_str("Div: set result to a")
                );

                *code += &format!("\tje pc_{:x}_div_done\n", ctx.pc);

                // Divide
                *code += &format!("pc_{:x}_div_divide:\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE,
                    REG_B,
                    ctx.comment_str("Div: value = b backup")
                );
                *code += &format!("\tmov rax, {} {}\n", REG_A, ctx.comment_str("Div: rax = a"));
                *code += &format!("\tbt rax, 63 {}\n", ctx.comment_str("Div: is a negative?"));
                *code += &format!("\tjnc pc_{:x}_a_is_positive\n", ctx.pc);
                *code += &format!(
                    "\tmov rdx, 0xffffffffffffffff {}\n",
                    ctx.comment_str("Div: a is negative, rdx = f's")
                );
                *code += &format!("\tjmp pc_{:x}_a_done\n", ctx.pc);
                *code += &format!("pc_{:x}_a_is_positive:\n", ctx.pc);
                *code +=
                    &format!("\tmov rdx, 0 {}\n", ctx.comment_str("Div: a is positive, rdx = 0"));
                *code += &format!("pc_{:x}_a_done:\n", ctx.pc);

                *code += &format!(
                    "\tidiv {} {}\n",
                    REG_VALUE,
                    ctx.comment_str("Div: rdx:rax / value(b backup) -> rax (rdx remainder)")
                );
                *code += &format!(
                    "\tmov {}, rax {}\n",
                    REG_C,
                    ctx.comment_str("Div: c = quotient(rax)")
                );
                *code += &format!("pc_{:x}_div_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Rem => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
                // If a=0x8000000000000000 (MIN_I64) and b=0xFFFFFFFFFFFFFFFF (-1) the result should
                // be -MIN_I64, which cannot be represented with 64 bits (overflow)
                // and it returns c=a.

                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder

                // Check divide by zero:
                // If b==0 return 0xffffffffffffffff
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B,
                    ctx.comment_str("Rem: if b == 0 return f's")
                );
                *code += &format!(
                    "\tjne pc_{:x}_rem_check_underflow {}\n",
                    ctx.pc,
                    ctx.comment_str("Rem: if b is not zero, check underflow")
                );
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    REG_A,
                    ctx.comment_str("Rem: set result to a")
                );

                *code += &format!("\tje pc_{:x}_rem_done\n", ctx.pc);

                // Check underflow:
                *code += &format!("pc_{:x}_rem_check_underflow:\n", ctx.pc);
                // If a==0x8000000000000000 && b==0xffffffffffffffff then c=a
                *code += &format!(
                    "\tmov {}, 0x8000000000000000 {}\n",
                    REG_VALUE,
                    ctx.comment_str("Rem: value == 0x8000000000000000")
                );
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_A,
                    REG_VALUE,
                    ctx.comment_str("Rem: if a == value(0x8000000000000000), then check b")
                );
                *code += &format!(
                    "\tjne pc_{:x}_rem_divide {}\n",
                    ctx.pc,
                    ctx.comment_str("Rem: if a is not 0x8000000000000000, then divide")
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff {}\n",
                    REG_VALUE,
                    ctx.comment_str("Rem: value == 0xffffffffffffffff")
                );
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_B,
                    REG_VALUE,
                    ctx.comment_str("Rem: if b == 0xffffffffffffffff, then return a")
                );
                *code += &format!(
                    "\tjne pc_{:x}_rem_divide {}\n",
                    ctx.pc,
                    ctx.comment_str("Rem: if b is not 0xffffffffffffffff, divide")
                );
                *code += &format!(
                    "\txor {}, {} {}\n",
                    REG_C,
                    REG_C,
                    ctx.comment_str("Rem: set result to 0")
                );

                *code += &format!("\tje pc_{:x}_rem_done\n", ctx.pc);

                // Divide
                *code += &format!("pc_{:x}_rem_divide:\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE,
                    REG_B,
                    ctx.comment_str("Rem: value = b backup")
                );
                *code += &format!("\tmov rax, {} {}\n", REG_A, ctx.comment_str("Rem: rax = a"));
                *code += &format!("\tbt rax, 63 {}\n", ctx.comment_str("Rem: is a negative?"));
                *code += &format!("\tjnc pc_{:x}_a_is_positive\n", ctx.pc);
                *code += &format!(
                    "\tmov rdx, 0xffffffffffffffff {}\n",
                    ctx.comment_str("Rem: a is negative, rdx = f's")
                );
                *code += &format!("\tjmp pc_{:x}_a_done\n", ctx.pc);
                *code += &format!("pc_{:x}_a_is_positive:\n", ctx.pc);
                *code +=
                    &format!("\tmov rdx, 0 {}\n", ctx.comment_str("Rem: a is positive, rdx = 0"));
                *code += &format!("pc_{:x}_a_done:\n", ctx.pc);

                *code += &format!(
                    "\tidiv {} {}\n",
                    REG_VALUE,
                    ctx.comment_str("Rem: rdx:rax / value(b backup) -> rax (rdx remainder)")
                );
                *code += &format!(
                    "\tmov {}, rdx {}\n",
                    REG_C,
                    ctx.comment_str("Rem: c = remainder(rdx)")
                );
                *code += &format!("pc_{:x}_rem_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::DivuW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B_W,
                    ctx.comment_str("DivuW: if b==0 then return all f's")
                );
                *code += &format!(
                    "\tjne pc_{:x}_divuw_b_is_not_zero {}\n",
                    ctx.pc,
                    ctx.comment_str("DivuW: if b is not zero, divide")
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff {}\n",
                    REG_C,
                    ctx.comment_str("DivuW: set result to f's")
                );
                *code += &format!("\tjmp pc_{:x}_divuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_divuw_b_is_not_zero:\n", ctx.pc);

                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE_W,
                    REG_B_W,
                    ctx.comment_str("DivuW: value = b backup")
                );
                *code += &format!("\tmov rdx, 0 {}\n", ctx.comment_str("DivuW: rdx = 0"));
                *code += &format!("\tmov eax, {} {}\n", REG_A_W, ctx.comment_str("DivuW: rax = a"));
                *code += &format!(
                    "\tdiv {} {}\n",
                    REG_VALUE_W,
                    ctx.comment_str("DivuW: rdx:rax / value(b backup) -> rax (rdx remainder)")
                );
                *code += &format!(
                    "\tmovsxd {}, eax {}\n",
                    REG_C,
                    ctx.comment_str("DivuW: sign extend 32 to 64 bits")
                );
                *code += &format!("pc_{:x}_divuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::RemuW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B_W,
                    ctx.comment_str("RemuW: if b==0 then return a")
                );
                *code += &format!("\tjne pc_{:x}_remuw_b_is_not_zero\n", ctx.pc);
                *code += &format!(
                    "\tmovsxd {}, {} {}\n",
                    REG_C,
                    REG_A_W,
                    ctx.comment_str("RemuW: return a, sign extend 32 to 64 bits")
                );
                *code += &format!("\tjmp pc_{:x}_remuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_remuw_b_is_not_zero:\n", ctx.pc);

                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE_W,
                    REG_B_W,
                    ctx.comment_str("RemuW: value = b backup")
                );
                *code += &format!("\tmov rdx, 0 {}\n", ctx.comment_str("RemuW: rdx = 0"));
                *code += &format!("\tmov eax, {} {}\n", REG_A_W, ctx.comment_str("RemuW: rax = a"));
                *code += &format!(
                    "\tdiv {} {}\n",
                    REG_VALUE_W,
                    ctx.comment_str("RemuW: rdx:rax / value(b backup) -> rax (rdx remainder)")
                );
                *code += &format!(
                    "\tmovsxd {}, edx {}\n",
                    REG_C,
                    ctx.comment_str("RemuW: sign extend 32 to 64 bits")
                );
                *code += &format!("pc_{:x}_remuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::DivW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder

                // Check divide by zero:
                // If b==0 return 0xffffffffffffffff
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B_W,
                    ctx.comment_str("DivW: if b == 0 return f's")
                );
                *code += &format!(
                    "\tjne pc_{:x}_divw_divide {}\n",
                    ctx.pc,
                    ctx.comment_str("DivW: if b is not zero, divide")
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff {}\n",
                    REG_C,
                    ctx.comment_str("DivW: set result to f's")
                );

                *code += &format!("\tje pc_{:x}_divw_done\n", ctx.pc);

                // Divide
                *code += &format!("pc_{:x}_divw_divide:\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE_W,
                    REG_B_W,
                    ctx.comment_str("DivW: value = b backup")
                );
                *code += &format!("\tmov eax, {} {}\n", REG_A_W, ctx.comment_str("DivW: rax = a"));
                *code +=
                    &format!("\tcdq {}\n", ctx.comment_str("DivW: EDX:EAX := sign-extend of EAX"));
                *code += &format!(
                    "\tidiv {} {}\n",
                    REG_VALUE_W,
                    ctx.comment_str("DivW: edx:eax / value(b backup) -> eax (edx remainder)")
                );
                *code += &format!(
                    "\tmovsx {}, eax {}\n",
                    REG_C,
                    ctx.comment_str("DivW: c = quotient(rax)")
                );
                *code += &format!("pc_{:x}_divw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::RemW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder.

                // Check divide by zero:
                // If b==0 return a
                *code += &format!(
                    "\tcmp {}, 0 {}\n",
                    REG_B_W,
                    ctx.comment_str("RemW: if b == 0 return f's")
                );
                *code += &format!(
                    "\tjne pc_{:x}_remw_divide {}\n",
                    ctx.pc,
                    ctx.comment_str("RemW: if b is not zero, divide")
                );
                *code += &format!(
                    "\tmovsx {}, {} {}\n",
                    REG_C,
                    REG_A_W,
                    ctx.comment_str("RemW: set result to a")
                );

                *code += &format!("\tje pc_{:x}_remw_done\n", ctx.pc);

                // Divide
                *code += &format!("pc_{:x}_remw_divide:\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE_W,
                    REG_B_W,
                    ctx.comment_str("RemW: value = b backup ")
                );
                *code += &format!("\tmov eax, {} {}\n", REG_A_W, ctx.comment_str("RemW: rax = a"));
                *code +=
                    &format!("\tcdq {}\n", ctx.comment_str("RemW: EDX:EAX := sign-extend of EAX"));
                *code += &format!(
                    "\tidiv {} {}\n",
                    REG_VALUE_W,
                    ctx.comment_str("RemW: edx:eax / value(b backup) -> eax (edx remainder)")
                );
                *code += &format!(
                    "\tmovsx {}, edx {}\n",
                    REG_C,
                    ctx.comment_str("RemW: c = remainder(edx)")
                );
                *code += &format!("pc_{:x}_remw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Minu => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("Minu: compare a and b")
                );
                *code += &format!("\tjb pc_{:x}_minu_a_is_below_b\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("c = b")
                );
                *code += &format!("pc_{:x}_minu_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Min => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("Min: compare a and b")
                );
                *code += &format!("\tjl pc_{:x}_min_a_is_below_b\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("c = b")
                );
                *code += &format!("pc_{:x}_min_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MinuW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C_W,
                    REG_B_W,
                    ctx.comment_str("MinuW: compare a and b")
                );
                *code += &format!("\tjb pc_{:x}_minuw_a_is_below_b\n", ctx.pc);
                *code +=
                    &format!("\tmov {}, {} {}\n", REG_C, REG_B, ctx.comment_str("MinuW: c = b "));
                *code += &format!("pc_{:x}_minuw_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MinW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C_W,
                    REG_B_W,
                    ctx.comment_str("MinW: compare a and b")
                );
                *code += &format!("\tjl pc_{:x}_minw_a_is_below_b\n", ctx.pc);
                *code +=
                    &format!("\tmov {}, {} {}\n", REG_C, REG_B, ctx.comment_str("MinW: c = b"));
                *code += &format!("pc_{:x}_minw_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Maxu => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("Maxu: compare a and b")
                );
                *code += &format!("\tja pc_{:x}_maxu_a_is_above_b\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("Maxu: c = b")
                );
                *code += &format!("pc_{:x}_maxu_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Max => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("Max: compare a and b")
                );
                *code += &format!("\tjg pc_{:x}_max_a_is_above_b\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_C,
                    ctx.b.string_value,
                    ctx.comment_str("Max: c = b")
                );
                *code += &format!("pc_{:x}_max_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MaxuW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C_W,
                    REG_B_W,
                    ctx.comment_str("MaxuW: compare a and b")
                );
                *code += &format!("\tja pc_{:x}_maxuw_a_is_above_b\n", ctx.pc);
                *code +=
                    &format!("\tmov {}, {} {}\n", REG_C, REG_B, ctx.comment_str("MaxuW: c = b"));
                *code += &format!("pc_{:x}_maxuw_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MaxW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tcmp {}, {} {}\n",
                    REG_C_W,
                    REG_B_W,
                    ctx.comment_str("MaxW: compare a and b")
                );
                *code += &format!("\tjg pc_{:x}_maxw_a_is_above_b\n", ctx.pc);
                *code +=
                    &format!("\tmov {}, {} {}\n", REG_C, REG_B, ctx.comment_str("MaxW: c = b"));
                *code += &format!("pc_{:x}_maxw_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Keccak => {
                // Use the memory address as the first and unique parameter
                *code += &ctx.full_line_comment("Keccak: rdi = A0".to_string());

                // Generate mem reads
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Use the memory address as the first and unique parameter
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );

                    // Copy read data into mem_reads_address and advance it
                    if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                        // If zip, check if chunk is active
                        if ctx.zip() {
                            *code += &format!(
                                "\ttest {}, 1 {}\n",
                                REG_ACTIVE_CHUNK,
                                ctx.comment_str("active_chunk == 1 ?")
                            );
                            *code += &format!("\tjnz pc_{:x}_keccak_active_chunk\n", ctx.pc);
                            *code += &format!("\tjmp pc_{:x}_keccak_active_chunk_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_keccak_active_chunk:\n", ctx.pc);
                        }
                        *code += &format!("\tmov {REG_ADDRESS}, rdi\n");
                        for k in 0..25 {
                            *code += &format!(
                                "\tmov {}, [{} + {}] {}\n",
                                REG_VALUE,
                                REG_ADDRESS,
                                k * 8,
                                ctx.comment(format!("value = mem[keccak_address[{k}]]"))
                            );
                            *code += &format!(
                                "\tmov [{} + {}*8 + {}], {} {}\n",
                                REG_MEM_READS_ADDRESS,
                                REG_MEM_READS_SIZE,
                                k * 8,
                                REG_VALUE,
                                ctx.comment(format!("mem_reads[{k}] = value"))
                            );
                        }

                        // Increment chunk.steps.mem_reads_size in 25 units
                        *code += &format!(
                            "\tadd {}, 25 {}\n",
                            REG_MEM_READS_SIZE,
                            ctx.comment_str("mem_reads_size += 25")
                        );

                        if ctx.zip() {
                            *code += &format!("pc_{:x}_keccak_active_chunk_done:\n", ctx.pc);
                        }
                    }

                    // Trace 25 memory read operations
                    if ctx.mem_op() {
                        *code += &format!("\tmov {REG_ADDRESS}, rdi\n");
                        Self::mem_op_array(ctx, code, REG_ADDRESS, false, 8, 25);
                        Self::mem_op_array(ctx, code, REG_ADDRESS, true, 8, 25);
                    }

                    // Call the keccak function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_keccak\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 25*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 25*8")
                    );
                }

                // Set result
                *code +=
                    &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("Keccak: c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sha256 => {
                // Use the memory address as the first and unique parameter
                *code += &ctx.full_line_comment("SHA256: rdi = b".to_string());

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Use the memory address as the first and unique parameter
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );

                    // Save data into mem_reads
                    if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                        // If zip, check if chunk is active
                        if ctx.zip() {
                            *code += &format!(
                                "\ttest {}, 1 {}\n",
                                REG_ACTIVE_CHUNK,
                                ctx.comment_str("active_chunk == 1 ?")
                            );
                            *code += &format!("\tjnz pc_{:x}_sha256_active_chunk\n", ctx.pc);
                            *code += &format!("\tjmp pc_{:x}_sha256_active_chunk_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_sha256_active_chunk:\n", ctx.pc);
                        }
                        Self::precompiled_save_mem_reads(ctx, code, 2, &[4, 8]);
                        if ctx.zip() {
                            *code += &format!("pc_{:x}_sha256_active_chunk_done:\n", ctx.pc);
                        }
                    }

                    // Save memory operations into mem_reads
                    if ctx.mem_op() {
                        Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[4, 8], 0, 0, 4);
                    }

                    // Call the SHA256 function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_sha256\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 14*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 12*8")
                    );
                }

                // Set result
                *code +=
                    &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("SHA256: c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::PubOut => {
                assert!(ctx.store_b_in_c);
                ctx.c.is_constant = ctx.b.is_constant;
                ctx.c.constant_value = ctx.b.constant_value;
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Arith256 => {
                *code += &ctx.full_line_comment("Arith256".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_arith256_active_chunk\n", ctx.pc);
                        *code += &format!("\tjmp pc_{:x}_arith256_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_arith256_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 5, &[4, 4, 4]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_arith256_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 17*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 17*8")
                    );
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 5, &[4, 4, 4], 3, 4, 4);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the arith256 function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_arith256\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Arith256Mod => {
                *code += &ctx.full_line_comment("Arith256Mod".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_arith256mod_active_chunk\n", ctx.pc);
                        *code += &format!("\tjmp pc_{:x}_arith256mod_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_arith256mod_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 5, &[4, 4, 4, 4]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_arith256mod_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 5, &[4, 4, 4, 4], 4, 4, 4);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the arith256_mod function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_arith256_mod\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 21*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 21*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Secp256k1Add => {
                *code += &ctx.full_line_comment("Secp256k1Add".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_secp256k1add_active_chunk\n", ctx.pc);
                        *code += &format!("\tjmp pc_{:x}_secp256k1add_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_secp256k1add_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[8, 8]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_secp256k1add_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[8, 8], 0, 0, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the secp256k1_add function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_secp256k1_add\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 18*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 18*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Secp256k1Dbl => {
                *code += &ctx.full_line_comment("Secp256k1Dbl".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Copy read data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_secp256k1dbl_active_chunk\n", ctx.pc);
                        *code += &format!("\tjmp pc_{:x}_secp256k1dbl_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_secp256k1dbl_active_chunk:\n", ctx.pc);
                    }
                    *code += &format!("\tmov {REG_ADDRESS}, rdi\n");
                    for k in 0..8 {
                        *code += &format!(
                            "\tmov {}, [{} + {}] {}\n",
                            REG_VALUE,
                            REG_ADDRESS,
                            k * 8,
                            ctx.comment(format!("value = mem[address[{k}]]"))
                        );
                        *code += &format!(
                            "\tmov [{} + {}*8 + {}], {} {}\n",
                            REG_MEM_READS_ADDRESS,
                            REG_MEM_READS_SIZE,
                            k * 8,
                            REG_VALUE,
                            ctx.comment(format!("mem_reads[{k}] = value"))
                        );
                    }

                    // Increment chunk.steps.mem_reads_size in 8 units
                    *code += &format!(
                        "\tadd {}, 8 {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size += 8")
                    );
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_secp256k1dbl_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_array(ctx, code, "rdi", false, 8, 8);
                    Self::mem_op_array(ctx, code, "rdi", true, 8, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the secp256k1_dbl function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_secp256k1_dbl\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 8*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 8*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::FcallParam => {
                assert!(ctx.store_b_in_c);
                assert!(ctx.a.is_constant);
                assert!(ctx.a.constant_value <= 32);
                *code += &ctx.full_line_comment("FcallParam".to_string());

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    if ctx.a.constant_value == 1 {
                        // Store param in params
                        *code += &format!(
                            "\tmov {}, qword {}[{} + {}*8] {}\n",
                            REG_AUX,
                            ctx.ptr,
                            ctx.fcall_ctx,
                            FCALL_PARAMS_SIZE,
                            ctx.comment_str("aux = params size")
                        );
                        *code += &format!(
                            "\tmov qword {}[{} + {}*8 + {}*8], {} {}\n",
                            ctx.ptr,
                            ctx.fcall_ctx,
                            REG_AUX,
                            FCALL_PARAMS,
                            REG_C,
                            ctx.comment_str("ctx.params[size] = b")
                        );
                        *code += &format!(
                            "\tinc qword {}[{} + {}*8] {}\n",
                            ctx.ptr,
                            ctx.fcall_ctx,
                            FCALL_PARAMS_SIZE,
                            ctx.comment_str("inc ctx.params_size")
                        );
                    } else {
                        // Store params in params
                        *code += &format!(
                            "\tmov {}, qword {}[{} + {}*8] {}\n",
                            REG_AUX,
                            ctx.ptr,
                            ctx.fcall_ctx,
                            FCALL_PARAMS_SIZE,
                            ctx.comment_str("aux = params size")
                        );
                        for i in 0..ctx.a.constant_value {
                            *code += &format!(
                                "\tmov {}, qword {}[{} + {}*8] {}\n",
                                REG_VALUE,
                                ctx.ptr,
                                REG_C,
                                i,
                                ctx.comment_str("value = params[b]")
                            );

                            *code += &format!(
                                "\tmov qword {}[{} + {}*8 + {}*8], {} {}\n",
                                ctx.ptr,
                                ctx.fcall_ctx,
                                REG_AUX,
                                FCALL_PARAMS,
                                REG_VALUE,
                                ctx.comment_str("params[aux] = param")
                            );
                            *code += &format!("\tinc {} {}\n", REG_AUX, ctx.comment_str("inc aux"));
                        }
                        *code += &format!(
                            "\tmov qword {}[{} + {}*8], {} {}\n",
                            ctx.ptr,
                            ctx.fcall_ctx,
                            FCALL_PARAMS_SIZE,
                            REG_AUX,
                            ctx.comment_str("ctx.params_size = aux")
                        );
                    }
                }

                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Fcall => {
                *code += &ctx.full_line_comment("Fcall".to_string());

                assert!(ctx.store_b_in_c);
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Store a (function id) in context
                    assert!(ctx.a.is_constant);
                    *code += &format!(
                        "\tmov qword {}[{} + {}*8], {} {}\n",
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_FUNCTION_ID,
                        ctx.a.constant_value,
                        ctx.comment_str("ctx.function id = a")
                    );

                    // Set the fcall context address as the first parameter
                    *code += &format!(
                        "\tlea rdi, {} {}\n",
                        ctx.fcall_ctx,
                        ctx.comment_str("rdi = fcall context")
                    );

                    // Call the fcall function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_fcall\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);

                    // Get free input address
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_ADDRESS,
                        FREE_INPUT_ADDR,
                        ctx.comment_str("address = free_input")
                    );

                    // Copy ctx.result[0] or 0 into free input
                    *code += &format!(
                        "\tmov {}, qword {}[{} + {}*8] {}\n",
                        REG_AUX,
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_RESULT_SIZE,
                        ctx.comment_str("aux = ctx.result_size")
                    );
                    *code += &format!("\tcmp {REG_AUX}, 0\n");
                    *code += &format!("\tjz pc_{:x}_fcall_result_zero\n", ctx.pc);
                    *code += &format!(
                        "\tmov {}, qword {}[{} + {}*8] {}\n",
                        REG_VALUE,
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_RESULT,
                        ctx.comment_str("value = ctx.result[0]")
                    );
                    *code += &format!(
                        "\tmov [{}], {} {}\n",
                        REG_ADDRESS,
                        REG_VALUE,
                        ctx.comment_str("free_input = value")
                    );
                    *code += &format!("\tjmp pc_{:x}_fcall_result_done\n", ctx.pc);
                    *code += &format!("pc_{:x}_fcall_result_zero:\n", ctx.pc);
                    *code += &format!(
                        "\tmov qword {}[{}], 0 {}\n",
                        ctx.ptr,
                        REG_ADDRESS,
                        ctx.comment_str("free_input = 0")
                    );
                    *code += &format!("pc_{:x}_fcall_result_done:\n", ctx.pc);

                    // Update fcall counters
                    *code += &format!(
                        "\tmov qword {}[{} + {}*8], 0 {}\n",
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_PARAMS_SIZE,
                        ctx.comment_str("ctx.params_size = 0")
                    );
                    *code += &format!(
                        "\tmov qword {}[{} + {}*8], 1 {}\n",
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_RESULT_GOT,
                        ctx.comment_str("ctx.result_got = 1")
                    );
                }

                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::FcallGet => {
                *code += &ctx.full_line_comment("FcallGet".to_string());

                assert!(ctx.store_b_in_c);

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Get value from fcall_ctx.result[got] and store it in free input address
                    *code += &format!(
                        "\tmov {}, qword {}[{} + {}*8] {}\n",
                        REG_AUX,
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_RESULT_GOT,
                        ctx.comment_str("aux = ctx.result_got")
                    );
                    *code += &format!(
                        "\tmov {}, qword {}[{} + {}*8 + {}*8] {}\n",
                        REG_VALUE,
                        ctx.ptr,
                        ctx.fcall_ctx,
                        REG_AUX,
                        FCALL_RESULT,
                        ctx.comment_str("value = ctx.result_got")
                    );
                    *code += &format!(
                        "\tmov {}, {} {}\n",
                        REG_ADDRESS,
                        FREE_INPUT_ADDR,
                        ctx.comment_str("address = free_input")
                    );
                    *code += &format!(
                        "\tmov qword {}[{}], {} {}\n",
                        ctx.ptr,
                        REG_ADDRESS,
                        REG_VALUE,
                        ctx.comment_str("free_input = value")
                    );
                    *code += &format!(
                        "\tinc qword {}[{} + {}*8] {}\n",
                        ctx.ptr,
                        ctx.fcall_ctx,
                        FCALL_RESULT_GOT,
                        ctx.comment_str("inc ctx.result_got")
                    );
                }

                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bn254CurveAdd => {
                *code += &ctx.full_line_comment("Bn254CurveAdd".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bn254curveadd_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bn254curveadd_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bn254curveadd_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[8, 8]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bn254curveadd_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[8, 8], 0, 0, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bn254_curve_add function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bn254_curve_add\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 18*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 18*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bn254CurveDbl => {
                *code += &ctx.full_line_comment("Bn254CurveDbl".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Copy read data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bn254curvedbl_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bn254curvedbl_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bn254curvedbl_active_chunk:\n", ctx.pc);
                    }
                    *code += &format!("\tmov {REG_ADDRESS}, rdi\n");
                    for k in 0..8 {
                        *code += &format!(
                            "\tmov {}, [{} + {}] {}\n",
                            REG_VALUE,
                            REG_ADDRESS,
                            k * 8,
                            ctx.comment(format!("value = mem[address[{k}]]"))
                        );
                        *code += &format!(
                            "\tmov [{} + {}*8 + {}], {} {}\n",
                            REG_MEM_READS_ADDRESS,
                            REG_MEM_READS_SIZE,
                            k * 8,
                            REG_VALUE,
                            ctx.comment(format!("mem_reads[{k}] = value"))
                        );
                    }

                    // Increment chunk.steps.mem_reads_size in 8 units
                    *code += &format!(
                        "\tadd {}, 8 {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size += 8")
                    );
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bn254curvedbl_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_array(ctx, code, "rdi", false, 8, 8);
                    Self::mem_op_array(ctx, code, "rdi", true, 8, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bn254_curve_dbl function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bn254_curve_dbl\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 8*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 8*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bn254ComplexAdd => {
                *code += &ctx.full_line_comment("Bn254ComplexAdd".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bn254complexadd_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bn254complexadd_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bn254complexadd_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[8, 8]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bn254complexadd_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[8, 8], 0, 0, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bn254_complex_add function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bn254_complex_add\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 18*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 18*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bn254ComplexSub => {
                *code += &ctx.full_line_comment("Bn254ComplexSub".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bn254complexsub_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bn254complexsub_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bn254complexsub_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[8, 8]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bn254complexsub_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[8, 8], 0, 0, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bn254_complex_sub function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bn254_complex_sub\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 18*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 18*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bn254ComplexMul => {
                *code += &ctx.full_line_comment("Bn254ComplexMul".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bn254complexmul_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bn254complexmul_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bn254complexmul_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[8, 8]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bn254complexmul_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[8, 8], 0, 0, 8);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bn254_complex_mul function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bn254_complex_mul\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 18*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 18*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Halt => {
                *code +=
                    &format!("\tmov {}, 1 {}\n", ctx.mem_error, ctx.comment_str("halt: error = 1"));
                ctx.c.is_constant = true;
                ctx.c.constant_value = 0;
                ctx.c.string_value = "0".to_string();
                ctx.c.is_saved = true;
                ctx.flag_is_always_one = true;
            }
            ZiskOp::Arith384Mod => {
                *code += &ctx.full_line_comment("Arith384Mod".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_arith384mod_active_chunk\n", ctx.pc);
                        *code += &format!("\tjmp pc_{:x}_arith384mod_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_arith384mod_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 5, &[6, 6, 6, 6]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_arith384mod_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 5, &[6, 6, 6, 6], 4, 4, 6);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the arith384_mod function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_arith384_mod\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 29*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 29*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bls12_381CurveAdd => {
                *code += &ctx.full_line_comment("Bls12_381CurveAdd".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bls12_381curveadd_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bls12_381curveadd_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bls12_381curveadd_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[12, 12]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bls12_381curveadd_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[12, 12], 0, 0, 12);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bls12_381_curve_add function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bls12_381_curve_add\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 22*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 22*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bls12_381CurveDbl => {
                *code += &ctx.full_line_comment("Bls12_381CurveDbl".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Copy read data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_bls12_381curvedbl_active_chunk\n", ctx.pc);
                        *code +=
                            &format!("\tjmp pc_{:x}_bls12_381curvedbl_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_bls12_381curvedbl_active_chunk:\n", ctx.pc);
                    }
                    *code += &format!("\tmov {REG_ADDRESS}, rdi\n");
                    for k in 0..12 {
                        *code += &format!(
                            "\tmov {}, [{} + {}] {}\n",
                            REG_VALUE,
                            REG_ADDRESS,
                            k * 8,
                            ctx.comment(format!("value = mem[address[{k}]]"))
                        );
                        *code += &format!(
                            "\tmov [{} + {}*8 + {}], {} {}\n",
                            REG_MEM_READS_ADDRESS,
                            REG_MEM_READS_SIZE,
                            k * 8,
                            REG_VALUE,
                            ctx.comment(format!("mem_reads[{k}] = value"))
                        );
                    }

                    // Increment chunk.steps.mem_reads_size in 8 units
                    *code += &format!(
                        "\tadd {}, 12 {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size += 12")
                    );
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_bls12_381curvedbl_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_array(ctx, code, "rdi", false, 8, 12);
                    Self::mem_op_array(ctx, code, "rdi", true, 8, 12);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bls12_381_curve_dbl function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bls12_381_curve_dbl\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 12*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 12*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bls12_381ComplexAdd => {
                *code += &ctx.full_line_comment("Bls12_381ComplexAdd".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code +=
                            &format!("\tjnz pc_{:x}_bls12_381complexadd_active_chunk\n", ctx.pc);
                        *code += &format!(
                            "\tjmp pc_{:x}_bls12_381complexadd_active_chunk_done\n",
                            ctx.pc
                        );
                        *code += &format!("pc_{:x}_bls12_381complexadd_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[12, 12]);
                    if ctx.zip() {
                        *code +=
                            &format!("pc_{:x}_bls12_381complexadd_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[12, 12], 0, 0, 12);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bls12_381_complex_add function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bls12_381_complex_add\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 26*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 26*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bls12_381ComplexSub => {
                *code += &ctx.full_line_comment("Bls12_381ComplexSub".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code +=
                            &format!("\tjnz pc_{:x}_bls12_381complexsub_active_chunk\n", ctx.pc);
                        *code += &format!(
                            "\tjmp pc_{:x}_bls12_381complexsub_active_chunk_done\n",
                            ctx.pc
                        );
                        *code += &format!("pc_{:x}_bls12_381complexsub_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[12, 12]);
                    if ctx.zip() {
                        *code +=
                            &format!("pc_{:x}_bls12_381complexsub_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[12, 12], 0, 0, 12);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bls12_381_complex_sub function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bls12_381_complex_sub\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 26*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 26*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Bls12_381ComplexMul => {
                *code += &ctx.full_line_comment("Bls12_381ComplexMul".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code +=
                            &format!("\tjnz pc_{:x}_bls12_381complexmul_active_chunk\n", ctx.pc);
                        *code += &format!(
                            "\tjmp pc_{:x}_bls12_381complexmul_active_chunk_done\n",
                            ctx.pc
                        );
                        *code += &format!("pc_{:x}_bls12_381complexmul_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 2, &[12, 12]);
                    if ctx.zip() {
                        *code +=
                            &format!("pc_{:x}_bls12_381complexmul_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 2, &[12, 12], 0, 0, 12);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the bls12_381_complex_mul function
                    Self::push_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_bls12_381_complex_mul\n";
                    Self::pop_internal_registers(ctx, code, false);
                    //Self::assert_rsp_is_aligned(ctx, code);
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 26*8 {}\n",
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 26*8")
                    );
                }

                // Set result
                *code += &format!("\txor {}, {} {}\n", REG_C, REG_C, ctx.comment_str("c = 0"));
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Add256 => {
                *code += &ctx.full_line_comment("Add256".to_string());

                // Use the memory address as the first and unique parameter
                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    *code += &format!(
                        "\tmov rdi, {} {}\n",
                        ctx.b.string_value,
                        ctx.comment_str("rdi = b = address")
                    );
                }

                // Save data into mem_reads
                if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                    // If zip, check if chunk is active
                    if ctx.zip() {
                        *code += &format!(
                            "\ttest {}, 1 {}\n",
                            REG_ACTIVE_CHUNK,
                            ctx.comment_str("active_chunk == 1 ?")
                        );
                        *code += &format!("\tjnz pc_{:x}_add256_active_chunk\n", ctx.pc);
                        *code += &format!("\tjmp pc_{:x}_add256_active_chunk_done\n", ctx.pc);
                        *code += &format!("pc_{:x}_add256_active_chunk:\n", ctx.pc);
                    }
                    Self::precompiled_save_mem_reads(ctx, code, 4, &[4, 4]);
                    if ctx.zip() {
                        *code += &format!("pc_{:x}_add256_active_chunk_done:\n", ctx.pc);
                    }
                }

                // Consume mem reads
                if ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tmov [{} + {}*8], {} {}\n",
                        REG_MEM_READS_ADDRESS,
                        REG_MEM_READS_SIZE,
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("Main[4] = precompiler data address")
                    );
                    *code += &format!(
                        "\tinc {} {}\n",
                        REG_MEM_READS_SIZE,
                        ctx.comment_str("mem_reads_size++")
                    );
                }
                if ctx.chunk_player_mt_collect_mem() || ctx.chunk_player_mem_reads_collect_main() {
                    *code += &format!(
                        "\tadd {}, 13*8 {}\n", // 13 = @a, @b, cin, @c, a[0..4], b[0..4], cout
                        REG_CHUNK_PLAYER_ADDRESS,
                        ctx.comment_str("chunk_address += 13*8")
                    );
                }

                // Save memory operations into mem_reads
                if ctx.mem_op() {
                    Self::mem_op_precompiled_read_and_write(ctx, code, 4, &[4, 4], 3, 3, 4);
                }

                if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main()
                {
                    // Call the add256 function
                    Self::push_internal_registers_except_c_and_flag(ctx, code, false);
                    // Self::assert_rsp_is_aligned(ctx, code);
                    *code += "\tcall _opcode_add256\n";
                    *code += &format!("\tmov {}, rax {}\n", REG_C, ctx.comment_str("c = rax"));
                    *code +=
                        &format!("\tmov {}, rax {}\n", REG_FLAG, ctx.comment_str("flag = rax"));
                    Self::pop_internal_registers_except_c_and_flag(ctx, code, false);

                    // this precompiles store the result in minimal trace
                    if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                        Self::precompiled_save_result(ctx, code);
                    }
                    // Self::assert_rsp_is_aligned(ctx, code);
                } else {
                    // Set REG_C, REG_FLAGS from last value of mt
                    Self::mem_op_precompiled_restore_c_and_flags(ctx, code);
                }

                // Set result
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = false;
            }
        }
    }

    /**********/
    /* SET PC */
    /**********/

    fn set_pc(
        ctx: &mut ZiskAsmContext,
        instruction: &ZiskInst,
        code: &mut String,
        id: &str,
        rom: &ZiskRom,
    ) {
        ctx.jump_to_dynamic_pc = false;
        ctx.jump_to_static_pc = String::new();
        if instruction.set_pc {
            *code += &ctx.full_line_comment("set pc".to_string());
            if ctx.c.is_constant {
                let new_pc = (ctx.c.constant_value as i64 + instruction.jmp_offset1) as u64;
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_PC,
                    new_pc,
                    ctx.comment_str("pc = c(const) + jmp_offset1")
                );
                // Check if target address exists in ROM before generating static jump
                if rom.sorted_pc_list.binary_search(&new_pc).is_ok() {
                    ctx.jump_to_static_pc =
                        format!("\tjmp pc_{:x} {}\n", new_pc, ctx.comment_str("jump to static pc"));
                } else {
                    ctx.jump_to_dynamic_pc = true;
                }
            } else {
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_PC,
                    ctx.c.string_value,
                    ctx.comment_str("pc = c")
                );
                if instruction.jmp_offset1 != 0 {
                    *code += &format!(
                        "\tadd {}, 0x{:x} {}\n",
                        REG_PC,
                        instruction.jmp_offset1,
                        ctx.comment_str("pc += jmp_offset1")
                    );
                }
                ctx.jump_to_dynamic_pc = true;
            }
        } else if ctx.flag_is_always_zero {
            let new_pc = (ctx.pc as i64 + instruction.jmp_offset2) as u64;
            if new_pc != ctx.next_pc {
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_PC,
                    new_pc,
                    ctx.comment_str("flag=0: pc+=jmp_offset2")
                );
                // Check if target address exists in ROM before generating static jump
                if rom.sorted_pc_list.binary_search(&new_pc).is_ok() {
                    ctx.jump_to_static_pc = format!(
                        "\tjmp pc_{:x} {}\n",
                        new_pc,
                        ctx.comment_str("jump to pc+offset2")
                    );
                } else {
                    ctx.jump_to_dynamic_pc = true;
                }
            } else if id == "z" {
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_PC,
                    ctx.next_pc,
                    ctx.comment_str("flag=0: pc += 4")
                );
            }
        } else if ctx.flag_is_always_one {
            let new_pc = (ctx.pc as i64 + instruction.jmp_offset1) as u64;
            if new_pc != ctx.next_pc {
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_PC,
                    new_pc,
                    ctx.comment_str("flag=1: pc+=jmp_offset1")
                );
                // Check if target address exists in ROM before generating static jump
                if rom.sorted_pc_list.binary_search(&new_pc).is_ok() {
                    ctx.jump_to_static_pc = format!(
                        "\tjmp pc_{:x} {}\n",
                        new_pc,
                        ctx.comment_str("jump to pc+offset1")
                    );
                } else {
                    ctx.jump_to_dynamic_pc = true;
                }
            } else if id == "z" {
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_PC,
                    ctx.next_pc,
                    ctx.comment_str("flag=1: pc += 4")
                );
            }
        } else {
            *code += &ctx.full_line_comment("pc = f(flag)".to_string());
            // Calculate the new pc
            *code += &format!("\tcmp {}, 1 {}\n", REG_FLAG, ctx.comment_str("flag == 1 ?"));
            *code += &format!("\tjne pc_{:x}_{}_flag_false\n", ctx.pc, id);
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_PC,
                (ctx.pc as i64 + instruction.jmp_offset1) as u64,
                ctx.comment_str("pc += i.jmp_offset1")
            );
            *code += &format!("\tjmp pc_{:x}_{}_flag_done\n", ctx.pc, id);
            *code += &format!("pc_{:x}_{}_flag_false:\n", ctx.pc, id);
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_PC,
                (ctx.pc as i64 + instruction.jmp_offset2) as u64,
                ctx.comment_str("pc += i.jmp_offset2")
            );
            *code += &format!("pc_{:x}_{}_flag_done:\n", ctx.pc, id);
            ctx.jump_to_dynamic_pc = true;
        }
    }

    fn jumpt_to_dynamic_pc(ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += &ctx.full_line_comment("jump to dynamic pc".to_string());
        // When executing program code, it can dynamically jump to any BIOS instruction
        // (low address) or to any program code address (high address).
        // When executing zisk float library code, it can dynamically jump to any BIOS instruction
        // (low address) or to any float library code address (high address) but not to program
        // code addresses.
        let high_address =
            if ctx.pc < FLOAT_LIB_ROM_ADDR { ctx.min_program_pc } else { FLOAT_LIB_ROM_ADDR };
        *code += &format!(
            "\tmov {}, 0x{:x} {}\n",
            REG_ADDRESS,
            high_address,
            ctx.comment_str("is pc a low address?")
        );
        *code += &format!("\tcmp {REG_PC}, {REG_ADDRESS}\n");
        *code += &format!("\tjb pc_{:x}_jump_to_low_address\n", ctx.pc);
        *code += &format!(
            "\tsub {}, {} {}\n",
            REG_PC,
            REG_ADDRESS,
            ctx.comment_str(&format!("pc -= 0x{:x}", ctx.min_program_pc))
        );
        *code += &format!(
            "\tlea {}, [map_pc_{:x}] {}\n",
            REG_ADDRESS,
            high_address,
            ctx.comment_str(&format!("address = map[0x{:x}]", high_address))
        );
        *code += &format!(
            "\tmov {}, [{} + {}*{}] {}\n",
            REG_ADDRESS,
            REG_ADDRESS,
            REG_PC,
            if ctx.jump_to_unaligned_pc() { 8 } else { 2 },
            ctx.comment_str("address = map[pc]")
        );
        *code += &format!("\tjmp {} {}\n", REG_ADDRESS, ctx.comment_str("jump to address"));
        *code += &format!("pc_{:x}_jump_to_low_address:\n", ctx.pc);
        *code += &format!("\tsub {}, 0x1000 {}\n", REG_PC, ctx.comment_str("pc -= 0x1000"));
        *code += &format!(
            "\tlea {}, [map_pc_1000] {}\n",
            REG_ADDRESS,
            ctx.comment_str("address = map[0x1000]")
        );
        *code += &format!(
            "\tmov {}, [{} + {}*2] {}\n",
            REG_ADDRESS,
            REG_ADDRESS,
            REG_PC,
            ctx.comment_str("address = map[pc]")
        );
        *code += &format!("\tjmp {} {}\n", REG_ADDRESS, ctx.comment_str("jump to address"));
    }

    /*************/
    /* MEM READS */
    /*************/

    fn a_src_mem_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            if ctx.store_a_in_c { REG_C } else { REG_A },
            ctx.comment_str("mem_reads[@+size*8] = a")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn a_src_mem_not_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
            REG_ADDRESS,
            ctx.comment_str("address = previous aligned address")
        );

        // Store previous aligned address value in mem_reads
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            REG_VALUE,
            REG_ADDRESS,
            ctx.comment_str("value = mem[prev_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8] = prev_a")
        );

        // Store next aligned address value in mem_reads
        *code += &format!(
            "\tmov {}, [{} + 8] {}\n",
            REG_VALUE,
            REG_ADDRESS,
            ctx.comment_str("value = mem[prev_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8+8] = next_a")
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!(
            "\tadd {}, 2 {}\n",
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size += 2")
        );
    }

    fn b_src_mem_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            if ctx.store_b_in_c { REG_C } else { REG_B },
            ctx.comment_str("mem_reads[@+size*8] = b")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn b_src_mem_not_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
            REG_ADDRESS,
            ctx.comment_str("address = previous aligned address")
        );

        // Store previous aligned address value in mem_reads, and advance address
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            REG_VALUE,
            REG_ADDRESS,
            ctx.comment_str("value = mem[prev_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_address[@+size*8] = prev_b")
        );

        // Store next aligned address value in mem_reads, and advance address
        *code += &format!(
            "\tmov {}, [{} + 8] {}\n",
            REG_VALUE,
            REG_ADDRESS,
            ctx.comment_str("value = mem[prev_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8+8] = next_b")
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!(
            "\tadd {}, 2 {}\n",
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size += 2")
        );
    }

    fn c_store_mem_not_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Get a copy of the address to preserve it
        *code +=
            &format!("\tmov {}, {} {}\n", REG_AUX, REG_ADDRESS, ctx.comment_str("aux = address"));

        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
            REG_AUX,
            ctx.comment_str("address = previous aligned address")
        );

        // Store previous aligned address value in mem_reads, and advance address
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            REG_VALUE,
            REG_AUX,
            ctx.comment_str("value = mem[prev_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8] = prev_c")
        );

        // Store next aligned address value in mem_reads, and advance address
        *code += &format!(
            "\tmov {}, [{} + 8] {}\n",
            REG_VALUE,
            REG_AUX,
            ctx.comment_str("value = mem[next_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8 +  8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8+8] = next_c")
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!(
            "\tadd {}, 2 {}\n",
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size += 2")
        );
    }

    fn c_store_ind_8_not_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Get a copy of the address to preserve it
        *code +=
            &format!("\tmov {}, {} {}\n", REG_AUX, REG_ADDRESS, ctx.comment_str("aux = address"));

        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 {}\n",
            REG_AUX,
            ctx.comment_str("aux = previous aligned address")
        );

        // Store previous aligned address value in mem_reads, and advance address
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            REG_VALUE,
            REG_AUX,
            ctx.comment_str("value = mem[prev_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8] = prev_c")
        );

        // Store next aligned address value in mem_reads, and advance it
        *code += &format!(
            "\tmov {}, [{} + 8] {}\n",
            REG_VALUE,
            REG_AUX,
            ctx.comment_str("value = mem[next_address]")
        );
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_VALUE,
            ctx.comment_str("mem_reads[@+size*8+8] = next_c")
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!(
            "\tadd {}, 2 {}\n",
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size += 2")
        );
    }

    fn precompiled_save_mem_reads(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        params_count: u64,
        load_sizes: &[usize],
    ) {
        // This index will be incremented as we insert data into mem_reads
        let mut mem_reads_index: u64 = 0;

        // We get a copy of the precompiled data address
        *code += &format!("\tmov {}, rdi {}\n", REG_ADDRESS, ctx.comment_str("address = rdi"));

        for i in 0..params_count {
            // Store next aligned address value in mem_reads, and advance it
            *code += &format!(
                "\tmov {}, [{} + {}*8] {}\n",
                REG_VALUE,
                REG_ADDRESS,
                i,
                ctx.comment(format!("value = mem[address+{i}]"))
            );

            *code += &format!(
                "\tmov [{} + {}*8 + {}*8], {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_MEM_READS_SIZE,
                mem_reads_index,
                REG_VALUE,
                ctx.comment_str("mem_reads[@+size*8+ind*8] = param")
            );
            mem_reads_index += 1;
        }

        for (i, size) in load_sizes.iter().enumerate() {
            // Store next aligned address value in mem_reads, and advance it
            *code += &format!(
                "\tmov {}, [{} + {}*8] {}\n",
                REG_VALUE,
                REG_ADDRESS,
                i,
                ctx.comment(format!("value = mem[address+{i}]"))
            );

            // For each chunk of the indirection, store it in mem_reads
            for l in 0..*size {
                *code += &format!(
                    "\tmov {}, [{} + {}*8] {}\n",
                    REG_AUX,
                    REG_VALUE,
                    l,
                    ctx.comment(format!("aux = mem[ind+{l}]"))
                );
                *code += &format!(
                    "\tmov [{} + {}*8 + {}*8], {} {}\n",
                    REG_MEM_READS_ADDRESS,
                    REG_MEM_READS_SIZE,
                    mem_reads_index,
                    REG_AUX,
                    ctx.comment_str("mem_reads[@+size*8+ind*8] = ind")
                );
                mem_reads_index += 1;
            }
        }

        // Increment chunk.steps.mem_reads_size
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_MEM_READS_SIZE,
            mem_reads_index,
            ctx.comment(format!("mem_reads_size+={mem_reads_index}"))
        );
    }

    fn precompiled_save_result(ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_C,
            ctx.comment_str("mem_reads[@+size*8] = C")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!(
            "\tinc {} {}\n",
            REG_MEM_READS_SIZE,
            ctx.comment("++mem_reads_size".to_string())
        );
    }

    /*********************/
    /* MEMORY OPERATIONS */
    /*********************/

    fn a_src_mem_op(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate the trace value on top of the address
        const WIDTH: u64 = 8;
        if ctx.address_is_constant {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_ADDRESS,
                (WIDTH << F_MEM_WIDTH_SHIFT) | ctx.address_constant_value,
                ctx.comment_str("aux = constant mem op")
            );
        } else {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                WIDTH << F_MEM_WIDTH_SHIFT,
                ctx.comment_str("aux = mem op mask")
            );
            *code += &format!(
                "\tor {}, {} {}\n",
                REG_ADDRESS,
                REG_AUX,
                ctx.comment_str("address |= mem op mask")
            );
        }

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_ADDRESS,
            ctx.comment_str("mem_reads[@+size*8] = mem op")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn b_src_mem_op(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate the trace value on top of the address
        const WIDTH: u64 = 8;
        if ctx.address_is_constant {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_ADDRESS,
                (WIDTH << F_MEM_WIDTH_SHIFT) | ctx.address_constant_value,
                ctx.comment_str("aux = constant mem op")
            );
        } else {
            // Calculate the trace value on top of the address
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                WIDTH << F_MEM_WIDTH_SHIFT,
                ctx.comment_str("aux = mem op mask")
            );
            *code += &format!(
                "\tor {}, {} {}\n",
                REG_ADDRESS,
                REG_AUX,
                ctx.comment_str("address |= mem op mask")
            );
        }

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_ADDRESS,
            ctx.comment_str("mem_reads[@+size*8] = mem op")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn b_src_ind_mem_op(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        reg_address: &str,
        width: u64,
    ) {
        if ctx.address_is_constant {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                reg_address,
                (width << F_MEM_WIDTH_SHIFT) + ctx.address_constant_value,
                ctx.comment_str("aux = constant mem op")
            );
        } else {
            // Calculate the trace value on top of the address
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                width << F_MEM_WIDTH_SHIFT,
                ctx.comment_str("aux = mem op mask")
            );
            *code += &format!(
                "\tor {}, {} {}\n",
                reg_address,
                REG_AUX,
                ctx.comment_str("address |= mem op mask")
            );
        }

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            reg_address,
            ctx.comment_str("mem_reads[@+size*8] = mem op")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn c_store_mem_mem_op(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate the trace value on top of the address
        const WRITE: u64 = 1;
        const WIDTH: u64 = 8;
        if ctx.address_is_constant {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_ADDRESS,
                (WRITE << F_MEM_WRITE_SHIFT)
                    + (WIDTH << F_MEM_WIDTH_SHIFT)
                    + ctx.address_constant_value,
                ctx.comment_str("aux = constant mem op")
            );
        } else {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                (WRITE << F_MEM_WRITE_SHIFT) + (WIDTH << F_MEM_WIDTH_SHIFT),
                ctx.comment_str("aux = mem op mask")
            );
            *code += &format!(
                "\tor {}, {} {}\n",
                REG_ADDRESS,
                REG_AUX,
                ctx.comment_str("address |= mem op mask")
            );
        }

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_ADDRESS,
            ctx.comment_str("mem_reads[@+size*8] = mem op")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn c_store_ind_mem_op(ctx: &mut ZiskAsmContext, code: &mut String, width: u64) {
        // Dynamic trace value: if rest of bytes were zero, set flag on bit F_MEM_CLEAR_WRITE_BYTE
        // With this information, the mem_planner can use a specific state machine for
        // this kind of byte writes
        if width == 1 {
            *code += &format!("\tmov {}, {} {}\n", REG_VALUE, REG_C, ctx.comment_str("value = c"));
        }

        // Calculate the fixed trace value adding write (bit 36) and width (bits 32-35) on top of
        // the address
        if ctx.address_is_constant {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_ADDRESS,
                F_MEM_WRITE | (width << F_MEM_WIDTH_SHIFT) | ctx.address_constant_value,
                ctx.comment_str("aux = constant mem op")
            );
        } else {
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                F_MEM_WRITE | (width << F_MEM_WIDTH_SHIFT),
                ctx.comment_str("aux = mem op mask")
            );
            *code += &format!(
                "\tor {}, {} {}\n",
                REG_ADDRESS,
                REG_AUX,
                ctx.comment_str("address |= mem op mask")
            );
        }

        // Dynamic trace value: if rest of bytes were zero, set flag on bit F_MEM_CLEAR_WRITE_BYTE
        if width == 1 {
            *code += &format!(
                "\tshr {}, 8 {}\n",
                REG_VALUE,
                ctx.comment_str("value & 0xFFFFFF00 == 0 ?")
            );
            *code += &format!(
                "\tjnz pc_{}_rest_of_bytes_not_zero {}\n",
                ctx.pc,
                ctx.comment_str("aux & 0xFFFFFF00 != 0 ?")
            );
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                F_MEM_CLEAR_WRITE_BYTE,
                ctx.comment_str("aux = F_MEM_CLEAR_WRITE_BYTE")
            );
            *code += &format!(
                "\tor {}, {} {}\n",
                REG_ADDRESS,
                REG_AUX,
                ctx.comment_str("address |= F_MEM_CLEAR_WRITE_BYTE")
            );
            *code += &format!("\npc_{}_rest_of_bytes_not_zero:\n", ctx.pc);
        }

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_ADDRESS,
            ctx.comment_str("mem_reads[@+size*8] = mem op")
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    fn mem_op_array(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        reg_address: &str,
        _write: bool,
        width: u64,
        length: u64,
    ) {
        let write: u64 = if _write { 1 } else { 0 };
        let mem_op_mask: u64 = (write << F_MEM_WRITE_SHIFT) | (width << F_MEM_WIDTH_SHIFT);

        // Get a copy of the address register
        *code += &format!(
            "\tmov {}, {} {}\n",
            REG_VALUE,
            reg_address,
            ctx.comment_str("value = address")
        );

        // Calculate the mask
        *code += &format!(
            "\tmov {}, 0x{:x} {}\n",
            REG_AUX,
            mem_op_mask,
            ctx.comment_str("aux = mem op mask + offset")
        );

        // Add the mask to the address
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_VALUE,
            REG_AUX,
            ctx.comment_str("value |= mem op mask")
        );

        // Iterate for all memory operations
        for i in 0..length {
            // Copy read data into mem_reads_address and increment it
            *code += &format!(
                "\tmov [{} + {}*8 + {}*8], {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_MEM_READS_SIZE,
                i,
                REG_VALUE,
                ctx.comment_str("mem_reads[@+size*8] = mem op")
            );

            if i != (length - 1) {
                // Get a copy of the address register
                *code += &format!("\tadd {}, 8 {}\n", REG_VALUE, ctx.comment_str("value += 8"));
            }
        }
        // Increment chunk.steps.mem_reads_size
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_MEM_READS_SIZE,
            length,
            ctx.comment_str("mem_reads_size += length")
        );
    }

    fn internal_mem_op_precompiled_read(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        params_count: u64,
        load_sizes: &[usize],
        update_index: bool,
    ) -> u64 {
        // Calculate the mask
        let mem_op_mask: u64 = 8u64 << 32;

        // This index will be incremented as we insert data into mem_reads
        let mut mem_reads_index: u64 = 0;

        // We get a copy of the precompiled data address
        *code += &format!("\tmov {}, rdi {}\n", REG_ADDRESS, ctx.comment_str("address = rdi"));

        for i in 0..params_count {
            // Store next aligned address value in mem_reads, and advance it
            *code += &format!(
                "\tmov {}, [{} + {}*8] {}\n",
                REG_VALUE,
                REG_ADDRESS,
                i,
                ctx.comment(format!("value = mem[address+{i}]"))
            );

            // Load the mask + offset
            *code += &format!(
                "\tmov {}, 0x{:x} {}\n",
                REG_AUX,
                mem_op_mask + 8 * i,
                ctx.comment_str("aux = mem op mask + offset")
            );

            // Add the address
            *code += &format!(
                "\tadd {}, {} {}\n",
                REG_AUX,
                REG_ADDRESS,
                ctx.comment_str("aux += address")
            );

            // Store it in the trace
            *code += &format!(
                "\tmov [{} + {}*8 + {}*8], {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_MEM_READS_SIZE,
                mem_reads_index,
                REG_AUX,
                ctx.comment_str("mem_reads[@+size*8+ind*8] = mem_op")
            );
            mem_reads_index += 1;
        }

        for (i, size) in load_sizes.iter().enumerate() {
            // Store next aligned address value in mem_reads, and advance it
            *code += &format!(
                "\tmov {}, [{} + {}*8] {}\n",
                REG_VALUE,
                REG_ADDRESS,
                i,
                ctx.comment(format!("value = mem[address+{i}]"))
            );

            // Store the first load_count iterations
            // load_size elements in mem_reads

            // For each chunk of the indirection, store it in mem_reads
            for l in 0..*size {
                // Load the mask + offset
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_AUX,
                    mem_op_mask + 8 * (l as u64),
                    ctx.comment_str("aux = mem op mask + offset")
                );

                // Add the address
                *code += &format!(
                    "\tadd {}, {} {}\n",
                    REG_AUX,
                    REG_VALUE,
                    ctx.comment_str("aux += address")
                );

                // Store it in the trace
                *code += &format!(
                    "\tmov [{} + {}*8 + {}*8], {} {}\n",
                    REG_MEM_READS_ADDRESS,
                    REG_MEM_READS_SIZE,
                    mem_reads_index,
                    REG_AUX,
                    ctx.comment_str("mem_reads[@+size*8+ind*8] = mem_op")
                );
                mem_reads_index += 1;
            }
        }
        if update_index {
            // Increment chunk.steps.mem_reads_size
            *code += &format!(
                "\tadd {}, {} {}\n",
                REG_MEM_READS_SIZE,
                mem_reads_index,
                ctx.comment(format!("mem_reads_size+={mem_reads_index}"))
            );
        }
        mem_reads_index
    }

    fn mem_op_precompiled_read_and_write(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        params_count: u64,
        load_sizes: &[usize],
        begin: u64,
        end: u64,
        write_size: u64,
    ) {
        let mem_reads_index =
            Self::internal_mem_op_precompiled_read(ctx, code, params_count, load_sizes, false);
        Self::internal_mem_op_precompiled_write(ctx, code, begin, end, write_size, mem_reads_index);
    }

    #[inline(always)]
    fn internal_mem_op_precompiled_write(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        begin: u64,
        end: u64,
        load_size: u64,
        initial_mem_reads_index: u64,
    ) {
        // Calculate the mask
        let mem_op_mask: u64 = F_MEM_WRITE + (8u64 << F_MEM_WIDTH_SHIFT);

        // This index will be incremented as we insert data into mem_reads
        let mut mem_reads_index: u64 = initial_mem_reads_index;

        if initial_mem_reads_index == 0 {
            // We get a copy of the precompiled data address
            *code += &format!("\tmov {}, rdi {}\n", REG_ADDRESS, ctx.comment_str("address = rdi"));
        }
        // For every parameter
        for i in begin..=end {
            // Store next aligned address value in mem_reads, and advance it
            *code += &format!(
                "\tmov {}, [{} + {}*8] {}\n",
                REG_VALUE,
                REG_ADDRESS,
                i,
                ctx.comment(format!("value = mem[address+{i}]"))
            );

            // For each of the indirection parameter, store it in mem_reads
            for l in 0..load_size {
                // Load the mask + offset
                *code += &format!(
                    "\tmov {}, 0x{:x} {}\n",
                    REG_AUX,
                    mem_op_mask + 8 * l,
                    ctx.comment_str("aux = mem op mask + offset")
                );

                // Add the address
                *code += &format!(
                    "\tadd {}, {} {}\n",
                    REG_AUX,
                    REG_VALUE,
                    ctx.comment_str("aux += address")
                );

                // Store it in the trace
                *code += &format!(
                    "\tmov [{} + {}*8 + {}*8], {} {}\n",
                    REG_MEM_READS_ADDRESS,
                    REG_MEM_READS_SIZE,
                    mem_reads_index,
                    REG_AUX,
                    ctx.comment_str("mem_reads[@+size*8+ind*8] = mem_op")
                );
                mem_reads_index += 1;
            }
        }

        // Increment chunk.steps.mem_reads_size
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_MEM_READS_SIZE,
            mem_reads_index,
            ctx.comment(format!("mem_reads_size+={mem_reads_index}"))
        );
    }

    fn mem_op_precompiled_restore_c_and_flags(ctx: &mut ZiskAsmContext, code: &mut String) {
        // We get a copy of the precompiled data address
        *code += &format!("\tmov {}, rdi {}\n", REG_ADDRESS, ctx.comment_str("address = rdi"));

        // read last mem_read into c
        *code += &format!(
            "\tmov {}, [{} + {}*8 - 8] {}\n",
            REG_C,
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            ctx.comment_str("c = mem_reads[@+size*8+ind*8]")
        );
        *code += &format!("\tmov {}, {} {}\n", REG_FLAG, REG_C, ctx.comment_str("flag = c"));
    }

    /*******************/
    /* CHUNK START/END */
    /*******************/

    fn chunk_start(ctx: &mut ZiskAsmContext, code: &mut String, id: &str) {
        if ctx.zip() {
            *code += &ctx.full_line_comment(
                "If chunk_id & 0x7 == chunk_mask then activate this chunk".to_string(),
            );
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_AUX,
                ctx.mem_chunk_id,
                ctx.comment_str("aux = chunk_id")
            );
            *code += &format!("\tinc {} {}\n", ctx.mem_chunk_id, ctx.comment_str("chunk_id++"));
            *code += &format!("\tand {}, 0x7 {}\n", REG_AUX, ctx.comment_str("aux &= mask"));
            *code += &format!(
                "\tcmp {}, {} {}\n",
                REG_AUX,
                ctx.mem_chunk_mask,
                ctx.comment_str("aux ?= chunk_mask")
            );
            *code += &format!("\tjz chunk_start_{id}_activate\n");
            *code += &format!(
                "\txor {}, {} {}\n",
                REG_ACTIVE_CHUNK,
                REG_ACTIVE_CHUNK,
                ctx.comment_str("deactivate chunk")
            );
            *code += &format!("\tjmp chunk_start_{id}_done\n");
            *code += &format!("chunk_start_{id}_activate:\n");
            *code +=
                &format!("\tmov {}, 1 {}\n", REG_ACTIVE_CHUNK, ctx.comment_str("activate chunk"));
        }

        if !ctx.chunk_player_mt_collect_mem() && !ctx.chunk_player_mem_reads_collect_main() {
            *code += &ctx.full_line_comment(
                "Increment number of chunks (first position in trace)".to_string(),
            );
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_ADDRESS,
                ctx.mem_trace_address,
                ctx.comment_str("address = trace_addr")
            );
            *code += &format!(
                "\tmov {}, [{}] {}\n",
                REG_VALUE,
                REG_ADDRESS,
                ctx.comment_str("value = trace_addr")
            );
            *code += &format!("\tinc {} {}\n", REG_VALUE, ctx.comment_str("inc value"));
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("trace_addr = value (trace_addr++)")
            );
        }

        if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
            *code += &ctx.full_line_comment("Write chunk start data".to_string());

            // Write chunk.start.pc
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_ADDRESS,
                ctx.mem_chunk_address,
                ctx.comment_str("address = chunk_address")
            );

            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_PC,
                ctx.comment_str("chunk.start.pc = value")
            );

            // Write chunk.start.sp
            *code +=
                &format!("\tmov {}, {} {}\n", REG_VALUE, ctx.mem_sp, ctx.comment_str("value = sp"));
            *code += &format!("\tadd {}, 8 {}\n", REG_ADDRESS, ctx.comment_str("address += 8"));
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("chunk.start.sp = value = sp")
            );

            // Write chunk.start.c
            *code += &format!("\tadd {}, 8 {}\n", REG_ADDRESS, ctx.comment_str("address += 8"));
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_C,
                ctx.comment_str("chunk.start.c = c")
            );

            // Write chunk.start.step
            *code += &format!("\tadd {}, 8 {}\n", REG_ADDRESS, ctx.comment_str("address += 8"));
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_VALUE,
                ctx.mem_step,
                ctx.comment_str("value = step")
            );
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("chunk.start.step = value = step")
            );
            *code += &format!(
                "\tmov {}, {} {}\n",
                ctx.mem_chunk_start_step,
                REG_VALUE,
                ctx.comment_str("chunk.start.step = value = step")
            );

            // Write chunk.start.reg
            for i in 1..34 {
                Self::read_riscv_reg(ctx, code, i, REG_VALUE, "value");
                *code += &format!(
                    "\tmov [{} + {}], {} {}\n",
                    REG_ADDRESS,
                    i * 8,
                    REG_VALUE,
                    ctx.comment(format!("chunk.start.reg[{i}] = value"))
                );
            }
            *code +=
                &format!("\tadd {}, 33*8 {}\n", REG_ADDRESS, ctx.comment_str("address += 33*8"));
        }

        if ctx.minimal_trace() || ctx.main_trace() || ctx.zip() || ctx.mem_op() || ctx.mem_reads() {
            *code += &ctx.full_line_comment("Write mem reads size".to_string());
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_AUX,
                ctx.mem_chunk_address,
                ctx.comment_str("aux = chunk_size")
            );
            if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
                *code += &format!("\tadd {}, 40*8 {}\n", REG_AUX, ctx.comment_str("aux += 40*8"));
            }
            if ctx.mem_op() {
                // Skip chunk.end
                *code += &format!("\tadd {}, 8 {}\n", REG_AUX, ctx.comment_str("aux += 8"));
            }

            // Set mem reads size to all F's
            *code += &format!(
                "\tmov {}, 0xFFFFFFFFFFFFFFFF {}\n",
                REG_VALUE,
                ctx.comment_str("value = F's")
            );
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_AUX,
                REG_VALUE,
                ctx.comment_str("mem_reads_size = value")
            );
            *code += &format!("\tadd {}, 8 {}\n", REG_AUX, ctx.comment_str("aux += 8"));
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_AUX,
                ctx.comment_str("mem_reads_address = aux")
            );
            *code += &ctx.full_line_comment("Reset mem_reads size".to_string());
            *code += &format!(
                "\txor {}, {} {}\n",
                REG_MEM_READS_SIZE,
                REG_MEM_READS_SIZE,
                ctx.comment_str("mem_reads_size = 0")
            );
        }
        if ctx.zip() {
            *code += &format!("chunk_start_{id}_done:\n");
        }

        *code += &ctx.full_line_comment("Reset step count down to chunk_size".to_string());
        *code += &format!(
            "\tmov {}, chunk_size {}\n",
            REG_STEP,
            ctx.comment_str("step_count_down = chunk_size")
        );
    }

    fn chunk_end(ctx: &mut ZiskAsmContext, code: &mut String, id: &str) {
        *code += &ctx.full_line_comment("Update total step from step count down".to_string());
        *code +=
            &format!("\tmov {}, {} {}\n", REG_VALUE, ctx.mem_step, ctx.comment_str("value = step"));
        *code += &format!(
            "\tadd {}, chunk_size {}\n",
            REG_VALUE,
            ctx.comment_str("value += chunk_size")
        );
        *code += &format!(
            "\tsub {}, {} {}\n",
            REG_VALUE,
            REG_STEP,
            ctx.comment_str("value -= step_count_down")
        );
        *code +=
            &format!("\tmov {}, {} {}\n", ctx.mem_step, REG_VALUE, ctx.comment_str("step = value"));

        if ctx.zip() {
            *code += &ctx.full_line_comment("If active_chunk == 0 skip this chunk".to_string());
            *code += &format!(
                "\ttest {}, 1 {}\n",
                REG_ACTIVE_CHUNK,
                ctx.comment_str("active_chunk ?= 0")
            );
            *code += &format!("\tjnz chunk_end_{id}_active_chunk\n");
            *code += &format!("\tjmp chunk_end_{id}_done\n");
            *code += &format!("chunk_end_{id}_active_chunk:\n");
        }

        if ctx.minimal_trace() || ctx.zip() || ctx.mem_reads() {
            *code += &ctx.full_line_comment("Write chunk last data".to_string());

            // Search position of chunk.last
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_ADDRESS,
                ctx.mem_chunk_address,
                ctx.comment_str("address = chunk_address")
            );
            *code += &format!(
                "\tadd {}, 37*8 {}\n",
                REG_ADDRESS,
                ctx.comment_str("address = chunk_address + 37*8")
            );

            // Write chunk.last.c
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_C,
                ctx.comment_str("chunk.last.c = c")
            );

            *code += &ctx.full_line_comment("Write chunk end data".to_string());
            *code += &format!("\tadd {}, 8 {}\n", REG_ADDRESS, ctx.comment_str("address += 8"));

            // Write chunk.last.end
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_VALUE,
                ctx.mem_end,
                ctx.comment_str("value = end")
            );
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("chunk.end = value = end")
            );
            *code += &format!("\tadd {}, 8 {}\n", REG_ADDRESS, ctx.comment_str("address += 8"));

            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_VALUE,
                ctx.mem_step,
                ctx.comment_str("value = step")
            );
            *code += &format!(
                "\tsub {}, {} {}\n",
                REG_VALUE,
                ctx.mem_chunk_start_step,
                ctx.comment_str("value = step_inc")
            );
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("chunk.steps.step = value = step_inc")
            );

            // Write mem_reads_size
            *code += &format!(
                "\tadd {}, 8 {}\n",
                REG_ADDRESS,
                ctx.comment_str("address += 8 = mem_reads_size")
            ); // mem_reads_size

            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_MEM_READS_SIZE,
                ctx.comment_str("mem_reads_size = size")
            );

            // Get value = mem_reads_size*8, i.e. memory size till next chunk
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_VALUE,
                REG_MEM_READS_SIZE,
                ctx.comment_str("value = mem_reads_size")
            );
            *code += &format!("\tsal {}, 3 {}\n", REG_VALUE, ctx.comment_str("value <<= 3"));

            // Update chunk address
            *code += &format!(
                "\tadd {}, 8 {}\n",
                REG_ADDRESS,
                ctx.comment_str("address += 8 = new_chunk_address")
            );
            *code += &format!(
                "\tadd {}, {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("address += value = mem_reads_size*8")
            ); // new chunk
            *code += &format!(
                "\tmov {}, {} {}\n",
                ctx.mem_chunk_address,
                REG_ADDRESS,
                ctx.comment_str("chunk_address = new_chunk_address")
            );
        }

        if ctx.main_trace() || ctx.mem_op() {
            // Store chunk address in reg address
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_ADDRESS,
                ctx.mem_chunk_address,
                ctx.comment_str("address = chunk_address")
            );

            // Write chunk.end
            if ctx.mem_op() {
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_VALUE,
                    ctx.mem_end,
                    ctx.comment_str("value = end")
                );
                *code += &format!(
                    "\tmov [{}], {} {}\n",
                    REG_ADDRESS,
                    REG_VALUE,
                    ctx.comment_str("chunk.end = value = end")
                );
                *code += &format!("\tadd {}, 8 {}\n", REG_ADDRESS, ctx.comment_str("address += 8"));
            }

            // Write chunk.mem_reads_size
            *code += &format!(
                "\tmov [{}], {} {}\n",
                REG_ADDRESS,
                REG_MEM_READS_SIZE,
                ctx.comment_str("mem_reads_size = size")
            );
            *code += &format!(
                "\tadd {}, 8 {}\n",
                REG_ADDRESS,
                ctx.comment_str("address += 8 = new_chunk_address")
            );

            // Increase chunk address
            *code += &format!(
                "\tmov {}, {} {}\n",
                REG_VALUE,
                REG_MEM_READS_SIZE,
                ctx.comment_str("value = mem_reads_size")
            );
            *code += &format!("\tsal {}, 3 {}\n", REG_VALUE, ctx.comment_str("value <<= 3"));
            *code += &format!(
                "\tadd {}, {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("address += value = mem_reads_size*8")
            );
            *code += &format!(
                "\tmov {}, {} {}\n",
                ctx.mem_chunk_address,
                REG_ADDRESS,
                ctx.comment_str("chunk_address = new_chunk_address")
            );
        }

        if ctx.minimal_trace() || ctx.main_trace() || ctx.zip() || ctx.mem_op() || ctx.mem_reads() {
            *code += &ctx.full_line_comment("Realloc trace if threshold is passed".to_string());
            *code += &format!(
                "\tmov {}, qword {}[trace_address_threshold] {}\n",
                REG_VALUE,
                ctx.ptr,
                ctx.comment_str("value = trace_address_threshold")
            );
            *code += &format!(
                "\tcmp {}, {} {}\n",
                REG_ADDRESS,
                REG_VALUE,
                ctx.comment_str("chunk_address ? trace_address_threshold")
            );
            *code += &format!("\tjb chunk_{id}_address_below_threshold\n");
            Self::push_internal_registers(ctx, code, true);
            //Self::assert_rsp_is_aligned(ctx, code);
            *code += "\tcall _realloc_trace\n";
            if ctx.call_chunk_done {
                //Self::assert_rsp_is_aligned(ctx, code);
                *code += "\tcall _chunk_done\n";
            }
            //Self::assert_rsp_is_aligned(ctx, code);
            Self::pop_internal_registers(ctx, code, true);
            *code += &format!("\tjmp chunk_{id}_address_done\n");
            *code += &format!("chunk_{id}_address_below_threshold:\n");
            if ctx.call_chunk_done {
                Self::push_internal_registers(ctx, code, true);
                //Self::assert_rsp_is_aligned(ctx, code);
                *code += "\tcall _chunk_done\n";
                //Self::assert_rsp_is_aligned(ctx, code);
                Self::pop_internal_registers(ctx, code, true);
            }
            *code += &format!("chunk_{id}_address_done:\n");
        } else if ctx.call_chunk_done {
            // Call the chunk_done function
            Self::push_internal_registers(ctx, code, true);
            //Self::assert_rsp_is_aligned(ctx, code);
            *code += "\tcall _chunk_done\n";
            //Self::assert_rsp_is_aligned(ctx, code);
            Self::pop_internal_registers(ctx, code, true);
        }
        if ctx.zip() {
            *code += &format!("chunk_end_{id}_done:\n");
        }
    }

    /**********************/
    /* REGISTERS PUSH/POP */
    /**********************/

    fn push_xmm_regs(ctx: &mut ZiskAsmContext, code: &mut String, extra_8: bool) {
        *code += &format!(
            "\tsub rsp, 16*16 + {} {}\n",
            if extra_8 { "8" } else { "0" },
            ctx.comment_str("push 16 xmm registers")
        );
        *code += "\tmovaps [rsp + 0*16], xmm0\n";
        *code += "\tmovaps [rsp + 1*16], xmm1\n";
        *code += "\tmovaps [rsp + 2*16], xmm2\n";
        *code += "\tmovaps [rsp + 3*16], xmm3\n";
        *code += "\tmovaps [rsp + 4*16], xmm4\n";
        *code += "\tmovaps [rsp + 5*16], xmm5\n";
        *code += "\tmovaps [rsp + 6*16], xmm6\n";
        *code += "\tmovaps [rsp + 7*16], xmm7\n";
        *code += "\tmovaps [rsp + 8*16], xmm8\n";
        *code += "\tmovaps [rsp + 9*16], xmm9\n";
        *code += "\tmovaps [rsp + 10*16], xmm10\n";
        *code += "\tmovaps [rsp + 11*16], xmm11\n";
        *code += "\tmovaps [rsp + 12*16], xmm12\n";
        *code += "\tmovaps [rsp + 13*16], xmm13\n";
        *code += "\tmovaps [rsp + 14*16], xmm14\n";
        *code += "\tmovaps [rsp + 15*16], xmm15\n";
    }
    fn pop_xmm_regs(ctx: &mut ZiskAsmContext, code: &mut String, extra_8: bool) {
        *code += "\tmovaps xmm0, [rsp + 0*16]\n";
        *code += "\tmovaps xmm1, [rsp + 1*16]\n";
        *code += "\tmovaps xmm2, [rsp + 2*16]\n";
        *code += "\tmovaps xmm3, [rsp + 3*16]\n";
        *code += "\tmovaps xmm4, [rsp + 4*16]\n";
        *code += "\tmovaps xmm5, [rsp + 5*16]\n";
        *code += "\tmovaps xmm6, [rsp + 6*16]\n";
        *code += "\tmovaps xmm7, [rsp + 7*16]\n";
        *code += "\tmovaps xmm8, [rsp + 8*16]\n";
        *code += "\tmovaps xmm9, [rsp + 9*16]\n";
        *code += "\tmovaps xmm10, [rsp + 10*16]\n";
        *code += "\tmovaps xmm11, [rsp + 11*16]\n";
        *code += "\tmovaps xmm12, [rsp + 12*16]\n";
        *code += "\tmovaps xmm13, [rsp + 13*16]\n";
        *code += "\tmovaps xmm14, [rsp + 14*16]\n";
        *code += "\tmovaps xmm15, [rsp + 15*16]\n";
        *code += &format!(
            "\tadd rsp, 16*16 + {} {}\n",
            if extra_8 { "8" } else { "0" },
            ctx.comment_str("pop 16 xmm registers")
        );
    }

    // fn assert_rsp_is_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
    //     *code += "\ttest rsp, 0xf\n";
    //     *code += &format!("\tjz pc_{:x}_{}_rsp_aligned\n", ctx.pc, ctx.assert_rsp_counter);
    //     *code += "\tint3\n";
    //     *code += &format!("pc_{:x}_{}_rsp_aligned:\n", ctx.pc, ctx.assert_rsp_counter);
    //     ctx.assert_rsp_counter += 1;
    // }

    fn push_external_registers(ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += &format!("\tmov {}, rsp {}\n", ctx.mem_rsp, ctx.comment_str("backup rsp"));
        //*code += "\tpush rsp\n";
        *code += "\tpush rbx\n";
        *code += "\tpush rbp\n";
        *code += "\tpush r12\n";
        *code += "\tpush r13\n";
        *code += "\tpush r14\n";
        *code += "\tpush r15\n";
        Self::push_xmm_regs(ctx, code, true);
    }

    fn pop_external_registers(ctx: &mut ZiskAsmContext, code: &mut String) {
        Self::pop_xmm_regs(ctx, code, true);
        *code += "\tpop r15\n";
        *code += "\tpop r14\n";
        *code += "\tpop r13\n";
        *code += "\tpop r12\n";
        *code += "\tpop rbp\n";
        *code += "\tpop rbx\n";
        //*code += "\tpop rsp\n";
        *code += &format!("\tmov rsp, {} {}\n", ctx.mem_rsp, ctx.comment_str("restore rsp"));
    }

    fn push_internal_registers(ctx: &mut ZiskAsmContext, code: &mut String, extra_8: bool) {
        *code += "\tpush rax\n";
        *code += "\tpush rcx\n";
        *code += "\tpush rdx\n";
        //*code += "\tpush rdi\n";
        // *code += "\tpush rsi\n";
        // *code += "\tpush rsp\n";
        *code += "\tpush r8\n";
        *code += "\tpush r9\n";
        *code += "\tpush r10\n";
        *code += "\tpush r11\n";
        Self::push_xmm_regs(ctx, code, !extra_8);
    }

    fn pop_internal_registers(ctx: &mut ZiskAsmContext, code: &mut String, extra_8: bool) {
        Self::pop_xmm_regs(ctx, code, !extra_8);
        *code += "\tpop r11\n";
        *code += "\tpop r10\n";
        *code += "\tpop r9\n";
        *code += "\tpop r8\n";
        // *code += "\tpop rsp\n";
        // *code += "\tpop rsi\n";
        //*code += "\tpop rdi\n";
        *code += "\tpop rdx\n";
        *code += "\tpop rcx\n";
        *code += "\tpop rax\n";
    }

    fn push_internal_registers_except_c_and_flag(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        extra_8: bool,
    ) {
        *code += "\tpush rax\n";
        *code += "\tpush rcx\n";
        //*code += "\tpush rdi\n";
        // *code += "\tpush rsi\n";
        // *code += "\tpush rsp\n";
        *code += "\tpush r8\n";
        *code += "\tpush r9\n";
        *code += "\tpush r10\n";
        *code += "\tpush r11\n";
        Self::push_xmm_regs(ctx, code, extra_8);
    }

    fn pop_internal_registers_except_c_and_flag(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        extra_8: bool,
    ) {
        Self::pop_xmm_regs(ctx, code, extra_8);
        *code += "\tpop r11\n";
        *code += "\tpop r10\n";
        *code += "\tpop r9\n";
        *code += "\tpop r8\n";
        // *code += "\tpop rsp\n";
        // *code += "\tpop rsi\n";
        //*code += "\tpop rdi\n";
        *code += "\tpop rcx\n";
        *code += "\tpop rax\n";
    }

    // Experimental code
    fn trace_reg_access(ctx: &mut ZiskAsmContext, code: &mut String, reg: u64, slot: u64) {
        // REG_VALUE is reg_step = STEP << 4 + 1 + slot
        *code +=
            &format!("\tmov {}, {} {}\n", REG_VALUE, ctx.mem_step, ctx.comment_str("value = step"));
        *code += &format!("\tsal {}, 3 {}\n", REG_VALUE, ctx.comment_str("value <<= 2"));
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_VALUE,
            slot + 1,
            ctx.comment(format!("value += {}", slot + 1))
        );

        // REG_ADDRESS is reg_steps[slot], i.e. prev_reg_steps
        *code += &format!(
            "\tmov {}, qword {}[reg_steps_{}] {}\n",
            REG_ADDRESS,
            ctx.ptr,
            slot,
            ctx.comment_str("address = reg_steps[slot]")
        );

        // reg_prev_steps[slot] = pref_reg_steps
        *code += &format!(
            "\tmov qword {}[reg_prev_steps_{}], {} {}\n",
            ctx.ptr,
            slot,
            REG_ADDRESS,
            ctx.comment_str("reg_prev_steps[slot] = address")
        );

        // Check if is first_reference==0
        *code += &format!(
            "\tmov {}, qword {}[first_step_uses_{}] {}\n",
            REG_AUX,
            ctx.ptr,
            reg,
            ctx.comment_str("aux = first_step_uses[reg]")
        );
        *code += &format!("\tjz pc_{:x}_{}_first_reference\n", ctx.pc, slot);
        // Not first reference
        *code += &format!("pc_{:x}_{}_not_first_reference:\n", ctx.pc, slot);
        *code += &format!(
            "\tmov qword {}[reg_step_ranges_{}], {} {}\n",
            ctx.ptr,
            slot,
            REG_VALUE,
            ctx.comment_str("reg_step_ranges[slot] = reg_step")
        );
        *code += &format!(
            "\tsub qword {}[reg_step_ranges_{}], {} {}\n",
            ctx.ptr,
            slot,
            REG_VALUE,
            ctx.comment_str("reg_step_ranges[slot] -= prev_reg_step")
        );
        *code += &format!("\tjmp pc_{:x}_{}_first_reference_done\n", ctx.pc, slot);
        // First reference
        *code += &format!("pc_{:x}_{}_first_reference:\n", ctx.pc, slot);
        *code += &format!(
            "\tmov qword {}[first_step_uses_{}], {} {}\n",
            ctx.ptr,
            reg,
            REG_VALUE,
            ctx.comment_str("first_step_uses[reg] = value")
        );
        *code += &format!("pc_{:x}_{}_first_reference_done:\n", ctx.pc, slot);

        // Store reg_steps
        *code += &format!(
            "\tmov qword {}[reg_steps_{}], {} {}\n",
            ctx.ptr,
            slot,
            REG_VALUE,
            ctx.comment_str("reg_steps[slot] = reg_step")
        );
    }

    // Experimental code
    fn clear_reg_step_ranges(ctx: &mut ZiskAsmContext, code: &mut String, slot: u64) {
        *code += &format!(
            "\tmov qword {}[reg_step_ranges_{}], 0 {}\n",
            ctx.ptr,
            slot,
            ctx.comment_str("reg_step_ranges[slot]=0")
        );
    }

    /*************/
    /* REGISTERS */
    /*************/

    fn reg_to_xmm_index(reg: u64) -> u64 {
        match reg {
            1 => 0,
            2 => 1,
            5 => 2,
            6 => 3,
            7 => 4,
            8 => 5,
            9 => 6,
            10 => 7,
            11 => 8,
            12 => 9,
            13 => 10,
            14 => 11,
            15 => 12,
            16 => 13,
            17 => 14,
            18 => 15,
            _ => {
                panic!("ZiskRom2Asm::reg_to_xmm_index() found invalid source slot={reg}");
            }
        }
    }

    // fn reg_to_rsp_index(reg: u64) -> u64 {
    //     match reg {
    //         0 => 0,
    //         3 => 1,
    //         4 => 2,
    //         19 => 3,
    //         20 => 4,
    //         21 => 5,
    //         22 => 6,
    //         23 => 7,
    //         24 => 8,
    //         25 => 9,
    //         26 => 10,
    //         27 => 11,
    //         28 => 12,
    //         29 => 13,
    //         30 => 14,
    //         31 => 15,
    //         32 => 16,
    //         33 => 17,
    //         34 => 18,
    //         _ => {
    //             panic!("ZiskRom2Asm::reg_to_rsp_index() found invalid source slot={}", reg);
    //         }
    //     }
    // }

    fn read_riscv_reg(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        src_slot: u64,
        dest_reg: &str,
        dest_desc: &str,
    ) {
        if XMM_MAPPED_REGS.contains(&src_slot) {
            let xmm_index = Self::reg_to_xmm_index(src_slot);
            *code += &format!(
                "\tmovq {}, xmm{} {}\n",
                dest_reg,
                xmm_index,
                ctx.comment(format!("{dest_desc} = reg[{src_slot}]"))
            );
        } else {
            *code += &format!(
                "\tmov {}, qword {}[reg_{}] {}\n",
                dest_reg,
                ctx.ptr,
                src_slot,
                ctx.comment(format!("{dest_desc} = reg[{src_slot}]"))
            );
        }
    }

    fn write_riscv_reg(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        dest_slot: u64,
        src_reg: &str,
        src_desc: &str,
    ) {
        let comment = format!("reg[{dest_slot}]={src_desc}");
        if XMM_MAPPED_REGS.contains(&dest_slot) {
            let xmm_index = Self::reg_to_xmm_index(dest_slot);
            *code += &format!("\tmovq xmm{}, {} {}\n", xmm_index, src_reg, ctx.comment(comment));
        } else {
            *code += &format!(
                "\tmov qword {}[reg_{}], {} {}\n",
                ctx.ptr,
                dest_slot,
                src_reg,
                ctx.comment(comment)
            );
        }
    }

    fn write_riscv_reg_constant(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        dest_slot: u64,
        value: u64,
        value_desc: &str,
    ) {
        let comment = format!("reg[{dest_slot}]={value_desc}");
        if XMM_MAPPED_REGS.contains(&dest_slot) {
            let xmm_index = Self::reg_to_xmm_index(dest_slot);
            *code += &format!("\tmov {REG_AUX}, 0x{value:x}\n");

            *code += &format!("\tmovq xmm{}, {} {}\n", xmm_index, REG_AUX, ctx.comment(comment));
        } else {
            *code += &format!("\tmov {REG_AUX}, 0x{value:x}\n");
            *code += &format!(
                "\tmov qword {}[reg_{}], {} {}\n",
                ctx.ptr,
                dest_slot,
                REG_AUX,
                ctx.comment(comment)
            );
        }
    }

    /*****************/
    /* ROM HISTOGRAM */
    /*****************/

    /// This function calculates the address of the rom histogram for the provided pc
    ///
    /// ROM histogram structure:
    ///
    /// ROM trace control:
    ///     [8B] version
    ///     [8B] exit_code (0=success, 1=not completed)
    ///     [8B] allocated_size = xxx (bytes)
    ///     [8B] used_size = xxx (bytes)
    /// BIOS histogram: (TRACE_ADDR_NUMBER)
    ///     [8B] multiplicity_size = B
    ///     [8B] multiplicity[0] → 4096
    ///     [8B] multiplicity[1] → 4096 + 4
    ///     …
    ///     [8B] multiplicity[B-1] → 4096 + 4*(B-1)
    /// Program histogram:
    ///     [8B] multiplicity_size = P
    ///     [8B] multiplicity[0] → 0x80000000
    ///     [8B] multiplicity[1] → 0x80000000 + 1
    ///     …
    ///     [8B] multiplicity[P-1] → 0x80000000 + (P-1)
    ///
    fn get_rom_histogram_trace_address(rom: &ZiskRom, pc: u64) -> u64 {
        assert!(rom.max_bios_pc >= ROM_ENTRY);
        assert!(rom.max_bios_pc < ROM_ADDR);
        assert!(rom.max_program_pc >= ROM_ADDR);
        assert!(rom.max_program_pc <= ROM_ADDR_MAX);
        if pc < ROM_ADDR {
            TRACE_ADDR_NUMBER + (1 + ((pc - ROM_ENTRY) >> 2)) * 8
        } else {
            TRACE_ADDR_NUMBER
                + (1 + ((rom.max_bios_pc - ROM_ENTRY) >> 2) + 1 + 1 + pc - ROM_ADDR) * 8
        }
    }

    fn chunk_player_start(ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += &ctx.full_line_comment("Chunk plater start".to_string());

        // Read from minimal trace setup
        ////////////////////////////////

        // Get chunk player address and store it in a register
        *code += &format!(
            "\tmov {}, qword {} [chunk_player_address] {}\n",
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.ptr,
            ctx.comment_str("set chunk player address")
        );

        // Init pc = chunk[0]
        *code += &format!(
            "\tmov {}, qword {} [{}] {}\n",
            REG_PC,
            ctx.ptr,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("pc = chunk.start.pc")
        );

        // Init sp = chunk[1]
        *code += &format!(
            "\tmov {}, qword {} [{} + 8] {}\n",
            REG_VALUE,
            ctx.ptr,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("value = chunk.start.sp")
        );
        *code += &format!(
            "\tmov {}, {} {}\n",
            ctx.mem_sp,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("sp = value")
        );

        // Init register c = chunk[2]
        *code += &format!(
            "\tmov {}, qword {} [{} + 8*2] {}\n",
            REG_C,
            ctx.ptr,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("c = chunk.start.c")
        );

        // Init step = chunk[3]
        *code += &format!(
            "\tmov {}, qword {} [{} + 8*3] {}\n",
            REG_VALUE,
            ctx.ptr,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("value = chunk.start.step")
        );
        *code +=
            &format!("\tmov {}, {} {}\n", ctx.mem_step, REG_VALUE, ctx.comment_str("step = value"));

        // Init RISC-V registers, reg[1...33] = chunk[4...36]
        *code += &format!("\tmov qword {}[reg_0], 0\n", ctx.ptr);
        for i in 0..34 {
            *code += &format!(
                "\tmov {}, qword {} [{} + 8*(4 + {})] {}\n",
                REG_VALUE,
                ctx.ptr,
                REG_CHUNK_PLAYER_ADDRESS,
                i,
                ctx.comment_str(&format!("value = chunk.start.reg[{}]", i + 1))
            );
            Self::write_riscv_reg(ctx, code, i + 1, REG_VALUE, "value");
        }
        *code += &format!("\tmov qword {}[reg_34], 0\n", ctx.ptr);

        // Skip last c = chunk[37]

        // Skip end = chunk[38]

        // Skip steps = chunk[39]

        // Skip mem reads size = chunk[40]

        // Advance address to the first memory reads position. i.e. to chunk.mem_reads[0]
        *code += &format!(
            "\tadd {}, 8*41 {}\n",
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("advance chunk player address")
        );

        // Set step count down
        //////////////////////

        *code += &ctx.full_line_comment("Set step count down to chunk_size".to_string());
        *code += &format!(
            "\tmov {}, chunk_size {}\n",
            REG_STEP,
            ctx.comment_str("step_count_down = chunk_size")
        );

        // Output trace setup
        /////////////////////

        *code += &ctx.full_line_comment("Write mem reads address".to_string());
        *code += &format!(
            "\tmov {}, {} {}\n",
            REG_AUX,
            ctx.mem_chunk_address,
            ctx.comment_str("aux = chunk_size")
        );
        *code += &format!("\tadd {}, 8 {}\n", REG_AUX, ctx.comment_str("aux += 8"));
        *code += &format!(
            "\tmov {}, {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_AUX,
            ctx.comment_str("mem_reads_address = aux")
        );
        *code += &ctx.full_line_comment("Reset mem_reads size".to_string());
        *code += &format!(
            "\txor {}, {} {}\n",
            REG_MEM_READS_SIZE,
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size = 0")
        );

        // Jump to start pc
        Self::jumpt_to_dynamic_pc(ctx, code);
    }

    fn chunk_player_end(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Update step
        *code += &ctx.full_line_comment("Update total step from step count down".to_string());
        *code +=
            &format!("\tmov {}, {} {}\n", REG_VALUE, ctx.mem_step, ctx.comment_str("value = step"));
        *code += &format!(
            "\tadd {}, chunk_size {}\n",
            REG_VALUE,
            ctx.comment_str("value += chunk_size")
        );
        *code += &format!(
            "\tsub {}, {} {}\n",
            REG_VALUE,
            REG_STEP,
            ctx.comment_str("value -= step_count_down")
        );
        *code +=
            &format!("\tmov {}, {} {}\n", ctx.mem_step, REG_VALUE, ctx.comment_str("step = value"));

        // Update mem reads size
        *code += &format!(
            "\tmov {}, {} {}\n",
            REG_ADDRESS,
            ctx.mem_trace_address,
            ctx.comment_str("address = chunk_address")
        );
        *code += &format!(
            "\tmov [{}], {} {}\n",
            REG_ADDRESS,
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size = size")
        );
    }

    fn chunk_player_mem_read_aligned(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        reg_to_store_value: &str,
        reg_to_store_value_desc: &str,
        micro_step: u64,
    ) {
        // Read value from minimal trace
        ////////////////////////////////

        // Read value from memory and store in the proper register: a or c
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            reg_to_store_value,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment(format!("{reg_to_store_value_desc} = mt[address]"))
        );

        // Increment chunk player address
        *code += &format!(
            "\tadd {}, 8 {}\n",
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("chunk_address += 8")
        );

        // Trace memory
        ///////////////

        // Build the mask for this case
        const WIDTH: u64 = 8;
        const WRITE: u64 = 0;
        let addr_step_mask: u64 =
            (WIDTH << F_MEM_WIDTH_SHIFT) + (WRITE << F_MEM_WRITE_SHIFT) + (micro_step << 38);

        // Add mask to address
        *code += &format!(
            "\tmov {}, 0x{:x} {}\n",
            REG_AUX,
            addr_step_mask,
            ctx.comment_str("aux = addr_step_mask")
        );
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_ADDRESS,
            REG_AUX,
            ctx.comment_str("address += addr_step_mask")
        );

        // Build the step and add it to the address
        *code +=
            &format!("\tmov {}, chunk_size {}\n", REG_AUX, ctx.comment_str("aux = chunk_size"));
        *code += &format!(
            "\tsub {}, {} {}\n",
            REG_AUX,
            REG_STEP,
            ctx.comment_str("aux -= step_count_down")
        );
        *code += &format!("\tshl {}, 40 {}\n", REG_AUX, ctx.comment_str("aux <<= 40"));
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_ADDRESS,
            REG_AUX,
            ctx.comment_str("addr_step += step")
        );

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_ADDRESS,
            ctx.comment_str("mem_reads[@+size*8] = addr_step")
        );

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            reg_to_store_value,
            ctx.comment(format!("mem_reads[@+size*8+8] = {reg_to_store_value_desc} = a"))
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!(
            "\tadd {}, 2 {}\n",
            REG_MEM_READS_SIZE,
            ctx.comment_str("mem_reads_size += 2")
        );
    }
    fn chunk_player_a_src_mem_not_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Copy address into reg c
        *code += &format!("\tmov rcx, {} {}\n", REG_ADDRESS, ctx.comment_str("rcx = address"));

        // Calculate address misalignment: 1, 2, 3, 4, 5, 6 or 7 bytes
        *code += &format!("\tand rcx, 0x7 {}\n", ctx.comment_str("rcx = misaligned bytes"));
        *code += &format!("\tshl rcx, 0x3 {}\n", ctx.comment_str("rcx = misaligned bits"));

        // Store previous aligned value in reg value
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            REG_VALUE,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("value = mt[prev_address]")
        );

        // Shif it right the number of address misaligned bits
        *code += &format!("\tshr {}, cl {}\n", REG_VALUE, ctx.comment_str("value >> bits"));

        // Store next aligned value in reg aux
        *code += &format!(
            "\tmov {}, [{} + 8] {}\n",
            REG_AUX,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("aux = mt[next_address]")
        );

        // Shift it left 64 - the number of address misaligned bits
        // Calculate address misalignment: 1, 2, 3, 4, 5, 6 or 7 bytes
        *code += &format!("\tneg rcx {}\n", ctx.comment_str("rcx = 64 - rcx"));
        *code += &format!("\tadd rcx, 0x64 {}\n", ctx.comment_str("rcx = 64 - rcx"));

        // Shif it left the number of address misaligned bits
        *code += &format!("\tshl {}, cl {}\n", REG_AUX, ctx.comment_str("aux << bits"));

        // Shif it right the number of address misaligned bits
        *code += &format!("\tor {}, {} {}\n", REG_VALUE, REG_AUX, ctx.comment_str("value |= aux"));

        // Read value from memory and store in the proper register: a or c
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            if ctx.store_a_in_c { REG_C } else { REG_A },
            REG_VALUE,
            ctx.comment(format!("{} = mt[address]", if ctx.store_a_in_c { "c" } else { "a" }))
        );

        // Increment chunk player address
        *code += &format!(
            "\tadd {}, 16 {}\n",
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("chunk_address += 8")
        );
    }

    fn chunk_player_b_src_mem_not_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Copy address into reg c
        *code += &format!("\tmov rcx, {} {}\n", REG_ADDRESS, ctx.comment_str("rcx = address"));

        // Calculate address misalignment: 1, 2, 3, 4, 5, 6 or 7 bytes
        *code += &format!("\tand rcx, 0x7 {}\n", ctx.comment_str("rcx = misaligned bytes"));
        *code += &format!("\tshl rcx, 0x3 {}\n", ctx.comment_str("rcx = misaligned bits"));

        // Store previous aligned value in reg value
        *code += &format!(
            "\tmov {}, [{}] {}\n",
            REG_VALUE,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("value = mt[prev_address]")
        );

        // Shif it right the number of address misaligned bits
        *code += &format!("\tshr {}, cl {}\n", REG_VALUE, ctx.comment_str("value >> bits"));

        // Store next aligned value in reg aux
        *code += &format!(
            "\tmov {}, [{} + 8] {}\n",
            REG_AUX,
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("aux = mt[next_address]")
        );

        // Shift it left 64 - the number of address misaligned bits
        // Calculate address misalignment: 1, 2, 3, 4, 5, 6 or 7 bytes
        *code += &format!("\tneg rcx {}\n", ctx.comment_str("rcx = - rcx"));
        *code += &format!("\tadd rcx, 64 {}\n", ctx.comment_str("rcx = 64 - rcx"));

        // Shif it left the number of address misaligned bits
        *code += &format!("\tshl {}, cl {}\n", REG_AUX, ctx.comment_str("aux << bits"));

        // Shif it right the number of address misaligned bits
        *code += &format!("\tor {}, {} {}\n", REG_VALUE, REG_AUX, ctx.comment_str("value |= aux"));

        // Read value from memory and store in the proper register: a or c
        *code += &format!(
            "\tmov {}, {} {}\n",
            if ctx.store_b_in_c { REG_C } else { REG_B },
            REG_VALUE,
            ctx.comment(format!("{} = mt[address]", if ctx.store_b_in_c { "c" } else { "b" }))
        );

        // Increment chunk player address
        *code += &format!(
            "\tadd {}, 16 {}\n",
            REG_CHUNK_PLAYER_ADDRESS,
            ctx.comment_str("chunk_address += 16")
        );
    }

    fn chunk_player_mem_write(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        width: u64,
        reg_write_value: &str,
        number_of_memory_reads: u64,
    ) {
        assert!(number_of_memory_reads <= 2);

        // Trace memory
        ///////////////

        // Build the mask for this case
        const WRITE: u64 = 1;
        const MICRO_STEP: u64 = 3;
        let addr_step_mask: u64 =
            (width << F_MEM_WIDTH_SHIFT) + (WRITE << F_MEM_WRITE_SHIFT) + (MICRO_STEP << 40);

        // Add mask to address
        *code += &format!(
            "\tmov {}, 0x{:x} {}\n",
            REG_AUX,
            addr_step_mask,
            ctx.comment_str("aux = addr_step_mask")
        );
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_ADDRESS,
            REG_AUX,
            ctx.comment_str("address += addr_step_mask")
        );

        // Build the step and add it to the address
        *code +=
            &format!("\tmov {}, chunk_size {}\n", REG_AUX, ctx.comment_str("aux = chunk_size"));
        *code += &format!(
            "\tsub {}, {} {}\n",
            REG_AUX,
            REG_STEP,
            ctx.comment_str("aux -= step_count_down")
        );
        *code += &format!("\tshl {}, 42 {}\n", REG_AUX, ctx.comment_str("aux <<= 40"));
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_ADDRESS,
            REG_AUX,
            ctx.comment_str("addr_step += step")
        );

        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            REG_ADDRESS,
            ctx.comment_str("mem_reads[@+size*8] = addr_step")
        );
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));

        // Read values from minimal trace
        /////////////////////////////////

        for _i in 0..number_of_memory_reads {
            // Read value from memory and store in the proper register: a or c
            *code += &format!(
                "\tmov {}, [{}] {}\n",
                REG_AUX,
                REG_CHUNK_PLAYER_ADDRESS,
                ctx.comment("aux = mt[address]".to_string())
            );

            // Increment chunk player address
            *code += &format!(
                "\tadd {}, 8 {}\n",
                REG_CHUNK_PLAYER_ADDRESS,
                ctx.comment_str("chunk_address += 8")
            );

            // Copy read data into mem_reads_address and increment it
            *code += &format!(
                "\tmov [{} + {}*8], {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_MEM_READS_SIZE,
                REG_ADDRESS,
                ctx.comment_str("mem_reads[@+size*8] = addr_step")
            );
            *code +=
                &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
        }

        // Write value
        //////////////

        // Copy write data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} {}\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            reg_write_value,
            ctx.comment_str("mem_reads[@+size*8] = write value")
        );
        *code += &format!("\tinc {} {}\n", REG_MEM_READS_SIZE, ctx.comment_str("mem_reads_size++"));
    }

    #[allow(dead_code)]
    fn chunk_player_buffer(
        ctx: &mut ZiskAsmContext,
        code: &mut String,
        buffer_address_register: &str,
        buffer_size: u64,
        write: u64,
    ) {
        assert!(buffer_size > 0);

        // Store address in aux
        *code += &format!(
            "\tmov {}, {} {}\n",
            REG_AUX,
            buffer_address_register,
            ctx.comment_str("aux += buffer_address")
        );

        // Build the mask for this case
        const WIDTH: u64 = 8;
        const MICRO_STEP: u64 = 2;
        let addr_step_mask: u64 =
            (WIDTH << F_MEM_WIDTH_SHIFT) + (write << F_MEM_WRITE_SHIFT) + (MICRO_STEP << 40);

        // For every element
        for i in 0..buffer_size {
            // Load address first, increment it later
            if i == 0 {
                // Store address in aux
                *code += &format!(
                    "\tmov {}, {} {}\n",
                    REG_AUX,
                    buffer_address_register,
                    ctx.comment_str("aux += buffer_address")
                );
            } else {
                *code +=
                    &format!("\tadd {}, 8 {}\n", REG_AUX, ctx.comment_str("buffer_address += 8"));
            }

            // Trace memory
            ///////////////

            // Build the step into value
            *code += &format!(
                "\tmov {}, chunk_size {}\n",
                REG_VALUE,
                ctx.comment_str("value = chunk_size")
            );
            *code += &format!(
                "\tsub {}, {} {}\n",
                REG_VALUE,
                REG_STEP,
                ctx.comment_str("value -= step_count_down")
            );
            *code += &format!("\tshl {}, 42 {}\n", REG_VALUE, ctx.comment_str("value <<= 40"));

            // Add mask to value
            *code += &format!(
                "\tadd {}, 0x{:x} {}\n",
                REG_VALUE,
                addr_step_mask,
                ctx.comment_str("value = addr_step_mask")
            );

            // Add address to value
            *code += &format!(
                "\tadd {}, {} {}\n",
                REG_VALUE,
                REG_AUX,
                ctx.comment_str("value += address")
            );

            // Copy read data into mem_reads_address and increment it
            *code += &format!(
                "\tmov [{} + {}*8 + {}*8], {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_MEM_READS_SIZE,
                2 * i,
                REG_VALUE,
                ctx.comment_str("mem_reads[@+size*8] = addr_step")
            );

            // Trace value
            //////////////

            // Read value
            *code += &format!(
                "\tmov {}, [{} + {}*8] {}\n",
                REG_VALUE,
                REG_CHUNK_PLAYER_ADDRESS,
                i,
                ctx.comment("value = mt[address]".to_string())
            );

            // Trace value
            *code += &format!(
                "\tmov [{} + {}*8 + {}*8], {} {}\n",
                REG_MEM_READS_ADDRESS,
                REG_MEM_READS_SIZE,
                (2 * i) + 1,
                REG_VALUE,
                ctx.comment_str("mem_reads[@+size*8] = value")
            );
        }

        // Increment mem reads size
        *code += &format!(
            "\tadd {}, {} {}\n",
            REG_MEM_READS_SIZE,
            buffer_size * 2,
            ctx.comment_str("mem_reads_size += buffer_size")
        );

        // Increment chunk player address
        *code += &format!(
            "\tadd {}, 8*{} {}\n",
            REG_CHUNK_PLAYER_ADDRESS,
            buffer_size,
            ctx.comment_str("chunk_address += 8*buffer_size")
        );
    }
}
