//! What one `DmaLoop` row sequence proves, and how an operation is classified for it.
//!
//! `DmaLoop` proves the loop phase of every kind of operation, aligned or not (see
//! `precompiles/dma/pil/dma_loop.pil`). The planner can nevertheless route the aligned and the
//! unaligned loops of the same opcode to different airs -- an aligned memcpy to
//! `Dma64AlignedMemCpy` and an unaligned one to `DmaLoop`, say -- so the unit of routing is not the
//! opcode but the CLASS below, and both the planner and the collector read it out of the encoding
//! with [`DmaLoopInput::class_of`].

use zisk_common::{A, B, DMA_ENCODED, OP, STEP};
use zisk_core::zisk_ops::ZiskOp;
use zisk_precomp_helpers::DmaInfo;

use crate::{
    DMA_COUNTER_INPUTCPY, DMA_COUNTER_MEMCMP, DMA_COUNTER_MEMCPY, DMA_COUNTER_MEMSET,
    DMA_LOOP_OPS_BY_ROW,
};

/// Aligned memcpy (delegated by the controller, or direct from Main).
pub const DMA_LOOP_CLASS_MEMCPY: usize = DMA_COUNTER_MEMCPY;
/// memset, always aligned: it has no source.
pub const DMA_LOOP_CLASS_MEMSET: usize = DMA_COUNTER_MEMSET;
/// Aligned memcmp.
pub const DMA_LOOP_CLASS_MEMCMP: usize = DMA_COUNTER_MEMCMP;
/// inputcpy, always aligned: it has no source in memory.
pub const DMA_LOOP_CLASS_INPUTCPY: usize = DMA_COUNTER_INPUTCPY;
/// memcpy whose source and destination offsets differ.
pub const DMA_LOOP_CLASS_MEMCPY_UNALIGNED: usize = 4;
/// memcmp whose source and destination offsets differ.
pub const DMA_LOOP_CLASS_MEMCMP_UNALIGNED: usize = 5;
/// Number of classes.
pub const DMA_LOOP_CLASSES: usize = 6;

/// The part of one operation's loop that one `DmaLoop` instance proves: `rows` rows starting
/// `skip` rows into the sequence.
#[derive(Debug, Clone)]
pub struct DmaLoopInput {
    /// Opcode as Main sent it.
    pub op: u8,
    /// Packed operation info (see `DmaInfo`).
    pub encoded: u64,
    /// Main step: every memory access of the operation happens at it.
    pub step: u64,
    /// Destination 64-bit word of the FIRST word of the sequence (not of this input).
    pub dst64: u32,
    /// Source 64-bit word of the first word of the sequence; 0 for an operation without a source,
    /// whose @[src64] just runs along unused.
    pub src64: u32,
    /// Byte offset of the source against the destination, 0 for an aligned sequence.
    pub offset: u8,
    /// memset fill byte, 0 for every other operation.
    pub fill_byte: u8,
    /// Rows of the sequence proved by previous instances.
    pub skip: u32,
    /// Rows this input takes.
    pub rows: u32,
    /// The sequence goes on in the next instance: this input ends the one it is in.
    pub is_last_instance_input: bool,
    /// The words this input's slots read, plus the one after them when the sequence goes on (its
    /// bytes are what the last row hands over to the next instance). Empty for a memset, whose
    /// "reads" are the fill byte.
    pub values: Vec<u64>,
}

impl DmaInputPosition for DmaLoopInput {
    fn must_be_first(&self) -> bool {
        self.skip > 0
    }
    fn must_be_last(&self) -> bool {
        self.is_last_instance_input
    }
}

use crate::DmaInputPosition;

