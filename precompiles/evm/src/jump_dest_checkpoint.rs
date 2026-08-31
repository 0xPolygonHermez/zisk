//! What the planner hands each `jump_dest` instance.
//!
//! The unit throughout is **rows**, not operations: one `jump_dest` spans
//! `ceil(count/64) * ROWS_X_BLOCK` rows, so a segment boundary can land in the
//! middle of one. `chunks` says, per chunk, how many rows this segment takes and
//! how many to skip first; `last_chunk` marks where the segment ends and
//! `is_last_segment` feeds the air value of the same name.

use std::collections::HashMap;

use zisk_common::{ChunkId, CollectSkipper};

#[derive(Default, Debug)]
pub struct JumpDestCheckPoint {
    /// Per chunk: rows this segment takes from it, and the skipper that walks
    /// past the rows belonging to earlier segments.
    pub chunks: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Last chunk of the segment, so its collector knows it closes the trace.
    pub last_chunk: Option<ChunkId>,

    /// Whether this is the final segment of the instance chain.
    pub is_last_segment: bool,
}
