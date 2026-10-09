//! What a `CompactBinary` instance is planned to prove: a slice of each of the four families.

use crate::{ChunkCollect, ADD_KINDS, EXT_KINDS};
use std::collections::HashMap;
use zisk_common::ChunkId;

/// The plan meta of a `CompactBinary` instance: what each block collects from each chunk, exactly
/// the `(count, skip)` per kind the standalone air of that block would have been given.
///
/// A block that was given nothing has an empty map and is all padding; the instance is planned
/// whenever any block has work.
#[derive(Debug, Default, Clone)]
pub struct CompactBinaryCollectInfo {
    /// The `basic_` block: a `Binary` air's share.
    pub basic: HashMap<ChunkId, ChunkCollect<ADD_KINDS>>,
    /// The `add_` block: a `BinaryAdd` air's share.
    pub add: HashMap<ChunkId, ChunkCollect<ADD_KINDS>>,
    /// The `add_hi_` block: a `BinaryAddHi` air's share.
    pub add_hi: HashMap<ChunkId, ChunkCollect<ADD_KINDS>>,
    /// The `ext_` block: a `BinaryExtension` air's share.
    pub ext: HashMap<ChunkId, ChunkCollect<EXT_KINDS>>,
}

impl CompactBinaryCollectInfo {
    /// The chunks the instance has to be fed: every chunk any of its blocks collects from, sorted
    /// and deduplicated, since the collect phase indexes an instance's collectors by the position
    /// of the chunk in this list.
    pub fn chunks(&self) -> Vec<ChunkId> {
        let mut chunks: Vec<ChunkId> = self
            .basic
            .keys()
            .chain(self.add.keys())
            .chain(self.add_hi.keys())
            .chain(self.ext.keys())
            .copied()
            .collect();
        chunks.sort_unstable();
        chunks.dedup();
        chunks
    }

    pub fn is_empty(&self) -> bool {
        self.basic.is_empty()
            && self.add.is_empty()
            && self.add_hi.is_empty()
            && self.ext.is_empty()
    }
}