impl DmaLoopInput {
    /// The class the loop of this operation belongs to, or `None` when it has no loop at all.
    #[inline(always)]
    pub fn class_of(op: u8, encoded: u64) -> Option<usize> {
        if DmaInfo::get_loop_count(encoded) == 0 {
            return None;
        }
        let aligned = DmaInfo::dst_is_aligned_with_src(encoded);
        match op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
                Some(if aligned { DMA_LOOP_CLASS_MEMCPY } else { DMA_LOOP_CLASS_MEMCPY_UNALIGNED })
            }
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                Some(if aligned { DMA_LOOP_CLASS_MEMCMP } else { DMA_LOOP_CLASS_MEMCMP_UNALIGNED })
            }
            ZiskOp::DMA_XMEMSET => Some(DMA_LOOP_CLASS_MEMSET),
            ZiskOp::DMA_INPUTCPY => Some(DMA_LOOP_CLASS_INPUTCPY),
            _ => None,
        }
    }

    /// Whether the class reads its words one offset apart from where it writes them.
    #[inline(always)]
    pub const fn is_unaligned_class(class: usize) -> bool {
        class == DMA_LOOP_CLASS_MEMCPY_UNALIGNED || class == DMA_LOOP_CLASS_MEMCMP_UNALIGNED
    }

    /// Slots (64-bit reads) the whole sequence takes: one per word, plus, when unaligned, the
    /// extra read whose bytes the last write borrows. It is the `count + 1 - offset_0` the air
    /// closes the sequence on.
    #[inline(always)]
    pub fn total_slots(class: usize, encoded: u64) -> usize {
        DmaInfo::get_loop_count(encoded) + Self::is_unaligned_class(class) as usize
    }

    /// Rows the whole sequence takes. A sequence never shares a row with another, so its slots
    /// round up on their own -- which is also how `DmaCounterInputGen` budgets them.
    #[inline(always)]
    pub fn total_rows(class: usize, encoded: u64) -> usize {
        Self::total_slots(class, encoded).div_ceil(DMA_LOOP_OPS_BY_ROW)
    }

    /// Rows the operation on the bus takes, or 0 when it has no loop.
    pub fn rows_of(data: &[u64]) -> usize {
        let encoded = data[DMA_ENCODED];
        Self::class_of(data[OP] as u8, encoded).map_or(0, |class| Self::total_rows(class, encoded))
    }

    /// Builds the input of `max_rows` rows at most, starting `skip` rows into the sequence.
    pub fn from(data: &[u64], data_ext: &[u64], skip: usize, max_rows: usize) -> Self {
        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        let class = Self::class_of(op, encoded).expect("DmaLoopInput of an operation with no loop");
        debug_assert!(
            !(DmaInfo::is_direct(encoded)
                && matches!(op, ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP)),
            "a memcmp is never direct: its result needs the controller"
        );

        let total_slots = Self::total_slots(class, encoded);
        let skip_slots = skip * DMA_LOOP_OPS_BY_ROW;
        let pending_slots = total_slots - skip_slots;
        let rows = pending_slots.div_ceil(DMA_LOOP_OPS_BY_ROW).min(max_rows);
        let slots = pending_slots.min(rows * DMA_LOOP_OPS_BY_ROW);
        let continues = slots < pending_slots;

        let has_src = matches!(
            op,
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY | ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP
        );

        // The PRE writes the bytes that align the destination, so the loop starts at `dst + pre`,
        // which is word aligned, and it reads from `src + pre`, whose word is `src64` plus the one
        // the PRE may have stepped into (`src64_inc_by_pre`). A direct operation has no PRE.
        let pre_count = DmaInfo::get_pre_count(encoded) as u32;
        let dst = data[A] as u32 + pre_count;
        debug_assert_eq!(dst & 0x07, 0, "the loop of 0x{op:02X} does not start word aligned");
        let src64 = if has_src { (data[B] as u32 + pre_count) >> 3 } else { 0 };

        let offset =
            if Self::is_unaligned_class(class) { DmaInfo::get_loop_src_offset(encoded) } else { 0 };

        let values = if op == ZiskOp::DMA_XMEMSET {
            Vec::new()
        } else {
            let from = DmaInfo::get_loop_data_offset(encoded) + skip_slots;
            data_ext[from..from + slots + continues as usize].to_vec()
        };

        Self {
            op,
            encoded,
            step: data[STEP],
            dst64: dst >> 3,
            src64,
            offset,
            fill_byte: if op == ZiskOp::DMA_XMEMSET { DmaInfo::get_fill_byte(encoded) } else { 0 },
            skip: skip as u32,
            rows: rows as u32,
            is_last_instance_input: continues,
            values,
        }
    }

    /// The class of this input's loop.
    #[inline(always)]
    pub fn class(&self) -> usize {
        Self::class_of(self.op, self.encoded).expect("a DmaLoopInput always has a loop")
    }

    /// Slots this input covers: whole rows, except on the last row of the sequence, which stops
    /// at the slot the sequence ends on.
    #[inline(always)]
    pub fn slots(&self) -> usize {
        let pending = Self::total_slots(self.class(), self.encoded)
            - self.skip as usize * DMA_LOOP_OPS_BY_ROW;
        pending.min(self.rows as usize * DMA_LOOP_OPS_BY_ROW)
    }

    /// The 64-bit word the slot `slot` of this input reads: the fill word for a memset.
    #[inline(always)]
    pub fn value(&self, slot: usize) -> u64 {
        if self.op == ZiskOp::DMA_XMEMSET {
            u64::from_le_bytes([self.fill_byte; 8])
        } else {
            self.values[slot]
        }
    }

    /// Whether this is a memcpy proved without the controller, which reads its count from memory.
    #[inline(always)]
    pub fn loads_count(&self) -> bool {
        self.op == ZiskOp::DMA_MEMCPY && DmaInfo::is_direct(self.encoded)
    }

    #[cfg(feature = "save_dma_inputs")]
    /// Writes a list of `DmaLoopInput` to a text file with columns separated by |. The path is
    /// taken from the DEBUG_OUTPUT_PATH environment variable, `tmp/` by default.
    pub fn dump_to_file(inputs: &[Vec<Self>], filename: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let mut file = std::fs::File::create(format!("{path}{filename}"))?;
        writeln!(file, "pos|op|dst64|src64|offset|skip|rows|last|step|encoded|values")?;
        for (pos, input) in inputs.iter().flatten().enumerate() {
            let values: Vec<String> = input.values.iter().map(|v| format!("0x{v:016X}")).collect();
            writeln!(
                file,
                "{pos}|{:02X}|0x{:08X}|0x{:08X}|{}|{}|{}|{}|{}|0x{:016X}|{}",
                input.op,
                input.dst64,
                input.src64,
                input.offset,
                input.skip,
                input.rows,
                input.is_last_instance_input,
                input.step,
                input.encoded,
                values.join(",")
            )?;
        }
        Ok(())
    }
}
