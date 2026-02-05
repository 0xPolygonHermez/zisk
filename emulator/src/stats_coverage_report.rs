//! Emulator coverage information

use crate::StatsReport;
use std::{collections::HashMap, str::FromStr};
use zisk_core::{zisk_ops::ZiskOp, ZiskRom};

/// Keeps statistics of the emulator operations
#[derive(Debug, Clone, Default)]
pub struct StatsCoverageReport {}

impl StatsCoverageReport {
    pub fn report_opcodes_coverage(
        pc_histogram: &HashMap<u64, u64>,
        report: &mut StatsReport,
        no_frops: &[u64],
        frops: &[u64],
        title: &str,
        rom: &ZiskRom,
    ) {
        let mut ops: [u64; 256] = [0; 256];
        for i in 0..256_usize {
            ops[i] = no_frops[i] + frops[i];
        }
        let mut ops_total_counter: u64 = 0;
        let mut ops_used_counter: u64 = 0;
        let mut no_frops_used_counter: u64 = 0;
        let mut frops_used_counter: u64 = 0;

        let mut ops_used_text: String = String::new();
        let mut ops_unused_text: String = String::new();
        let mut no_frops_used_text: String = String::new();
        let mut no_frops_unused_text: String = String::new();
        let mut frops_used_text: String = String::new();

        for i in 0..256_usize {
            if let Ok(inst) = ZiskOp::try_from_code(i as u8) {
                ops_total_counter += 1;
                if ops[i] > 0 {
                    ops_used_counter += 1;
                    ops_used_text.push_str(&format!("{}, ", inst.name()));
                } else {
                    ops_unused_text.push_str(&format!("{}, ", inst.name()));
                }
                if no_frops[i] > 0 {
                    no_frops_used_counter += 1;
                    no_frops_used_text.push_str(&format!("{}, ", inst.name()));
                } else {
                    no_frops_unused_text.push_str(&format!("{}, ", inst.name()));
                }
                if frops[i] > 0 {
                    frops_used_counter += 1;
                    frops_used_text.push_str(&format!("{}, ", inst.name()));
                }
            }
        }
        let r = format!(
            "\n{}:\nAVAILABLE: {}\nUSED: {}\nUSED NO FROPS: {} ({:.2}%) [{}]\nUNUSED NO FROPS: {} ({:.2}%) [{}]\nUSED FROPS: {} ({:.2}%) [{}]\n",
            title,
            ops_total_counter,
            ops_used_counter,
            no_frops_used_counter,
            (no_frops_used_counter as f64 * 100.0) / (ops_total_counter as f64),
            no_frops_used_text,
            ops_total_counter - no_frops_used_counter,
            ((ops_total_counter - no_frops_used_counter) as f64 * 100.0) / (ops_total_counter as f64),
            no_frops_unused_text,
            frops_used_counter,
            (frops_used_counter as f64 * 100.0) / (ops_total_counter as f64),
            frops_used_text
        );

        report.add(&r);

        // Create a RISC-V instruction string histogram
        let mut riscv_histogram: HashMap<String, u64> = HashMap::new();

        // Fill it with the supported RISC-V instructions
        for riscv_inst in RISCV_IMACFD_ZICSR_INSTRUCTIONS.iter() {
            riscv_histogram.insert(String::from_str(riscv_inst).unwrap(), 0);
        }

        // For each executed Zisk instruction, get its RISC-V instruction and update the histogram
        let mut unsupported_riscv_instructions: Vec<String> = Vec::new();
        for pc in pc_histogram.keys() {
            // Get the RISC-V instruction at this pc from rom
            let zisk_inst = rom.get_instruction(*pc);

            // If the Zisk instruction did not come from a RISC-V instruction, skip it
            if zisk_inst.riscv_inst.is_none() {
                continue;
            }

            // Get the RISC-V instruction string
            let riscv_inst = zisk_inst.riscv_inst.as_ref().unwrap();

            // Get the count of times this Zisk instruction was executed
            let count = pc_histogram.get(pc).unwrap();

            // Update the RISC-V histogram
            let total_count = riscv_histogram.get_mut(riscv_inst);
            if total_count.is_none() {
                unsupported_riscv_instructions
                    .push(zisk_inst.riscv_inst.as_ref().unwrap().to_string());
                continue;
            }

            // Update the count
            let total_count = total_count.unwrap();
            *total_count += count;
        }

        // Calculate RISC-V instruction coverage
        let used_riscv_instructions_counter: u64 =
            riscv_histogram.values().filter(|&&count| count > 0).count() as u64;
        let riscv_coverage = used_riscv_instructions_counter as f64
            / RISCV_IMACFD_ZICSR_INSTRUCTIONS.len() as f64
            * 100.0;
        let r = format!(
            "\nRISC-V INSTRUCTION COVERAGE: {:.2}% ({} out of {})\n",
            riscv_coverage,
            used_riscv_instructions_counter,
            RISCV_IMACFD_ZICSR_INSTRUCTIONS.len()
        );
        report.add(&r);

        // Report unsupported RISC-V instructions, in alphabetical order
        if !unsupported_riscv_instructions.is_empty() {
            let mut r = String::from("UNSUPPORTED RISC-V INSTRUCTIONS EXECUTED: ");
            for inst in unsupported_riscv_instructions.iter() {
                r += &format!("{} ", inst);
            }
            r += "\n";
            report.add(&r);
        }
        let mut r = String::from("EXECUTED RISC-V INSTRUCTIONS: ");
        let mut executed_riscv_instructions: Vec<String> = Vec::new();
        for (inst, count) in riscv_histogram.iter() {
            if *count > 0 {
                executed_riscv_instructions.push(inst.to_string());
            }
        }
        executed_riscv_instructions.sort();
        for inst in executed_riscv_instructions.iter() {
            r += &format!("{} ", inst);
        }
        r += "\n";
        report.add(&r);

        // Report non-executed RISC-V instructions, in alphabetical order
        let mut non_executed_riscv_instructions: Vec<String> = Vec::new();
        let mut r = String::from("NON_EXECUTED RISC-V INSTRUCTIONS: ");
        for (inst, count) in riscv_histogram.iter() {
            if *count == 0 {
                non_executed_riscv_instructions.push(inst.to_string());
            }
        }
        non_executed_riscv_instructions.sort();
        for inst in non_executed_riscv_instructions.iter() {
            r += &format!("{} ", inst);
        }
        r += "\n";
        report.add(&r);
    }
}

