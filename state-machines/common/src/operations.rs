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
pub enum Binary32Op {
    And(u32, u32),
    Or(u32, u32),
}

#[derive(Debug, Clone)]
pub enum Binary64Op {
    And(u64, u64),
    Or(u64, u64),
}

#[derive(Debug, Clone)]
pub enum Binary3264Op {
    And32(u32, u32),
    And64(u64, u64),
    Or32(u32, u32),
    Or64(u64, u64),
}

impl From<Binary3264Op> for Binary32Op {
    fn from(op: Binary3264Op) -> Binary32Op {
        match op {
            Binary3264Op::And32(a, b) => Binary32Op::And(a, b),
            Binary3264Op::Or32(a, b) => Binary32Op::Or(a, b),
            _ => panic!("Invalid conversion"),
        }
    }
}

impl From<Binary3264Op> for Binary64Op {
    fn from(op: Binary3264Op) -> Binary64Op {
        match op {
            Binary3264Op::And64(a, b) => Binary64Op::And(a, b),
            Binary3264Op::Or64(a, b) => Binary64Op::Or(a, b),
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
