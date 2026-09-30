use zisk_common::OperationBabyJubJubAddData;

#[derive(Debug)]
pub enum BabyJubJubInput {
    Add(BabyJubJubAddInput),
}

#[derive(Debug)]
pub struct BabyJubJubAddInput {
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl BabyJubJubAddInput {
    pub fn from(values: &OperationBabyJubJubAddData<u64>) -> Self {
        Self {
            p1_addr: values[2] as u32,
            p2_addr: values[3] as u32,
            step: values[4],
            p1: values[5..13].try_into().unwrap(),
            p2: values[13..21].try_into().unwrap(),
        }
    }
}