pub const RISCV_IMACFD_ZICSR_INSTRUCTIONS: [&str; 193] = [
    // ============================================
    // I Extension - Base Integer Instruction Set
    // ============================================
    // RV32I/RV64I Base Instructions
    "lui",
    "auipc",
    "jal",
    "jalr",
    "beq",
    "bne",
    "blt",
    "bge",
    "bltu",
    "bgeu",
    "lb",
    "lh",
    "lw",
    "lbu",
    "lhu",
    "lwu",
    "sb",
    "sh",
    "sw",
    "addi",
    "slti",
    "sltiu",
    "xori",
    "ori",
    "andi",
    "slli",
    "srli",
    "srai",
    "add",
    "sub",
    "sll",
    "slt",
    "sltu",
    "xor",
    "srl",
    "sra",
    "or",
    "and",
    "fence",
    "fence.i",
    "ecall",
    "ebreak",
    // RV64I-specific instructions
    "lw",
    "ld",
    "sd", // Note: lw/sw exist in RV32I, but have different behavior in RV64I
    "addiw",
    "slliw",
    "srliw",
    "sraiw",
    "addw",
    "subw",
    "sllw",
    "srlw",
    "sraw",
    // ============================================
    // M Extension - Integer Multiplication/Division
    // ============================================
    "mul",
    "mulh",
    "mulhsu",
    "mulhu",
    "div",
    "divu",
    "rem",
    "remu",
    // RV64M-specific
    "mulw",
    "divw",
    "divuw",
    "remw",
    "remuw",
    // ============================================
    // A Extension - Atomic Instructions
    // ============================================
    // Atomic Memory Operations
    "lr.w",
    "sc.w",
    "amoswap.w",
    "amoadd.w",
    "amoxor.w",
    "amoand.w",
    "amoor.w",
    "amomin.w",
    "amomax.w",
    "amominu.w",
    "amomaxu.w",
    // RV64A-specific
    "lr.d",
    "sc.d",
    "amoswap.d",
    "amoadd.d",
    "amoxor.d",
    "amoand.d",
    "amoor.d",
    "amomin.d",
    "amomax.d",
    "amominu.d",
    "amomaxu.d",
    // ============================================
    // C Extension - Compressed Instructions
    // ============================================
    // RV32C/RV64C Compressed Instructions
    "c.addi4spn",
    "c.fld",
    "c.lw",
    "c.flw",
    "c.ld",
    "c.fsd",
    "c.sw",
    "c.fsw",
    "c.sd",
    "c.addi",
    "c.jal",
    "c.li",
    "c.addi16sp",
    "c.lui",
    "c.srli",
    "c.srai",
    "c.andi",
    "c.sub",
    "c.xor",
    "c.or",
    "c.and",
    "c.j",
    "c.beqz",
    "c.bnez",
    "c.slli",
    "c.fldsp",
    "c.lwsp",
    "c.flwsp",
    "c.ldsp",
    "c.jr",
    "c.mv",
    "c.ebreak",
    "c.jalr",
    "c.add",
    "c.fsdsp",
    "c.swsp",
    "c.fswsp",
    "c.sdsp",
    // ============================================
    // F Extension - Single-Precision Floating-Point
    // ============================================
    "flw",
    "fsw",
    "fmadd.s",
    "fmsub.s",
    "fnmsub.s",
    "fnmadd.s",
    "fadd.s",
    "fsub.s",
    "fmul.s",
    "fdiv.s",
    "fsqrt.s",
    "fsgnj.s",
    "fsgnjn.s",
    "fsgnjx.s",
    "fmin.s",
    "fmax.s",
    "fcvt.w.s",
    "fcvt.wu.s",
    "fcvt.s.w",
    "fcvt.s.wu",
    "fmv.x.w",
    "fmv.w.x",
    "feq.s",
    "flt.s",
    "fle.s",
    "fclass.s",
    // RV64F-specific
    "fcvt.l.s",
    "fcvt.lu.s",
    "fcvt.s.l",
    "fcvt.s.lu",
    // ============================================
    // D Extension - Double-Precision Floating-Point
    // ============================================
    "fld",
    "fsd",
    "fmadd.d",
    "fmsub.d",
    "fnmsub.d",
    "fnmadd.d",
    "fadd.d",
    "fsub.d",
    "fmul.d",
    "fdiv.d",
    "fsqrt.d",
    "fsgnj.d",
    "fsgnjn.d",
    "fsgnjx.d",
    "fmin.d",
    "fmax.d",
    "fcvt.s.d",
    "fcvt.d.s",
    "fcvt.w.d",
    "fcvt.wu.d",
    "fcvt.d.w",
    "fcvt.d.wu",
    "feq.d",
    "flt.d",
    "fle.d",
    "fclass.d",
    "fcvt.l.d",
    "fcvt.lu.d",
    "fcvt.d.l",
    "fcvt.d.lu",
    // ============================================
    // Zicsr Extension - Control and Status Registers
    // ============================================
    // CSR Read/Write
    "csrrw",
    "csrrs",
    "csrrc",
    "csrrwi",
    "csrrsi",
    "csrrci",
    // CSR Read & Set/Clear (pseudo-instructions, but important for compilers)
    // "csrr",
    // "csrw",
    // "csrs",
    // "csrc",
    // "csrwi",
    // "csrsi",
    // "csrci",
    // Privileged Instructions (often grouped with Zicsr)
    // "mret",
    // "sret",
    // "wfi",
    // Timer and Counter Instructions
    // "rdcycle",
    // "rdtime",
    // "rdinstret",
    // "rdcycleh",
    // "rdtimeh",
    // "rdinstreth", // RV32 only
    // Machine Mode CSR Access
    // "mrs",
    // "msr", // Common aliases
    // Hypervisor CSRs (when H extension is present)
    // "hfv",
    // "hsv",
];
