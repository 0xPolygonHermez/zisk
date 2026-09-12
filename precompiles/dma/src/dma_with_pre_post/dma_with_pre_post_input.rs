use zisk_common::{A, B, DMA_ENCODED, DMA_MEMCMP_COUNT_BUS, OP, STEP};
use zisk_core::zisk_ops::ZiskOp;
use zisk_precomp_helpers::DmaInfo;

/// One whole DMA operation, with everything the fused air needs for its DMA row *and* for its
/// PRE/POST rows.
///
/// `Dma` and `DmaPrePost` take one input per row, so the planner can cut an operation in half and
/// let two instances share it. `DmaWithPrePost` cannot: the extra PRE row has to sit right after
/// the DMA row of the same operation, so the unit of collection is the operation, never the row.
/// That is also why the values of both sub-operations travel together in a single input.
#[derive(Debug)]
pub struct DmaWithPrePostInput {
    /// Destination address of the operation (byte address, as sent to the main bus).
    pub dst: u32,

    /// Source address for memcpy/memcmp; the count for inputcpy; zero for memset — the same
    /// convention `DmaInput` uses, because the DMA columns are shared.
    pub src: u32,

    /// Main step of the operation.
    pub step: u64,

    /// Packed operation info (see `DmaInfo`): counts, offsets, fill byte / memcmp result.
    pub encoded: u64,

    /// Count as read from memory, only meaningful for memcmp (where it differs from the effective
    /// count by `count_diff`) and for memset.
    pub count_bus: u32,

    /// Zisk opcode.
    pub op: u8,

    /// The 64-bit src words the PRE sub-operation reads. The second one is only read when
    /// `DmaInfo::is_double_read_pre`.
    pub pre_src_values: [u64; 2],

    /// The 64-bit src words the POST sub-operation reads.
    pub post_src_values: [u64; 2],

    /// The dst 64-bit word as it was before the PRE write.
    pub pre_dst_value: u64,

    /// The dst 64-bit word as it was before the POST write.
    pub post_dst_value: u64,
}

impl DmaWithPrePostInput {
    /// Rows the operation takes in the fused air: the DMA row, plus the extra PRE row when the
    /// operation needs both a PRE and a POST (the POST always rides on the DMA row).
    pub const SINGLE_ROW: usize = 1;
    pub const DOUBLE_ROW: usize = 2;

    /// `true` when the operation needs both sub-operations and therefore two rows.
    #[inline(always)]
    pub const fn is_double(encoded: u64) -> bool {
        DmaInfo::get_pre_count(encoded) > 0 && DmaInfo::get_post_count(encoded) > 0
    }

    /// Rows this operation takes.
    #[inline(always)]
    pub const fn rows(&self) -> usize {
        if Self::is_double(self.encoded) {
            Self::DOUBLE_ROW
        } else {
            Self::SINGLE_ROW
        }
    }

    /// Builds the input of a memcpy, memcmp or inputcpy operation.
    ///
    /// `data_ext` holds, in this order, the dst words read before the writes (`pre_writes` of
    /// them), then the src words. The `DmaInfo::get_*_offset` helpers locate each piece.
    pub fn from(data: &[u64], data_ext: &[u64]) -> Self {
        let encoded = data[DMA_ENCODED];
        let op = data[OP] as u8;

        let pre_src_values = if DmaInfo::get_pre_count(encoded) > 0 {
            let offset = DmaInfo::get_pre_data_offset(encoded);
            [
                data_ext[offset],
                if DmaInfo::is_double_read_pre(encoded) { data_ext[offset + 1] } else { 0 },
            ]
        } else {
            [0, 0]
        };

        let post_src_values = if DmaInfo::get_post_count(encoded) > 0 {
            let offset = DmaInfo::get_post_data_offset(encoded);
            [
                data_ext[offset],
                if DmaInfo::is_double_read_post(encoded) { data_ext[offset + 1] } else { 0 },
            ]
        } else {
            [0, 0]
        };

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
            pre_src_values,
            post_src_values,
            pre_dst_value: if DmaInfo::get_pre_count(encoded) > 0 {
                data_ext[DmaInfo::get_pre_write_offset(encoded)]
            } else {
                0
            },
            post_dst_value: if DmaInfo::get_post_count(encoded) > 0 {
                data_ext[DmaInfo::get_post_write_offset(encoded)]
            } else {
                0
            },
        }
    }

    /// Builds the input of a memset operation. There is no source, so `data_ext` only holds the
    /// dst words read before the writes: the PRE one first, the POST one after it.
    pub fn from_memset(data: &[u64], data_ext: &[u64]) -> Self {
        let encoded = data[DMA_ENCODED];
        let pre_count = DmaInfo::get_pre_count(encoded);
        Self {
            dst: data[A] as u32,
            src: 0,
            step: data[STEP],
            encoded,
            op: data[OP] as u8,
            count_bus: DmaInfo::get_count(encoded) as u32,
            pre_src_values: [0, 0],
            post_src_values: [0, 0],
            pre_dst_value: if pre_count > 0 { data_ext[0] } else { 0 },
            post_dst_value: if DmaInfo::get_post_count(encoded) > 0 {
                data_ext[(pre_count > 0) as usize]
            } else {
                0
            },
        }
    }

    /// Builds the input of any DMA operation, picking the `data_ext` layout by opcode.
    #[inline(always)]
    pub fn from_op(data: &[u64], data_ext: &[u64]) -> Self {
        match data[OP] as u8 {
            ZiskOp::DMA_XMEMSET => Self::from_memset(data, data_ext),
            ZiskOp::DMA_MEMCPY
            | ZiskOp::DMA_XMEMCPY
            | ZiskOp::DMA_MEMCMP
            | ZiskOp::DMA_XMEMCMP
            | ZiskOp::DMA_INPUTCPY => Self::from(data, data_ext),
            op => panic!("Invalid DMA operation 0x{op:02X}"),
        }
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of `DmaWithPrePostInput` instances to a text file with columns separated by |.
    /// Path is taken from DEBUG_OUTPUT_PATH environment variable, defaulting to "tmp/".
    pub fn dump_to_file(inputs: &[Vec<Self>], filename: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        writeln!(
            file,
            "{:>8}|{:>10}|{:>10}|{:>2}|{:>18}|{:>8}|{:>14}|pre|post|{:>10}|rows",
            "pos", "src", "dst", "op", "encoded", "count_bus", "step", "loop"
        )?;

        for (pos, input) in inputs.iter().flatten().enumerate() {
            writeln!(
                file,
                "{:>8}|0x{:08X}|0x{:08X}|{:02X}|0x{:016X}|{:>8}|{:>14}|{}|{}|{:>10}|{}",
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
                input.rows(),
            )?;
        }

        Ok(())
    }
}
