use crate::MemAlignResponse;
use std::fmt;
use zisk_core::ZiskRequiredMemory;

fn format_u64_hex(value: u64) -> String {
    let hex_str = format!("{:016x}", value);
    hex_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

impl fmt::Debug for MemAlignResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "more:{0} step:{1} value:{2:016X}({2:})",
            self.more_address,
            self.step,
            self.value.unwrap_or(0)
        )
    }
}

pub fn mem_align_call(
    mem_op: &ZiskRequiredMemory,
    mem_values: [u64; 2],
    phase: u8,
) -> MemAlignResponse {
    // DEBUG: only for testing
    let offset = (mem_op.address & 0x7) * 8;
    let width = (mem_op.width as u64) * 8;
    let double_address = (offset + width as u32) > 64;
    let mem_value = mem_values[phase as usize];
    let mask = 0xFFFF_FFFF_FFFF_FFFFu64 >> (64 - width);
    if mem_op.is_write {
        if phase == 0 {
            MemAlignResponse {
                more_address: double_address,
                step: mem_op.step + 1,
                value: Some(
                    (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 ^ (mask << offset)))
                        | ((mem_op.value & mask) << offset),
                ),
            }
        } else {
            MemAlignResponse {
                more_address: false,
                step: mem_op.step + 1,
                value: Some(
                    (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 << (offset + width as u32 - 64)))
                        | ((mem_op.value & mask) >> (128 - (offset + width as u32))),
                ),
            }
        }
    } else {
        MemAlignResponse {
            more_address: double_address && phase == 0,
            step: mem_op.step + 1,
            value: None,
        }
    }
}
