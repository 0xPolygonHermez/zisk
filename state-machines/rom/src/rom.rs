use std::{path::PathBuf, sync::Arc};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, BufferAllocator, SetupCtx};
use proofman_util::create_buffer_fast;

use zisk_core::{Riscv2zisk, SRC_IMM, SRC_IND};
use zisk_pil::{
    Pilout, RomL2Row, RomL2Trace, RomM1Row, RomM1Trace, RomS0Row, RomS0Trace, ROM_AIRGROUP_ID,
    ROM_L_AIR_IDS, ROM_M_AIR_IDS, ROM_S_AIR_IDS,
};
//use ziskemu::ZiskEmulatorErr;
use std::error::Error;

pub struct RomSM<F> {
    wcm: Arc<WitnessManager<F>>,
}

impl<F: Field> RomSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, _sctx: Arc<SetupCtx>) -> Arc<Self> {
        let rom_sm = Self { wcm: wcm.clone() };
        let rom_sm = Arc::new(rom_sm);

        let rom_air_ids = &[ROM_S_AIR_IDS[0], ROM_M_AIR_IDS[0], ROM_L_AIR_IDS[0]];
        wcm.register_component(rom_sm.clone(), Some(ROM_AIRGROUP_ID), Some(rom_air_ids));

        rom_sm
    }

    pub fn prove(&self, rom_path: PathBuf) -> Result<(), Box<dyn std::error::Error + Send>> {
        let buffer_allocator = self.wcm.get_ectx().buffer_allocator.clone();
        let sctx = self.wcm.get_sctx();

        let (prover_buffer, _, air_id) = Self::compute_trace(rom_path, buffer_allocator, sctx)?;

        let air_instance = AirInstance::new(
            self.wcm.get_arc_sctx().clone(),
            ROM_AIRGROUP_ID,
            air_id,
            None,
            prover_buffer,
        );
        self.wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);

        Ok(())
    }

    pub fn compute_trace(
        rom_path: PathBuf,
        buffer_allocator: Arc<dyn BufferAllocator>,
        sctx: &SetupCtx,
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

        let pilout = Pilout::pilout();
        let sizes = (
            pilout.get_air(ROM_AIRGROUP_ID, ROM_S_AIR_IDS[0]).num_rows(),
            pilout.get_air(ROM_AIRGROUP_ID, ROM_M_AIR_IDS[0]).num_rows(),
            pilout.get_air(ROM_AIRGROUP_ID, ROM_L_AIR_IDS[0]).num_rows(),
        );

        let number_of_instructions = rom.insts.len();

        match number_of_instructions {
            n if n <= sizes.0 => Self::create_rom_s(sizes.0, rom, n, buffer_allocator, sctx),
            n if n <= sizes.1 => Self::create_rom_m(sizes.1, rom, n, buffer_allocator, sctx),
            n if n < sizes.2 => Self::create_rom_l(sizes.2, rom, n, buffer_allocator, sctx),
            _ => panic!("RomSM::compute_trace() found rom too big size={}", number_of_instructions),
        }
    }

    fn create_rom_s(
        rom_s_size: usize,
        rom: zisk_core::ZiskRom,
        number_of_instructions: usize,
        buffer_allocator: Arc<dyn BufferAllocator>,
        sctx: &SetupCtx,
    ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
        // Set trace size
        let trace_size = rom_s_size;

        // Allocate a prover buffer
        let (buffer_size, offsets) = buffer_allocator
            .get_buffer_info(sctx, ROM_AIRGROUP_ID, ROM_S_AIR_IDS[0])
            .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));
        let mut prover_buffer = create_buffer_fast(buffer_size as usize);

        // Create an empty ROM trace
        let mut rom_trace =
            RomS0Trace::<F>::map_buffer(&mut prover_buffer, trace_size, offsets[0] as usize)
                .expect("RomSM::compute_trace() failed mapping buffer to ROMS0Trace");

        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, inst_builder) in rom.insts.into_iter().enumerate() {
            let inst = inst_builder.1.i;
            rom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_trace[i].a_offset_imm0 = F::from_canonical_u64(inst.a_offset_imm0);
            rom_trace[i].a_imm1 =
                F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_trace[i].b_offset_imm0 = F::from_canonical_u64(inst.b_offset_imm0);
            rom_trace[i].b_imm1 =
                F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            rom_trace[i].b_src_ind =
                F::from_canonical_u64(if inst.b_src == SRC_IND { inst.b_offset_imm0 } else { 0 });
            rom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
            rom_trace[i].op = F::from_canonical_u8(inst.op);
            rom_trace[i].store_offset = F::from_canonical_u64(inst.store_offset as u64);
            rom_trace[i].jmp_offset1 = F::from_canonical_u64(inst.jmp_offset1 as u64);
            rom_trace[i].jmp_offset2 = F::from_canonical_u64(inst.jmp_offset2 as u64);
            rom_trace[i].multiplicity = F::from_canonical_u64(1); // TODO: review
            rom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
        }

        // Padd with zeroes
        for i in number_of_instructions..trace_size {
            rom_trace[i] = RomS0Row::default();
        }

        Ok((prover_buffer, offsets[0], ROM_S_AIR_IDS[0]))
    }

    fn create_rom_m(
        rom_m_size: usize,
        rom: zisk_core::ZiskRom,
        number_of_instructions: usize,
        buffer_allocator: Arc<dyn BufferAllocator>,
        sctx: &SetupCtx,
    ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
        // Set trace size
        let trace_size = rom_m_size;

        // Allocate a prover buffer
        let (buffer_size, offsets) = buffer_allocator
            .get_buffer_info(sctx, ROM_AIRGROUP_ID, ROM_M_AIR_IDS[0])
            .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));
        let mut prover_buffer = create_buffer_fast(buffer_size as usize);

        // Create an empty ROM trace
        let mut rom_trace =
            RomM1Trace::<F>::map_buffer(&mut prover_buffer, trace_size, offsets[0] as usize)
                .expect("RomSM::compute_trace() failed mapping buffer to ROMM0Trace");

        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, inst_builder) in rom.insts.into_iter().enumerate() {
            let inst = inst_builder.1.i;
            rom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_trace[i].a_offset_imm0 = F::from_canonical_u64(inst.a_offset_imm0);
            rom_trace[i].a_imm1 =
                F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_trace[i].b_offset_imm0 = F::from_canonical_u64(inst.b_offset_imm0);
            rom_trace[i].b_imm1 =
                F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            rom_trace[i].b_src_ind =
                F::from_canonical_u64(if inst.b_src == SRC_IND { inst.b_offset_imm0 } else { 0 });
            rom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
            rom_trace[i].op = F::from_canonical_u8(inst.op);
            rom_trace[i].store_offset = F::from_canonical_u64(inst.store_offset as u64);
            rom_trace[i].jmp_offset1 = F::from_canonical_u64(inst.jmp_offset1 as u64);
            rom_trace[i].jmp_offset2 = F::from_canonical_u64(inst.jmp_offset2 as u64);
            rom_trace[i].multiplicity = F::from_canonical_u64(1); // TODO: review
            rom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
        }

        // Padd with zeroes
        for i in number_of_instructions..trace_size {
            rom_trace[i] = RomM1Row::default();
        }

        Ok((prover_buffer, offsets[0], ROM_M_AIR_IDS[0]))
    }

    fn create_rom_l(
        rom_l_size: usize,
        rom: zisk_core::ZiskRom,
        number_of_instructions: usize,
        buffer_allocator: Arc<dyn BufferAllocator>,
        sctx: &SetupCtx,
    ) -> Result<(Vec<F>, u64, usize), Box<dyn Error + Send>> {
        // Set trace size
        let trace_size = rom_l_size;

        // Allocate a prover buffer
        let (buffer_size, offsets) = buffer_allocator
            .get_buffer_info(sctx, ROM_AIRGROUP_ID, ROM_L_AIR_IDS[0])
            .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));
        let mut prover_buffer = create_buffer_fast(buffer_size as usize);

        // Create an empty ROM trace
        let mut rom_trace =
            RomL2Trace::<F>::map_buffer(&mut prover_buffer, trace_size, offsets[0] as usize)
                .expect("RomSM::compute_trace() failed mapping buffer to ROML0Trace");

        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, inst_builder) in rom.insts.into_iter().enumerate() {
            let inst = inst_builder.1.i;
            rom_trace[i].line = F::from_canonical_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_trace[i].a_offset_imm0 = F::from_canonical_u64(inst.a_offset_imm0);
            rom_trace[i].a_imm1 =
                F::from_canonical_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_trace[i].b_offset_imm0 = F::from_canonical_u64(inst.b_offset_imm0);
            rom_trace[i].b_imm1 =
                F::from_canonical_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            rom_trace[i].b_src_ind =
                F::from_canonical_u64(if inst.b_src == SRC_IND { inst.b_offset_imm0 } else { 0 });
            rom_trace[i].ind_width = F::from_canonical_u64(inst.ind_width);
            rom_trace[i].op = F::from_canonical_u8(inst.op);
            rom_trace[i].store_offset = F::from_canonical_u64(inst.store_offset as u64);
            rom_trace[i].jmp_offset1 = F::from_canonical_u64(inst.jmp_offset1 as u64);
            rom_trace[i].jmp_offset2 = F::from_canonical_u64(inst.jmp_offset2 as u64);
            rom_trace[i].multiplicity = F::from_canonical_u64(1); // TODO: review
            rom_trace[i].flags = F::from_canonical_u64(inst.get_flags());
        }

        // Padd with zeroes
        for i in number_of_instructions..trace_size {
            rom_trace[i] = RomL2Row::default();
        }

        Ok((prover_buffer, offsets[0], ROM_L_AIR_IDS[0]))
    }
}

impl<F: Field> WitnessComponent<F> for RomSM<F> {}
