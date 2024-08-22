pub type OpResult = (u64, bool);

#[derive(Debug, Clone)]
pub enum Arith32Op {
    Add(u32, u32),
    Sub(u32, u32),
}

#[derive(Debug, Clone)]
pub enum Arith64Op {
    Add(u64, u64),
    Sub(u64, u64),
}

#[derive(Debug, Clone)]
pub enum Arith3264Op {
    Add32(u32, u32),
    Add64(u64, u64),
    Sub32(u32, u32),
    Sub64(u64, u64),
}

impl From<Arith3264Op> for Arith32Op {
    fn from(op: Arith3264Op) -> Arith32Op {
        match op {
            Arith3264Op::Add32(a, b) => Arith32Op::Add(a, b),
            Arith3264Op::Sub32(a, b) => Arith32Op::Sub(a, b),
            _ => panic!("Invalid conversion"),
        }
    }
}

impl From<Arith3264Op> for Arith64Op {
    fn from(op: Arith3264Op) -> Arith64Op {
        match op {
            Arith3264Op::Add64(a, b) => Arith64Op::Add(a, b),
            Arith3264Op::Sub64(a, b) => Arith64Op::Sub(a, b),
            _ => panic!("Invalid conversion"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FreqOp {
    Add(u64, u64),
}

#[derive(Debug, Clone)]
pub enum MemOp {
    Read(u64),
    Write(u64, u64),
}

#[derive(Debug, Clone)]
pub enum MemUnalignedOp {
    Read(u64, usize),
    Write(u64, usize, u64),
}
