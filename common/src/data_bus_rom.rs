use crate::PayloadType;
use zisk_core::{InstContext, ZiskInst};

pub const ROM_BUS_ID: u16 = 7890;

pub const ROM_BUS_DATA_SIZE: usize = 3;

const STEP: usize = 0;
const PC: usize = 1;
const END: usize = 2;

pub type RomData<D> = [D; ROM_BUS_DATA_SIZE];

pub struct RomBusData<D>(std::marker::PhantomData<D>);

impl RomBusData<u64> {
    #[inline(always)]
    pub fn from_instruction(instruction: &ZiskInst, inst_ctx: &InstContext) -> RomData<u64> {
        [
            inst_ctx.step,          // STEP
            inst_ctx.pc,            // PC
            instruction.end as u64, // END
        ]
    }

    #[inline(always)]
    pub fn get_step(data: &RomData<u64>) -> PayloadType {
        data[STEP]
    }

    #[inline(always)]
    pub fn get_pc(data: &RomData<u64>) -> PayloadType {
        data[PC]
    }

    #[inline(always)]
    pub fn get_end(data: &RomData<u64>) -> PayloadType {
        data[END]
    }
}
