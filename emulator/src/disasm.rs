//! Disassembly writer module
//! Generates objdump-like output with execution counts

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Result, Write};

use crate::ElfSymbolReader;
use zisk_core::{ZiskInst, ZiskRom};

pub struct DisasmWriter {
    file: BufWriter<File>,
    pc_histogram: HashMap<u64, u64>,
    symbols: Option<ElfSymbolReader>,
}

impl DisasmWriter {
    pub fn new(path: &str) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self { file: BufWriter::new(file), pc_histogram: HashMap::new(), symbols: None })
    }

    pub fn set_pc_histogram(&mut self, histogram: HashMap<u64, u64>) {
        self.pc_histogram = histogram;
    }

    pub fn set_symbols(&mut self, symbols: ElfSymbolReader) {
        self.symbols = Some(symbols);
    }

    pub fn write_header(&mut self, title: &str) -> Result<()> {
        writeln!(&mut self.file)?;
        writeln!(&mut self.file, "Disassembly with execution counts:")?;
        writeln!(&mut self.file, "{}", title)?;
        writeln!(&mut self.file)?;
        Ok(())
    }

    pub fn write_disassembly(&mut self, rom: &ZiskRom) -> Result<()> {
        let mut local_labels: HashMap<u64, String> = HashMap::new();

        // First pass: identify all jump targets to generate labels
        for (idx, pc) in rom.sorted_pc_list.iter().enumerate() {
            let inst = rom.get_instruction(*pc);
            // Get next PC for detecting fall-through
            let next_pc = if idx + 1 < rom.sorted_pc_list.len() {
                Some(rom.sorted_pc_list[idx + 1])
            } else {
                None
            };

            // Check for jumps to generate labels
            if inst.set_pc {
                let target1 = (*pc as i64 + inst.jmp_offset1) as u64;
                // Only create label if it's not the next instruction (not fall-through)
                if Some(target1) != next_pc
                    && !local_labels.contains_key(&target1)
                    && rom.sorted_pc_list.binary_search(&target1).is_ok()
                {
                    if let Some(ref symbols) = self.symbols {
                        if let Some(sym) = symbols.get_symbol_at_address(target1) {
                            local_labels.insert(target1, sym.name.clone());
                        } else {
                            let label = format!(".L{}", local_labels.len());
                            local_labels.insert(target1, label);
                        }
                    } else {
                        let label = format!(".L{}", local_labels.len());
                        local_labels.insert(target1, label);
                    }
                }

                if inst.jmp_offset2 != 0 {
                    let target2 = (*pc as i64 + inst.jmp_offset2) as u64;
                    // Only create label if it's not the next instruction (not fall-through)
                    if Some(target2) != next_pc
                        && !local_labels.contains_key(&target2)
                        && rom.sorted_pc_list.binary_search(&target2).is_ok()
                    {
                        if let Some(ref symbols) = self.symbols {
                            if let Some(sym) = symbols.get_symbol_at_address(target2) {
                                local_labels.insert(target2, sym.name.clone());
                            } else {
                                let label = format!(".L{}", local_labels.len());
                                local_labels.insert(target2, label);
                            }
                        } else {
                            let label = format!(".L{}", local_labels.len());
                            local_labels.insert(target2, label);
                        }
                    }
                }
            }
        }

        // Second pass: generate disassembly
        for (idx, pc) in rom.sorted_pc_list.iter().enumerate() {
            // Check if this PC is a function entry point
            if let Some(ref symbols) = self.symbols {
                if let Some(sym) = symbols.get_symbol_at_address(*pc) {
                    // Write function header
                    writeln!(&mut self.file)?;
                    writeln!(&mut self.file, "{:016x} <{}>:", pc, sym.name)?;
                }
            }

            // Check if this PC has a label (jump target)
            if let Some(label) = local_labels.get(pc) {
                if let Some(ref symbols) = self.symbols {
                    if symbols.get_symbol_at_address(*pc).is_none() {
                        writeln!(&mut self.file)?;
                        writeln!(&mut self.file, "{:016x} <{}>:", pc, label)?;
                    }
                } else {
                    writeln!(&mut self.file)?;
                    writeln!(&mut self.file, "{:016x} <{}>:", pc, label)?;
                }
            }

            let inst = rom.get_instruction(*pc);
            let exec_count = self.pc_histogram.get(pc).unwrap_or(&0);

            // Get next PC for detecting fall-through jumps
            let next_pc = if idx + 1 < rom.sorted_pc_list.len() {
                Some(rom.sorted_pc_list[idx + 1])
            } else {
                None
            };

            // Determine if this is the first Zisk instruction for a RISC-V instruction
            let is_first_zisk_for_riscv = if idx > 0 {
                let prev_pc = rom.sorted_pc_list[idx - 1];
                let prev_inst = rom.get_instruction(prev_pc);
                prev_inst.riscv_inst != inst.riscv_inst
            } else {
                true
            };

            // Format: PC | EXEC_COUNT | RISCV_INST | ZISK_INST
            if is_first_zisk_for_riscv {
                if let Some(ref riscv_inst) = inst.riscv_inst {
                    writeln!(
                        &mut self.file,
                        "  {:08x}:  {:12}  {:30}  {}",
                        pc,
                        exec_count,
                        riscv_inst,
                        inst_to_asm(inst, &local_labels, next_pc)
                    )?;
                } else {
                    // Zisk instruction without RISC-V source (initialization)
                    writeln!(
                        &mut self.file,
                        "  {:08x}:  {:12}  {:30}  {}",
                        pc,
                        exec_count,
                        "",
                        inst_to_asm(inst, &local_labels, next_pc)
                    )?;
                }
            } else {
                // Additional Zisk instruction from same RISC-V instruction
                writeln!(
                    &mut self.file,
                    "  {:08x}:  {:12}  {:30}  {}",
                    pc,
                    exec_count,
                    "",
                    inst_to_asm(inst, &local_labels, next_pc)
                )?;
            }
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.file.flush()
    }
}

