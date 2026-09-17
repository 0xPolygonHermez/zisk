use crate::{MemInput, MemPreviousSegment};
use proofman_common::{AirInstance, ProofmanResult};
#[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;
use zisk_sm_mem_common::MemModuleSegmentCheckPoint;

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64) -> Self {
        MemInput { addr, is_write, step, value }
    }
}

/// An instance's operations as the collectors handed them over: one vector per chunk, walked as a
/// single sequence and **never concatenated**.
///
/// Concatenating them used to be the largest single cost of a `Mem` witness computation -- 65M
/// operations at 24 bytes is 1.5 GB copied, and `Iterator::flatten` gives `collect` no usable size
/// hint, so the destination was grown and recopied as it went. Nothing on the offsets path needs a
/// contiguous slice: it only ever walks the operations in order, placing each one straight into the
/// slot the offsets table names. So the flatten is simply not performed; `iter` chains the chunks
/// lazily instead.
#[derive(Clone, Copy)]
pub struct MemOps<'a> {
    chunks: &'a [Vec<MemInput>],
    len: usize,
}

impl<'a> MemOps<'a> {
    pub fn new(chunks: &'a [Vec<MemInput>]) -> Self {
        Self { chunks, len: chunks.iter().map(|c| c.len()).sum() }
    }

    /// Total operations across every chunk.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The operations in chunk order, which is the order they were collected in.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'a MemInput> + Clone {
        self.chunks.iter().flat_map(|c| c.iter())
    }

    /// One contiguous copy, for the paths that need random access. Only the legacy fill does --
    /// it reads `mem_ops[index - 1]` -- and it also sorts, so it needs ownership anyway. The
    /// offsets path never calls this, which is the whole point.
    pub fn to_flat_vec(self) -> Vec<MemInput> {
        let mut out = Vec::with_capacity(self.len);
        for chunk in self.chunks {
            out.extend(
                chunk.iter().map(|op| MemInput::new(op.addr, op.is_write, op.step, op.value)),
            );
        }
        out
    }
}

pub trait MemModule<F: Clone>: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn compute_witness(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_addr_range(&self) -> (u32, u32);
    fn is_dual(&self) -> bool;
    fn get_mem_name(&self) -> &str;
    fn is_initializable(&self) -> bool;
}

#[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
pub fn save_offsets_to_file(seg: &MemModuleSegmentCheckPoint, file_name: &str) {
    println!("[MemDebug] saving offsets to {} .....", file_name);
    let file = File::create(file_name).unwrap();
    let mut writer = BufWriter::new(file);
    let base = seg.offsets_base_addr as u64;
    for index in 0..seg.addr_range_slots {
        let addr = index as u64 * 8 + base;
        let value = seg.offset_at(index);
        writeln!(writer, "{} {:#010X} {}", index, addr, value).unwrap();
    }
    println!("[MemDebug] done");
}
