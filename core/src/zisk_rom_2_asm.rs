//! Zisk ROM to ASM
//!
//! Generates i86_64 assembly code that implements the Zisk ROM program
use std::path::Path;

use crate::{
    zisk_ops::ZiskOp, AsmGenerationMethod, ZiskInst, ZiskRom, FREE_INPUT_ADDR, M64, P2_32,
    ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP,
    STORE_IND, STORE_MEM, STORE_NONE, STORE_REG,
};

// Regs rax, rcx, rdx, rdi, rsi, rsp, and r8-r11 are caller-save, not saved across function calls.
// Reg rax is used to store a function’s return value.
// Regs rbx, rbp, and r12-r15 are callee-save, saved across function calls.

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
const REG_STEP_DOWN: &str = "r14";
const REG_VALUE: &str = "r9";
const REG_VALUE_W: &str = "r9d";
//const REG_VALUE_H: &str = "r9w";
//const REG_VALUE_B: &str = "r9b";
const REG_ADDRESS: &str = "r10";
const REG_MEM_READS_ADDRESS: &str = "r12";
const REG_MEM_READS_SIZE: &str = "r13";
const REG_AUX: &str = "r11";

const MEM_STEP: &str = "qword ptr [MEM_STEP]";
const MEM_PC: &str = "qword ptr [MEM_PC]";
const MEM_SP: &str = "qword ptr [MEM_SP]";
const MEM_END: &str = "qword ptr [MEM_END]";

const TRACE_ADDR: &str = "0xb0000020";
const TRACE_ADDR_NUMBER: u64 = 0xb0000020;

const MEM_TRACE_ADDRESS: &str = "qword ptr [MEM_TRACE_ADDRESS]";
const MEM_CHUNK_ADDRESS: &str = "qword ptr [MEM_CHUNK_ADDRESS]";
const MEM_CHUNK_START_STEP: &str = "qword ptr [MEM_CHUNK_START_STEP]";

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

#[derive(Default, Debug, Clone)]
pub struct ZiskAsmRegister {
    is_constant: bool,   // register is a constant value known at compilation time
    constant_value: u64, // register constant value, only valid if is_constant==true
    is_saved: bool,      // register has been saved to memory/register
    string_value: String, /* register string value: a constant value (e.g. "0x3f") or a register
                          * (e.g. "rax") */
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
    generate_fast: bool,          // 0
    generate_minimal_trace: bool, // 1
    generate_rom_histogram: bool, // 2
    generate_main_trace: bool,    // 3
    generate_chunks: bool,        // 4

    a: ZiskAsmRegister,
    b: ZiskAsmRegister,
    c: ZiskAsmRegister,

    store_a_in_c: bool,
    store_a_in_a: bool,
    store_b_in_c: bool,
    store_b_in_b: bool,
}
pub struct ZiskRom2Asm {}

