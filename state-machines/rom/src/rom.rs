use std::{path::PathBuf, sync::Arc};

use itertools::Itertools;
use log::info;
use p3_field::PrimeField;
use sm_common::{ComponentProvider, Instance, InstanceExpanderCtx, Metrics, Plan, Planner};

use crate::{RomCounter, RomInstance, RomPlanner};
use zisk_core::{Riscv2zisk, ZiskRom, SRC_IMM};
use zisk_pil::{MainTrace, RomRomTrace, RomRomTraceRow, RomTrace, RomTraceRow};

pub struct RomSM {
    zisk_rom: Arc<ZiskRom>,
}

impl RomSM {
    const MY_NAME: &'static str = "RomSM   ";

    pub fn new(zisk_rom: Arc<ZiskRom>) -> Arc<Self> {
        Arc::new(Self { zisk_rom })
    }

    pub fn prove_instance<F: PrimeField>(
        rom: &ZiskRom,
        plan: &Plan,
        rom_trace: &mut RomTrace<F>,
        mut rom_custom_trace: &mut RomRomTrace<F>,
    ) {
        let metadata = plan.meta.as_ref().unwrap().downcast_ref::<RomCounter>().unwrap();

        let pc_histogram = &metadata.rom.inst_count;
        let main_trace_len = MainTrace::<F>::NUM_ROWS as u64;

        info!(
            "{}: ··· Creating Rom instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            pc_histogram.len(),
            main_trace_len,
            pc_histogram.len() as f64 / main_trace_len as f64 * 100.0
        );

        // For every instruction in the rom, fill its corresponding ROM trace
        //for (i, inst_builder) in rom.insts.clone().into_iter().enumerate() {
        for (i, key) in rom.insts.keys().sorted().enumerate() {
            // Get the Zisk instruction
            let inst = &rom.insts[key].i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            if pc_histogram.is_empty() {
                multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
            } else {
                let counter = pc_histogram.get(&inst.paddr);
                if counter.is_some() {
                    multiplicity = *counter.unwrap();
                    if inst.paddr == metadata.end_pc {
                        multiplicity +=
                            main_trace_len - 1 - (metadata.steps % (main_trace_len - 1));
                    }
                } else {
                    continue; // We skip those pc's that are not used in this execution
                }
            }
            rom_trace[i].multiplicity = F::from_canonical_u64(multiplicity);
        }

        // Padd with zeroes
        for i in rom.insts.len()..rom_trace.num_rows() {
            rom_trace[i] = RomTraceRow::default();
        }

        Self::compute_trace_rom(rom, &mut rom_custom_trace);
    }

    pub fn compute_trace_rom<F: PrimeField>(rom: &ZiskRom, rom_custom_trace: &mut RomRomTrace<F>) {
        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, key) in rom.insts.keys().sorted().enumerate() {
            // Get the Zisk instruction
            let inst = &rom.insts[key].i;

            // Convert the i64 offsets to F
            let jmp_offset1 = if inst.jmp_offset1 >= 0 {
                F::from_canonical_u64(inst.jmp_offset1 as u64)
            } else {
                F::neg(F::from_canonical_u64((-inst.jmp_offset1) as u64))
            };
            let jmp_offset2 = if inst.jmp_offset2 >= 0 {
                F::from_canonical_u64(inst.jmp_offset2 as u64)
            } else {
                F::neg(F::from_canonical_u64((-inst.jmp_offset2) as u64))
            };
            let store_offset = if inst.store_offset >= 0 {
                F::from_canonical_u64(inst.store_offset as u64)
            } else {
                F::neg(F::from_canonical_u64((-inst.store_offset) as u64))
            };
            let a_offset_imm0 = if inst.a_offset_imm0 as i64 >= 0 {
                F::from_canonical_u64(inst.a_offset_imm0)
            } else {
                F::neg(F::from_canonical_u64((-(inst.a_offset_imm0 as i64)) as u64))
            };
            let b_offset_imm0 = if inst.b_offset_imm0 as i64 >= 0 {
                F::from_canonical_u64(inst.b_offset_imm0)
            } else {
                F::neg(F::from_canonical_u64((-(inst.b_offset_imm0 as i64)) as u64))
            };

            // Fill the rom trace row fields
            rom_custom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_custom_trace[i].a_offset_imm0 = a_offset_imm0;
            rom_custom_trace[i].a_imm1 =
                F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_custom_trace[i].b_offset_imm0 = b_offset_imm0;
            rom_custom_trace[i].b_imm1 =
                F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            rom_custom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
            rom_custom_trace[i].op = F::from_canonical_u8(inst.op);
            rom_custom_trace[i].store_offset = store_offset;
            rom_custom_trace[i].jmp_offset1 = jmp_offset1;
            rom_custom_trace[i].jmp_offset2 = jmp_offset2;
            rom_custom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
        }

        // Padd with zeroes
        for i in rom.insts.len()..rom_custom_trace.num_rows() {
            rom_custom_trace[i] = RomRomTraceRow::default();
        }
    }

    pub fn compute_custom_trace_rom<F: PrimeField>(
        rom_path: PathBuf,
        rom_custom_trace: &mut RomRomTrace<F>,
    ) {
        // Get the ELF file path as a string
        let elf_filename: String = rom_path.to_str().unwrap().into();
        println!("Proving ROM for ELF file={}", elf_filename);

        // Load and parse the ELF file, and transpile it into a ZisK ROM using Riscv2zisk

        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename, String::new(), String::new(), String::new());

        // Convert program to rom
        let rom = riscv2zisk.run().expect("RomSM::prover() failed converting elf to rom");

        Self::compute_trace_rom(&rom, rom_custom_trace);
    }
}

impl<F: PrimeField> ComponentProvider<F> for RomSM {
    fn get_counter(&self) -> Box<dyn Metrics> {
        Box::new(RomCounter::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(RomPlanner {})
    }

    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn Instance<F>> {
        Box::new(RomInstance::new(self.zisk_rom.clone(), iectx))
    }
}
