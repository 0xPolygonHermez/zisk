use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_MEMCMP_COUNT_BUS, STEP};
use zisk_core::zisk_ops::ZiskOp;

#[derive(Debug)]
pub struct DmaInput {
    pub src: u32,
    pub dst: u32,
    pub op: u8,
    pub encoded: u64,
    pub count_bus: u32,
    pub step: u64, // main step
}

impl DmaInput {
    pub fn from(encoded: u64, op: u8, data: &[u64], _data_ext: &[u64]) -> Self {
        Self {
            dst: data[A] as u32,
            src: data[B] as u32,
            step: data[STEP],
            encoded,
            op,
            count_bus: if op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP {
                data[DMA_MEMCMP_COUNT_BUS] as u32
            } else {
                0
            },
        }
    }
    pub fn from_memset(encoded: u64, op: u8, data: &[u64], _data_ext: &[u64]) -> Self {
        Self {
            dst: data[A] as u32,
            // src: (data[A] & 0x7) as u32,
            src: 0,
            step: data[STEP],
            encoded,
            op,
            count_bus: DmaInfo::get_count(encoded) as u32,
        }
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of DmaInput instances to a text file with columns separated by |.
    /// Path is taken from DEBUG_OUTPUT_PATH environment variable, defaulting to "tmp/".
    pub fn dump_to_file(inputs: &[Vec<Self>], filename: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        // Write header
        writeln!(
            file,
            "{:>8}|{:>10}|{:>10}|{:>2}|{:>18}|{:>8}|{:>14}|{}|{}|{:>10}",
            "pos", "src", "dst", "op", "encoded", "count_bus", "step", "pre", "post", "loop"
        )?;

        // Write data rows
        for (pos, input) in inputs.iter().flatten().enumerate() {
            writeln!(
                file,
                "{:>8}|0x{:08X}|0x{:08X}|{:02X}|0x{:016X}|{:>8}|{:>14}|{}|{}|{:>10}",
                pos,
                input.src,
                input.dst,
                input.op,
                input.encoded,
                input.count_bus,
                input.step,
                DmaInfo::get_pre_count(input.encoded),
                DmaInfo::get_post_count(input.encoded),
                DmaInfo::get_loop_count(input.encoded),
            )?;
        }

        Ok(())
    }
}
