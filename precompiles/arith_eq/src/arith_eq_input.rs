use zisk_core::zisk_ops::swap_endianness_elements;

use zisk_common::{
    OperationArith256Data, OperationArith256ModData, OperationBn254ComplexAddData,
    OperationBn254ComplexMulData, OperationBn254ComplexSubData, OperationBn254CurveAddData,
    OperationBn254CurveDblData, OperationSecp256k1AddData, OperationSecp256k1DblData,
    OperationSecp256r1AddData, OperationSecp256r1DblData,
};

#[derive(Debug)]
pub enum ArithEqInput {
    Arith256(Arith256Input),
    Arith256Mod(Arith256ModInput),
    Secp256k1Add(Secp256k1AddInput),
    Secp256k1Dbl(Secp256k1DblInput),
    Bn254CurveAdd(Bn254CurveAddInput),
    Bn254CurveDbl(Bn254CurveDblInput),
    Bn254ComplexAdd(Bn254ComplexAddInput),
    Bn254ComplexSub(Bn254ComplexSubInput),
    Bn254ComplexMul(Bn254ComplexMulInput),
    Secp256r1Add(Secp256r1AddInput),
    Secp256r1Dbl(Secp256r1DblInput),
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
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationArith256Data<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.a, 4);
            swap_endianness_elements(&mut input.b, 4);
            swap_endianness_elements(&mut input.c, 4);
        }
        input
    }

    pub fn from(values: &OperationArith256Data<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[5] as u32,
            b_addr: values[6] as u32,
            c_addr: values[7] as u32,
            dl_addr: values[8] as u32,
            dh_addr: values[9] as u32,
            step: values[4],
            a: values[10..14].try_into().unwrap(),
            b: values[14..18].try_into().unwrap(),
            c: values[18..22].try_into().unwrap(),
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

impl Arith256ModInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationArith256ModData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.a, 4);
            swap_endianness_elements(&mut input.b, 4);
            swap_endianness_elements(&mut input.c, 4);
            swap_endianness_elements(&mut input.module, 4);
        }
        input
    }

    pub fn from(values: &OperationArith256ModData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[5] as u32,
            b_addr: values[6] as u32,
            c_addr: values[7] as u32,
            module_addr: values[8] as u32,
            d_addr: values[9] as u32,
            step: values[4],
            a: values[10..14].try_into().unwrap(),
            b: values[14..18].try_into().unwrap(),
            c: values[18..22].try_into().unwrap(),
            module: values[22..26].try_into().unwrap(),
        }
    }
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

impl Secp256k1AddInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationSecp256k1AddData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.p1, 4);
            swap_endianness_elements(&mut input.p2, 4);
        }
        input
    }

    pub fn from(values: &OperationSecp256k1AddData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..15].try_into().unwrap(),
            p2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256k1DblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}

impl Secp256k1DblInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationSecp256k1DblData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.p1, 4);
        }
        input
    }

    pub fn from(values: &OperationSecp256k1DblData<u64>) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..13].try_into().unwrap() }
    }
}

#[derive(Debug)]
pub struct Bn254CurveAddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl Bn254CurveAddInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationBn254CurveAddData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.p1, 4);
            swap_endianness_elements(&mut input.p2, 4);
        }
        input
    }

    pub fn from(values: &OperationBn254CurveAddData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..15].try_into().unwrap(),
            p2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bn254CurveDblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}

impl Bn254CurveDblInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationBn254CurveDblData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.p1, 4);
        }
        input
    }

    pub fn from(values: &OperationBn254CurveDblData<u64>) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..13].try_into().unwrap() }
    }
}

#[derive(Debug)]
pub struct Bn254ComplexAddInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; 8],
    pub f2: [u64; 8],
}

impl Bn254ComplexAddInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationBn254ComplexAddData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.f1, 4);
            swap_endianness_elements(&mut input.f2, 4);
        }
        input
    }

    pub fn from(values: &OperationBn254ComplexAddData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..15].try_into().unwrap(),
            f2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bn254ComplexSubInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; 8],
    pub f2: [u64; 8],
}

impl Bn254ComplexSubInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationBn254ComplexSubData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.f1, 4);
            swap_endianness_elements(&mut input.f2, 4);
        }
        input
    }

    pub fn from(values: &OperationBn254ComplexSubData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..15].try_into().unwrap(),
            f2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bn254ComplexMulInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; 8],
    pub f2: [u64; 8],
}

impl Bn254ComplexMulInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationBn254ComplexMulData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.f1, 4);
            swap_endianness_elements(&mut input.f2, 4);
        }
        input
    }

    pub fn from(values: &OperationBn254ComplexMulData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..15].try_into().unwrap(),
            f2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256r1AddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl Secp256r1AddInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationSecp256r1AddData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.p1, 4);
            swap_endianness_elements(&mut input.p2, 4);
        }
        input
    }

    pub fn from(values: &OperationSecp256r1AddData<u64>) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..15].try_into().unwrap(),
            p2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256r1DblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}

impl Secp256r1DblInput {
    /// From the bus data of the little-endian op, or of its big-endian twin: a big-endian
    /// op carries its operands as they sit in memory (big-endian integers), and the executors
    /// work on little-endian limbs, so they are converted here (addresses are not affected).
    pub fn from_bus(values: &OperationSecp256r1DblData<u64>, big_endian: bool) -> Self {
        let mut input = Self::from(values);
        if big_endian {
            swap_endianness_elements(&mut input.p1, 4);
        }
        input
    }

    pub fn from(values: &OperationSecp256r1DblData<u64>) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..13].try_into().unwrap() }
    }
}
