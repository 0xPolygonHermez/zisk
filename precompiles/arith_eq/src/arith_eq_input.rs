use data_bus::OperationArith256Data;

#[derive(Debug)]
pub enum ArithEqInput {
    Arith256(Arith256Input),
    Arith256Mod(Arith256ModInput),
    Secp256k1Add(Secp256k1AddInput),
    Secp256k1Dbl(Secp256k1DblInput),
}

#[derive(Debug)]
pub struct Arith256Input {
    pub addr: u32,
    pub a_addr: u32,
    pub b_addr: u32,
    pub c_addr: u32,
    pub dh_addr: u32,
    pub dl_addr: u32,
    pub step: u64,
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub c: [u64; 4],
}

impl Arith256Input {
    pub fn from(values: &OperationArith256Data<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[4] as u32,
            b_addr: values[5] as u32,
            c_addr: values[6] as u32,
            dl_addr: values[7] as u32,
            dh_addr: values[8] as u32,
            step: values[2],
            a: values[9..13].try_into().unwrap(),
            b: values[13..17].try_into().unwrap(),
            c: values[17..21].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Arith256ModInput {
    pub addr: u32,
    pub a_addr: u32,
    pub b_addr: u32,
    pub c_addr: u32,
    pub module_addr: u32,
    pub d_addr: u32,
    pub step: u64,
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub c: [u64; 4],
    pub module: [u64; 4],
}

#[derive(Debug)]
pub struct Secp256k1AddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

#[derive(Debug)]
pub struct Secp256k1DblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}
