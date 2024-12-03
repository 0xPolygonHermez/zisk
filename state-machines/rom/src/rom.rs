use std::{any::Any, path::PathBuf, sync::Arc};

use p3_field::{Field, PrimeField};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, BufferAllocator, ProofCtx, SetupCtx};
use proofman_util::create_buffer_fast;

use sm_common::{
    CheckPoint, ChunkId, ComponentProvider, InstanceExpanderCtx, InstanceXXXX, Plan, Planner,
    StateMachine, Survey, SurveyCounter, SurveyStats, Surveyor, WitnessBuffer,
};
use zisk_common::InstObserver;
use zisk_core::{InstContext, Riscv2zisk, ZiskInst, ZiskPcHistogram, ZiskRom, SRC_IMM};
use zisk_pil::{Pilout, RomRow, RomTrace, MAIN_AIR_IDS, ROM_AIR_IDS, ZISK_AIRGROUP_ID};
use ziskemu::EmuTrace;

use std::error::Error;

use crate::{RomPlanner, RomSurveyor};

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

    pub fn prove(
        &self,
        rom: &ZiskRom,
        pc_histogram: &ZiskPcHistogram,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let buffer_allocator = self.wcm.get_ectx().buffer_allocator.clone();
        let sctx = self.wcm.get_sctx();

        if pc_histogram.end_pc == 0 {
            panic!("RomSM::prove() detected pc_histogram.end_pc == 0"); // TODO: return an error
        }

        let main_trace_len =
            self.wcm.get_pctx().pilout.get_air(ZISK_AIRGROUP_ID, MAIN_AIR_IDS[0]).num_rows() as u64;

        let (prover_buffer, _, air_id) =
            Self::compute_trace_rom(rom, buffer_allocator, &sctx, pc_histogram, main_trace_len)?;

        let air_instance =
            AirInstance::new(sctx.clone(), ZISK_AIRGROUP_ID, air_id, None, prover_buffer);
        let (is_mine, instance_gid) =
            self.wcm.get_ectx().dctx.write().unwrap().add_instance(ZISK_AIRGROUP_ID, air_id, 1);
        if is_mine {
            self.wcm
                .get_pctx()
                .air_instance_repo
                .add_air_instance(air_instance, Some(instance_gid));
        }

        Ok(())
    }
    pub fn compute_trace(
        rom_path: PathBuf,
        buffer_allocator: Arc<dyn BufferAllocator<F>>,
        sctx: &SetupCtx<F>,
    ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
        // Get the ELF file path as a string
        let elf_filename: String = rom_path.to_str().unwrap().into();
        println!("Proving ROM for ELF file={}", elf_filename);

        // Load and parse the ELF file, and transpile it into a ZisK ROM using Riscv2zisk

        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename, String::new(), String::new(), String::new());

        // Convert program to rom
        let rom_result = riscv2zisk.run();
        if rom_result.is_err() {
            //return Err(ZiskEmulatorErr::Unknown(zisk_rom.err().unwrap().to_string()));
            panic!("RomSM::prover() failed converting elf to rom");
        }
        let rom = rom_result.unwrap();

        let empty_pc_histogram = ZiskPcHistogram::default();

        Self::compute_trace_rom(&rom, buffer_allocator, sctx, &empty_pc_histogram, 0)
    }

    pub fn compute_trace_rom(
        rom: &ZiskRom,
        buffer_allocator: Arc<dyn BufferAllocator<F>>,
        sctx: &SetupCtx<F>,
        pc_histogram: &ZiskPcHistogram,
        main_trace_len: u64,
    ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
        let pilout = Pilout::pilout();
        let sizes = (
            pilout.get_air(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]).num_rows(),
            // pilout.get_air(ZISK_AIRGROUP_ID, ROM_M_AIR_IDS[0]).num_rows(),
            // pilout.get_air(ZISK_AIRGROUP_ID, ROM_L_AIR_IDS[0]).num_rows(),
        );

        let number_of_instructions = rom.insts.len();

        Self::create_rom_s(
            sizes.0,
            rom,
            number_of_instructions,
            buffer_allocator,
            sctx,
            pc_histogram,
            main_trace_len,
        )
        // match number_of_instructions {
        //     n if n <= sizes.0 => Self::create_rom_s(
        //         sizes.0,
        //         rom,
        //         n,
        //         buffer_allocator,
        //         sctx,
        //         pc_histogram,
        //         main_trace_len,
        //     ),
        //     n if n <= sizes.1 => Self::create_rom_m(
        //         sizes.1,
        //         rom,
        //         n,
        //         buffer_allocator,
        //         sctx,
        //         pc_histogram,
        //         main_trace_len,
        //     ),
        //     n if n < sizes.2 => Self::create_rom_l(
        //         sizes.2,
        //         rom,
        //         n,
        //         buffer_allocator,
        //         sctx,
        //         pc_histogram,
        //         main_trace_len,
        //     ),
        //     _ => panic!("RomSM::compute_trace() found rom too big size={}",
        // number_of_instructions), }
    }

    fn create_rom_s(
        rom_s_size: usize,
        rom: &zisk_core::ZiskRom,
        number_of_instructions: usize,
        buffer_allocator: Arc<dyn BufferAllocator<F>>,
        sctx: &SetupCtx<F>,
        pc_histogram: &ZiskPcHistogram,
        main_trace_len: u64,
    ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
        // Set trace size
        let trace_size = rom_s_size;

        // Allocate a prover buffer
        let (buffer_size, offsets) = buffer_allocator
            .get_buffer_info(sctx, ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])
            .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));
        let mut prover_buffer = create_buffer_fast(buffer_size as usize);

        // Create an empty ROM trace
        let mut rom_trace =
            RomTrace::<F>::map_buffer(&mut prover_buffer, trace_size, offsets[0] as usize)
                .expect("RomSM::compute_trace() failed mapping buffer to ROMSRow");

        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, inst_builder) in rom.insts.clone().into_iter().enumerate() {
            // Get the Zisk instruction
            let inst = inst_builder.1.i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            if pc_histogram.map.is_empty() {
                multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
            } else {
                let counter = pc_histogram.map.get(&inst.paddr);
                if counter.is_some() {
                    multiplicity = *counter.unwrap();
                    if inst.paddr == pc_histogram.end_pc {
                        multiplicity +=
                            main_trace_len - 1 - (pc_histogram.steps % (main_trace_len - 1));
                    }
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
        for i in number_of_instructions..trace_size {
            rom_trace[i] = RomRow::default();
        }

        Ok((prover_buffer, offsets[0], ROM_AIR_IDS[0]))
    }

    // fn create_rom_m(
    //     rom_m_size: usize,
    //     rom: &zisk_core::ZiskRom,
    //     number_of_instructions: usize,
    //     buffer_allocator: Arc<dyn BufferAllocator>,
    //     sctx: &SetupCtx,
    //     pc_histogram: ZiskPcHistogram,
    //     main_trace_len: u64,
    // ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
    //     // Set trace size
    //     let trace_size = rom_m_size;

    //     // Allocate a prover buffer
    //     let (buffer_size, offsets) = buffer_allocator
    //         .get_buffer_info(sctx, ZISK_AIRGROUP_ID, ROM_M_AIR_IDS[0])
    //         .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));
    //     let mut prover_buffer = create_buffer_fast(buffer_size as usize);

    //     // Create an empty ROM trace
    //     let mut rom_trace =
    //         RomM1Trace::<F>::map_buffer(&mut prover_buffer, trace_size, offsets[0] as usize)
    //             .expect("RomSM::compute_trace() failed mapping buffer to ROMMRow");

    //     // For every instruction in the rom, fill its corresponding ROM trace
    //     for (i, inst_builder) in rom.insts.clone().into_iter().enumerate() {
    //         // Get the Zisk instruction
    //         let inst = inst_builder.1.i;

    //         // Calculate the multiplicity, i.e. the number of times this pc is used in this
    //         // execution
    //         let mut multiplicity: u64;
    //         if pc_histogram.map.is_empty() {
    //             multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
    //         } else {
    //             let counter = pc_histogram.map.get(&inst.paddr);
    //             if counter.is_some() {
    //                 multiplicity = *counter.unwrap();
    //                 if inst.paddr == pc_histogram.end_pc {
    //                     multiplicity += main_trace_len - 1 - (pc_histogram.steps %
    // main_trace_len);                 }
    //             } else {
    //                 continue; // We skip those pc's that are not used in this execution
    //             }
    //         }

    //         // Convert the i64 offsets to F
    //         let jmp_offset1 = if inst.jmp_offset1 >= 0 {
    //             F::from_canonical_u64(inst.jmp_offset1 as u64)
    //         } else {
    //             F::neg(F::from_canonical_u64((-inst.jmp_offset1) as u64))
    //         };
    //         let jmp_offset2 = if inst.jmp_offset2 >= 0 {
    //             F::from_canonical_u64(inst.jmp_offset2 as u64)
    //         } else {
    //             F::neg(F::from_canonical_u64((-inst.jmp_offset2) as u64))
    //         };
    //         let store_offset = if inst.store_offset >= 0 {
    //             F::from_canonical_u64(inst.store_offset as u64)
    //         } else {
    //             F::neg(F::from_canonical_u64((-inst.store_offset) as u64))
    //         };
    //         let a_offset_imm0 = if inst.a_offset_imm0 as i64 >= 0 {
    //             F::from_canonical_u64(inst.a_offset_imm0)
    //         } else {
    //             F::neg(F::from_canonical_u64((-(inst.a_offset_imm0 as i64)) as u64))
    //         };
    //         let b_offset_imm0 = if inst.b_offset_imm0 as i64 >= 0 {
    //             F::from_canonical_u64(inst.b_offset_imm0)
    //         } else {
    //             F::neg(F::from_canonical_u64((-(inst.b_offset_imm0 as i64)) as u64))
    //         };

    //         // Fill the rom trace row fields
    //         rom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc,
    // paddr, line         rom_trace[i].a_offset_imm0 = a_offset_imm0;
    //         rom_trace[i].a_imm1 =
    //             F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0
    // });         rom_trace[i].b_offset_imm0 = b_offset_imm0;
    //         rom_trace[i].b_imm1 =
    //             F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0
    // });         //rom_trace[i].b_src_ind =
    //         //    F::from_canonical_u64(if inst.b_src == SRC_IND { 1 } else { 0 });
    //         rom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
    //         rom_trace[i].op = F::from_canonical_u8(inst.op);
    //         rom_trace[i].store_offset = store_offset;
    //         rom_trace[i].jmp_offset1 = jmp_offset1;
    //         rom_trace[i].jmp_offset2 = jmp_offset2;
    //         rom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
    //         rom_trace[i].multiplicity = F::from_canonical_u64(multiplicity);
    //     }

    //     // Padd with zeroes
    //     for i in number_of_instructions..trace_size {
    //         rom_trace[i] = RomM1Row::default();
    //     }

    //     Ok((prover_buffer, offsets[0], ROM_M_AIR_IDS[0]))
    // }

    // fn create_rom_l(
    //     rom_l_size: usize,
    //     rom: &zisk_core::ZiskRom,
    //     number_of_instructions: usize,
    //     buffer_allocator: Arc<dyn BufferAllocator>,
    //     sctx: &SetupCtx,
    //     pc_histogram: ZiskPcHistogram,
    //     main_trace_len: u64,
    // ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
    //     // Set trace size
    //     let trace_size = rom_l_size;

    //     // Allocate a prover buffer
    //     let (buffer_size, offsets) = buffer_allocator
    //         .get_buffer_info(sctx, ZISK_AIRGROUP_ID, ROM_L_AIR_IDS[0])
    //         .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));
    //     let mut prover_buffer = create_buffer_fast(buffer_size as usize);

    //     // Create an empty ROM trace
    //     let mut rom_trace =
    //         RomL2Trace::<F>::map_buffer(&mut prover_buffer, trace_size, offsets[0] as usize)
    //             .expect("RomSM::compute_trace() failed mapping buffer to ROMLRow");

    //     // For every instruction in the rom, fill its corresponding ROM trace
    //     for (i, inst_builder) in rom.insts.clone().into_iter().enumerate() {
    //         // Get the Zisk instruction
    //         let inst = inst_builder.1.i;

    //         // Calculate the multiplicity, i.e. the number of times this pc is used in this
    //         // execution
    //         let mut multiplicity: u64;
    //         if pc_histogram.map.is_empty() {
    //             multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
    //         } else {
    //             let counter = pc_histogram.map.get(&inst.paddr);
    //             if counter.is_some() {
    //                 multiplicity = *counter.unwrap();
    //                 if inst.paddr == pc_histogram.end_pc {
    //                     multiplicity += main_trace_len - 1 - (pc_histogram.steps %
    // main_trace_len);                 }
    //             } else {
    //                 continue; // We skip those pc's that are not used in this execution
    //             }
    //         }

    //         // Convert the i64 offsets to F
    //         let jmp_offset1 = if inst.jmp_offset1 >= 0 {
    //             F::from_canonical_u64(inst.jmp_offset1 as u64)
    //         } else {
    //             F::neg(F::from_canonical_u64((-inst.jmp_offset1) as u64))
    //         };
    //         let jmp_offset2 = if inst.jmp_offset2 >= 0 {
    //             F::from_canonical_u64(inst.jmp_offset2 as u64)
    //         } else {
    //             F::neg(F::from_canonical_u64((-inst.jmp_offset2) as u64))
    //         };
    //         let store_offset = if inst.store_offset >= 0 {
    //             F::from_canonical_u64(inst.store_offset as u64)
    //         } else {
    //             F::neg(F::from_canonical_u64((-inst.store_offset) as u64))
    //         };
    //         let a_offset_imm0 = if inst.a_offset_imm0 as i64 >= 0 {
    //             F::from_canonical_u64(inst.a_offset_imm0)
    //         } else {
    //             F::neg(F::from_canonical_u64((-(inst.a_offset_imm0 as i64)) as u64))
    //         };
    //         let b_offset_imm0 = if inst.b_offset_imm0 as i64 >= 0 {
    //             F::from_canonical_u64(inst.b_offset_imm0)
    //         } else {
    //             F::neg(F::from_canonical_u64((-(inst.b_offset_imm0 as i64)) as u64))
    //         };

    //         // Fill the rom trace row fields
    //         rom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc,
    // paddr, line         rom_trace[i].a_offset_imm0 = a_offset_imm0;
    //         rom_trace[i].a_imm1 =
    //             F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0
    // });         rom_trace[i].b_offset_imm0 = b_offset_imm0;
    //         rom_trace[i].b_imm1 =
    //             F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0
    // });         //rom_trace[i].b_src_ind =
    //         //    F::from_canonical_u64(if inst.b_src == SRC_IND { 1 } else { 0 });
    //         rom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
    //         rom_trace[i].op = F::from_canonical_u8(inst.op);
    //         rom_trace[i].store_offset = store_offset;
    //         rom_trace[i].jmp_offset1 = jmp_offset1;
    //         rom_trace[i].jmp_offset2 = jmp_offset2;
    //         rom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
    //         rom_trace[i].multiplicity = F::from_canonical_u64(multiplicity);
    //     }

    //     // Padd with zeroes
    //     for i in number_of_instructions..trace_size {
    //         rom_trace[i] = RomL2Row::default();
    //     }

    //     Ok((prover_buffer, offsets[0], ROM_L_AIR_IDS[0]))
    // }

    pub fn prove_instance(
        pctx: Arc<ProofCtx<F>>,
        rom: &ZiskRom,
        plan: &Plan,
        min_traces: Arc<Vec<EmuTrace>>,
        buffer: &mut WitnessBuffer<F>,
    ) {
        let rom_air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]);
        let rom_rows = rom_air.num_rows();

        let metadata = plan.meta.as_ref().unwrap().downcast_ref::<RomSurveyor>().unwrap();
        let pc_histogram = &metadata.rom.inst_count;

        // Create an empty ROM trace
        let mut rom_trace =
            RomTrace::<F>::map_buffer(&mut buffer.buffer, rom_rows, buffer.offset as usize)
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
                    //TODO! UNCOMMENT
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
    fn get_surveyor(&self) -> Box<dyn Surveyor> {
        Box::new(RomSurveyor::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(RomPlanner {})
    }

    fn get_instance(self: Arc<Self>, iectx: InstanceExpanderCtx<F>) -> Box<dyn InstanceXXXX> {
        Box::new(RomInstance::new(self.wcm.clone(), self.zisk_rom.clone(), iectx))
    }
}

impl<F: Field> WitnessComponent<F> for RomSM<F> {}

pub struct RomInstance<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
    zisk_rom: Arc<ZiskRom>,
    iectx: InstanceExpanderCtx<F>,
}

impl<F: PrimeField> RomInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        zisk_rom: Arc<ZiskRom>,
        iectx: InstanceExpanderCtx<F>,
    ) -> Self {
        Self { wcm, zisk_rom, iectx }
    }
}
impl<F: PrimeField> InstanceXXXX for RomInstance<F> {
    fn expand(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn prove(
        &mut self,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        RomSM::prove_instance(
            self.wcm.get_pctx(),
            &self.zisk_rom,
            &mut self.iectx.plan,
            min_traces,
            &mut self.iectx.buffer,
        );

        let buffer = std::mem::take(&mut self.iectx.buffer.buffer);

        let air_instance = AirInstance::new(
            self.wcm.get_sctx().clone(),
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            self.iectx.plan.segment_id,
            buffer,
        );

        self.wcm
            .get_pctx()
            .air_instance_repo
            .add_air_instance(air_instance, Some(self.iectx.instance_global_idx));
        Ok(())
    }
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
