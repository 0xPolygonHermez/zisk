use zisk_common::OperationBlake2Data;

#[derive(Debug)]
pub struct Blake2Input {
    pub addr_main: u32,
    pub step_main: u64,
    pub index: u64,
    pub state_addr: u32,
    pub input_addr: u32,
    pub state: [u64; 16],
    pub input: [u64; 16],
}

impl Blake2Input {
    pub fn from(values: &OperationBlake2Data<u64>) -> Self {
        Self {
            addr_main: values[3] as u32,
            step_main: values[4],
            index: values[5],
            state_addr: values[6] as u32,
            input_addr: values[7] as u32,
            state: values[8..24].try_into().unwrap(),
            input: values[24..40].try_into().unwrap(),
        }
    }
}
