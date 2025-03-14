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