/// Convert a ZiskInst to assembly-like string representation
/// Format: operation dest, a, b (RISC-V like syntax)
fn inst_to_asm(inst: &ZiskInst, labels: &HashMap<u64, String>, next_pc: Option<u64>) -> String {
    use zisk_core::{
        SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE,
        STORE_REG,
    };

    let mut asm = String::new();

    // Operation name
    asm.push_str(inst.op_str);

    let mut operands = Vec::new();

    // 1. Destination (c register - where result is stored)
    if inst.store != STORE_NONE {
        let dest = match inst.store {
            STORE_REG => {
                format!("x{}", inst.store_offset)
            }
            STORE_MEM => {
                if inst.store_use_sp {
                    if inst.store_offset >= 0 {
                        format!("[sp+{}]", inst.store_offset)
                    } else {
                        format!("[sp{}]", inst.store_offset)
                    }
                } else {
                    format!("[0x{:x}]", inst.store_offset)
                }
            }
            STORE_IND => {
                if inst.store_offset >= 0 {
                    format!("[a+{}]", inst.store_offset)
                } else {
                    format!("[a{}]", inst.store_offset)
                }
            }
            _ => "?".to_string(),
        };
        operands.push(dest);
    }

    // 2. Source A
    let src_a = match inst.a_src {
        SRC_C => "c".to_string(),
        SRC_REG => {
            format!("x{}", inst.a_offset_imm0)
        }
        SRC_MEM => {
            if inst.a_use_sp_imm1 != 0 {
                let offset = inst.a_offset_imm0 as i64;
                if offset >= 0 {
                    format!("[sp+{}]", offset)
                } else {
                    format!("[sp{}]", offset)
                }
            } else {
                format!("[0x{:x}]", inst.a_offset_imm0)
            }
        }
        SRC_IMM => {
            let imm = inst.a_offset_imm0 as i64 | ((inst.a_use_sp_imm1 as i64) << 32);
            if (0..=9).contains(&imm) {
                format!("{}", imm)
            } else {
                format!("0x{:x}", imm as u64)
            }
        }
        SRC_STEP => "step".to_string(),
        _ => "?".to_string(),
    };
    operands.push(src_a);

    // 3. Source B (if used)
    if inst.b_src != 0 {
        let src_b = match inst.b_src {
            SRC_C => "c".to_string(),
            SRC_REG => {
                format!("x{}", inst.b_offset_imm0)
            }
            SRC_MEM => {
                if inst.b_use_sp_imm1 != 0 {
                    let offset = inst.b_offset_imm0 as i64;
                    if offset >= 0 {
                        format!("[sp+{}]", offset)
                    } else {
                        format!("[sp{}]", offset)
                    }
                } else {
                    format!("[0x{:x}]", inst.b_offset_imm0)
                }
            }
            SRC_IMM => {
                let imm = inst.b_offset_imm0 as i64 | ((inst.b_use_sp_imm1 as i64) << 32);
                if (0..=9).contains(&imm) {
                    format!("{}", imm)
                } else {
                    format!("0x{:x}", imm as u64)
                }
            }
            SRC_IND => {
                let offset = inst.b_offset_imm0 as i64;
                let width = match inst.ind_width {
                    1 => "b",
                    2 => "h",
                    4 => "w",
                    8 => "d",
                    _ => "",
                };
                if inst.b_use_sp_imm1 != 0 {
                    if offset >= 0 {
                        format!("[a+sp+{}]{}", offset, width)
                    } else {
                        format!("[a+sp{}]{}", offset, width)
                    }
                } else if offset >= 0 {
                    format!("[a+{}]{}", offset, width)
                } else {
                    format!("[a{}]{}", offset, width)
                }
            }
            _ => "?".to_string(),
        };
        operands.push(src_b);
    }

    // Format operands
    if !operands.is_empty() {
        asm.push(' ');
        asm.push_str(&operands.join(", "));
    }

    // 4. Jump targets (Zisk peculiarity: two jump offsets)
    // jmp_offset1: used if flag is active
    // jmp_offset2: used as default jump
    // Don't show jumps to next instruction (fall-through) to be more RISC-V like
    if inst.set_pc {
        let mut jump_targets = Vec::new();

        let target1_is_next =
            inst.jmp_offset1 != 0 && Some((inst.paddr as i64 + inst.jmp_offset1) as u64) == next_pc;
        let target2_is_next =
            inst.jmp_offset2 != 0 && Some((inst.paddr as i64 + inst.jmp_offset2) as u64) == next_pc;

        if inst.jmp_offset1 != 0 && !target1_is_next {
            let target = (inst.paddr as i64 + inst.jmp_offset1) as u64;
            if let Some(label) = labels.get(&target) {
                jump_targets.push((true, label.clone()));
            } else {
                jump_targets.push((true, format!("0x{:x}", target)));
            }
        }

        if inst.jmp_offset2 != 0 && !target2_is_next {
            let target = (inst.paddr as i64 + inst.jmp_offset2) as u64;
            if let Some(label) = labels.get(&target) {
                jump_targets.push((false, label.clone()));
            } else {
                jump_targets.push((false, format!("0x{:x}", target)));
            }
        }

        if !jump_targets.is_empty() {
            if operands.is_empty() {
                asm.push(' ');
            } else {
                asm.push_str(", ");
            }

            // If only one target, don't use prefix (it's implicit)
            // If both targets, use true:/false: prefix to distinguish
            if jump_targets.len() == 1 {
                asm.push_str(&jump_targets[0].1);
            } else {
                let formatted: Vec<String> = jump_targets
                    .iter()
                    .map(|(is_true, label)| {
                        if *is_true {
                            format!("true:{}", label)
                        } else {
                            format!("false:{}", label)
                        }
                    })
                    .collect();
                asm.push_str(&formatted.join(", "));
            }
        }
    }

    // 5. Additional flags/modifiers as comments
    let mut comments = Vec::new();

    if inst.m32 {
        comments.push("32-bit");
    }
    if inst.end {
        comments.push("END");
    }
    if inst.is_external_op {
        comments.push("external");
    }
    if inst.store_pc {
        comments.push("store_pc");
    }
    if inst.is_precompiled {
        comments.push("with_step");
    }

    if !comments.is_empty() {
        asm.push_str(" ; ");
        asm.push_str(&comments.join(", "));
    }

    asm
}
