use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, OP, STEP};
use zisk_core::zisk_ops::ZiskOp;

#[derive(Debug)]
pub struct DmaUnalignedInput {
    pub src: u32,
    pub dst: u32,
    pub is_last_instance_input: bool,
    pub is_mem_eq: bool,
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
    ) -> Self {
        let encoded = data[DMA_ENCODED];
        let op = data[OP] as u8;
        debug_assert!(
            op == ZiskOp::DMA_MEMCPY
                || op == ZiskOp::DMA_XMEMCPY
                || op == ZiskOp::DMA_MEMCMP
                || op == ZiskOp::DMA_XMEMCMP,
            "Unexpected operation on DmaUnalignedInput 0x{op:02X}",
        );
        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let data_offset = DmaInfo::get_loop_data_offset(encoded) + skip;

        // unaligned need an extra row to read part of next bytes
        let pending_count = DmaInfo::get_loop_count(encoded) + 1 - skip;
        let count: usize = std::cmp::min(pending_count, max_count);

        // if count not enough to finish unaligned memcpy, add extra source because one row
        // use next source value
        let src_values_count = if count < pending_count { count + 1 } else { count };
        let op = data[OP] as u8;
        assert!(DmaInfo::get_loop_count(encoded) > 0);
        Self {
            dst: data[A] as u32 + pre_count,
            src: data[B] as u32 + DmaInfo::get_src64_inc_by_pre(encoded) as u32 * 8,
            trace_offset: trace_offset as u32,
            is_last_instance_input: max_count < pending_count,
            step: data[STEP],
            skip: skip as u32,
            count: count as u32,
            encoded,
            is_mem_eq: op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP,
            src_values: data_ext[data_offset..data_offset + src_values_count].to_vec(),
        }
    }
    pub fn get_rows(&self) -> usize {
        DmaInfo::get_loop_count(self.encoded)
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of DmaUnalignedInput instances to a text file with columns separated by |.
    /// Path is taken from DEBUG_OUTPUT_PATH environment variable, defaulting to "tmp/".
    pub fn dump_to_file(inputs: &[Vec<Self>], filename: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        // Write header
        writeln!(
            file,
            "{:>8}|{:>10}|{:>10}|{:>22}|{:>9}|{:>12}|{:>8}|{:>8}|{:>14}|{:>18}|src_values",
            "pos",
            "src",
            "dst",
            "is_last_instance_input",
            "is_mem_eq",
            "trace_offset",
            "skip",
            "count",
            "step",
            "encoded"
        )?;

        // Write data rows
        for (pos, input) in inputs.iter().flatten().enumerate() {
            let src_values_hex: Vec<String> =
                input.src_values.iter().map(|v| format!("0x{:016X}", v)).collect();
            writeln!(
                file,
                "{:>8}|0x{:08X}|0x{:08X}|{:>22}|{:>9}|{:>12}|{:>8}|{:>8}|{:>14}|0x{:016X}|{}",
                pos,
                input.src,
                input.dst,
                input.is_last_instance_input,
                input.is_mem_eq,
                input.trace_offset,
                input.skip,
                input.count,
                input.step,
                input.encoded,
                src_values_hex.join(",")
            )?;
        }

        Ok(())
    }
}

impl crate::DmaInputPosition for DmaUnalignedInput {
    fn must_be_first(&self) -> bool {
        self.skip > 0
    }
    fn must_be_last(&self) -> bool {
        self.is_last_instance_input
    }
}
