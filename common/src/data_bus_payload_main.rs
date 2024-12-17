use zisk_core::{InstContext, ZiskInst};

use crate::PayloadType;

const MAIN_DATA_SIZE: usize = 8;
const STEP: usize = 0;
const PC: usize = 1;
const OP: usize = 2;
const OP_TYPE: usize = 3;
const A: usize = 4;
const B: usize = 5;
const C: usize = 6;
const END: usize = 7;

pub type MainData<D> = [D; MAIN_DATA_SIZE];

pub const MAIN_BUS_OPID: u16 = 5000;

pub struct DataBusMain<D>(std::marker::PhantomData<D>);

impl DataBusMain<u64> {
    #[inline(always)]
    pub fn new_payload(inst: &ZiskInst, inst_ctx: &InstContext) -> MainData<u64> {
        [
            inst_ctx.step,       // STEP
            inst_ctx.pc,         // PC
            inst.op as u64,      // OP
            inst.op_type as u64, // OP_TYPE
            inst_ctx.a,          // A
            inst_ctx.b,          // B
            inst_ctx.c,          // C
            inst_ctx.end as u64, // END
        ]
    }

    #[inline(always)]
    pub fn get_step(data: &MainData<u64>) -> PayloadType {
        data[STEP]
    }

    #[inline(always)]
    pub fn get_pc(data: &MainData<u64>) -> PayloadType {
        data[PC]
    }

    #[inline(always)]
    pub fn get_op(data: &MainData<u64>) -> PayloadType {
        data[OP]
    }

    #[inline(always)]
    pub fn get_op_type(data: &MainData<u64>) -> PayloadType {
        data[OP_TYPE]
    }

    #[inline(always)]
    pub fn get_a(data: &MainData<u64>) -> PayloadType {
        data[A]
    }

    #[inline(always)]
    pub fn get_b(data: &MainData<u64>) -> PayloadType {
        data[B]
    }

    #[inline(always)]
    pub fn get_c(data: &MainData<u64>) -> PayloadType {
        data[C]
    }

    #[inline(always)]
    pub fn get_end(data: &MainData<u64>) -> PayloadType {
        data[END]
    }
}

// pub trait MainDataExt {
//     fn get_step(&self) -> Data;
//     fn get_pc(&self) -> Data;
//     fn get_op(&self) -> Data;
//     fn get_op_type(&self) -> Data;
//     fn get_a(&self) -> Data;
//     fn get_b(&self) -> Data;
//     fn get_c(&self) -> Data;
//     fn get_end(&self) -> Data;
// }

// impl MainDataExt for MainData {
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
