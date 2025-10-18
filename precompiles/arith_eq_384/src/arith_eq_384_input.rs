use zisk_common::{
    OperationArith384ModData, OperationBls12_381ComplexAddData, OperationBls12_381ComplexMulData,
    OperationBls12_381ComplexSubData, OperationBls12_381CurveAddData,
    OperationBls12_381CurveDblData,
};

use crate::{ARITH_EQ_384_U64S, ARITH_EQ_384_U64S_DOUBLE};

#[derive(Debug)]
pub enum ArithEq384Input {
    Arith384Mod(Arith384ModInput),
    Bls12_381CurveAdd(Bls12_381CurveAddInput),
    Bls12_381CurveDbl(Bls12_381CurveDblInput),
    Bls12_381ComplexAdd(Bls12_381ComplexAddInput),
    Bls12_381ComplexSub(Bls12_381ComplexSubInput),
    Bls12_381ComplexMul(Bls12_381ComplexMulInput),
}

#[derive(Debug)]
pub struct Arith384ModInput {
    pub addr: u32,
    pub a_addr: u32,
    pub b_addr: u32,
    pub c_addr: u32,
    pub module_addr: u32,
    pub d_addr: u32,
    pub step: u64,
    pub a: [u64; ARITH_EQ_384_U64S],
    pub b: [u64; ARITH_EQ_384_U64S],
    pub c: [u64; ARITH_EQ_384_U64S],
    pub module: [u64; ARITH_EQ_384_U64S],
}

impl Arith384ModInput {
    pub fn from(values: &OperationArith384ModData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[5] as u32,
            b_addr: values[6] as u32,
            c_addr: values[7] as u32,
            module_addr: values[8] as u32,
            d_addr: values[9] as u32,
            step: values[4],
            a: values[10..16].try_into().unwrap(),
            b: values[16..22].try_into().unwrap(),
            c: values[22..28].try_into().unwrap(),
            module: values[28..34].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381CurveAddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub p2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381CurveAddInput {
    pub fn from(values: &OperationBls12_381CurveAddData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..19].try_into().unwrap(),
            p2: values[19..31].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381CurveDblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381CurveDblInput {
    pub fn from(values: &OperationBls12_381CurveDblData<u64>) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..17].try_into().unwrap() }
    }
}

#[derive(Debug)]
pub struct Bls12_381ComplexAddInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub f2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381ComplexAddInput {
    pub fn from(values: &OperationBls12_381ComplexAddData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..19].try_into().unwrap(),
            f2: values[19..31].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381ComplexSubInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub f2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381ComplexSubInput {
    pub fn from(values: &OperationBls12_381ComplexSubData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..19].try_into().unwrap(),
            f2: values[19..31].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381ComplexMulInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub f2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381ComplexMulInput {
    pub fn from(values: &OperationBls12_381ComplexMulData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..19].try_into().unwrap(),
            f2: values[19..31].try_into().unwrap(),
        }
    }
}
