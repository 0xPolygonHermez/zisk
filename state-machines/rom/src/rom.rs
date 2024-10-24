use std::{path::PathBuf, sync::Arc};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use zisk_core::{Riscv2zisk, SRC_IMM, SRC_IND};
use zisk_pil::RomS0Trace;
use zisk_pil::{ROM_AIRGROUP_ID, ROM_L_AIR_IDS, ROM_M_AIR_IDS, ROM_S_AIR_IDS};
//use ziskemu::ZiskEmulatorErr;

pub struct RomSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Field> RomSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let rom_sm = Self { _phantom: std::marker::PhantomData };
        let rom_sm = Arc::new(rom_sm);

        let rom_air_ids = &[ROM_S_AIR_IDS[0], ROM_M_AIR_IDS[0], ROM_L_AIR_IDS[0]];
        wcm.register_component(rom_sm.clone(), Some(ROM_AIRGROUP_ID), Some(rom_air_ids));

        rom_sm
    }

    pub fn prove(&self, rom_path: PathBuf) -> Result<(), Box<dyn std::error::Error + Send>> {
        let prover_buffer = &mut [F::zero(); 1];
        let offset = 0;

        Self::compute_trace(rom_path, prover_buffer, offset)
    }

    pub fn compute_trace(
        _rom_path: PathBuf,
        _prover_buffer: &mut [F],
        _offset: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        // FIXME! Implement proof logic
        println!("Proving ROM");
        let elf_filename: String = _rom_path.to_str().unwrap().into();

        // Convert the ELF file to ZisK ROM
        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename, String::new(), String::new(), String::new());

        // Convert program to rom
        let zisk_rom = riscv2zisk.run();
        if zisk_rom.is_err() {
            //return Err(ZiskEmulatorErr::Unknown(zisk_rom.err().unwrap().to_string()));
            panic!("RomSM::prover() failed converting elf to rom");
        }

        const CHUNK_SIZE: usize = 4096;
        let mut rom_trace = RomS0Trace::<F>::new(CHUNK_SIZE);

        for (i, inst) in zisk_rom.unwrap().insts.into_iter().enumerate() {
            rom_trace[i].line = F::from_canonical_u64(inst.1.i.paddr);
            rom_trace[i].a_offset_imm0 = F::from_canonical_u64(inst.1.i.a_offset_imm0);
            rom_trace[i].a_imm1 = F::from_canonical_u64(if inst.1.i.a_src == SRC_IMM {
                inst.1.i.a_use_sp_imm1
            } else {
                0
            });
            rom_trace[i].b_offset_imm0 = F::from_canonical_u64(inst.1.i.b_offset_imm0);
            rom_trace[i].b_imm1 = F::from_canonical_u64(if inst.1.i.b_src == SRC_IMM {
                inst.1.i.b_use_sp_imm1
            } else {
                0
            });
            rom_trace[i].b_src_ind = F::from_canonical_u64(if inst.1.i.b_src == SRC_IND {
                inst.1.i.b_offset_imm0
            } else {
                0
            });
            rom_trace[i].ind_width = F::from_canonical_u64(inst.1.i.ind_width);
            rom_trace[i].op = F::from_canonical_u8(inst.1.i.op);
            rom_trace[i].store_offset = F::from_canonical_u64(inst.1.i.store_offset as u64);
            rom_trace[i].jmp_offset1 = F::from_canonical_u64(inst.1.i.jmp_offset1 as u64);
            rom_trace[i].jmp_offset2 = F::from_canonical_u64(inst.1.i.jmp_offset2 as u64);
            rom_trace[i].multiplicity = F::from_canonical_u64(1); // TODO: review
        }

        Ok(())
    }
}

impl<F: Field> WitnessComponent<F> for RomSM<F> {}
