use zisk_common::OperationKeccakData;

#[derive(Debug)]
pub struct KeccakfInput {
    pub step_main: u64,
    pub addr_main: u32,
    pub state: [u64; 25],
}

impl KeccakfInput {
    pub fn from(values: &OperationKeccakData<u64>) -> Self {
        Self {
            step_main: values[2],
            addr_main: values[3] as u32,
            state: values[4..29].try_into().unwrap(),
        }
    }
}
