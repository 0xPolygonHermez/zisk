use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, OP, STEP};

#[derive(Debug)]
pub struct DmaPrePostInput {
    pub src: u32,
    pub dst: u32,
    pub step: u64,
    pub encoded: u64, // contains fill_byte/memcmp_result
    pub src_values: [u64; 2],
    pub dst_pre_value: u64,
    pub op: u8,
}
impl DmaPrePostInput {
    pub fn get_count(data: &[u64]) -> usize {
        let encoded = data[DMA_ENCODED];
        (DmaInfo::get_pre_count(encoded) > 0) as usize
            + (DmaInfo::get_post_count(encoded) > 0) as usize
    }
    pub fn from(data: &[u64], data_ext: &[u64], skip: u32, max_count: u32) -> Vec<Self> {
        let encoded = data[DMA_ENCODED];
        let op = data[OP] as u8;
        let mut inputs = Vec::new();
        let pre_count = DmaInfo::get_pre_count(encoded);
        let mut skipped = 0;

        if pre_count > 0 {
            if skipped < skip {
                skipped += 1;
            } else {
                let src_offset = DmaInfo::get_pre_data_offset(encoded);
                let input = Self {
                    dst: data[A] as u32,
                    src: data[B] as u32,
                    step: data[STEP],
                    encoded,
                    src_values: [
                        data_ext[src_offset],
                        if DmaInfo::is_double_read_pre(encoded) {
                            data_ext[src_offset + 1]
                        } else {
                            0
                        },
                    ],
                    op,
                    dst_pre_value: data_ext[DmaInfo::get_pre_write_offset(encoded)],
                };
                inputs.push(input);
            }
        }
        let post_count = DmaInfo::get_post_count(encoded);
        if post_count > 0 && skipped >= skip && max_count as usize > inputs.len() {
            let src_offset = DmaInfo::get_post_data_offset(encoded);
            let loop_count = DmaInfo::get_loop_count(encoded);
            let input = Self {
                dst: data[A] as u32 + pre_count as u32 + loop_count as u32 * 8,
                src: data[B] as u32 + pre_count as u32 + loop_count as u32 * 8,
                step: data[STEP],
                encoded,
                src_values: [
                    data_ext[src_offset],
                    if DmaInfo::is_double_read_post(encoded) {
                        data_ext[src_offset + 1]
                    } else {
                        0
                    },
                ],
                dst_pre_value: data_ext[DmaInfo::get_post_write_offset(encoded)],
                op,
            };
            inputs.push(input);
        }
        inputs
    }
    pub fn from_memset(data: &[u64], data_ext: &[u64], skip: u32, max_count: u32) -> Vec<Self> {
        let encoded = data[DMA_ENCODED];
        let op = data[OP] as u8;
        let mut inputs = Vec::new();
        let pre_count = DmaInfo::get_pre_count(encoded);
        let mut skipped = 0;
        if pre_count > 0 {
            if skipped < skip {
                skipped += 1;
            } else {
                inputs.push(Self {
                    dst: data[A] as u32,
                    src: 0,
                    step: data[STEP],
                    encoded,
                    src_values: [0, 0],
                    op,
                    dst_pre_value: data_ext[0],
                });
            }
        }
        let post_count = DmaInfo::get_post_count(encoded);
        if post_count > 0 && skipped >= skip && max_count as usize > inputs.len() {
            let loop_count = DmaInfo::get_loop_count(encoded);
            inputs.push(Self {
                dst: data[A] as u32 + pre_count as u32 + loop_count as u32 * 8,
                src: pre_count as u32 + loop_count as u32 * 8,
                step: data[STEP],
                encoded,
                src_values: [0, 0],
                // pre value words are at begging
                dst_pre_value: data_ext[(pre_count > 0) as usize],
                op,
            });
        }
        inputs
    }
    // memcmp has different format, because need to read dst and src, for this reason has a more
    // easy format, first all dst (a), and after all src (b)
    pub fn from_memcmp(data: &[u64], data_ext: &[u64], skip: u32, max_count: u32) -> Vec<Self> {
        let dst = data[A] as u32;
        let src = data[B] as u32;
        let encoded = data[DMA_ENCODED];
        let count = DmaInfo::get_count(encoded);
        let op = data[OP] as u8;
        let dst_words = (((dst + count as u32 + 7) >> 3) - (dst >> 3)) as usize;
        let src_words = (((src + count as u32 + 7) >> 3) - (src >> 3)) as usize;
        let mut inputs = Vec::new();
        let pre_count = DmaInfo::get_pre_count(encoded);
        let mut skipped = 0;
        if data[STEP] == 31841694 {
            println!(
                "DATA data:{data:?}  data_ext:{data_ext:?} S:{} PRE_COUNT:{pre_count} POST_COUNT:{} SKIP:{skip} MAX_COUNT:{max_count}",
                data[STEP],  DmaInfo::get_post_count(encoded)
            );
        }

        if pre_count > 0 {
            if skipped < skip {
                skipped += 1;
            } else {
                let input = Self {
                    dst,
                    src,
                    step: data[STEP],
                    encoded,
                    src_values: [
                        data_ext[dst_words],
                        if DmaInfo::is_double_read_pre(encoded) {
                            data_ext[dst_words + 1]
                        } else {
                            0
                        },
                    ],
                    op,
                    dst_pre_value: data_ext[0],
                };
                inputs.push(input);
            }
        }
        let post_count = DmaInfo::get_post_count(encoded);
        if post_count > 0 && skipped >= skip && max_count as usize > inputs.len() {
            // src_offset it's last src words
            let src_offset =
                dst_words + src_words - 1 - DmaInfo::is_double_read_post(encoded) as usize;
            let loop_count = DmaInfo::get_loop_count(encoded);
            let input = Self {
                dst: dst + pre_count as u32 + loop_count as u32 * 8,
                src: src + pre_count as u32 + loop_count as u32 * 8,
                step: data[STEP],
                encoded,
                src_values: [
                    data_ext[src_offset],
                    if DmaInfo::is_double_read_post(encoded) {
                        data_ext[src_offset + 1]
                    } else {
                        0
                    },
                ],
                dst_pre_value: data_ext[dst_words - 1],
                op,
            };
            inputs.push(input);
        }
        inputs
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of DmaPrePostInput instances to a text file with columns separated by |.
    /// Path is taken from DEBUG_OUTPUT_PATH environment variable, defaulting to "tmp/".
    pub fn dump_to_file(inputs: &[Vec<Self>], filename: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        // Write header
        writeln!(
            file,
            "{:>8}|{:>10}|{:>10}|{:>14}|{:>18}|{:>18}|{:>4}|src_values",
            "pos", "src", "dst", "step", "encoded", "dst_pre_value", "op"
        )?;

        // Write data rows
        for (pos, input) in inputs.iter().flatten().enumerate() {
            let src_values_hex: Vec<String> =
                input.src_values.iter().map(|v| format!("0x{:016X}", v)).collect();
            writeln!(
                file,
                "{:>8}|0x{:08X}|0x{:08X}|{:>14}|0x{:016X}|0x{:016X}|{:>4}|{}",
                pos,
                input.src,
                input.dst,
                input.step,
                input.encoded,
                input.dst_pre_value,
                input.op,
                src_values_hex.join(",")
            )?;
        }

        Ok(())
    }
}
