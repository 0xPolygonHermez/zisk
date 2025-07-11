use zisk_common::OperationSha256Data;

#[derive(Debug)]
pub struct Sha256fInput {
    pub step_main: u64,
    pub addr_main: u32,
    pub state_addr: u32,
    pub input_addr: u32,
    pub state: [u64; 4],
    pub input: [u64; 8],
}

impl Sha256fInput {
    pub fn from(values: &OperationSha256Data<u64>) -> Self {
        Self {
            step_main: values[2],
            addr_main: values[3] as u32,
            state_addr: values[4] as u32,
            input_addr: values[5] as u32,
            state: values[6..10].try_into().unwrap(),
            input: values[10..18].try_into().unwrap(),
        }
    }
}
