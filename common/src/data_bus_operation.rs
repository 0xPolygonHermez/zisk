use crate::PayloadType;
use zisk_core::{InstContext, ZiskInst};

pub const OPERATION_BUS_ID: u16 = 5000;

pub const OPERATION_BUS_DATA_SIZE: usize = 5;

const STEP: usize = 0;
const OP: usize = 1;
const OP_TYPE: usize = 2;
const A: usize = 3;
const B: usize = 4;

pub type OperationData<D> = [D; OPERATION_BUS_DATA_SIZE];

pub struct OperationBusData<D>(std::marker::PhantomData<D>);

impl OperationBusData<u64> {
    #[inline(always)]
    pub fn new_payload(inst: &ZiskInst, inst_ctx: &InstContext) -> OperationData<u64> {
        let a = if inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a };
        let b = if inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b };

        [
            inst_ctx.step,       // STEP
            inst.op as u64,      // OP
            inst.op_type as u64, // OP_TYPE
            a,                   // A
            b,                   // B
        ]
    }

    #[inline(always)]
    pub fn get_step(data: &OperationData<u64>) -> PayloadType {
        data[STEP]
    }

    #[inline(always)]
    pub fn get_op(data: &OperationData<u64>) -> u8 {
        data[OP] as u8
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
}
