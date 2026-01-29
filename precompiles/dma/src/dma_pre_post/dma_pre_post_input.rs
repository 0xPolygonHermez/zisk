use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, STEP};

#[derive(Debug)]
pub struct DmaPrePostInput {
    pub src: u32,
    pub dst: u32,
    pub step: u64,
    pub encoded: u64,
    pub src_values: [u64; 2],
    pub dst_pre_value: u64,
}
impl std::fmt::Display for DmaPrePostInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DmaPrePostInput {{ src: 0x{:08x}, dst: 0x{:08x}, step: {}, encoded: 0x{:016x} ({}), src_values: [0x{:016x}, 0x{:016x}], dst_pre_value: 0x{:016x} }}",
            self.src, self.dst, self.step, self.encoded, DmaInfo::to_string(self.encoded), self.src_values[0], self.src_values[1], self.dst_pre_value
        )
    }
}
impl DmaPrePostInput {
    pub fn get_count(data: &[u64]) -> usize {
        let encoded = data[DMA_ENCODED];
        (DmaInfo::get_pre_count(encoded) > 0) as usize
            + (DmaInfo::get_post_count(encoded) > 0) as usize
    }
    pub fn from(data: &[u64], data_ext: &[u64], skip: u32, max_count: u32) -> Vec<Self> {
        let encoded = data[DMA_ENCODED];
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
            };
            inputs.push(input);
        }
        inputs
    }
}
