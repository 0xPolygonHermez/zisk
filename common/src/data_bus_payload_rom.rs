use zisk_core::{InstContext, ZiskInst};

use crate::PayloadType;

const ROM_DATA_SIZE: usize = 3;

const STEP: usize = 0;
const PC: usize = 1;
const END: usize = 2;

pub type RomData<D> = [D; ROM_DATA_SIZE];

pub const ROM_BUS_ID: u16 = 7890;

pub struct RomBusData<D>(std::marker::PhantomData<D>);

impl RomBusData<u64> {
    #[inline(always)]
    pub fn new_payload(_: &ZiskInst, inst_ctx: &InstContext) -> RomData<u64> {
        [
            inst_ctx.step,       // STEP
            inst_ctx.pc,         // PC
            inst_ctx.end as u64, // END
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

// pub trait RomDataExt {
//     fn get_step(&self) -> Data;
//     fn get_pc(&self) -> Data;
//     fn get_op(&self) -> Data;
//     fn get_op_type(&self) -> Data;
//     fn get_a(&self) -> Data;
//     fn get_b(&self) -> Data;
//     fn get_c(&self) -> Data;
//     fn get_end(&self) -> Data;
// }

// impl RomDataExt for RomData {
//     #[inline(always)]
//     fn get_step(&self) -> Data {
//         self[STEP]
//     }

//     #[inline(always)]
//     fn get_pc(&self) -> Data {
//         self[PC]
//     }

//     #[inline(always)]
//     fn get_op(&self) -> Data {
//         self[OP]
//     }

//     #[inline(always)]
//     fn get_op_type(&self) -> Data {
//         self[OP_TYPE]
//     }

//     #[inline(always)]
//     fn get_a(&self) -> Data {
//         self[A]
//     }

//     #[inline(always)]
//     fn get_b(&self) -> Data {
//         self[B]
//     }

//     #[inline(always)]
//     fn get_c(&self) -> Data {
//         self[C]
//     }

//     #[inline(always)]
//     fn get_end(&self) -> Data {
//         self[END]
//     }
// }
