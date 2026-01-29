use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, STEP};

use crate::DMA_64_ALIGNED_OPS_BY_ROW;

#[derive(Debug)]
pub struct Dma64AlignedInput {
    pub src: u32,
    pub dst: u32,
    pub is_first_instance_input: bool,
    pub is_last_instance_input: bool,
    pub is_mem_eq: bool,
    pub trace_offset: u32, // offset inside trace to paralelize
    pub skip_rows: u32,    // inside input how many rows skip
    pub rows: u32,         // number of rows used
    pub step: u64,
    pub encoded: u64,
    pub src_values: Vec<u64>,
}

impl Dma64AlignedInput {
    pub fn get_rows(data: &[u64]) -> usize {
        let encoded = data[DMA_ENCODED];
        if DmaInfo::get_dst_offset(encoded) == DmaInfo::get_src_offset(encoded) {
            let count = DmaInfo::get_loop_count(encoded);
            if count > 0 {
                count.div_ceil(DMA_64_ALIGNED_OPS_BY_ROW)
            } else {
                0
            }
        } else {
            0
        }
    }
    pub fn get_count(data: &[u64]) -> usize {
        let encoded = data[DMA_ENCODED];
        if DmaInfo::get_dst_offset(encoded) == DmaInfo::get_src_offset(encoded) {
            DmaInfo::get_loop_count(encoded)
        } else {
            0
        }
    }
    pub fn from(
        data: &[u64],
        data_ext: &[u64],
        trace_offset: usize,
        skip_rows: usize,
        max_rows: usize,
        is_last_instance_input: bool,
    ) -> Self {
        let encoded = data[DMA_ENCODED];
        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let skip_count = skip_rows * DMA_64_ALIGNED_OPS_BY_ROW;
        let data_offset = DmaInfo::get_loop_data_offset(encoded) + skip_count;
        let count = DmaInfo::get_loop_count(encoded) - skip_count;
        let total_rows = DmaInfo::get_loop_count(encoded).div_ceil(DMA_64_ALIGNED_OPS_BY_ROW);
        let rows = std::cmp::min(total_rows - skip_rows, max_rows) as u32;
        Self {
            dst: data[A] as u32 + pre_count,
            src: data[B] as u32 + pre_count,
            trace_offset: trace_offset as u32,
            is_first_instance_input: trace_offset == 0,
            is_last_instance_input,
            step: data[STEP],
            skip_rows: skip_rows as u32,
            rows,
            encoded,
            src_values: data_ext[data_offset..data_offset + count].to_vec(),
            is_mem_eq: false,
        }
    }
}
