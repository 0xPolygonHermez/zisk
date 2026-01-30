use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, STEP};

#[derive(Debug)]
pub struct DmaUnalignedInput {
    pub src: u32,
    pub dst: u32,
    pub is_first_instance_input: bool,
    pub is_last_instance_input: bool,
    pub trace_offset: u32, // offset inside trace to paralelize
    pub skip: u32,         // inside input how many rows skip
    pub count: u32,        // number of rows used
    pub step: u64,
    pub encoded: u64,
    pub src_values: Vec<u64>,
}

impl DmaUnalignedInput {
    pub fn get_count(data: &[u64]) -> usize {
        let encoded = data[DMA_ENCODED];
        if DmaInfo::get_dst_offset(encoded) == DmaInfo::get_src_offset(encoded) {
            0
        } else {
            let count = DmaInfo::get_loop_count(encoded);
            if count > 0 {
                count + 1
            } else {
                0
            }
        }
    }
    pub fn get_last_count(&self) -> usize {
        let rows = self.count as usize;
        let initial_count = self.get_initial_count();
        initial_count - rows + 1
    }
    pub fn get_initial_count(&self) -> usize {
        DmaInfo::get_count(self.encoded) - self.skip as usize
    }
    pub fn from(
        data: &[u64],
        data_ext: &[u64],
        trace_offset: usize,
        skip: usize,
        max_count: usize,
        is_last_instance_input: bool,
    ) -> Self {
        let encoded = data[DMA_ENCODED];

        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let data_offset = DmaInfo::get_loop_data_offset(encoded) + skip;

        // unaligned need an extra row to read part of next bytes
        let pending_count = DmaInfo::get_loop_count(encoded) + 1 - skip;
        let count = std::cmp::min(pending_count, max_count);
        if data_offset >= data_ext.len() || (data_offset + count) > data_ext.len() {
            println!(
                "PROBLEM ON INPUT GENERATION STEP:{} data_ext.len={} src_values[{data_offset}..{}] {}",
                data[STEP],
                data_ext.len(),
                data_offset + count,
                DmaInfo::to_string(encoded)
            );
        }
        // if count not enough to finish unaligned memcpy, add extra source because one row
        // use next source value
        let src_values_count = if count < pending_count { count + 1 } else { count };
        assert!(DmaInfo::get_loop_count(encoded) > 0);
        Self {
            dst: data[A] as u32 + pre_count,
            src: data[B] as u32 + DmaInfo::get_src64_inc_by_pre(encoded) as u32 * 8,
            trace_offset: trace_offset as u32,
            is_first_instance_input: trace_offset == 0,
            is_last_instance_input,
            step: data[STEP],
            skip: skip as u32,
            count: count as u32,
            encoded,
            src_values: data_ext[data_offset..data_offset + src_values_count].to_vec(),
        }
    }
    pub fn get_rows(&self) -> usize {
        DmaInfo::get_loop_count(self.encoded)
    }
}
