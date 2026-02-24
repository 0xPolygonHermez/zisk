use zisk_common::OperationPoseidon2Data;

#[derive(Debug)]
pub struct Poseidon2Input {
    pub step_main: u64,
    pub addr_main: u32,
    pub state: [u64; 16],
}

impl Poseidon2Input {
    pub fn from(values: &OperationPoseidon2Data<u64>) -> Self {
        Self {
            step_main: values[4],
            addr_main: values[3] as u32,
            state: values[5..21].try_into().unwrap(),
        }
    }
}