impl ZiskRom2Asm {
    /// Saves ZisK rom into an i64-64 assembly file: first save to a string, then
    /// save the string to the file
    pub fn save_to_asm_file(
        rom: &ZiskRom,
        file_name: &Path,
        generation_method: AsmGenerationMethod,
    ) {
        // Get a string with the ASM data
        let mut s = String::new();
        Self::save_to_asm(rom, &mut s, generation_method);

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
    pub fn save_to_asm(rom: &ZiskRom, code: &mut String, generation_method: AsmGenerationMethod) {
        // Select the ASM generation method
        let mut generate_fast = false;
        let mut generate_minimal_trace = false;
        let mut generate_rom_histogram = false;
        let mut generate_main_trace = false;
        let mut generate_chunks = false;

        match generation_method {
            AsmGenerationMethod::AsmFast => generate_fast = true,
            AsmGenerationMethod::AsmMinimalTraces => generate_minimal_trace = true,
            AsmGenerationMethod::AsmRomHistogram => generate_rom_histogram = true,
            AsmGenerationMethod::AsmMainTrace => generate_main_trace = true,
            AsmGenerationMethod::AsmChunks => generate_chunks = true,
        }

        // Clear output data, just in case
        code.clear();

        // Store less usual code branches in distant memory to improve cache hits
        let mut unusual_code: String = String::new();

        // Create context
        let mut ctx = ZiskAsmContext {
            log_output: true,
            call_chunk_done: true,
            generate_fast,
            generate_minimal_trace,
            generate_rom_histogram,
            generate_main_trace,
            generate_chunks,
            ..Default::default()
        };

        *code += ".intel_syntax noprefix\n";
        *code += ".code64\n";
        *code += ".section .rodata\n";
        *code += "msg: .ascii \"Zisk assembly emulator\\n\"\n";
        *code += ".set msglen, (. - msg)\n\n";

        *code += ".section .data\n";
        *code += ".comm MEM_STEP, 8, 8\n";
        *code += ".comm MEM_SP, 8, 8\n";
        *code += ".comm MEM_END, 8, 8\n";
        *code += ".comm MEM_PC, 8, 8\n";
        *code += ".comm MEM_TRACE_ADDRESS, 8, 8\n";
        *code += ".comm MEM_CHUNK_ADDRESS, 8, 8\n";
        *code += ".comm MEM_CHUNK_START_STEP, 8, 8\n";

        // Allocate space for the registers
        for r in 0u64..35u64 {
            if !XMM_MAPPED_REGS.contains(&r) {
                *code += &format!(".comm reg_{}, 8, 8\n", r);
            }
        }

        if ctx.generate_main_trace {
            for i in 0..3 {
                *code += &format!(".comm reg_steps_{}, 8, 8\n", i);
            }
            for i in 0..3 {
                *code += &format!(".comm reg_prev_steps_{}, 8, 8\n", i);
            }
            for i in 0..3 {
                *code += &format!(".comm reg_step_ranges_{}, 8, 8\n", i);
            }
            for i in 0..35 {
                *code += &format!(".comm first_step_uses_{}, 8, 8\n", i);
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
        *code += ".extern opcode_arith256\n";
        *code += ".extern opcode_arith256_mod\n";
        *code += ".extern opcode_secp256k1_add\n";
        *code += ".extern opcode_secp256k1_dbl\n";
        *code += ".extern opcode_fcall\n";
        *code += ".extern chunk_done\n";
        *code += ".extern print_fcall_ctx\n";
        *code += ".extern realloc_trace\n\n";

        if ctx.generate_minimal_trace || ctx.generate_main_trace {
            *code += ".extern chunk_size\n";
            *code += ".extern trace_address_threshold\n\n";
        }

        if ctx.generate_chunks || ctx.generate_minimal_trace || ctx.generate_main_trace {
            // Chunk start
            *code += "chunk_start:\n";
            Self::chunk_start(&mut ctx, code);
            *code += "\tret\n\n";

            // Chunk end
            *code += "chunk_end:\n";
            Self::chunk_end(&mut ctx, code, "end");
            *code += "\tret\n\n";

            // Chunk end and start
            *code += "chunk_end_and_start:\n";
            Self::chunk_end(&mut ctx, code, "end_and_start");
            Self::chunk_start(&mut ctx, code);
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
        if ctx.generate_fast {
            *code += "\tmov rax, 0\n";
        } else if ctx.generate_minimal_trace {
            *code += "\tmov rax, 1\n";
        } else if ctx.generate_rom_histogram {
            *code += "\tmov rax, 2\n";
        } else if ctx.generate_main_trace {
            *code += "\tmov rax, 3\n";
        } else if ctx.generate_chunks {
            *code += "\tmov rax, 4\n";
        }
        *code += "\tret\n\n";

        *code += ".global emulator_start\n";
        *code += "emulator_start:\n";

        Self::push_external_registers(&mut ctx, code);

        // Registers initialization
        *code += &format!("\tmov {}, 0 /* Register initialization: a = 0 */\n", REG_A);
        *code += &format!("\tmov {}, 0 /* Register initialization: b = 0 */\n", REG_B);
        *code += &format!("\tmov {}, 0 /* Register initialization: c = 0 */\n", REG_C);
        *code += &format!("\tmov {}, 0 /* Register initialization: flag = 0 */\n", REG_FLAG);
        *code += &format!("\tmov {}, 0 /* Memory initialization: step = 0 */\n", MEM_STEP);
        *code += &format!("\tmov {}, 0 /* Memory initialization: sp = 0 */\n", MEM_SP);
        *code += &format!("\tmov {}, 0 /* Memory initialization: end = 0 */\n", MEM_END);
        if ctx.generate_minimal_trace || ctx.generate_main_trace {
            *code += &format!(
                "\tmov {}, {} /* Memory initialization: value = TRACE_ADDR */\n",
                REG_VALUE, TRACE_ADDR
            );
            *code += &format!(
                "\tmov {}, {} /* Memory initialization: trace_address = value = TRACE_ADDR */\n",
                MEM_TRACE_ADDRESS, REG_VALUE
            );
            *code += &format!("\tadd {}, 8 /* Memory initialization: value += 8 */\n", REG_VALUE);
            *code += &format!(
                "\tmov {}, {} /* Memory initialization: chunk_address = value = TRACE_ADDR + 8 */\n\n",
                MEM_CHUNK_ADDRESS, REG_VALUE
            );
        }

        // Initialize registers to zero
        *code += "\t/* Init registers to zero */\n";
        for r in 0u64..35u64 {
            if !XMM_MAPPED_REGS.contains(&r) {
                *code += &format!("\tmov qword ptr [reg_{}], 0\n", r);
            }
        }
        for r in 0..16 {
            *code += &format!("\tpxor xmm{}, xmm{}\n", r, r);
        }

        *code += "\t/* Init fcall_context to zero */\n";
        *code += &format!("\tlea {}, fcall_ctx /* address = fcall context */\n", REG_ADDRESS);
        for i in 0..70 {
            if (i == FCALL_PARAMS_CAPACITY) || (i == FCALL_RESULT_CAPACITY) {
                *code += &format!("\tmov qword ptr [{} + {}*8], 32\n", REG_ADDRESS, i);
            } else {
                *code += &format!("\tmov qword ptr [{} + {}*8], 0\n", REG_ADDRESS, i);
            }
        }

        // For all program addresses in the vector, create an assembly set of instructions with an
        // instruction label
        for k in 0..rom.sorted_pc_list.len() {
            // Get pc
            ctx.pc = rom.sorted_pc_list[k];

            // Call chunk_start the first time, for the first chunk
            if (ctx.generate_minimal_trace || ctx.generate_main_trace) && (k == 0) {
                *code += &format!("\tmov {}, 0x{:08x} /* value = pc */\n", REG_VALUE, ctx.pc);
                *code += &format!("\tmov {}, {} /* pc = value */\n", MEM_PC, REG_VALUE);
                *code += "\tcall chunk_start /* Call chunk_start the first time */\n";
            }

            ctx.next_pc =
                if (k + 1) < rom.sorted_pc_list.len() { rom.sorted_pc_list[k + 1] } else { M64 };
            let instruction = &rom.insts[&ctx.pc].i;

            // Instruction label
            *code += "\n";
            *code += &format!("pc_{:x}: /*{} */\n", ctx.pc, instruction.to_text().as_str());

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
            if ctx.generate_rom_histogram {
                let address = Self::get_rom_histogram_trace_address(rom, ctx.pc);
                *code += "\t/* rom histogram */\n";
                *code += &format!("\tmov {}, 0x{:08x}\n", REG_ADDRESS, address);
                *code += &format!("\tinc qword ptr [{}]\n", REG_ADDRESS);
            }

            // Set special storage destinations for a and b registers, based on operations, in order
            // to save instructions
            let zisk_op = ZiskOp::try_from_code(instruction.op).unwrap();
            ctx.store_a_in_c = false;
            ctx.store_a_in_a = false;
            ctx.store_b_in_c = false;
            ctx.store_b_in_b = false;

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
                *code += "\t/* b=SRC_C */\n";
                if ctx.store_b_in_c {
                    // No need to copy c to b, since we need b to be stored in c
                    ctx.b.is_saved = false;
                } else {
                    *code += &format!("\tmov {}, {} /* b = c */\n", REG_B, REG_C);
                    ctx.b.is_saved = true;
                }
                if ctx.generate_main_trace {
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
                    *code += "\t/* a=SRC_C */\n";
                    if ctx.store_a_in_c {
                        // No need to copy c to a, since we need a to be stored in c
                        ctx.a.is_saved = false;
                    } else {
                        *code += &format!("\tmov {}, {} /* a = c */\n", REG_A, REG_C);
                        ctx.a.is_saved = true;
                    }
                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }
                }
                SRC_REG => {
                    *code += &format!("\t/* a=SRC_REG reg={} */\n", instruction.a_offset_imm0);

                    assert!(instruction.a_offset_imm0 <= 34);

                    // Read from memory and store in the proper register: a or c
                    let dest_reg = if ctx.store_a_in_c { REG_C } else { REG_A };
                    let dest_desc = if ctx.store_a_in_c { "c" } else { "a" };
                    Self::read_riscv_reg(code, instruction.a_offset_imm0, dest_reg, dest_desc);

                    if ctx.generate_main_trace {
                        Self::trace_reg_access(&mut ctx, code, instruction.a_offset_imm0, 0);
                    }
                }
                SRC_MEM => {
                    *code += "\t/* a=SRC_MEM */\n";

                    // Calculate memory address
                    *code += &format!(
                        "\tmov {}, 0x{:x} /* address = i.a_offset_imm0 */\n",
                        REG_ADDRESS, instruction.a_offset_imm0
                    );
                    if instruction.a_use_sp_imm1 != 0 {
                        *code +=
                            &format!("\tadd {}, {} /* address += sp */\n", REG_ADDRESS, MEM_SP);
                    }

                    // Read value from memory and store in the proper register: a or c
                    *code += &format!(
                        "\tmov {}, [{}] /* {} = mem[address] */\n",
                        if ctx.store_a_in_c { REG_C } else { REG_A },
                        REG_ADDRESS,
                        if ctx.store_a_in_c { "c" } else { "a" }
                    );

                    // Mem reads
                    if ctx.generate_minimal_trace {
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
                            *code += &format!("\ttest {}, 0x7 /* address &= 7 */\n", REG_ADDRESS);
                            *code += &format!("\tjnz pc_{:x}_a_address_not_aligned /* check if address is not aligned */\n", ctx.pc);
                            Self::a_src_mem_aligned(&mut ctx, code);
                            unusual_code += &format!("pc_{:x}_a_address_not_aligned:\n", ctx.pc);
                            Self::a_src_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code +=
                                &format!("\tjmp pc_{:x}_a_address_check_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_a_address_check_done:\n", ctx.pc);
                        }
                    }

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }

                    ctx.a.is_saved = true;
                }
                SRC_IMM => {
                    *code += "\t/* a=SRC_IMM */\n";
                    ctx.a.is_constant = true;
                    ctx.a.constant_value =
                        instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32);
                    ctx.a.string_value = format!("0x{:x}", ctx.a.constant_value);
                    if ctx.store_a_in_c {
                        *code += &format!(
                            "\tmov {}, {} /* c = constant */\n",
                            REG_C, ctx.a.string_value
                        );
                        ctx.a.is_saved = false;
                    } else if ctx.store_a_in_a {
                        *code += &format!(
                            "\tmov {}, {} /* a = constant */\n",
                            REG_A, ctx.a.string_value
                        );
                        ctx.a.is_saved = true;
                    } else {
                        ctx.a.is_saved = false;
                    }
                    // DEBUG: Used only to get register traces:
                    //*s += &format!("\tmov {}, {} /* a=a_value */\n", REG_A, ctx.a.string_value);

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }
                }
                SRC_STEP => {
                    *code += "\t/* a=SRC_STEP */\n";
                    let store_a_reg = if ctx.store_a_in_c { REG_C } else { REG_A };
                    let store_a_reg_name = if ctx.store_a_in_c { "c" } else { "a" };
                    *code += &format!(
                        "\tmov {}, {} /* {} = step */\n",
                        store_a_reg, MEM_STEP, store_a_reg_name
                    );
                    if ctx.generate_minimal_trace {
                        *code += &format!(
                            "\tadd {}, chunk_size /* {} += chunk_size */\n",
                            store_a_reg, store_a_reg_name
                        );
                        *code += &format!(
                            "\tsub {}, {} /* {} -= step_down */\n",
                            store_a_reg, REG_STEP_DOWN, store_a_reg_name
                        );
                    }
                    ctx.a.is_saved = !ctx.store_a_in_c;

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 0);
                    }
                }
                _ => {
                    panic!("ZiskRom::source_a() Invalid a_src={} pc={}", instruction.a_src, ctx.pc)
                }
            }

            // Copy a value to main trace
            if ctx.generate_main_trace {
                *code += "\t/* Main[1]=a */\n";
                if ctx.store_a_in_c {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 1*8], {}\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_C
                    );
                } else if ctx.a.is_constant && !ctx.store_a_in_a {
                    *code += &format!(
                        "\tmov {}, 0x{:x} /* value=a_const */\n",
                        REG_A, ctx.a.constant_value
                    );
                    *code += &format!(
                        "\tmov [{} + {}*8 + 1*8], {}\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_A
                    );
                } else {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 1*8], {}\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_A
                    );
                }
            }

            // Copy rom_index<<32 + addr1 to main trace
            // where addr1 = b_offset_imm0 + REG_A(if b=SRC_IND)
            if ctx.generate_main_trace {
                *code += "\t/* Main[0]=rom_index<<32+addr1 */\n";
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
                    *code += &format!(
                        "\tmov {}, {} /* value=rom_index<<32+addr1 (const) */\n",
                        REG_VALUE, value
                    );
                } else {
                    // In this case the value to store is not constant
                    assert!(instruction.b_src == SRC_IND);
                    *code += &format!(
                        "\tmov {}, {} /* value=a */\n",
                        REG_VALUE,
                        if ctx.store_a_in_c { REG_C } else { REG_A }
                    );
                    if instruction.b_offset_imm0 as i64 >= 0 {
                        *code += &format!(
                            "\tmov {}, 0x{:x} /* aux=rom_index<<32+b_offset_imm0 */\n",
                            REG_AUX,
                            instruction.b_offset_imm0 + ((rom_index & 0xffffffff) << 32)
                        );
                        *code += &format!("\tadd {}, {} /* value+=aux */\n", REG_VALUE, REG_AUX);
                    } else {
                        *code += &format!(
                            "\tmov {}, 0x{:x} /* aux=-b_offset_imm0 */\n",
                            REG_AUX,
                            -(instruction.b_offset_imm0 as i64)
                        );
                        *code += &format!(
                            "\tsub {}, {} /* value-=b_offset_imm0 */\n",
                            REG_VALUE, REG_AUX
                        );
                        *code += &format!(
                            "\tmov {}, 0x{:x} /* aux+=rom_index<<32 */\n",
                            REG_AUX,
                            (rom_index & 0xffffffff) << 32
                        );
                        *code += &format!(
                            "\tadd {}, {} /* value+=aux=rom_index<<32+b_offset_imm0 */\n",
                            REG_VALUE, REG_AUX
                        );
                    }
                }
                *code += &format!(
                    "\tmov [{} + {}*8 + 0*8], {}\n",
                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                );
            }

            /************/
            /* B SOURCE */
            /************/

            // Set register b content: all except SRC_C
            match instruction.b_src {
                SRC_C => {}
                SRC_REG => {
                    *code += &format!("\t/* b=SRC_REG reg={} */\n", instruction.b_offset_imm0);

                    assert!(instruction.b_offset_imm0 <= 34);

                    // Read from memory and store in the proper register: b or c
                    let dest_reg = if ctx.store_b_in_c { REG_C } else { REG_B };
                    let dest_desc = if ctx.store_b_in_c { "c" } else { "b" };
                    Self::read_riscv_reg(code, instruction.b_offset_imm0, dest_reg, dest_desc);

                    if ctx.generate_main_trace {
                        Self::trace_reg_access(&mut ctx, code, instruction.b_offset_imm0, 1);
                    }
                }
                SRC_MEM => {
                    *code += "\t/* b=SRC_MEM */\n";

                    // Calculate memory address
                    *code += &format!(
                        "\tmov {}, 0x{:x} /* address = i.b_offset_imm0 */\n",
                        REG_ADDRESS, instruction.b_offset_imm0
                    );
                    if instruction.b_use_sp_imm1 != 0 {
                        *code +=
                            &format!("\tadd {}, {} /* address += sp */\n", REG_ADDRESS, MEM_SP);
                    }

                    // Read value from memory and store in the proper register: b or c
                    *code += &format!(
                        "\tmov {}, [{}] /* {} = mem[address] */\n",
                        if ctx.store_b_in_c { REG_C } else { REG_B },
                        REG_ADDRESS,
                        if ctx.store_b_in_c { "c" } else { "b" }
                    );

                    // Mem reads
                    if ctx.generate_minimal_trace {
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
                            *code += &format!("\ttest {}, 0x7 /* address &= 7 */\n", REG_ADDRESS);
                            *code += &format!("\tjnz pc_{:x}_b_address_not_aligned /* check if address is not aligned */\n", ctx.pc);
                            Self::b_src_mem_aligned(&mut ctx, code);
                            unusual_code += &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);
                            Self::b_src_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code +=
                                &format!("\tjmp pc_{:x}_b_address_check_done\n", ctx.pc);
                            *code += &format!("pc_{:x}_b_address_check_done:\n", ctx.pc);
                        }
                    }

                    ctx.b.is_saved = !ctx.store_b_in_c;

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 1);
                    }
                }
                SRC_IMM => {
                    *code += "\t/* b=SRC_IMM */\n";
                    ctx.b.is_constant = true;
                    ctx.b.constant_value =
                        instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32);
                    ctx.b.string_value = format!("0x{:x}", ctx.b.constant_value);
                    if ctx.store_b_in_c {
                        *code += &format!(
                            "\tmov {}, {} /* c = constant */\n",
                            REG_C, ctx.b.string_value
                        );
                        ctx.b.is_saved = false;
                    } else if ctx.store_b_in_b {
                        *code += &format!(
                            "\tmov {}, {} /* b = constant */\n",
                            REG_B, ctx.b.string_value
                        );
                        ctx.b.is_saved = true;
                    } else {
                        ctx.b.is_saved = false;
                    }
                    // DEBUG: Used only to get register traces:
                    //*s += &format!("\tmov {}, {} /*b=b_value */\n", REG_B, ctx.b.string_value);

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 1);
                    }
                }
                SRC_IND => {
                    *code += &format!("\t/* b=SRC_IND width={}*/\n", instruction.ind_width);

                    // Make sure register a is stored in REG_A
                    // However, since b's source is an indirection, a's source is normally a register
                    if ctx.a.is_constant && !ctx.a.is_saved {
                        *code +=
                            &format!("\tmov {}, {} /* WARNING */\n", REG_A, ctx.a.string_value);
                        ctx.a.is_saved = true;
                    }

                    // Use REG_A if a's value is not needed beyond the b indirection,
                    // or REG_ADDRESS otherwise
                    let reg_address: &str;
                    if instruction.op == ZiskOp::CopyB.code()
                        || instruction.op == ZiskOp::SignExtendB.code()
                        || instruction.op == ZiskOp::SignExtendH.code()
                        || instruction.op == ZiskOp::SignExtendH.code()
                    {
                        reg_address = REG_A;
                    } else {
                        *code += &format!(
                            "\tmov {}, {} /* address = a */\n",
                            REG_ADDRESS, ctx.a.string_value
                        );
                        reg_address = REG_ADDRESS;
                    }

                    // Calculate memory address
                    if instruction.b_offset_imm0 != 0 {
                        *code += &format!(
                            "\tadd {}, 0x{:x} /* address += i.b_offset_imm0 */\n",
                            reg_address, instruction.b_offset_imm0
                        );
                    }
                    if instruction.b_use_sp_imm1 != 0 {
                        *code +=
                            &format!("\tadd {}, {} /* address += sp */\n", reg_address, MEM_SP);
                    }

                    // Read from memory and store in the proper register: b or c
                    match instruction.ind_width {
                        8 => {
                            // Read 8-bytes value from address
                            *code += &format!(
                                "\tmov {}, qword ptr [{}] /* {} = mem[address] */\n",
                                if ctx.store_b_in_c { REG_C } else { REG_B },
                                reg_address,
                                if ctx.store_b_in_c { "c" } else { "b" }
                            );
                        }
                        4 => {
                            // Read 4-bytes value from address
                            *code += &format!(
                                "\tmov {}, [{}] /* {} = mem[address] */\n",
                                if ctx.store_b_in_c { REG_C_W } else { REG_B_W },
                                reg_address,
                                if ctx.store_b_in_c { "c" } else { "b" }
                            );
                        }
                        2 => {
                            // Read 2-bytes value from address
                            *code += &format!(
                                "\tmovzx {}, word ptr [{}] /* {} = mem[address] */\n",
                                if ctx.store_b_in_c { REG_C } else { REG_B },
                                reg_address,
                                if ctx.store_b_in_c { "c" } else { "b" }
                            );
                        }
                        1 => {
                            // Read 1-bytes value from address
                            *code += &format!(
                                "\tmovzx {}, byte ptr [{}] /* {} = mem[address] */\n",
                                if ctx.store_b_in_c { REG_C } else { REG_B },
                                reg_address,
                                if ctx.store_b_in_c { "c" } else { "b" }
                            );
                        }
                        _ => panic!(
                            "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                            instruction.ind_width, ctx.pc
                        ),
                    }

                    // Store memory reads in minimal trace
                    if ctx.generate_minimal_trace {
                        match instruction.ind_width {
                            8 => {
                                // // Check if address is aligned, i.e. it is a multiple of 8
                                *code +=
                                    &format!("\ttest {}, 0x7 /* address &= 7 */\n", reg_address);
                                *code += &format!("\tjnz pc_{:x}_b_address_not_aligned /* check if address is not aligned */\n", ctx.pc);

                                // b register memory address is fully alligned

                                // Copy read data into mem_reads_address and increment it
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} /* mem_reads[@+size*8]=b */\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    if ctx.store_b_in_c { REG_C } else { REG_B }
                                );

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} /* mem_reads_size++ */\n",
                                    REG_MEM_READS_SIZE
                                );

                                // b memory address is not aligned

                                unusual_code +=
                                    &format!("pc_{:x}_b_address_not_aligned:\n", ctx.pc);

                                // Calculate previous aligned address
                                unusual_code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
                                    reg_address
                                );

                                // Store previous aligned address value in mem_reads, and advance address
                                unusual_code += &format!(
                                    "\tmov {}, [{}] /* value = mem[prev_address] */\n",
                                    REG_VALUE, reg_address
                                );
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_b */\n",
                                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                );

                                // Calculate next aligned address
                                unusual_code += &format!(
                                    "\tadd {}, 8 /* address = next aligned address */\n",
                                    reg_address
                                );

                                // Store next aligned address value in mem_reads, and advance it
                                unusual_code += &format!(
                                    "\tmov {}, [{}] /* value = mem[next_address] */\n",
                                    REG_VALUE, reg_address
                                );
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8 + 8], {} /* mem_reads[@+size*8+8] = next_b */\n",
                                    REG_MEM_READS_ADDRESS,
                                    REG_MEM_READS_SIZE,
                                    REG_VALUE
                                );

                                // Increment chunk.steps.mem_reads_size twice
                                unusual_code += &format!(
                                    "\tadd {}, 2 /* mem_reads_size += 2*/\n",
                                    REG_MEM_READS_SIZE
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
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
                                    reg_address
                                );

                                // Store previous aligned address value in mem_reads, advancing address
                                *code += &format!(
                                    "\tmov {}, [{}] /* value = mem[prev_address] */\n",
                                    REG_VALUE, reg_address
                                );
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_b */\n",
                                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                );

                                // Calculate next aligned address, keeping a copy of previous aligned
                                // address in value
                                *code += &format!(
                                    "\tmov {}, {} /* value = copy of prev_address */\n",
                                    REG_VALUE, reg_address
                                );
                                let address_increment = instruction.ind_width - 1;
                                *code += &format!(
                                    "\tadd {}, {} /* address += {} */\n",
                                    reg_address, address_increment, address_increment
                                );
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = next aligned address */\n",
                                    reg_address
                                );
                                *code += &format!(
                                    "\tcmp {}, {} /* prev_address = next_address ? */\n",
                                    REG_VALUE, reg_address
                                );
                                *code += &format!(
                                    "\tjnz pc_{:x}_b_ind_different_address /* jump if they are the same */\n",
                                    ctx.pc
                                );

                                // Same address

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} /* mem_reads_size++ */\n",
                                    REG_MEM_READS_SIZE
                                );

                                // Different address

                                unusual_code +=
                                    &format!("pc_{:x}_b_ind_different_address:\n", ctx.pc);

                                // Store next aligned address value in mem_reads
                                unusual_code += &format!(
                                    "\tmov {}, [{}] /* value = mem[next_address] */\n",
                                    REG_VALUE, reg_address
                                );

                                // Copy read data into mem_reads_address and advance it
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8 + 8], {} /* mem_reads[@+size*8+8] = next_b */\n",
                                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                );

                                // Increment chunk.steps.mem_reads_size
                                unusual_code += &format!(
                                    "\tadd {}, 2 /* mem_reads_size+=2 */\n",
                                    REG_MEM_READS_SIZE
                                );

                                unusual_code +=
                                    &format!("\tjmp pc_{:x}_b_ind_address_done\n", ctx.pc);

                                // Done
                                *code += &format!("pc_{:x}_b_ind_address_done:\n", ctx.pc);
                            }
                            1 => {
                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
                                    reg_address
                                );

                                // Store previous aligned address value in mem_reads, and increment address
                                *code += &format!(
                                    "\tmov {}, [{}] /* value = mem[prev_address] */\n",
                                    REG_VALUE, reg_address
                                );
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_b */\n",
                                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                );

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} /* mem_reads_size++ */\n",
                                    REG_MEM_READS_SIZE
                                );
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                    }
                    ctx.b.is_saved = !ctx.store_b_in_c;

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 1);
                    }
                }
                _ => panic!(
                    "ZiskRom2Asm::save_to_asm() Invalid b_src={} pc={}",
                    instruction.b_src, ctx.pc
                ),
            }

            // Copy b value to main trace
            if ctx.generate_main_trace {
                *code += "\t/* Main[2]=b */\n";
                if ctx.store_b_in_c {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} /* b=c */\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_C
                    );
                } else if ctx.b.is_constant && !ctx.store_b_in_b {
                    *code += &format!(
                        "\tmov {}, 0x{:x} /* value=b_const */\n",
                        REG_B, ctx.b.constant_value
                    );
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} /* b=const */\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_B
                    );
                } else {
                    *code += &format!(
                        "\tmov [{} + {}*8 + 2*8], {} /* b */\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_B
                    );
                }
            }

            /*************/
            /* Operation */
            /*************/

            // Execute operation, storing result is registers c and flag
            //*s += &format!("\t/* operation: (c, flag) = op(a, b) */\n");
            Self::operation_to_asm(&mut ctx, instruction.op, code, &mut unusual_code);

            // At this point, REG_C must contain the value of c
            assert!(ctx.c.is_saved);

            // Copy c value to main trace
            if ctx.generate_main_trace {
                *code += "\t/* Main[3]=c */\n";
                *code += &format!(
                    "\tmov [{} + {}*8 + 3*8], {}\n",
                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_C
                );
            }

            /***********/
            /* STORE C */
            /***********/

            // Store register c
            match instruction.store {
                STORE_NONE => {
                    *code += "\t/* STORE_NONE */\n";

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 2);
                    }
                }
                STORE_REG => {
                    assert!(instruction.store_offset >= 0);
                    assert!(instruction.store_offset <= 34);

                    // Copy previous reg value to main trace
                    if ctx.generate_main_trace {
                        *code += "\t/* Main[4]=prev_reg_c */\n";
                        Self::read_riscv_reg(
                            code,
                            instruction.store_offset as u64,
                            REG_VALUE,
                            "value",
                        );

                        *code += &format!(
                            "\tmov [{} + {}*8 + 4*8], {} /* main[@+size*8+4*8]=prev_reg */\n",
                            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                        );
                    }

                    *code += &format!("\t/* STORE_REG reg={} */\n", instruction.store_offset);

                    // Store in mem[address]
                    if instruction.store_ra {
                        let value = (ctx.pc as i64 + instruction.jmp_offset2) as u64;
                        Self::write_riscv_reg_constant(
                            code,
                            instruction.store_offset as u64,
                            value,
                            "pc + jmp_offset2",
                        );
                    } else {
                        Self::write_riscv_reg(code, instruction.store_offset as u64, REG_C, "c");
                    }

                    if ctx.generate_main_trace {
                        Self::trace_reg_access(&mut ctx, code, instruction.store_offset as u64, 2);
                    }
                }
                STORE_MEM => {
                    *code += "\t/* STORE_MEM */\n";

                    // Calculate memory address and store it in REG_ADDRESS
                    *code += &format!(
                        "\tmov {}, 0x{:x}/* address = i.store_offset */\n",
                        REG_ADDRESS, instruction.store_offset
                    );
                    if instruction.store_use_sp {
                        *code +=
                            &format!("\tadd {}, {} /* address += sp */\n", REG_ADDRESS, MEM_SP);
                    }

                    // Mem reads
                    if ctx.generate_minimal_trace {
                        if !instruction.store_use_sp {
                            if (instruction.store_offset & 0x7) != 0 {
                                Self::c_store_mem_not_aligned(&mut ctx, code);
                            }
                        } else {
                            *code += &format!("\ttest {}, 0x7 /* address &= 7 */\n", REG_ADDRESS);
                            *code += &format!("\tjnz pc_{:x}_c_address_not_aligned\n", ctx.pc);
                            unusual_code += &format!("pc_{:x}_c_address_not_aligned:\n", ctx.pc);
                            Self::c_store_mem_not_aligned(&mut ctx, &mut unusual_code);
                            unusual_code += &format!("\tjmp pc_{:x}_c_address_aligned\n", ctx.pc);
                            *code += &format!("pc_{:x}_c_address_aligned:\n", ctx.pc);
                        }
                    }

                    // Store mem[address] = value
                    if instruction.store_ra {
                        *code += &format!(
                            "\tmov {}, 0x{:x} /* value = pc + jmp_offset2 */\n",
                            REG_VALUE,
                            (ctx.pc as i64 + instruction.jmp_offset2) as u64
                        );
                        *code += &format!(
                            "\tmov [{}], {} /* mem[address] = value */\n",
                            REG_ADDRESS, REG_VALUE
                        );
                    } else {
                        *code +=
                            &format!("\tmov [{}], {} /* mem[address] = c */\n", REG_ADDRESS, REG_C);
                    }

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 2);
                    }
                }
                STORE_IND => {
                    *code += &format!("\t/* STORE_IND width={} */\n", instruction.ind_width);

                    // Calculate memory address and store it in REG_ADDRESS
                    *code += &format!(
                        "\tmov {}, {} /* address = a */\n",
                        REG_ADDRESS, ctx.a.string_value
                    );
                    if instruction.store_offset != 0 {
                        *code += &format!(
                            "\tadd {}, 0x{:x} /* address += i.store_offset */\n",
                            REG_ADDRESS, instruction.store_offset as u64
                        );
                    }
                    if instruction.store_use_sp {
                        *code +=
                            &format!("\tadd {}, {} /* address += sp */\n", REG_ADDRESS, MEM_SP);
                    }

                    let address_is_constant = ctx.a.is_constant && !instruction.store_use_sp;
                    let address_constant_value = if address_is_constant {
                        (ctx.a.constant_value as i64 + instruction.store_offset) as u64
                    } else {
                        0
                    };
                    let address_is_aligned =
                        address_is_constant && ((address_constant_value & 0x7) == 0);

                    // Save data in mem_reads
                    if ctx.generate_minimal_trace {
                        match instruction.ind_width {
                            8 => {
                                // Check if address is aligned, i.e. it is a multiple of 8
                                if address_is_constant {
                                    if !address_is_aligned {
                                        Self::c_store_ind_8_not_aligned(&mut ctx, code);
                                    }
                                } else {
                                    *code += &format!(
                                        "\ttest {}, 0x7 /* address &= 7 */\n",
                                        REG_ADDRESS
                                    );
                                    *code += &format!("\tjnz pc_{:x}_c_address_not_aligned /* check if address is aligned */\n", ctx.pc);
                                    unusual_code +=
                                        &format!("pc_{:x}_c_address_not_aligned:\n", ctx.pc);
                                    Self::c_store_ind_8_not_aligned(&mut ctx, &mut unusual_code);
                                    unusual_code += &format!(
                                        "\tjmp pc_{:x}_c_address_done /* address is aligned; done */\n",
                                        ctx.pc
                                    );
                                    *code += &format!("pc_{:x}_c_address_done:\n", ctx.pc);
                                }
                            }
                            4 | 2 => {
                                // Get a copy of the address to preserve it
                                *code += &format!(
                                    "\tmov {}, {} /* aux = address */\n",
                                    REG_AUX, REG_ADDRESS
                                );

                                // Calculate previous aligned address
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
                                    REG_AUX
                                );

                                // Store previous aligned address value in mem_reads, advancing address
                                *code += &format!(
                                    "\tmov {}, [{}] /* value = mem[prev_address] */\n",
                                    REG_VALUE, REG_AUX
                                );
                                *code += &format!(
                                    "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_c */\n",
                                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                );

                                // Calculate next aligned address, keeping a copy of previous aligned
                                // address in value
                                *code += &format!(
                                    "\tmov {}, {} /* value = copy of prev_address */\n",
                                    REG_VALUE, REG_AUX
                                );
                                let address_increment = instruction.ind_width - 1;
                                *code += &format!(
                                    "\tadd {}, {} /* address += {} */\n",
                                    REG_AUX, address_increment, address_increment
                                );
                                *code += &format!(
                                    "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = next aligned address */\n",
                                    REG_AUX
                                );
                                *code += &format!(
                                    "\tcmp {}, {} /* prev_address = next_address ? */\n",
                                    REG_VALUE, REG_AUX
                                );
                                *code += &format!(
                                    "\tjnz pc_{:x}_c_ind_different_address /* jump if they are the same */\n",
                                    ctx.pc
                                );

                                // Same address

                                // Increment chunk.steps.mem_reads_size
                                *code += &format!(
                                    "\tinc {} /* mem_reads_size++ */\n",
                                    REG_MEM_READS_SIZE
                                );

                                // Different address

                                unusual_code +=
                                    &format!("pc_{:x}_c_ind_different_address:\n", ctx.pc);

                                // Store next aligned address value in mem_reads
                                unusual_code += &format!(
                                    "\tmov {}, [{}] /* value = mem[next_address] */\n",
                                    REG_VALUE, REG_AUX
                                );

                                // Copy read data into mem_reads_address and advance it
                                unusual_code += &format!(
                                    "\tmov [{} + {}*8 + 8], {} /* mem_reads[@+size*8+8] = next_c */\n",
                                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                );

                                // Increment chunk.steps.mem_reads_size
                                unusual_code += &format!(
                                    "\tadd {}, 2 /* mem_reads_size+=2 */\n",
                                    REG_MEM_READS_SIZE
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
                                        "\tmov {}, [{}] /* value = mem[address] */\n",
                                        REG_VALUE, REG_ADDRESS
                                    );
                                    *code += &format!(
                                        "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_c */\n",
                                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                    );

                                    // Increment chunk.steps.mem_reads_size
                                    *code += &format!(
                                        "\tinc {} /* mem_reads_size++ */\n",
                                        REG_MEM_READS_SIZE
                                    );
                                } else {
                                    // Get a copy of the address to preserve it
                                    *code += &format!(
                                        "\tmov {}, {} /* aux = address */\n",
                                        REG_AUX, REG_ADDRESS
                                    );

                                    // Calculate previous aligned address
                                    *code += &format!(
                                        "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
                                        REG_AUX
                                    );

                                    // Store previous aligned address value in mem_reads, and increment address
                                    *code += &format!(
                                        "\tmov {}, [{}] /* value = mem[prev_address] */\n",
                                        REG_VALUE, REG_AUX
                                    );
                                    *code += &format!(
                                        "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_c */\n",
                                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                                    );

                                    // Increment chunk.steps.mem_reads_size
                                    *code += &format!(
                                        "\tinc {} /* mem_reads_size++ */\n",
                                        REG_MEM_READS_SIZE
                                    );
                                }
                            }
                            _ => panic!(
                                "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                                instruction.ind_width, ctx.pc
                            ),
                        }
                    }

                    // Store mem[address] = value
                    match instruction.ind_width {
                        8 => {
                            if instruction.store_ra {
                                *code += &format!(
                                    "\tmov qword ptr [{}], {} /* width=8: mem[address] = pc + jmp_offset2 */\n",
                                    REG_ADDRESS,
                                    (ctx.pc as i64 + instruction.jmp_offset2) as u64
                                );
                            } else {
                                *code += &format!(
                                    "\tmov [{}], {} /* width=8: mem[address] = c */\n",
                                    REG_ADDRESS, REG_C
                                );
                            }
                        }
                        4 => {
                            if instruction.store_ra {
                                *code += &format!(
                                    "\tmov dword ptr [{}], {} /* width=4: mem[address] = pc + jmp_offset2 */\n",
                                    REG_ADDRESS,
                                    (ctx.pc as i64 + instruction.jmp_offset2) as u64
                                );
                            } else {
                                *code += &format!(
                                    "\tmov [{}], {} /* width=4: mem[address] = c */\n",
                                    REG_ADDRESS, REG_C_W
                                );
                            }
                        }
                        2 => {
                            if instruction.store_ra {
                                *code += &format!(
                                    "\tmov word ptr [{}], {} /* width=2: mem[address] = pc + jmp_offset2 */\n",
                                    REG_ADDRESS,
                                    (ctx.pc as i64 + instruction.jmp_offset2) as u64
                                );
                            } else {
                                *code += &format!(
                                    "\tmov [{}], {} /* width=2: mem[address] = c */\n",
                                    REG_ADDRESS, REG_C_H
                                );
                            }
                        }
                        1 => {
                            if instruction.store_ra {
                                *code += &format!(
                                    "\tmov word ptr [{}], {} /* width=1: mem[address] = pc + jmp_offset2 */\n",
                                    REG_ADDRESS,
                                    (ctx.pc as i64 + instruction.jmp_offset2) as u64
                                );
                            } else {
                                *code += &format!(
                                    "\tmov [{}], {} /* width=1: mem[address] = c */\n",
                                    REG_ADDRESS, REG_C_B
                                );
                            }
                            if ctx.log_output {
                                *code += &format!(
                                    "\tmov {}, 0xa0000200 /* width=1: aux = UART */\n",
                                    REG_FLAG,
                                );
                                *code += &format!(
                                    "\tcmp {}, {} /* width=1: if address = USART then print char */\n",
                                    REG_ADDRESS, REG_FLAG
                                );
                                *code += &format!(
                                    "\tjne pc_{:x}_store_c_not_uart /* width=1: continue */\n",
                                    ctx.pc,
                                );
                                if instruction.store_ra {
                                    *code += &format!(
                                        "\tmov dil, 0x{:x} /* width=1: rdi = value */\n",
                                        (ctx.pc as i64 + instruction.jmp_offset2) as u64 as u8
                                    );
                                } else {
                                    *code +=
                                        &format!("\tmov dil, {} /* width=1: rdi = c */\n", REG_C_B);
                                }
                                Self::push_internal_registers(&mut ctx, code);
                                *code += "\tcall _print_char /* width=1: call print_char() */\n";
                                Self::pop_internal_registers(&mut ctx, code);
                                *code += &format!("pc_{:x}_store_c_not_uart:\n", ctx.pc);
                            }
                        }
                        _ => panic!(
                            "ZiskRom2Asm::save_to_asm() Invalid ind_width={} pc={}",
                            instruction.ind_width, ctx.pc
                        ),
                    }

                    if ctx.generate_main_trace {
                        Self::clear_reg_step_ranges(&mut ctx, code, 2);
                    }
                }
                _ => panic!(
                    "ZiskRom2Asm::save_to_asm() Invalid store={} pc={}",
                    instruction.store, ctx.pc
                ),
            }

            // if ctx.c.is_constant && !ctx.c.string_value.eq(REG_C) {
            //     *s += &format!(
            //         "\tmov {}, {} /* STORE: make sure c=value */\n",
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

            if ctx.generate_main_trace {
                *code += "\t/* Main[5] = prev_reg_mem[0] + (prev_reg_mem[1] & 0xfffff ) << 40 */\n";
                *code += &format!("\tmov {}, qword ptr [reg_prev_steps_1]\n", REG_VALUE);
                *code += &format!("\tshl {}, 40\n", REG_VALUE); // 64-40=24 bits
                *code += &format!("\tmov {}, qword ptr [reg_prev_steps_0]\n", REG_AUX);
                *code += &format!("\tadd {}, {}\n", REG_VALUE, REG_AUX);
                *code += &format!(
                    "\tmov [{} + {}*8 + 5*8], {} /* main[@+size*8+5*8]=value */\n",
                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                );

                *code += "\t/* Main[6] = prev_reg_mem[2] + (prev_reg_mem[1] & 0xfffff00000 ) << 21 + flag<<24 */\n";
                *code += &format!("\tmov {}, qword ptr [reg_prev_steps_1]\n", REG_VALUE);
                *code += &format!("\tmov {}, 0xfffff00000\n", REG_AUX);
                *code += &format!("\tand {}, {}\n", REG_VALUE, REG_AUX);
                *code += &format!("\tshl {}, 21\n", REG_VALUE);
                *code += &format!("\tmov {}, qword ptr [reg_prev_steps_2]\n", REG_AUX);
                *code += &format!("\tadd {}, {}\n", REG_VALUE, REG_AUX);
                if ctx.flag_is_always_one {
                    *code += &format!("\tmov {}, 0x10000000000\n", REG_AUX);
                    *code += &format!("\tadd {}, {}\n", REG_VALUE, REG_AUX);
                } else if ctx.flag_is_always_zero {
                    // Nothing to add
                } else {
                    *code += &format!("\tmov {}, {}\n", REG_AUX, REG_FLAG);
                    *code += &format!("\tshl {}, 24\n", REG_AUX);
                    *code += &format!("\tadd {}, {}\n", REG_VALUE, REG_AUX);
                }
                *code += &format!(
                    "\tmov [{} + {}*8 + 6*8], {} /* main[@+size*8+6*8]=value */\n",
                    REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
                );

                // Increment chunk.steps.mem_reads_size in 7 u64 slots
                *code += &format!("\tadd {}, 7 /* mem_reads_size += 7 */\n", REG_MEM_READS_SIZE);
            }

            /********/
            /* STEP */
            /********/

            // Decrement step counter
            *code += "\t/* STEP */\n";
            if ctx.generate_fast || ctx.generate_rom_histogram || ctx.generate_main_trace {
                *code += &format!("\tinc {} /* increment step */\n", MEM_STEP);
            }
            if ctx.generate_chunks || ctx.generate_minimal_trace || ctx.generate_main_trace {
                *code += &format!("\tdec {} /* decrement step_down */\n", REG_STEP_DOWN);
                if instruction.end {
                    *code += &format!("\tmov {}, 1 /* end = 1 */\n", MEM_END);
                    *code += &format!("\tmov {}, 0x{:08x} /* value = pc */\n", REG_VALUE, ctx.pc);
                    *code += &format!("\tmov {}, {} /* pc = value */\n", MEM_PC, REG_VALUE);
                    *code += "\tcall chunk_end\n";
                } else {
                    *code += &format!("\tjz pc_{:x}_step_zero\n", ctx.pc);
                    unusual_code += &format!("pc_{:x}_step_zero:\n", ctx.pc);
                    Self::set_pc(&mut ctx, instruction, &mut unusual_code, "z");
                    unusual_code += "\tcall chunk_end_and_start\n";
                    unusual_code += &format!("\tjmp pc_{:x}_step_done\n", ctx.pc);
                    Self::set_pc(&mut ctx, instruction, code, "nz");
                    *code += &format!("pc_{:x}_step_done:\n", ctx.pc);
                }
            }
            if ctx.generate_fast || ctx.generate_rom_histogram {
                if instruction.end {
                    *code += &format!("\tmov {}, 1 /* end = 1 */\n", MEM_END);
                }
                Self::set_pc(&mut ctx, instruction, code, "nz");
            }

            // Used only to get logs of step
            // *s += &format!("\tmov {}, {} /* value = step */\n", REG_VALUE, MEM_STEP);
            // *s += &format!("\tand {}, 0xfffff /* value = step */\n", REG_VALUE);
            // *s += &format!("\tcmp {}, 0 /* value = step */\n", REG_VALUE);
            // *s += &format!("\tjne  pc_{:x}_inc_step_done /* value = step */\n", ctx.pc);
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
            // *s += &format!("\tmov {}, {} /* copy step into value */\n", REG_VALUE, MEM_STEP);
            // *s += &format!("\tand {}, 0xffff /* value &= k */\n", REG_VALUE);
            // *s += &format!(
            //     "\tjnz pc_{:x}_no_store_data /* skip if storing is not required */\n",
            //     ctx.pc
            // );
            // *s += &format!("\t/* Store data */\n");
            // *s += &format!("pc_{:x}_no_store_data:\n", ctx.pc);

            // Jump to new pc, if not the next one
            if instruction.end {
                *code += "\tjmp execute_end /* end */\n";
            } else if !ctx.jump_to_static_pc.is_empty() {
                *code += ctx.jump_to_static_pc.as_str();
            } else if ctx.jump_to_dynamic_pc {
                *code += "\t/* jump to dynamic pc */\n";
                *code += &format!("\tmov {}, {} /* value=pc */\n", REG_VALUE, MEM_PC);
                *code += &format!("\tmov {}, 0x80000000 /* is pc a low address? */\n", REG_ADDRESS);
                *code += &format!("\tcmp {}, {}\n", REG_VALUE, REG_ADDRESS);
                *code += &format!("\tjb pc_{:x}_jump_to_low_address\n", ctx.pc);
                *code += &format!("\tsub {}, {} /* pc -= 0x80000000 */\n", REG_VALUE, REG_ADDRESS);
                *code += &format!("\tmov rax, {} /* rax = pc */\n", REG_VALUE);
                *code += "\tlea rbx, [map_pc_80000000] /* rbx = index table base address */\n";
                *code += "\tmov rax, [rbx + rax*2] /* rax = table entry address */\n";
                *code += "\tjmp rax /* jump to table entry address */\n";
                *code += &format!("pc_{:x}_jump_to_low_address:\n", ctx.pc);
                *code += &format!("\tsub {}, 0x1000 /* pc -= 0x1000 */\n", REG_VALUE);
                *code += &format!("\tmov rax, {} /* rax = pc */\n", REG_VALUE);
                *code += "\tlea rbx, [map_pc_1000] /* rbx = index table base address */\n";
                *code += "\tmov rax, [rbx + rax*2] /* rax = table entry address */\n";
                *code += "\tjmp rax /* jump to table entry address */\n";
            }
        }

        *code += "\n";

        *code += "execute_end:\n";

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

        // For all program addresses in the vector, create an assembly set of instructions with a
        // map label
        *code += "\n";
        *code += ".section .rodata\n";
        *code += ".align 64\n";
        for key in &rom.sorted_pc_list {
            // Skip internal pc addresses
            if (key & 0x03) != 0 {
                continue;
            }
            // Map fixed-length pc labels to real variable-length instruction labels
            // This is used to implement dynamic jumps, i.e. to jump to an address that is not
            // a constant in the instruction, but dynamically built as part of the emulation

            // Only use labels in boundary pc addresses
            // match *key {
            //     0x1000 | 0x10000000 | 0x80000000 => {
            //         *s += &format!("\nmap_pc_{:x}: \t.quad pc_{:x}", key, key)
            //     }
            //     _ => *s += &format!(", pc_{:x}", key),
            // }

            // Use labels always
            *code += &format!("map_pc_{:x}: \t.quad pc_{:x}\n", key, key);
        }
        *code += "\n";

        let mut lines = code.lines();
        //let mut empty_lines_counter = 0u64;
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
                //empty_lines_counter += 1;
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

        #[cfg(debug_assertions)]
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
                *code += &format!("\tmov {}, 0 /* Flag: c = 0 */\n", REG_C);
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
                    "\tmovsx {}, {} /* SignExtendW: sign extend b(8b) to c(64b) */\n",
                    REG_C, REG_B_B
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SignExtendH => {
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tmovsx {}, {} /* SignExtendW: sign extend b(16b) to c(64b) */\n",
                    REG_C, REG_B_H
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SignExtendW => {
                assert!(ctx.store_b_in_b);
                *code += &format!(
                    "\tmovsxd {}, {} /* SignExtendW: sign extend b(32b) to c(64b) */\n",
                    REG_C, REG_B_W
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Add => {
                if ctx.a.is_constant && (ctx.a.constant_value == 0) {
                    assert!(ctx.store_b_in_c);
                    *code += "\t/* Add: c = a(0) + b = b */\n";
                } else if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    assert!(ctx.store_a_in_c);
                    *code += "\t/* Add: c = a + b(0) = a */\n";
                } else {
                    assert!(ctx.store_a_in_c);
                    *code += "\t/* Add: c = a */\n";
                    *code += &format!(
                        "\tadd {}, {} /* Add: c = c + b = a + b */\n",
                        REG_C, ctx.b.string_value
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::AddW => {
                assert!(ctx.store_b_in_b);
                // DEBUG: Used only to preserve b value
                // s +=
                //     &format!("\tmov {}, {} /* AddW: value = b */\n", REG_VALUE, ctx.b.string_value);
                if ctx.a.is_constant && (ctx.a.constant_value == 0) {
                    *code += "\t/* AddW: ignoring a since a = 0 */\n";
                } else {
                    *code +=
                        &format!("\tadd {}, {} /* AddW: b += a */\n", REG_B, ctx.a.string_value);
                }
                *code += "\tcdqe /* AddW: trunk b */\n";
                *code += &format!("\tmov {}, {} /* AddW: c = b */\n", REG_C, REG_B);
                ctx.c.is_saved = true;
                // DEBUG: Used only to preserve b value
                //s += &format!("\tmov {}, {} /* AddW: b = value */\n", REG_B, REG_VALUE);
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sub => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += "\t/* Sub: ignoring b since b = 0 */\n";
                } else {
                    *code += &format!(
                        "\tsub {}, {} /* Sub: c = c - b = a - b */\n",
                        REG_C, ctx.b.string_value
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SubW => {
                assert!(ctx.store_a_in_a);
                // DEBUG: Used only to preserve b value
                // s += &format!(
                //     "\tmov {}, {} /* SubW: address = a */\n",
                //     REG_ADDRESS, ctx.a.string_value
                // );
                // s +=
                //     &format!("\tmov {}, {} /* SubW: value = b */\n", REG_VALUE, ctx.b.string_value);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += "\t/* SubW: ignoring b since b = 0 */\n";
                } else {
                    *code +=
                        &format!("\tsub {}, {} /* SubW: a -= b */\n", REG_A, ctx.b.string_value);
                }
                *code += &format!("\tmov {}, {} /* SubW: b = a = a - b*/\n", REG_B, REG_A);
                *code += "\tcdqe /* SubW: trunk b */\n";
                *code += &format!("\tmov {}, {} /* SubW: c = b */\n", REG_C, REG_B);
                ctx.c.is_saved = true;
                // DEBUG: Used only to preserver a,b values
                // s += &format!("\tmov {}, {} /* SubW: a = address */\n", REG_A, REG_ADDRESS);
                // s += &format!("\tmov {}, {} /* SubW: b = value */\n", REG_B, REG_VALUE);
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sll => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tshl {}, 0x{:x} /* Sll: c = a << b */\n",
                        REG_C,
                        ctx.b.constant_value & 0x3f
                    );
                } else {
                    *code += &format!("\tmov rcx, {} /* Sll: c = b */\n", REG_B);
                    *code += &format!("\tshl {}, cl /* Sll: c(value) = a << b */\n", REG_C);
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SllW => {
                *code +=
                    &format!("\tmov {}, {} /* SllW: value = a */\n", REG_VALUE, ctx.a.string_value);
                *code += &format!("\tmov rcx, {} /* SllW: c = b */\n", ctx.b.string_value);
                *code += &format!("\tshl {}, cl /* SllW: value = a << b */\n", REG_VALUE_W);
                *code += &format!(
                    "\tmovsxd {}, {} /* SllW: sign extend to quad value -> c */\n",
                    REG_C, REG_VALUE_W
                );
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Sra => {
                assert!(ctx.store_a_in_c);
                *code += &format!("\tmov rcx, {} /* Sra: rcx = b */\n", ctx.b.string_value);
                *code += &format!("\tsar {}, cl /* Sra: c = c >> b(cl) */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Srl => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tshr {}, 0x{:x} /* Srl: c = a >> b */\n",
                        REG_C,
                        ctx.b.constant_value & 0x3f
                    );
                } else {
                    *code += &format!("\tmov rcx, {} /* Srl: b = value */\n", ctx.b.string_value);
                    *code += &format!("\tshr {}, cl /* Srl: c(value) = a >> b */\n", REG_C);
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SraW => {
                if ctx.b.is_constant {
                    *code +=
                        &format!("\tmov {}, {} /* SraW: c = a */\n", REG_VALUE, ctx.a.string_value);
                    *code += &format!(
                        "\tsar {}, 0x{:x} /* SraW: c = a >> b */\n",
                        REG_VALUE_W,
                        ctx.b.constant_value & 0x3f
                    );
                    *code += &format!(
                        "\tmovsxd {}, {} /* SraW: sign extend to quad */\n",
                        REG_C, REG_VALUE_W
                    );
                } else {
                    *code += &format!(
                        "\tmov {}, {} /* SraW: c(value) = a */\n",
                        REG_VALUE, ctx.a.string_value
                    );
                    *code += &format!("\tmov rcx, {} /* SraW: rcx = b */\n", REG_B);
                    *code += &format!("\tsar {}, cl /* SraW: c(value) = a >> b */\n", REG_VALUE_W);
                    *code += &format!(
                        "\tmovsxd {}, {} /* SraW: sign extend to quad */\n",
                        REG_C, REG_VALUE_W
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::SrlW => {
                if ctx.b.is_constant {
                    *code +=
                        &format!("\tmov {}, {} /* SrlW: c = a */\n", REG_VALUE, ctx.a.string_value);
                    *code += &format!(
                        "\tshr {}, 0x{:x} /* SrlW: c = a >> b */\n",
                        REG_VALUE_W,
                        ctx.b.constant_value & 0x3f
                    );
                    *code += &format!(
                        "\tmovsxd {}, {} /* SrlW: sign extend to quad */\n",
                        REG_C, REG_VALUE_W
                    );
                } else {
                    *code +=
                        &format!("\tmov {}, {} /* SrlW: c = a */\n", REG_VALUE, ctx.a.string_value);
                    *code += &format!("\tmov rcx, {} /* SrlW: b = value */\n", ctx.b.string_value);
                    *code += &format!("\tshr {}, cl /* SrlW: c(value) = a >> b */\n", REG_VALUE_W);
                    *code += &format!(
                        "\tmovsxd {}, {} /* SlrW: sign extend to quad */\n",
                        REG_C, REG_VALUE_W
                    );
                }
                ctx.c.is_saved = true;
                //s += &format!("\tmov {}, {} /* SrlW: c = value */\n", REG_C, REG_VALUE);
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Eq => {
                assert!(ctx.store_a_in_a);
                *code += &format!("\tcmp {}, {} /* Eq: a == b ? */\n", REG_A, ctx.b.string_value);
                *code += &format!("\tje pc_{:x}_equal_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_equal_done\n", ctx.pc);
                *code += &format!("pc_{:x}_equal_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_equal_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::EqW => {
                // Make sure a is in REG_A to compare it against b (constant, expression or reg)
                if ctx.a.is_constant {
                    *code += &format!(
                        "\tmov {}, 0x{:x} /* EqW: a = const_value */\n",
                        REG_A,
                        ctx.a.constant_value & 0xffffffff
                    );
                }
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} /* EqW: a == b ? */\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff
                    );
                } else {
                    *code += &format!("\tcmp {}, {} /* EqW: a == b ? */\n", REG_A_W, REG_B_W);
                }
                *code += &format!("\tje pc_{:x}_equal_w_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_equal_w_done\n", ctx.pc);
                *code += &format!("pc_{:x}_equal_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_equal_w_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Ltu => {
                assert!(ctx.store_a_in_a);
                *code += &format!("\tcmp {}, {} /* Ltu: a == b ? */\n", REG_A, ctx.b.string_value);
                *code += &format!("\tjb pc_{:x}_ltu_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_ltu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_ltu_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_ltu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Lt => {
                assert!(ctx.store_a_in_a);
                // If b is constant and too big, move it to its register
                if ctx.b.is_constant && (ctx.b.constant_value >= P2_32) {
                    *code += &format!(
                        "\tmov {}, {} /* Lt: b = const_value */\n",
                        REG_B, ctx.b.string_value
                    );
                    ctx.b.is_constant = false;
                    ctx.b.string_value = REG_B.to_string();
                }
                *code += &format!("\tcmp {}, {} /* Lt: a == b ? */\n", REG_A, ctx.b.string_value);
                *code += &format!("\tjl pc_{:x}_lt_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_lt_done\n", ctx.pc);
                *code += &format!("pc_{:x}_lt_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_lt_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LtuW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} /* LtuW: a == b ? */\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff
                    );
                } else {
                    *code += &format!("\tcmp {}, {} /* LtuW: a == b ? */\n", REG_A_W, REG_B_W);
                }
                *code += &format!("\tjb pc_{:x}_ltuw_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_ltuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_ltuw_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_ltuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LtW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} /* LtW: a == b ? */\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff
                    );
                } else {
                    *code += &format!("\tcmp {}, {} /* LtW: a == b ? */\n", REG_A_W, REG_B_W);
                }
                *code += &format!("\tjl pc_{:x}_ltw_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_ltw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_ltw_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_ltw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Leu => {
                assert!(ctx.store_a_in_a);
                // If b is constant and too big, move it to its register
                if ctx.b.is_constant && (ctx.b.constant_value >= P2_32) {
                    *code += &format!(
                        "\tmov {}, {} /* Leu: b = const_value */\n",
                        REG_B, ctx.b.string_value
                    );
                    ctx.b.is_constant = false;
                    ctx.b.string_value = REG_B.to_string();
                }
                *code += &format!("\tcmp {}, {} /* Leu: a == b ? */\n", REG_A, ctx.b.string_value);
                *code += &format!("\tpc_{:x}_jbe leu_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tpc_{:x}_jmp leu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_leu_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_leu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::Le => {
                assert!(ctx.store_a_in_a);
                // If b is constant and too big, move it to its register
                if ctx.b.is_constant && (ctx.b.constant_value >= P2_32) {
                    *code += &format!(
                        "\tmov {}, {} /* Le: b = const_value */\n",
                        REG_B, ctx.b.string_value
                    );
                    ctx.b.is_constant = false;
                    ctx.b.string_value = REG_B.to_string();
                }
                *code += &format!("\tcmp {}, {} /* Le: a == b ? */\n", REG_A, ctx.b.string_value);
                *code += &format!("\tjle pc_{:x}_lte_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_lte_done\n", ctx.pc);
                *code += &format!("pc_{:x}_lte_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_lte_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LeuW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} /* LeuW: a == b ? */\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff
                    );
                } else {
                    *code += &format!("\tcmp {}, {} /* LeuW: a == b ? */\n", REG_A_W, REG_B_W);
                }
                *code += &format!("\tjbe pc_{:x}_leuw_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_leuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_leuw_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_leuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::LeW => {
                assert!(ctx.store_a_in_a);
                // Compare against b, either as a numeric constant or as a register
                if ctx.b.is_constant {
                    *code += &format!(
                        "\tcmp {}, 0x{:x} /* LeW: a == b ? */\n",
                        REG_A_W,
                        ctx.b.constant_value & 0xffffffff
                    );
                } else {
                    *code += &format!("\tcmp {}, {} /* LeW: a == b ? */\n", REG_A_W, REG_B_W);
                }
                *code += &format!("\tjle pc_{:x}_lew_true\n", ctx.pc);
                *code += &format!("\tmov {}, 0 /* c = 0 */\n", REG_C);
                *code += &format!("\tmov {}, 0 /* flag = 0 */\n", REG_FLAG);
                *code += &format!("\tjmp pc_{:x}_lew_done\n", ctx.pc);
                *code += &format!("pc_{:x}_lew_true:\n", ctx.pc);
                *code += &format!("\tmov {}, 1 /* c = 1 */\n", REG_C);
                *code += &format!("\tmov {}, 1 /* flag = 1 */\n", REG_FLAG);
                *code += &format!("pc_{:x}_lew_done:\n", ctx.pc);
                ctx.c.is_saved = true;
            }
            ZiskOp::And => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0xffffffffffffffff) {
                    *code += "\t/* And: ignoring b since b = f's */\n";
                } else {
                    *code += &format!(
                        "\tand {}, {} /* And: c = c AND b = a AND b */\n",
                        REG_C, ctx.b.string_value
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Or => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += "\t/* Or: ignoring b since b = 0 */\n";
                } else {
                    *code += &format!(
                        "\tor {}, {} /* Or: c = c OR b = a OR b */\n",
                        REG_C, ctx.b.string_value
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Xor => {
                assert!(ctx.store_a_in_c);
                if ctx.b.is_constant && (ctx.b.constant_value == 0) {
                    *code += "\t/* Xor: ignoring b since b = 0 */\n";
                } else {
                    *code += &format!(
                        "\txor {}, {} /* Xor: c = c XOR b = a XOR b */\n",
                        REG_C, ctx.b.string_value
                    );
                }
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mulu => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\tmul {} /* Mulu: rax*reg -> rdx:rax */\n", REG_A);
                *code += &format!("\tmov {}, rax /* Mulu: c = result(rax) */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Muluh => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\tmul {} /* Muluh: rax*reg -> rdx:rax */\n", REG_A);
                *code += &format!("\tmov {}, rdx /* Muluh: c = high result(rdx) */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mulsuh => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\tmov rsi, {} /* Mulsuh: rsi=b */\n", REG_B);
                *code += &format!("\tmov rax, {} /* Mulsuh: rax=a */\n", REG_A);
                *code += &format!("\tmov {}, rax /* Mulsuh: value=a */\n", REG_VALUE);
                *code += &format!("\tsar {}, 63 /* Mulsuh: value=a>>63=a_bit_63 */\n", REG_VALUE);
                *code += "\tmov rdx, 0 /* Mulsuh: rdx=0, rdx:rax=a */\n";
                *code += "\tmul rsi /* Mulsuh: rdx:rax=a*b (unsigned) */\n";
                *code += "\tmov rcx, rax /* Mulsuh: rax=a */\n";
                *code += &format!("\tmov rax, {} /* Mulsuh: rax=a_bit_63 */\n", REG_VALUE);
                *code += "\timul rax, rsi /* Mulsuh: rax=rax*b=a_bit_63*b */\n";
                *code += "\tadd rdx, rax /* Mulsuh: rdx=rdx+a_bit_63*b */\n";
                *code += &format!("\tmov {}, rdx /* Mulsuh: c=high result(rdx) */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mul => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\timul {} /* Mul: rax*reg -> rdx:rax */\n", REG_A);
                *code += &format!("\tmov {}, rax /* Mul: c = result(rax) */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Mulh => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\timul {} /* Mulh: rax*reg -> rdx:rax */\n", REG_A);
                *code += &format!("\tmov {}, rdx /* Mulh: c = high result(rdx) */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MulW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                // RDX:RAX := RAX ∗ r/m64
                *code += &format!("\tmul {} /* MulW: rax*reg -> rdx:rax */\n", REG_A_W);
                *code +=
                    &format!("\tmovsxd {}, {} /* MulW: sign extend to quad */\n", REG_C, REG_B_W);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Divu => {
                assert!(ctx.store_b_in_b);
                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder
                // If b==0 return 0xffffffffffffffff
                *code += &format!("\tcmp {}, 0 /* Divu: if b == 0 return f's */\n", REG_B);
                *code += &format!(
                    "\tjne pc_{:x}_divu_b_is_not_zero /* Divu: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code +=
                    &format!("\tmov {}, 0xffffffffffffffff /* Divu: set result to f's */\n", REG_C);
                *code += &format!("\tje pc_{:x}_divu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_divu_b_is_not_zero:\n", ctx.pc);

                *code += &format!("\tmov {}, {} /* Divu: value = b backup */\n", REG_VALUE, REG_B);
                *code += "\tmov rdx, 0 /* Divu: rdx = 0 */\n";
                *code += &format!("\tmov rax, {} /* Divu: rax = a */\n", ctx.a.string_value);
                *code += &format!(
                    "\tdiv {} /* Divu: rdx:rax / value(b backup) -> rax (rdx remainder)*/\n",
                    REG_VALUE
                );
                *code += &format!("\tmov {}, rax /* Divu: c = quotient(rax) */\n", REG_C);
                *code += &format!("pc_{:x}_divu_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Remu => {
                assert!(ctx.store_b_in_b);
                // Unsigned divide RDX:RAX by r/m64, with result stored in RAX := Quotient, RDX :=
                // Remainder
                // If b==0 return a
                *code += &format!("\tcmp {}, 0 /* Remu: if b == 0 return a */\n", REG_B);
                *code += &format!(
                    "\tjne pc_{:x}_remu_b_is_not_zero /* Remu: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code += &format!(
                    "\tmov {}, {} /* Remu: set result to f's */\n",
                    REG_C, ctx.a.string_value
                );
                *code += &format!("\tje pc_{:x}_remu_done\n", ctx.pc);
                *code += &format!("pc_{:x}_remu_b_is_not_zero:\n", ctx.pc);

                *code += &format!("\tmov {}, {} /* Remu: value = b backup */\n", REG_VALUE, REG_B);
                *code += "\tmov rdx, 0 /* Remu: rdx = 0 */\n";
                *code += &format!("\tmov rax, {} /* Remu: rax = a */\n", ctx.a.string_value);
                *code += &format!(
                    "\tdiv {} /* Remu: rdx:rax / value(b backup) -> rax (rdx remainder)*/\n",
                    REG_VALUE
                );
                *code += &format!("\tmov {}, rdx /* Remu: c = remainder(rdx) */\n", REG_C);
                *code += &format!("pc_{:x}_remu_done:\n", ctx.pc);
                ctx.c.is_saved = true;

                // s += &format!("\tmov {}, 0 /* Remu: c = remainder(rdx) */\n", REG_ADDRESS);
                // s += &format!(
                //     "\tmov {}, [{}] /* Remu: c = remainder(rdx) */\n",
                //     REG_ADDRESS, REG_ADDRESS
                // );
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
                *code += &format!("\tcmp {}, 0 /* Div: if b == 0 return f's */\n", REG_B);
                *code += &format!(
                    "\tjne pc_{:x}_div_check_underflow /* Div: if b is not zero, divide */\n",
                    ctx.pc
                );
                *unusual_code += &format!("pc_{:x}_div_check_underflow:\n", ctx.pc);
                *unusual_code +=
                    &format!("\tmov {}, 0xffffffffffffffff /* Div: set result to f's */\n", REG_C);

                *unusual_code += &format!("\tjmp pc_{:x}_div_done\n", ctx.pc);

                // Check underflow:
                // If a==0x8000000000000000 && b==0xffffffffffffffff then c=a
                *code += &format!(
                    "\tmov {}, 0x8000000000000000 /* Div: value == 0x8000000000000000 */\n",
                    REG_VALUE
                );
                *code += &format!(
                    "\tcmp {}, {} /* Div: if a == value(0x8000000000000000), then check b */\n",
                    REG_A, REG_VALUE
                );
                *code += &format!(
                    "\tjne pc_{:x}_div_divide /* Div: if a is not 0x8000000000000000, then divide */\n",
                    ctx.pc
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff /* Div: value == 0xffffffffffffffff */\n",
                    REG_VALUE
                );
                *code += &format!(
                    "\tcmp {}, {} /* Div: if b == 0xffffffffffffffff, then return a */\n",
                    REG_B, REG_VALUE
                );
                *code += &format!(
                    "\tjne pc_{:x}_div_divide /* Div: if b is not 0xffffffffffffffff, divide */\n",
                    ctx.pc
                );
                *code += &format!("\tmov {}, {} /* Div: set result to a */\n", REG_C, REG_A);

                *code += &format!("\tje pc_{:x}_div_done\n", ctx.pc);

                // Divide
                *code += &format!("pc_{:x}_div_divide:\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* Div: value = b backup */\n", REG_VALUE, REG_B);
                *code += &format!("\tmov rax, {} /* Div: rax = a */\n", REG_A);
                *code += "\tbt rax, 63 /* Div: is a negative? */\n";
                *code += &format!("\tjnc pc_{:x}_a_is_positive\n", ctx.pc);
                *code += "\tmov rdx, 0xffffffffffffffff /* Div: a is negative, rdx = f's */\n";
                *code += &format!("\tjmp pc_{:x}_a_done\n", ctx.pc);
                *code += &format!("pc_{:x}_a_is_positive:\n", ctx.pc);
                *code += "\tmov rdx, 0 /* Div: a is positive, rdx = 0 */\n";
                *code += &format!("pc_{:x}_a_done:\n", ctx.pc);

                *code += &format!(
                    "\tidiv {} /* Div: rdx:rax / value(b backup) -> rax (rdx remainder)*/\n",
                    REG_VALUE
                );
                *code += &format!("\tmov {}, rax /* Div: c = quotient(rax) */\n", REG_C);
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
                *code += &format!("\tcmp {}, 0 /* Rem: if b == 0 return f's */\n", REG_B);
                *code += &format!(
                    "\tjne pc_{:x}_rem_check_underflow /* Rem: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code += &format!("\tmov {}, {} /* Rem: set result to a */\n", REG_C, REG_A);

                *code += &format!("\tje pc_{:x}_rem_done\n", ctx.pc);

                // Check underflow:
                // If a==0x8000000000000000 && b==0xffffffffffffffff then c=a
                *code += &format!(
                    "\tmov {}, 0x8000000000000000 /* Rem: value == 0x8000000000000000 */\n",
                    REG_VALUE
                );
                *code += &format!(
                    "\tcmp {}, {} /* Rem: if a == value(0x8000000000000000), then check b */\n",
                    REG_A, REG_VALUE
                );
                *code += &format!(
                    "\tjne pc_{:x}_rem_divide /* Rem: if a is not 0x8000000000000000, then divide */\n",
                    ctx.pc
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff /* Rem: value == 0xffffffffffffffff */\n",
                    REG_VALUE
                );
                *code += &format!(
                    "\tcmp {}, {} /* Rem: if b == 0xffffffffffffffff, then return a */\n",
                    REG_B, REG_VALUE
                );
                *code += &format!(
                    "\tjne pc_{:x}_rem_divide /* Rem: if b is not 0xffffffffffffffff, divide */\n",
                    ctx.pc
                );
                *code += &format!("\tmov {}, 0 /* Rem: set result to 0 */\n", REG_C);

                *code += &format!("\tje pc_{:x}_rem_done\n", ctx.pc);

                // Divide
                *code += &format!("pc_{:x}_rem_divide:\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* Rem: value = b backup */\n", REG_VALUE, REG_B);
                *code += &format!("\tmov rax, {} /* Rem: rax = a */\n", REG_A);
                *code += "\tbt rax, 63 /* Rem: is a negative? */\n";
                *code += &format!("\tjnc pc_{:x}_a_is_positive\n", ctx.pc);
                *code += "\tmov rdx, 0xffffffffffffffff /* Rem: a is negative, rdx = f's */\n";
                *code += &format!("\tjmp pc_{:x}_a_done\n", ctx.pc);
                *code += &format!("pc_{:x}_a_is_positive:\n", ctx.pc);
                *code += "\tmov rdx, 0 /* Rem: a is positive, rdx = 0 */\n";
                *code += &format!("pc_{:x}_a_done:\n", ctx.pc);

                *code += &format!(
                    "\tidiv {} /* Rem: rdx:rax / value(b backup) -> rax (rdx remainder)*/\n",
                    REG_VALUE
                );
                *code += &format!("\tmov {}, rdx /* Rem: c = remainder(rdx) */\n", REG_C);
                *code += &format!("pc_{:x}_rem_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::DivuW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                *code +=
                    &format!("\tcmp {}, 0 /* DivuW: if b==0 then return all f's */\n", REG_B_W);
                *code += &format!(
                    "\tjne pc_{:x}_divuw_b_is_not_zero /* DivuW: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code += &format!(
                    "\tmov {}, 0xffffffffffffffff /* DivuW: set result to f's */\n",
                    REG_C
                );
                *code += &format!("\tjmp pc_{:x}_divuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_divuw_b_is_not_zero:\n", ctx.pc);

                *code +=
                    &format!("\tmov {}, {} /* DivuW: value = b backup */\n", REG_VALUE_W, REG_B_W);
                *code += "\tmov rdx, 0 /* DivuW: rdx = 0 */\n";
                *code += &format!("\tmov eax, {} /* DivuW: rax = a */\n", REG_A_W);
                *code += &format!(
                    "\tdiv {} /* DivuW: rdx:rax / value(b backup) -> rax (rdx remainder)*/\n",
                    REG_VALUE_W
                );
                *code +=
                    &format!("\tmovsxd {}, eax /* DivuW: sign extend 32 to 64 bits */\n", REG_C);
                *code += &format!("pc_{:x}_divuw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::RemuW => {
                assert!(ctx.store_a_in_a);
                assert!(ctx.store_b_in_b);
                *code += &format!("\tcmp {}, 0 /* RemuW: if b==0 then return a */\n", REG_B_W);
                *code += &format!(
                    "\tjne pc_{:x}_remuw_b_is_not_zero /* RemuW: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code += &format!(
                    "\tmovsxd {}, {} /* RemuW: return a, sign extend 32 to 64 bits */\n",
                    REG_C, REG_A_W
                );
                *code += &format!("\tjmp pc_{:x}_remuw_done\n", ctx.pc);
                *code += &format!("pc_{:x}_remuw_b_is_not_zero:\n", ctx.pc);

                *code +=
                    &format!("\tmov {}, {} /* RemuW: value = b backup */\n", REG_VALUE_W, REG_B_W);
                *code += "\tmov rdx, 0 /* RemuW: rdx = 0 */\n";
                *code += &format!("\tmov eax, {} /* RemuW: rax = a */\n", REG_A_W);
                *code += &format!(
                    "\tdiv {} /* RemuW: rdx:rax / value(b backup) -> rax (rdx remainder)*/\n",
                    REG_VALUE_W
                );
                *code +=
                    &format!("\tmovsxd {}, edx /* RemuW: sign extend 32 to 64 bits */\n", REG_C);
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
                *code += &format!("\tcmp {}, 0 /* DivW: if b == 0 return f's */\n", REG_B_W);
                *code += &format!(
                    "\tjne pc_{:x}_divw_divide /* DivW: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code +=
                    &format!("\tmov {}, 0xffffffffffffffff /* DivW: set result to f's */\n", REG_C);

                *code += &format!("\tje pc_{:x}_divw_done\n", ctx.pc);

                // Divide
                *code +=
                    &format!("\tmov {}, {} /* DivW: value = b backup */\n", REG_VALUE_W, REG_B_W);
                *code += &format!("\tmov eax, {} /* DivW: rax = a */\n", REG_A_W);
                *code += "\tcdq /* DivW: EDX:EAX := sign-extend of EAX */\n";
                *code += &format!(
                    "\tidiv {} /* DivW: edx:eax / value(b backup) -> eax (edx remainder)*/\n",
                    REG_VALUE_W
                );
                *code += &format!("\tmovsx {}, eax /* DivW: c = quotient(rax) */\n", REG_C);
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
                *code += &format!("\tcmp {}, 0 /* RemW: if b == 0 return f's */\n", REG_B_W);
                *code += &format!(
                    "\tjne pc_{:x}_remw_divide /* RemW: if b is not zero, divide */\n",
                    ctx.pc
                );
                *code += &format!("\tmovsx {}, {} /* RemW: set result to a */\n", REG_C, REG_A_W);

                *code += &format!("\tje pc_{:x}_remw_done\n", ctx.pc);

                // Divide
                *code +=
                    &format!("\tmov {}, {} /* RemW: value = b backup */\n", REG_VALUE_W, REG_B_W);
                *code += &format!("\tmov eax, {} /* RemW: rax = a */\n", REG_A_W);
                *code += "\tcdq /* RemW: EDX:EAX := sign-extend of EAX */\n";
                *code += &format!(
                    "\tidiv {} /* RemW: edx:eax / value(b backup) -> eax (edx remainder)*/\n",
                    REG_VALUE_W
                );
                *code += &format!("\tmovsx {}, edx /* RemW: c = remainder(edx) */\n", REG_C);
                *code += &format!("pc_{:x}_remw_done:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Minu => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} /* Minu: compare a and b */\n",
                    REG_C, ctx.b.string_value
                );
                *code += &format!("\tjb pc_{:x}_minu_a_is_below_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* c = b */\n", REG_C, ctx.b.string_value);
                *code += &format!("pc_{:x}_minu_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Min => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} /* Min: compare a and b */\n",
                    REG_C, ctx.b.string_value
                );
                *code += &format!("\tjl pc_{:x}_min_a_is_below_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* c = b */\n", REG_C, ctx.b.string_value);
                *code += &format!("pc_{:x}_min_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MinuW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!("\tcmp {}, {} /* MinuW: compare a and b */\n", REG_C_W, REG_B_W);
                *code += &format!("\tjb pc_{:x}_minuw_a_is_below_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* MinuW: c = b */\n", REG_C, REG_B);
                *code += &format!("pc_{:x}_minuw_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MinW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!("\tcmp {}, {} /* MinW: compare a and b */\n", REG_C_W, REG_B_W);
                *code += &format!("\tjl pc_{:x}_minw_a_is_below_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* MinW: c = b */\n", REG_C, REG_B);
                *code += &format!("pc_{:x}_minw_a_is_below_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Maxu => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} /* Maxu: compare a and b */\n",
                    REG_C, ctx.b.string_value
                );
                *code += &format!("\tja pc_{:x}_maxu_a_is_above_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* Maxu: c = b */\n", REG_C, ctx.b.string_value);
                *code += &format!("pc_{:x}_maxu_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Max => {
                assert!(ctx.store_a_in_c);
                *code += &format!(
                    "\tcmp {}, {} /* Max: compare a and b */\n",
                    REG_C, ctx.b.string_value
                );
                *code += &format!("\tjg pc_{:x}_max_a_is_above_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* Max: c = b */\n", REG_C, ctx.b.string_value);
                *code += &format!("pc_{:x}_max_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MaxuW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!("\tcmp {}, {} /* MaxuW: compare a and b */\n", REG_C_W, REG_B_W);
                *code += &format!("\tja pc_{:x}_maxuw_a_is_above_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* MaxuW: c = b */\n", REG_C, REG_B);
                *code += &format!("pc_{:x}_maxuw_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::MaxW => {
                assert!(ctx.store_a_in_c);
                assert!(ctx.store_b_in_b);
                *code += &format!("\tcmp {}, {} /* MaxW: compare a and b */\n", REG_C_W, REG_B_W);
                *code += &format!("\tjg pc_{:x}_maxw_a_is_above_b\n", ctx.pc);
                *code += &format!("\tmov {}, {} /* MaxW: c = b */\n", REG_C, REG_B);
                *code += &format!("pc_{:x}_maxw_a_is_above_b:\n", ctx.pc);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Keccak => {
                // Use the memory address as the first and unique parameter */
                *code += "\t/* Keccak: rdi = A0 */\n";
                Self::read_riscv_reg(code, 10, "rdi", "rdi");

                // Copy read data into mem_reads_address and advance it
                if ctx.generate_minimal_trace {
                    *code += &format!("\tmov {}, rdi\n", REG_ADDRESS);
                    for k in 0..25 {
                        *code += &format!(
                            "\tmov {}, [{} + {}] /* value = mem[keccak_address[{}]] */\n",
                            REG_VALUE,
                            REG_ADDRESS,
                            k * 8,
                            k
                        );
                        *code += &format!(
                            "\tmov [{} + {}*8 + {}], {} /* mem_reads[{}] = value */\n",
                            REG_MEM_READS_ADDRESS,
                            REG_MEM_READS_SIZE,
                            k * 8,
                            REG_VALUE,
                            k
                        );
                    }

                    // Increment chunk.steps.mem_reads_size in 25 units
                    *code +=
                        &format!("\tadd {}, 25 /* mem_reads_size+=25 */\n", REG_MEM_READS_SIZE);
                }
                // Call the keccak function
                Self::push_internal_registers(ctx, code);
                *code += "\tcall _opcode_keccak\n";
                Self::pop_internal_registers(ctx, code);

                // Set result
                *code += &format!("\tmov {}, 0 /* Keccak: c=0 */\n", REG_C);
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
                *code += "\t/* Arith256 */\n";

                // Use the memory address as the first and unique parameter */
                *code += &format!("\tmov rdi, {} /* rdi = b = address */\n", ctx.b.string_value);

                // Save data into mem_reads
                if ctx.generate_minimal_trace {
                    Self::precompiled_save_mem_reads(ctx, code, 5, 3, 4);
                }

                // Call the secp256k1_add function
                Self::push_internal_registers(ctx, code);
                *code += "\tcall _opcode_arith256\n";
                Self::pop_internal_registers(ctx, code);

                // Set result
                *code += &format!("\tmov {}, 0 /* c=0 */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Arith256Mod => {
                *code += "\t/* Arith256Mod */\n";

                // Use the memory address as the first and unique parameter */
                *code += &format!("\tmov rdi, {} /* rdi = b = address */\n", ctx.b.string_value);

                // Save data into mem_reads
                if ctx.generate_minimal_trace {
                    Self::precompiled_save_mem_reads(ctx, code, 5, 4, 4);
                }

                // Call the secp256k1_add function
                Self::push_internal_registers(ctx, code);
                *code += "\tcall _opcode_arith256_mod\n";
                Self::pop_internal_registers(ctx, code);

                // Set result
                *code += &format!("\tmov {}, 0 /* c=0 */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Secp256k1Add => {
                *code += "\t/* Secp256k1Add */\n";

                // Use the memory address as the first and unique parameter */
                *code += &format!("\tmov rdi, {} /* rdi = b = address */\n", ctx.b.string_value);

                // Save data into mem_reads
                if ctx.generate_minimal_trace {
                    Self::precompiled_save_mem_reads(ctx, code, 2, 2, 8);
                }

                // Call the secp256k1_add function
                Self::push_internal_registers(ctx, code);
                *code += "\tcall _opcode_secp256k1_add\n";
                Self::pop_internal_registers(ctx, code);

                // Set result
                *code += &format!("\tmov {}, 0 /* c=0 */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Secp256k1Dbl => {
                *code += "\t/* Secp256k1Dbl */\n";

                // Use the memory address as the first and unique parameter */
                *code += &format!("\tmov rdi, {} /* rdi = b = address */\n", ctx.b.string_value);

                // Copy read data into mem_reads
                if ctx.generate_minimal_trace {
                    *code += &format!("\tmov {}, rdi\n", REG_ADDRESS);
                    for k in 0..8 {
                        *code += &format!(
                            "\tmov {}, [{} + {}] /* value = mem[address[{}]] */\n",
                            REG_VALUE,
                            REG_ADDRESS,
                            k * 8,
                            k
                        );
                        *code += &format!(
                            "\tmov [{} + {}*8 + {}], {} /* mem_reads[{}] = value */\n",
                            REG_MEM_READS_ADDRESS,
                            REG_MEM_READS_SIZE,
                            k * 8,
                            REG_VALUE,
                            k
                        );
                    }

                    // Increment chunk.steps.mem_reads_size in 8 units
                    *code += &format!("\tadd {}, 8 /* mem_reads_size+=8 */\n", REG_MEM_READS_SIZE);
                }

                // Call the secp256k1_dbl function
                Self::push_internal_registers(ctx, code);
                *code += "\tcall _opcode_secp256k1_dbl\n";
                Self::pop_internal_registers(ctx, code);

                // Set result
                *code += &format!("\tmov {}, 0 /* c=0 */\n", REG_C);
                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::FcallParam => {
                assert!(ctx.store_b_in_c);
                assert!(ctx.a.is_constant);
                assert!(ctx.a.constant_value <= 32);
                *code += "\t/* FcallParam */\n";

                if ctx.a.constant_value == 1 {
                    // Store param in params
                    *code += &format!(
                        "\tmov {}, qword ptr [fcall_ctx + {}*8] /* aux = params size */\n",
                        REG_AUX, FCALL_PARAMS_SIZE
                    );
                    *code += &format!(
                        "\tmov qword ptr [fcall_ctx + {}*8 + {}*8], {} /* ctx.params[size] = b */\n",
                        REG_AUX, FCALL_PARAMS, REG_C
                    );
                    *code += &format!(
                        "\tinc qword ptr [fcall_ctx + {}*8] /* inc ctx.params_size */\n",
                        FCALL_PARAMS_SIZE
                    );
                } else {
                    // Store params in params
                    *code += &format!(
                        "\tmov {}, qword ptr [fcall_ctx + {}*8] /* aux = params size */\n",
                        REG_AUX, FCALL_PARAMS_SIZE
                    );
                    for i in 0..ctx.a.constant_value {
                        *code += &format!(
                            "\tmov {}, qword ptr [{} + {}*8] /* value=params[b] */\n",
                            REG_VALUE, REG_C, i
                        );

                        *code += &format!(
                            "\tmov qword ptr [fcall_ctx + {}*8 + {}*8], {} /* params[aux] = param */\n",
                            REG_AUX, FCALL_PARAMS, REG_VALUE
                        );
                        *code += &format!("\tinc {} /* inc aux */\n", REG_AUX);
                    }
                    *code += &format!(
                        "\tmov qword ptr [fcall_ctx + {}*8], {} /* ctx.params_size = aux */\n",
                        FCALL_PARAMS_SIZE, REG_AUX
                    );
                }

                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::Fcall => {
                *code += "\t/* Fcall */\n";

                assert!(ctx.store_b_in_c);

                // Store a (function id) in context
                assert!(ctx.a.is_constant);
                *code += &format!(
                    "\tmov qword ptr [fcall_ctx + {}*8], {} /* ctx.function id = a */\n",
                    FCALL_FUNCTION_ID, ctx.a.constant_value
                );

                // Set the fcall context address as the first parameter */
                *code += "\tlea rdi, fcall_ctx /* rdi = fcall context */\n";

                // Call the fcall function
                Self::push_internal_registers(ctx, code);
                *code += "\tcall _opcode_fcall\n";
                Self::pop_internal_registers(ctx, code);

                // Get free input address
                *code += &format!(
                    "\tmov {}, {} /* address=free_input */\n",
                    REG_ADDRESS, FREE_INPUT_ADDR
                );

                // Copy ctx.result[0] or 0 into free input
                *code += &format!(
                    "\tmov {}, qword ptr [fcall_ctx + {}*8] /* aux=ctx.result_size */\n",
                    REG_AUX, FCALL_RESULT_SIZE
                );
                *code += &format!("\tcmp {}, 0 /* aux vs 0 */\n", REG_AUX);
                *code += &format!("\tjz pc_{:x}_fcall_result_zero\n", ctx.pc);
                *code += &format!(
                    "\tmov {}, qword ptr [fcall_ctx + {}*8] /* value=ctx.result[0] */\n",
                    REG_VALUE, FCALL_RESULT
                );
                *code +=
                    &format!("\tmov [{}], {} /* free_input=value */\n", REG_ADDRESS, REG_VALUE);
                *code += &format!("\tjmp pc_{:x}_fcall_result_done\n", ctx.pc);
                *code += &format!("pc_{:x}_fcall_result_zero:\n", ctx.pc);
                *code += &format!("\tmov qword ptr [{}], 0 /* free_input=0 */\n", REG_ADDRESS);
                *code += &format!("pc_{:x}_fcall_result_done:\n", ctx.pc);

                // Update fcall counters
                *code += &format!(
                    "\tmov qword ptr [fcall_ctx + {}*8], 0 /* ctx.params_size=0 */\n",
                    FCALL_PARAMS_SIZE
                );
                *code += &format!(
                    "\tmov qword ptr [fcall_ctx + {}*8], 1 /* ctx.result_got=1 */\n",
                    FCALL_RESULT_GOT
                );

                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
            ZiskOp::FcallGet => {
                *code += "\t/* FcallGet */\n";

                assert!(ctx.store_b_in_c);

                // Get value from fcall_ctx.result[got] and store it in free input address
                *code += &format!(
                    "\tmov {}, qword ptr [fcall_ctx + {}*8] /* aux=ctx.result_got */\n",
                    REG_AUX, FCALL_RESULT_GOT
                );
                *code += &format!(
                    "\tmov {}, qword ptr [fcall_ctx + {}*8 + {}*8] /* value=ctx.result[got] */\n",
                    REG_VALUE, REG_AUX, FCALL_RESULT
                );
                *code += &format!(
                    "\tmov {}, {} /* address=free_input */\n",
                    REG_ADDRESS, FREE_INPUT_ADDR
                );
                *code += &format!(
                    "\tmov qword ptr [{}], {} /* free_input=value */\n",
                    REG_ADDRESS, REG_VALUE
                );
                *code += &format!(
                    "\tinc qword ptr [fcall_ctx + {}*8] /* inc ctx.result_got */\n",
                    FCALL_RESULT_GOT
                );

                ctx.c.is_saved = true;
                ctx.flag_is_always_zero = true;
            }
        }
    }

    fn set_pc(ctx: &mut ZiskAsmContext, instruction: &ZiskInst, code: &mut String, id: &str) {
        ctx.jump_to_dynamic_pc = false;
        ctx.jump_to_static_pc = String::new();
        if instruction.set_pc {
            *code += "\t/* set pc */\n";
            if ctx.c.is_constant {
                let new_pc = (ctx.c.constant_value as i64 + instruction.jmp_offset1) as u64;
                *code += &format!(
                    "\tmov {}, 0x{:x} /* value = c(const) + i.jmp_offset1 */\n",
                    REG_VALUE, new_pc
                );
                *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
                ctx.jump_to_static_pc = format!("\tjmp pc_{:x} /* jump to static pc */\n", new_pc);
            } else {
                *code += &format!("\tmov {}, {} /* value = c */\n", REG_VALUE, ctx.c.string_value);
                if instruction.jmp_offset1 != 0 {
                    *code += &format!(
                        "\tadd {}, 0x{:x} /* value += i.jmp_offset1 */\n",
                        REG_VALUE, instruction.jmp_offset1
                    );
                }
                *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
                ctx.jump_to_dynamic_pc = true;
            }
        } else if ctx.flag_is_always_zero {
            if ctx.pc as i64 + instruction.jmp_offset2 != ctx.next_pc as i64 {
                *code += &format!(
                    "\tmov {}, 0x{:x} /* flag=0: pc += i.jmp_offset2 */\n",
                    REG_VALUE,
                    (ctx.pc as i64 + instruction.jmp_offset2) as u64
                );
                // *s += &format!(
                //     "\tadd {}, 0x{:x} /* set_pc 3: pc += i.jmp_offset2 */\n",
                //     MEM_PC, instruction.jmp_offset2
                // );
                *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
                ctx.jump_to_dynamic_pc = true;
            } else if id == "z" {
                *code +=
                    &format!("\tmov {}, 0x{:x} /* flag=0: pc += 4 */\n", REG_VALUE, ctx.next_pc);
                *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
            }
        } else if ctx.flag_is_always_one {
            if ctx.pc as i64 + instruction.jmp_offset1 != ctx.next_pc as i64 {
                *code += &format!(
                    "\tmov {}, 0x{:x} /* flag=1: pc += i.jmp_offset1 */\n",
                    REG_VALUE,
                    (ctx.pc as i64 + instruction.jmp_offset1) as u64
                );
                // *s += &format!(
                //     "\tadd {}, 0x{:x} /* set_pc 4: pc += i.jmp_offset1 */\n",
                //     MEM_PC, instruction.jmp_offset1
                // );
                *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
                ctx.jump_to_dynamic_pc = true;
            } else if id == "z" {
                *code +=
                    &format!("\tmov {}, 0x{:x} /* flag=1: pc += 4 */\n", REG_VALUE, ctx.next_pc);
                *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
            }
        } else {
            *code += "\t/* pc = f(flag) */\n";
            // Calculate the new pc
            *code += &format!("\tcmp {}, 1 /* flag == 1 ? */\n", REG_FLAG);
            *code += &format!("\tjne pc_{:x}_{}_flag_false\n", ctx.pc, id);
            *code += &format!(
                "\tmov {}, 0x{:x} /* pc += i.jmp_offset1 */\n",
                REG_VALUE,
                (ctx.pc as i64 + instruction.jmp_offset1) as u64
            );
            *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
            *code += &format!("\tjmp pc_{:x}_{}_flag_done\n", ctx.pc, id);
            *code += &format!("pc_{:x}_{}_flag_false:\n", ctx.pc, id);
            *code += &format!(
                "\tmov {}, 0x{:x} /* pc += i.jmp_offset2 */\n",
                REG_VALUE,
                (ctx.pc as i64 + instruction.jmp_offset2) as u64
            );
            *code += &format!("\tmov {}, {} /* pc=value */\n", MEM_PC, REG_VALUE);
            *code += &format!("pc_{:x}_{}_flag_done:\n", ctx.pc, id);
            // *s += &format!(
            //     "\tadd {}, 0x{:x} /* pc += i.jmp_offset2 */\n",
            //     MEM_PC, instruction.jmp_offset2
            // );
            ctx.jump_to_dynamic_pc = true;
        }
    }

    fn a_src_mem_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = a */\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            if ctx.store_a_in_c { REG_C } else { REG_A }
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} /* mem_reads_size++ */\n", REG_MEM_READS_SIZE);
    }

    fn a_src_mem_not_aligned(_ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
            REG_ADDRESS
        );

        // Store previous aligned address value in mem_reads
        *code +=
            &format!("\tmov {}, [{}] /* value = mem[prev_address] */\n", REG_VALUE, REG_ADDRESS);
        *code += &format!(
            "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_a */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Store next aligned address value in mem_reads
        *code += &format!(
            "\tmov {}, [{} + 8] /* value = mem[prev_address] */\n",
            REG_VALUE, REG_ADDRESS
        );
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} /* mem_reads[@+size*8+8] = next_a */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!("\tadd {}, 2 /* mem_reads_size+=2*/\n", REG_MEM_READS_SIZE);
    }

    fn b_src_mem_aligned(ctx: &mut ZiskAsmContext, code: &mut String) {
        // Copy read data into mem_reads_address and increment it
        *code += &format!(
            "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = b */\n",
            REG_MEM_READS_ADDRESS,
            REG_MEM_READS_SIZE,
            if ctx.store_b_in_c { REG_C } else { REG_B }
        );

        // Increment chunk.steps.mem_reads_size
        *code += &format!("\tinc {} /* mem_reads_size++ */\n", REG_MEM_READS_SIZE);
    }

    fn b_src_mem_not_aligned(_ctx: &mut ZiskAsmContext, code: &mut String) {
        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
            REG_ADDRESS
        );

        // Store previous aligned address value in mem_reads, and advance address
        *code +=
            &format!("\tmov {}, [{}] /* value = mem[prev_address] */\n", REG_VALUE, REG_ADDRESS);
        *code += &format!(
            "\tmov [{} + {}*8], {} /* mem_address[@+size*8] = prev_b */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Store next aligned address value in mem_reads, and advance address
        *code += &format!(
            "\tmov {}, [{} + 8] /* value = mem[prev_address] */\n",
            REG_VALUE, REG_ADDRESS
        );
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} /* mem_reads[@+size*8+8] = next_b */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!("\tadd {}, 2 /* mem_reads_size+=2*/\n", REG_MEM_READS_SIZE);
    }

    fn c_store_mem_not_aligned(_ctx: &mut ZiskAsmContext, code: &mut String) {
        // Get a copy of the address to preserve it
        *code += &format!("\tmov {}, {} /* aux = address */\n", REG_AUX, REG_ADDRESS);

        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
            REG_AUX
        );

        // Store previous aligned address value in mem_reads, and advance address
        *code += &format!("\tmov {}, [{}] /* value = mem[prev_address] */\n", REG_VALUE, REG_AUX);
        *code += &format!(
            "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_c */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Store next aligned address value in mem_reads, and advance address
        *code +=
            &format!("\tmov {}, [{} + 8] /* value = mem[next_address] */\n", REG_VALUE, REG_AUX);
        *code += &format!(
            "\tmov [{} + {}*8 +  8], {} /* mem_reads[@+size*8+8] = next_c */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!("\tadd {}, 2 /* mem_reads_size+=2*/\n", REG_MEM_READS_SIZE);
    }

    fn c_store_ind_8_not_aligned(_ctx: &mut ZiskAsmContext, code: &mut String) {
        // Get a copy of the address to preserve it
        *code += &format!("\tmov {}, {} /* aux = address */\n", REG_AUX, REG_ADDRESS);

        // Calculate previous aligned address
        *code += &format!(
            "\tand {}, 0xFFFFFFFFFFFFFFF8 /* address = previous aligned address */\n",
            REG_AUX
        );

        // Store previous aligned address value in mem_reads, and advance address
        *code += &format!("\tmov {}, [{}] /* value = mem[prev_address] */\n", REG_VALUE, REG_AUX);
        *code += &format!(
            "\tmov [{} + {}*8], {} /* mem_reads[@+size*8] = prev_c */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Store next aligned address value in mem_reads, and advance it
        *code +=
            &format!("\tmov {}, [{} + 8] /* value = mem[next_address] */\n", REG_VALUE, REG_AUX);
        *code += &format!(
            "\tmov [{} + {}*8 + 8], {} /* mem_reads[@+size*8+8] = next_c */\n",
            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, REG_VALUE
        );

        // Increment chunk.steps.mem_reads_size twice
        *code += &format!("\tadd {}, 2 /* mem_reads_size+=2*/\n", REG_MEM_READS_SIZE);
    }

    fn chunk_start(ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += "\t/* Increment number of chunks (first position in trace) */\n";
        *code +=
            &format!("\tmov {}, {} /* address = trace_addr */\n", REG_ADDRESS, MEM_TRACE_ADDRESS);
        *code += &format!("\tmov {}, [{}] /* value = trace_addr */\n", REG_VALUE, REG_ADDRESS);
        *code += &format!("\tinc {} /* inc value */\n", REG_VALUE);
        *code += &format!(
            "\tmov [{}], {} /* trace_addr = value (trace_addr++) */\n",
            REG_ADDRESS, REG_VALUE
        );

        if ctx.generate_minimal_trace {
            *code += "\t/* Write chunk start data */\n";

            // Write chunk.start.pc
            *code += &format!(
                "\tmov {}, {} /* address = chunk_address */\n",
                REG_ADDRESS, MEM_CHUNK_ADDRESS
            );

            *code += &format!("\tmov {}, {} /* value = pc */\n", REG_VALUE, MEM_PC);
            *code +=
                &format!("\tmov [{}], {} /* chunk.start.pc = value */\n", REG_ADDRESS, REG_VALUE);

            // Write chunk.start.sp
            *code += &format!("\tmov {}, {} /* value = sp */\n", REG_VALUE, MEM_SP);
            *code += &format!("\tadd {}, 8 /* address += 8 */\n", REG_ADDRESS);
            *code += &format!(
                "\tmov [{}], {} /* chunk.start.sp = value = sp */\n",
                REG_ADDRESS, REG_VALUE
            );

            // Write chunk.start.c
            *code += &format!("\tadd {}, 8 /* address += 8 */\n", REG_ADDRESS);
            *code += &format!("\tmov [{}], {} /* chunk.start.c = c */\n", REG_ADDRESS, REG_C);

            // Write chunk.start.step
            *code += &format!("\tadd {}, 8 /* address += 8 */\n", REG_ADDRESS);
            *code += &format!("\tmov {}, {} /* value = step */\n", REG_VALUE, MEM_STEP);
            *code += &format!(
                "\tmov [{}], {} /* chunk.start.step = value = step */\n",
                REG_ADDRESS, REG_VALUE
            );
            *code += &format!(
                "\tmov [{}], {} /* chunk_start_step = value = step */\n",
                MEM_CHUNK_START_STEP, REG_VALUE
            );

            // Write chunk.start.reg
            for i in 1..34 {
                Self::read_riscv_reg(code, i, REG_VALUE, "value");
                *code += &format!(
                    "\tmov [{} + {}], {} /* chunk.start.reg[{}] = value */\n",
                    REG_ADDRESS,
                    i * 8,
                    REG_VALUE,
                    i
                );
            }
            *code += &format!("\tadd {}, 33*8 /* address += 33*8 */\n", REG_ADDRESS);
        }

        *code += "\t/* Reset step_down to chunk_size */\n";
        *code += &format!("\tmov {}, chunk_size /* value = chunk_size */\n", REG_VALUE);
        *code += &format!("\tmov {}, {} /* step_down = chunk_size */\n", REG_STEP_DOWN, REG_VALUE);

        if ctx.generate_minimal_trace || ctx.generate_main_trace {
            *code += "\t/* Write mem reads size */\n";
            *code += &format!("\tmov {}, {} /* aux = chunk_size */\n", REG_AUX, MEM_CHUNK_ADDRESS);
            if ctx.generate_minimal_trace {
                *code += &format!("\tadd {}, 40*8 /* aux += 40*8 */\n", REG_AUX);
            }
            *code += &format!("\tadd {}, 8 /* aux += 8 */\n", REG_AUX);
            *code += &format!(
                "\tmov {}, {} /* mem_reads_address = aux */\n",
                REG_MEM_READS_ADDRESS, REG_AUX
            );
            *code += "\t/* Reset mem_reads size */\n";
            *code += &format!("\tmov {}, 0 /* mem_reads_size = 0 */\n", REG_MEM_READS_SIZE);
        }
    }

    fn chunk_end(ctx: &mut ZiskAsmContext, code: &mut String, id: &str) {
        *code += "\t/* Update step from step_down */\n";
        *code += &format!("\tmov {}, {} /* value = step */\n", REG_VALUE, MEM_STEP);
        *code += &format!("\tadd {}, chunk_size /* value += chunk_size */\n", REG_VALUE);
        *code += &format!("\tsub {}, {} /* value -= step_down */\n", REG_VALUE, REG_STEP_DOWN);
        *code += &format!("\tmov {}, {} /* step = value */\n", MEM_STEP, REG_VALUE);

        if ctx.generate_minimal_trace {
            *code += "\t/* Write chunk last data */\n";

            // Search position of chunk.last
            *code += &format!(
                "\tmov {}, {} /* address = chunk_address */\n",
                REG_ADDRESS, MEM_CHUNK_ADDRESS
            );
            *code += &format!("\tadd {}, 37*8 /* address = chunk_address + 37*8 */\n", REG_ADDRESS);

            // Write chunk.last.c
            *code += &format!("\tmov [{}], {} /* chunk.last.c = c */\n", REG_ADDRESS, REG_C);

            *code += "\t/* Write chunk end data */\n";
            *code += &format!("\tadd {}, 8 /* address += 8 */\n", REG_ADDRESS);
            *code += &format!("\tmov {}, {} /* value = end */\n", REG_VALUE, MEM_END);
            *code +=
                &format!("\tmov [{}], {} /* chunk.end = value = end */\n", REG_ADDRESS, REG_VALUE);

            *code += &format!("\tadd {}, 8 /* address += 8 */\n", REG_ADDRESS); // steps
            *code += &format!("\tmov {}, {} /* value = step */\n", REG_VALUE, MEM_STEP);
            *code +=
                &format!("\tsub {}, {} /* value = step_inc */\n", REG_VALUE, MEM_CHUNK_START_STEP);
            *code += &format!(
                "\tmov [{}], {} /* chunk.steps.step = value = step_inc */\n",
                REG_ADDRESS, REG_VALUE
            );

            // Write mem_reads_size
            *code += &format!("\tadd {}, 8 /* address += 8 = mem_reads_size */\n", REG_ADDRESS); // mem_reads_size

            *code += &format!(
                "\tmov [{}], {} /* mem_reads_size = size */\n",
                REG_ADDRESS, REG_MEM_READS_SIZE
            );

            // Get value = mem_reads_size*8, i.e. memory size till next chunk
            *code += &format!(
                "\tmov {}, {} /* value = mem_reads_size */\n",
                REG_VALUE, REG_MEM_READS_SIZE
            );
            *code += &format!("\tsal {}, 3 /* value <<= 3 */\n", REG_VALUE);

            // Update chunk address
            *code += &format!("\tadd {}, 8 /* address += 8 = new_chunk_address */\n", REG_ADDRESS); // new chunk
            *code += &format!(
                "\tadd {}, {} /* address += value = mem_reads_size*8 */\n",
                REG_ADDRESS, REG_VALUE
            ); // new chunk
            *code += &format!(
                "\tmov {}, {} /* chunk_address = new_chunk_address */\n",
                MEM_CHUNK_ADDRESS, REG_ADDRESS
            );
        }

        if ctx.generate_main_trace {
            // Write size
            *code += &format!(
                "\tmov {}, {} /* address = chunk_address */\n",
                REG_ADDRESS, MEM_CHUNK_ADDRESS
            );
            *code += &format!(
                "\tmov [{}], {} /* mem_reads_size = size */\n",
                REG_ADDRESS, REG_MEM_READS_SIZE
            );
            *code += &format!("\tadd {}, 8 /* address += 8 = new_chunk_address */\n", REG_ADDRESS); // new chunk

            // Increase chunk address
            *code += &format!(
                "\tmov {}, {} /* value = mem_reads_size */\n",
                REG_VALUE, REG_MEM_READS_SIZE
            );
            *code += &format!("\tsal {}, 3 /* value <<= 3 */\n", REG_VALUE);
            *code += &format!(
                "\tadd {}, {} /* address += value = mem_reads_size*8 */\n",
                REG_ADDRESS, REG_VALUE
            ); // new chunk
            *code += &format!(
                "\tmov {}, {} /* chunk_address = new_chunk_address */\n",
                MEM_CHUNK_ADDRESS, REG_ADDRESS
            );
        }

        if ctx.generate_minimal_trace || ctx.generate_main_trace {
            *code += "\t/* Realloc trace if threshold is passed */\n";
            *code += &format!(
                "\tmov {}, qword ptr [trace_address_threshold] /* value = trace_address_threshold */\n",
                REG_VALUE
            );
            *code += &format!(
                "\tcmp {}, {} /* chunk_address ? trace_address_threshold */\n",
                REG_ADDRESS, REG_VALUE
            );
            *code += &format!("\tjb chunk_{}_address_below_threshold\n", id);
            Self::push_internal_registers(ctx, code);
            *code += "\tcall _realloc_trace\n";
            if ctx.call_chunk_done {
                *code += "\tcall _chunk_done\n";
            }
            Self::pop_internal_registers(ctx, code);
            *code += &format!("chunk_{}_address_below_threshold:\n", id);
        } else if ctx.call_chunk_done {
            // Call the chunk_done function
            Self::push_internal_registers(ctx, code);
            *code += "\tcall _chunk_done\n";
            Self::pop_internal_registers(ctx, code);
        }
    }

    fn push_external_registers(_ctx: &mut ZiskAsmContext, code: &mut String) {
        //*s += "\tpush rsp\n";
        *code += "\tpush rbx\n";
        *code += "\tpush rbp\n";
        *code += "\tpush r12\n";
        *code += "\tpush r13\n";
        *code += "\tpush r14\n";
        *code += "\tpush r15\n";
    }

    fn pop_external_registers(_ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += "\tpop r15\n";
        *code += "\tpop r14\n";
        *code += "\tpop r13\n";
        *code += "\tpop r12\n";
        *code += "\tpop rbp\n";
        *code += "\tpop rbx\n";
        //*s += "\tpop rsp\n";
    }

    fn push_internal_registers(_ctx: &mut ZiskAsmContext, code: &mut String) {
        *code += "\tpush rax\n";
        *code += "\tpush rcx\n";
        *code += "\tpush rdx\n";
        // *s += "\tpush rdi\n";
        // *s += "\tpush rsi\n";
        // *s += "\tpush rsp\n";
        // *s += "\tpush r8\n";
        *code += "\tpush r9\n";
        *code += "\tpush r10\n";
        //*s += "\tpush r11\n";
        for r in 0u64..16u64 {
            Self::push_xmm_reg(code, r);
        }
    }

    fn pop_internal_registers(_ctx: &mut ZiskAsmContext, code: &mut String) {
        for r in (0u64..16u64).rev() {
            Self::pop_xmm_reg(code, r);
        }
        //*s += "\tpop r11\n";
        *code += "\tpop r10\n";
        *code += "\tpop r9\n";
        // *s += "\tpop r8\n";
        // *s += "\tpop rsp\n";
        // *s += "\tpop rsi\n";
        // *s += "\tpop rdi\n";
        *code += "\tpop rdx\n";
        *code += "\tpop rcx\n";
        *code += "\tpop rax\n";
    }

    fn precompiled_save_mem_reads(
        _ctx: &mut ZiskAsmContext,
        code: &mut String,
        indirections_count: u64,
        load_count: u64,
        load_size: u64,
    ) {
        // This index will be incremented as we insert data into mem_reads
        let mut mem_reads_index: u64 = 0;

        // We get a copy of the precompiled data address
        *code += &format!("\tmov {}, rdi /* address = rdi */\n", REG_ADDRESS);

        // We make 2 rounds, a first one to store the indirection addresses, and a second one to
        // store the load data, up to load_count
        for j in 0..2 {
            // For every indirection
            for i in 0..indirections_count {
                // Store next aligned address value in mem_reads, and advance it
                *code += &format!(
                    "\tmov {}, [{} + {}*8] /* value = mem[address+{}] */\n",
                    REG_VALUE, REG_ADDRESS, i, i
                );

                // During the first iteration, store the indirection read value in mem_reads
                if j == 0 {
                    *code += &format!(
                        "\tmov [{} + {}*8 + {}*8], {} /* mem_reads[@+size*8+ind*8] = ind */\n",
                        REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, mem_reads_index, REG_VALUE
                    );
                    mem_reads_index += 1;
                }

                // During the second iteration, store the first load_count iterations
                // load_size elements in mem_reads
                if j == 1 {
                    // Only store the first load_count indirections
                    if i >= load_count {
                        break;
                    }

                    // For each chunk of the indirection, store it in mem_reads
                    for l in 0..load_size {
                        *code += &format!(
                            "\tmov {}, [{} + {}*8] /* aux = mem[ind+{}] */\n",
                            REG_AUX, REG_VALUE, l, l
                        );
                        *code += &format!(
                            "\tmov [{} + {}*8 + {}*8], {} /* mem_reads[@+size*8+ind*8] = ind */\n",
                            REG_MEM_READS_ADDRESS, REG_MEM_READS_SIZE, mem_reads_index, REG_AUX
                        );
                        mem_reads_index += 1;
                    }
                }
            }
        }

        // Increment chunk.steps.mem_reads_size
        *code += &format!(
            "\tadd {}, {} /* mem_reads_size+={}*/\n",
            REG_MEM_READS_SIZE, mem_reads_index, mem_reads_index
        );
    }

    fn trace_reg_access(ctx: &mut ZiskAsmContext, code: &mut String, reg: u64, slot: u64) {
        // REG_VALUE is reg_step = STEP << 4 + 1 + slot
        *code += &format!("\tmov {}, {} /* value = step */\n", REG_VALUE, MEM_STEP);
        *code += &format!("\tsal {}, 3 /* value <<= 2 */\n", REG_VALUE);
        *code += &format!("\tadd {}, {} /* value += {} */\n", REG_VALUE, slot + 1, slot + 1);

        // REG_ADDRESS is reg_steps[slot], i.e. prev_reg_steps
        *code += &format!(
            "\tmov {}, qword ptr [reg_steps_{}] /* address=reg_steps[slot] */\n",
            REG_ADDRESS, slot
        );

        // reg_prev_steps[slot] = pref_reg_steps
        *code += &format!(
            "\tmov qword ptr [reg_prev_steps_{}], {} /* reg_prev_steps[slot]=address */\n",
            slot, REG_ADDRESS
        );

        // Check if is first_reference==0
        *code += &format!(
            "\tmov {}, qword ptr [first_step_uses_{}] /* aux=first_step_uses[reg] */\n",
            REG_AUX, reg
        );
        *code += &format!("\tjz pc_{:x}_{}_first_reference\n", ctx.pc, slot);
        // Not first reference
        *code += &format!("pc_{:x}_{}_not_first_reference:\n", ctx.pc, slot);
        *code += &format!(
            "\tmov qword ptr [reg_step_ranges_{}], {} /* reg_step_ranges[slot]=reg_step */\n",
            slot, REG_VALUE
        );
        *code += &format!(
            "\tsub qword ptr [reg_step_ranges_{}], {} /* reg_step_ranges[slot]-=prev_reg_step */\n",
            slot, REG_VALUE
        );
        *code += &format!("\tjmp pc_{:x}_{}_first_reference_done\n", ctx.pc, slot);
        // First reference
        *code += &format!("pc_{:x}_{}_first_reference:\n", ctx.pc, slot);
        *code += &format!(
            "\tmov qword ptr [first_step_uses_{}], {} /* first_step_uses[reg]= */\n",
            reg, REG_VALUE
        );
        *code += &format!("pc_{:x}_{}_first_reference_done:\n", ctx.pc, slot);

        // Store reg_steps
        *code += &format!(
            "\tmov qword ptr [reg_steps_{}], {} /* reg_steps[slot]=reg_step */\n",
            slot, REG_VALUE
        );
    }

    fn clear_reg_step_ranges(_ctx: &mut ZiskAsmContext, code: &mut String, slot: u64) {
        *code += &format!(
            "\tmov qword ptr [reg_step_ranges_{}], 0 /* reg_step_ranges[slot]=0 */\n",
            slot
        );
    }

    fn reg_to_xmm_index(reg: u64) -> u64 {
        let xmm_index: u64;
        match reg {
            1 => xmm_index = 0,
            2 => xmm_index = 1,
            5 => xmm_index = 2,
            6 => xmm_index = 3,
            7 => xmm_index = 4,
            8 => xmm_index = 5,
            9 => xmm_index = 6,
            10 => xmm_index = 7,
            11 => xmm_index = 8,
            12 => xmm_index = 9,
            13 => xmm_index = 10,
            14 => xmm_index = 11,
            15 => xmm_index = 12,
            16 => xmm_index = 13,
            17 => xmm_index = 14,
            18 => xmm_index = 15,
            _ => {
                panic!("ZiskRom2Asm::reg_to_xmm_index() found invalid source slot={}", reg);
            }
        }
        xmm_index
    }

    fn read_riscv_reg(
        //_ctx: &mut ZiskAsmContext,
        code: &mut String,
        src_slot: u64,
        dest_reg: &str,
        dest_desc: &str,
    ) {
        if XMM_MAPPED_REGS.contains(&src_slot) {
            let xmm_index = Self::reg_to_xmm_index(src_slot);
            *code += &format!(
                "\tmovq {}, xmm{} /* {} = reg[{}] */\n",
                dest_reg, xmm_index, dest_desc, src_slot
            );
        } else {
            *code += &format!(
                "\tmov {}, qword ptr [reg_{}] /* {} = reg[{}] */\n",
                dest_reg, src_slot, dest_desc, src_slot
            );
        }
    }

    fn write_riscv_reg(
        //_ctx: &mut ZiskAsmContext,
        code: &mut String,
        dest_slot: u64,
        src_reg: &str,
        src_desc: &str,
    ) {
        if XMM_MAPPED_REGS.contains(&dest_slot) {
            let xmm_index = Self::reg_to_xmm_index(dest_slot);
            *code += &format!(
                "\tmovq xmm{}, {} /* reg[{}]={} */\n",
                xmm_index, src_reg, dest_slot, src_desc
            );
        } else {
            *code += &format!(
                "\tmov qword ptr [reg_{}], {} /* reg[{}] = {} */\n",
                dest_slot, src_reg, dest_slot, src_desc
            );
        }
    }

    fn write_riscv_reg_constant(
        //_ctx: &mut ZiskAsmContext,
        code: &mut String,
        dest_slot: u64,
        value: u64,
        value_desc: &str,
    ) {
        if XMM_MAPPED_REGS.contains(&dest_slot) {
            let xmm_index = Self::reg_to_xmm_index(dest_slot);
            *code += &format!("\tmov {}, {} /* aux={} */\n", REG_AUX, value, value_desc);

            *code +=
                &format!("\tmovq xmm{}, {} /* reg[{}]=aux */\n", xmm_index, REG_AUX, dest_slot);
        } else {
            *code += &format!("\tmov {}, {} /* aux={} */\n", REG_AUX, value, value_desc);
            *code += &format!(
                "\tmov qword ptr [reg_{}], {} /* reg[{}] = aux */\n",
                dest_slot, REG_AUX, dest_slot
            );
        }
    }

    fn push_xmm_reg(code: &mut String, xmm_index: u64) {
        *code += "\tsub rsp, 8\n";
        *code += &format!("\tmovq [rsp], xmm{} /* push xmm{} */\n", xmm_index, xmm_index);
    }

    fn pop_xmm_reg(code: &mut String, xmm_index: u64) {
        *code += &format!("\tmovq xmm{}, [rsp] /* pop xmm{} */\n", xmm_index, xmm_index);
        *code += "\tadd rsp, 8\n";
    }

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
}
