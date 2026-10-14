//! Input of one DMA operation routed to a `DmaCompact` instance.
//!
//! Unlike the other DMA machines, where one input produces one row (or one sequence of rows of a
//! single kind), a `DmaCompact` input carries EVERYTHING an operation needs, because the same
//! instance proves its controller row, its pre/post partial writes and its loop chain. The values
//! all come from `data_ext`, addressed with the offsets that the emulator encoded in `encoded`:
//!
//! ```text
//!   get_pre_write_offset  -> previous value of the destination word of the pre row
//!   get_post_write_offset -> previous value of the destination word of the post row
//!   get_pre_data_offset   -> source word(s) of the pre row
//!   get_loop_data_offset  -> source words of the loop (or the free input values of an inputcpy)
//!   get_post_data_offset  -> source word(s) of the post row
//! ```
//!
//! An operation is never split between instances: the planner only routes it here when its whole
//! row layout fits (see [`DmaCompactLayout`]). The AIR would allow splitting it (the loop chain
//! travels through the bus), but nothing needs it yet.

use precompiles_helpers::DmaInfo;
use zisk_common::{A, B, DMA_ENCODED, DMA_MEMCMP_COUNT_BUS, OP, STEP};
use zisk_core::zisk_ops::ZiskOp;

use crate::DmaCompactLayout;

#[derive(Debug, Clone)]
pub struct DmaCompactInput {
    /// Operation code as Main sent it (MEMCPY/XMEMCPY/MEMCMP/XMEMCMP/INPUTCPY/XMEMSET).
    pub op: u8,
    /// Destination address of the whole operation.
    pub dst: u32,
    /// Source address of the whole operation (0 when there is no source).
    pub src: u32,
    /// Main step: every memory access of the operation happens at this step.
    pub step: u64,
    /// Layout and results of the operation, as encoded by the emulator.
    pub encoded: u64,
    /// Count that travels on the operation bus. For a memcmp it is the requested length, which is
    /// larger than the compared one when a difference was found.
    pub count_bus: u32,
    /// Source words of the pre row: the word at `src & !7` and the next one when it is read. For
    /// an inputcpy it is the free input bytes in place, for a memset nothing.
    pub pre_src_values: [u64; 2],
    /// Previous value of the destination word written by the pre row.
    pub pre_dst_value: u64,
    /// Source words of the post row, as `pre_src_values`.
    pub post_src_values: [u64; 2],
    /// Previous value of the destination word written by the post row.
    pub post_dst_value: u64,
    /// Source words of the loop part, in order: `loop_words` of them, plus one more when the loop
    /// is unaligned (its read only tail row). For an inputcpy these are the free input values;
    /// for a memset it is empty (the value is the fill byte).
    pub loop_values: Vec<u64>,
}

impl DmaCompactInput {
    pub fn from(data: &[u64], data_ext: &[u64]) -> Self {
        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        let dst = data[A] as u32;

        let has_src = DmaCompactLayout::has_src(op);
        let src = if has_src { data[B] as u32 } else { 0 };

        let is_memcmp = op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP;
        let count_bus = if is_memcmp {
            data[DMA_MEMCMP_COUNT_BUS] as u32
        } else {
            DmaInfo::get_count(encoded) as u32
        };

        let pre_count = DmaInfo::get_pre_count(encoded);
        let post_count = DmaInfo::get_post_count(encoded);
        let layout = DmaCompactLayout::of(op, encoded);

        // A memset has no source words at all; an inputcpy has its free input bytes where a
        // memcpy has the words read from memory.
        let has_values = op != ZiskOp::DMA_XMEMSET;

        let (pre_src_values, pre_dst_value) = if pre_count > 0 {
            let offset = DmaInfo::get_pre_data_offset(encoded);
            (
                [
                    if has_values { data_ext[offset] } else { 0 },
                    if has_src && DmaInfo::is_double_read_pre(encoded) {
                        data_ext[offset + 1]
                    } else {
                        0
                    },
                ],
                data_ext[DmaInfo::get_pre_write_offset(encoded)],
            )
        } else {
            ([0, 0], 0)
        };

        let (post_src_values, post_dst_value) = if post_count > 0 {
            let offset = DmaInfo::get_post_data_offset(encoded);
            (
                [
                    if has_values { data_ext[offset] } else { 0 },
                    if has_src && DmaInfo::is_double_read_post(encoded) {
                        data_ext[offset + 1]
                    } else {
                        0
                    },
                ],
                data_ext[DmaInfo::get_post_write_offset(encoded)],
            )
        } else {
            ([0, 0], 0)
        };

        let loop_values = if !has_values || layout.loop_words == 0 {
            Vec::new()
        } else {
            let offset = DmaInfo::get_loop_data_offset(encoded);
            let words = layout.loop_words + usize::from(layout.unaligned());
            data_ext[offset..offset + words].to_vec()
        };

        Self {
            op,
            dst,
            src,
            step: data[STEP],
            encoded,
            count_bus,
            pre_src_values,
            pre_dst_value,
            post_src_values,
            post_dst_value,
            loop_values,
        }
    }

