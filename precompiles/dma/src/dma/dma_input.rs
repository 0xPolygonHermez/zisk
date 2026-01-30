use zisk_common::{
    OperationDmaMemCmpData, OperationDmaMemCpyData, A, B, OPERATION_BUS_DMA_MEMCMP_DATA_SIZE,
    OPERATION_BUS_DMA_MEMCPY_DATA_SIZE, STEP,
};

#[derive(Debug)]
pub enum DmaOperation {
    MemCpy,
    MemCmp,
    InputCpy,
    MemSet,
    MemCpy256,
}
#[derive(Debug)]
pub struct DmaInput {
    pub src: u32,
    pub dst: u32,
    pub operation: DmaOperation,
    pub encoded: u64,
    pub count_eq: u32,
    pub result: i32,
    pub step: u64, // main step
}

impl DmaInput {
    pub fn from_memcpy(data: &OperationDmaMemCpyData<u64>, _data_ext: &[u64]) -> Self {
        let encoded = data[OPERATION_BUS_DMA_MEMCPY_DATA_SIZE - 1];
        Self {
            dst: data[A] as u32,
            src: data[B] as u32,
            step: data[STEP],
            encoded,
            count_eq: 0,
            result: 0,
            operation: DmaOperation::MemCpy,
        }
    }

    pub fn from_memcmp(data: &OperationDmaMemCmpData<u64>, _data_ext: &[u64]) -> Self {
        let encoded = data[OPERATION_BUS_DMA_MEMCMP_DATA_SIZE - 2];
        let count_eq = data[OPERATION_BUS_DMA_MEMCMP_DATA_SIZE - 1] as u32;
        let result = (data[OPERATION_BUS_DMA_MEMCMP_DATA_SIZE - 1] >> 32) as i32;

        Self {
            dst: data[A] as u32,
            src: data[B] as u32,
            step: data[STEP],
            encoded,
            count_eq,
            result,
            operation: DmaOperation::MemCmp,
        }
    }
}
