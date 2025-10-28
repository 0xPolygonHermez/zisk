use crate::add256_constants::*;
use zisk_common::OperationAdd256Data;
use zisk_common::{A, B, OPERATION_BUS_DATA_SIZE};

#[derive(Debug)]
pub struct Add256Input {
    pub step_main: u64,
    pub addr_main: u32,
    pub addr_a: u32,
    pub addr_b: u32,
    pub addr_c: u32,
    pub cin: u64,
    pub a: [u64; 4],
    pub b: [u64; 4],
}

impl Add256Input {
    pub fn from(values: &OperationAdd256Data<u64>) -> Self {
        Self {
            step_main: values[A],
            addr_main: values[B] as u32,
            addr_a: values[OPERATION_BUS_DATA_SIZE] as u32,
            addr_b: values[OPERATION_BUS_DATA_SIZE + 1] as u32,
            addr_c: values[OPERATION_BUS_DATA_SIZE + READ_PARAMS + DIRECT_READ_PARAMS] as u32,
            cin: values[OPERATION_BUS_DATA_SIZE + READ_PARAMS],
            a: values[START_READ_PARAMS..START_READ_PARAMS + PARAM_CHUNKS].try_into().unwrap(),
            b: values[START_READ_PARAMS + PARAM_CHUNKS..START_READ_PARAMS + 2 * PARAM_CHUNKS]
                .try_into()
                .unwrap(),
        }
    }
}
