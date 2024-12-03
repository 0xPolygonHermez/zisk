use std::sync::Arc;

use p3_field::{Field, PrimeField};
use proofman::{WitnessComponent, WitnessManager};

use sm_common::{
    ComponentProvider, Instance, InstanceExpanderCtx, Metrics, Plan, Planner, WitnessBuffer,
};

use zisk_core::{ZiskRom, SRC_IMM};
use zisk_pil::{RomTrace, ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{RomCounter, RomInstance, RomPlanner};

pub struct RomSM<F> {
    wcm: Arc<WitnessManager<F>>,

    zisk_rom: Arc<ZiskRom>,
}

impl<F: PrimeField> RomSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, zisk_rom: Arc<ZiskRom>) -> Arc<Self> {
        let rom_sm = Self { wcm: wcm.clone(), zisk_rom };
        let rom_sm = Arc::new(rom_sm);

        let rom_air_ids = ROM_AIR_IDS;
        wcm.register_component(rom_sm.clone(), Some(ZISK_AIRGROUP_ID), Some(rom_air_ids));

        rom_sm
    }

    pub fn prove_instance(
        rom: &ZiskRom,
        plan: &Plan,
        buffer: &mut WitnessBuffer<F>,
        num_rows: usize,
    ) {
        let metadata = plan.meta.as_ref().unwrap().downcast_ref::<RomCounter>().unwrap();
        let pc_histogram = &metadata.rom.inst_count;

        // Create an empty ROM trace
        let mut rom_trace =
            RomTrace::<F>::map_buffer(&mut buffer.buffer, num_rows, buffer.offset as usize)
                .expect("RomSM::compute_trace() failed mapping buffer to ROMSRow");

        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, inst_builder) in rom.insts.clone().into_iter().enumerate() {
            // Get the Zisk instruction
            let inst = inst_builder.1.i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            if pc_histogram.is_empty() {
                multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
            } else {
                let counter = pc_histogram.get(&inst.paddr);
                if counter.is_some() {
                    multiplicity = *counter.unwrap();
                    // if inst.paddr == pc_histogram.end_pc {
                    //     multiplicity +=
                    //         main_trace_len - 1 - (pc_histogram.steps % (main_trace_len - 1));
                    // }
                } else {
                    continue; // We skip those pc's that are not used in this execution
                }
            }

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
            rom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_trace[i].a_offset_imm0 = a_offset_imm0;
            rom_trace[i].a_imm1 =
                F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_trace[i].b_offset_imm0 = b_offset_imm0;
            rom_trace[i].b_imm1 =
                F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            //rom_trace[i].b_src_ind =
            //    F::from_canonical_u64(if inst.b_src == SRC_IND { 1 } else { 0 });
            rom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
            rom_trace[i].op = F::from_canonical_u8(inst.op);
            rom_trace[i].store_offset = store_offset;
            rom_trace[i].jmp_offset1 = jmp_offset1;
            rom_trace[i].jmp_offset2 = jmp_offset2;
            rom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
            rom_trace[i].multiplicity = F::from_canonical_u64(multiplicity);
            /*println!(
                "ROM SM [{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}], {}",
                inst.paddr,
                inst.a_offset_imm0,
                if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 },
                inst.b_offset_imm0,
                if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 },
                if inst.b_src == SRC_IND { 1 } else { 0 },
                inst.ind_width,
                inst.op,
                inst.store_offset as u64,
                inst.jmp_offset1 as u64,
                inst.jmp_offset2 as u64,
                inst.get_flags(),
                multiplicity,
            );*/
        }

        // Padd with zeroes
        // for i in number_of_instructions..trace_size {
        //     rom_trace[i] = RomRow::default();
        // }

        // Ok((prover_buffer, offsets[0], ROM_AIR_IDS[0]))
    }
}

impl<F: PrimeField> ComponentProvider<F> for RomSM<F> {
    fn get_counter(&self) -> Box<dyn Metrics> {
        Box::new(RomCounter::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(RomPlanner {})
    }

    fn get_instance(self: Arc<Self>, iectx: InstanceExpanderCtx<F>) -> Box<dyn Instance> {
        Box::new(RomInstance::new(self.wcm.clone(), self.zisk_rom.clone(), iectx))
    }
}

impl<F: Field> WitnessComponent<F> for RomSM<F> {}
