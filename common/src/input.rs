use crate::{ExtOperationData, OperationBusData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    pub op: u8,
    pub a: u64,
    pub b: u64,
}

impl Input {
    #[allow(dead_code)]
    pub fn new(op: u8, a: u64, b: u64) -> Self {
        Self { op, a, b }
    }
    pub fn from(data: &ExtOperationData<u64>) -> Self {
        Self {
            op: OperationBusData::get_op(data),
            a: OperationBusData::get_a(data),
            b: OperationBusData::get_b(data),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryAddInput {
    pub a: u64,
    pub b: u64,
}

impl BinaryAddInput {
    #[allow(dead_code)]
    pub fn new(a: u64, b: u64) -> Self {
        Self { a, b }
    }
}
