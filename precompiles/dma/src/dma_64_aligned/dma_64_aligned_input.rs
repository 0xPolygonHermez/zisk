use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, OP, STEP};
use zisk_core::zisk_ops::ZiskOp;

use crate::DMA_64_ALIGNED_OPS_BY_ROW;

#[derive(Debug)]
pub struct Dma64AlignedInput {
    pub src: u32,
    pub dst: u32,
    pub is_last_instance_input: bool,
    pub op: u8,
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
        ops_x_rows: usize,
        max_rows: usize,
    ) -> Self {
        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let skip_count = skip_rows * ops_x_rows;
        let data_offset = DmaInfo::get_loop_data_offset(encoded) + skip_count;
        let count = DmaInfo::get_loop_count(encoded) - skip_count;
        let total_rows = DmaInfo::get_loop_count(encoded).div_ceil(ops_x_rows);
        let rows = std::cmp::min(total_rows - skip_rows, max_rows) as u32;
        Self {
            dst: data[A] as u32 + pre_count,
            src: data[B] as u32 + pre_count,
            trace_offset: trace_offset as u32,
            is_last_instance_input: max_rows < (total_rows - skip_rows),
            step: data[STEP],
            skip_rows: skip_rows as u32,
            rows,
            encoded,
            src_values: data_ext[data_offset..data_offset + count].to_vec(),
            op: match op {
                ZiskOp::DMA_MEMCPY => {
                    if DmaInfo::is_direct(encoded) {
                        ZiskOp::DMA_MEMCPY
                    } else {
                        ZiskOp::DMA_XMEMCPY
                    }
                }
                _ => op,
            },
        }
    }
    pub fn from_memset(
        data: &[u64],
        trace_offset: usize,
        skip_rows: usize,
        ops_x_rows: usize,
        max_rows: usize,
    ) -> Self {
        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let total_rows = DmaInfo::get_loop_count(encoded).div_ceil(ops_x_rows);
        let rows = std::cmp::min(total_rows - skip_rows, max_rows) as u32;
        Self {
            dst: data[A] as u32 + pre_count,
            src: 0,
            trace_offset: trace_offset as u32,
            is_last_instance_input: max_rows < (total_rows - skip_rows),
            step: data[STEP],
            skip_rows: skip_rows as u32,
            rows,
            encoded,
            src_values: vec![],
            op,
        }
    }
    pub fn from_memcmp(
        data: &[u64],
        data_ext: &[u64],
        trace_offset: usize,
        skip_rows: usize,
        ops_x_rows: usize,
        max_rows: usize,
    ) -> Self {
        let dst = data[A] as u32;
        let src = data[B] as u32;
        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let skip_count = skip_rows * ops_x_rows;
        let data_offset = (pre_count as usize) + skip_count;
        let count = DmaInfo::get_loop_count(encoded) - skip_count;
        let total_rows = DmaInfo::get_loop_count(encoded).div_ceil(ops_x_rows);
        let rows = std::cmp::min(total_rows - skip_rows, max_rows) as u32;
        Self {
            dst: dst + pre_count,
            src: src + pre_count,
            trace_offset: trace_offset as u32,
            is_last_instance_input: max_rows < (total_rows - skip_rows),
            step: data[STEP],
            skip_rows: skip_rows as u32,
            rows,
            encoded,
            src_values: data_ext[data_offset..data_offset + count].to_vec(),
            op: match op {
                ZiskOp::DMA_MEMCPY => {
                    if DmaInfo::is_direct(encoded) {
                        ZiskOp::DMA_MEMCPY
                    } else {
                        ZiskOp::DMA_XMEMCPY
                    }
                }
                _ => op,
            },
        }
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of Dma64AlignedInput instances to a text file with columns separated by |.
    /// Path is taken from DEBUG_OUTPUT_PATH environment variable, defaulting to "tmp/".
    pub fn save_debug_info(filename: &str, inputs: &[Vec<Self>]) -> std::io::Result<()> {
        use std::io::Write;

        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        // Write header
        writeln!(
            file,
            "{:>8}|{:>10}|{:>10}|{:>22}|{:>4}|{:>12}|{:>9}|{:>8}|{:>14}|{:>18}|{:>9}|src_values",
            "pos",
            "src",
            "dst",
            "is_last_input",
            "op",
            "trace_offset",
            "skip_rows",
            "rows",
            "step",
            "encoded",
            "fill_byte"
        )?;

        // Write data rows
        for (pos, input) in inputs.iter().flatten().enumerate() {
            let src_values_hex: Vec<String> =
                input.src_values.iter().map(|v| format!("0x{:016X}", v)).collect();
            writeln!(
                file,
                "{:>8}|0x{:08X}|0x{:08X}|{:>22}|{:>4}|{:>12}|{:>9}|{:>8}|{:>14}|0x{:016X}|{:>9}|{}",
                pos,
                input.src,
                input.dst,
                input.is_last_instance_input,
                input.op,
                input.trace_offset,
                input.skip_rows,
                input.rows,
                input.step,
                input.encoded,
                input.fill_byte,
                src_values_hex.join(",")
            )?;
        }

        Ok(())
    }
}

impl crate::DmaInputPosition for Dma64AlignedInput {
    fn must_be_first(&self) -> bool {
        self.skip_rows > 0
    }

    fn must_be_last(&self) -> bool {
        self.is_last_instance_input
    }
}