    /// Row layout of this operation.
    #[inline(always)]
    pub fn layout(&self) -> DmaCompactLayout {
        DmaCompactLayout::of(self.op, self.encoded)
    }

    /// Rows this operation occupies.
    #[inline(always)]
    pub fn rows(&self) -> usize {
        self.layout().rows()
    }

    #[inline(always)]
    pub fn is_memcmp(&self) -> bool {
        self.op == ZiskOp::DMA_MEMCMP || self.op == ZiskOp::DMA_XMEMCMP
    }

    /// The memcmp result as the two 32-bit chunks of its signed 64-bit value, and whether the
    /// result is negative and its absolute value (a byte difference).
    fn memcmp_result(&self) -> ([u64; 2], bool, u8) {
        let result = DmaInfo::get_memcmp_res_as_u64(self.encoded);
        let negative = DmaInfo::is_memcmp_negative(self.encoded);
        let abs_diff = if negative { (!result).wrapping_add(1) } else { result };
        debug_assert!(abs_diff <= 0xFF, "memcmp difference {abs_diff} is not a byte");
        ([result as u32 as u64, result >> 32], negative, abs_diff as u8)
    }

    /// Results of the pre and the post phase of a memcmp, as the controller row sends them. Zero
    /// when the operation is not a memcmp or the phase does not hold the result.
    pub fn phase_results(&self) -> ([u64; 2], [u64; 2]) {
        if !self.is_memcmp() {
            return ([0, 0], [0, 0]);
        }
        let (result, _, _) = self.memcmp_result();
        let pre = if DmaInfo::get_memcmp_pre_result_nz(self.encoded) { result } else { [0, 0] };
        let post = if DmaInfo::get_memcmp_post_result_nz(self.encoded) { result } else { [0, 0] };
        (pre, post)
    }

    /// What a memcmp pre/post row needs to know: `Some((nz, negative, abs_diff))` for a memcmp
    /// (with `nz` set only on the row that holds the first different byte), `None` otherwise.
    pub fn memcmp_row(&self, is_post: bool) -> Option<(bool, bool, u8)> {
        if !self.is_memcmp() {
            return None;
        }
        let nz = if is_post {
            DmaInfo::get_memcmp_post_result_nz(self.encoded)
        } else {
            DmaInfo::get_memcmp_pre_result_nz(self.encoded)
        };
        if nz {
            let (_, negative, abs_diff) = self.memcmp_result();
            Some((true, negative, abs_diff))
        } else {
            Some((false, false, 0))
        }
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of DmaCompactInput instances to a text file with columns separated by |.
    /// Path is taken from DEBUG_OUTPUT_PATH environment variable, defaulting to "tmp/".
    pub fn dump_to_file(inputs: &[Vec<Self>], filename: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        writeln!(
            file,
            "{:>8}|{:>4}|{:>10}|{:>10}|{:>14}|{:>18}|{:>10}|{:>5}|loop_values",
            "pos", "op", "dst", "src", "step", "encoded", "count_bus", "rows"
        )?;

        for (pos, input) in inputs.iter().flatten().enumerate() {
            let values: Vec<String> =
                input.loop_values.iter().map(|v| format!("0x{:016X}", v)).collect();
            writeln!(
                file,
                "{:>8}|0x{:02X}|0x{:08X}|0x{:08X}|{:>14}|0x{:016X}|{:>10}|{:>5}|{}",
                pos,
                input.op,
                input.dst,
                input.src,
                input.step,
                input.encoded,
                input.count_bus,
                input.rows(),
                values.join(",")
            )?;
        }

        Ok(())
    }
}
