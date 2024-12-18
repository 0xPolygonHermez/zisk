use crate::PayloadType;
use zisk_core::{InstContext, ZiskInst};

pub const OPERATION_BUS_ID: u16 = 5000;

pub const OPERATION_BUS_DATA_SIZE: usize = 8;

const STEP: usize = 0;
const PC: usize = 1;
const OP: usize = 2;
const OP_TYPE: usize = 3;
const A: usize = 4;
const B: usize = 5;
const C: usize = 6;
const END: usize = 7;

pub type OperationData<D> = [D; OPERATION_BUS_DATA_SIZE];

pub struct OperationBusData<D>(std::marker::PhantomData<D>);

impl OperationBusData<u64> {
    #[inline(always)]
    pub fn new_payload(inst: &ZiskInst, inst_ctx: &InstContext) -> OperationData<u64> {
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
    pub fn get_step(data: &OperationData<u64>) -> PayloadType {
        data[STEP]
    }

    #[inline(always)]
    pub fn get_pc(data: &OperationData<u64>) -> PayloadType {
        data[PC]
    }

    #[inline(always)]
    pub fn get_op(data: &OperationData<u64>) -> PayloadType {
        data[OP]
    }

    #[inline(always)]
    pub fn get_op_type(data: &OperationData<u64>) -> PayloadType {
        data[OP_TYPE]
    }

    #[inline(always)]
    pub fn get_a(data: &OperationData<u64>) -> PayloadType {
        data[A]
    }

    #[inline(always)]
    pub fn get_b(data: &OperationData<u64>) -> PayloadType {
        data[B]
    }

    #[inline(always)]
    pub fn get_c(data: &OperationData<u64>) -> PayloadType {
        data[C]
    }

    #[inline(always)]
    pub fn get_end(data: &OperationData<u64>) -> PayloadType {
        data[END]
    }
}
