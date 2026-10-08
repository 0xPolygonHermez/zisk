//! Precompile inputs logged by the minimal-trace emulation.

use std::collections::HashMap;

use zisk_core::ZiskOperationType;

/// One precompile's records in step order, each its operation's bus payload. Indices count every
/// record of the run; records before a chunk boundary can be dropped.
#[derive(Default)]
pub struct PrecompileLog {
    /// Index of the first record kept.
    first: usize,
    /// First step kept, a chunk's.
    first_step: u64,
    /// Start of each kept record in `words`.
    starts: Vec<usize>,
    words: Vec<u64>,
}

impl PrecompileLog {
    /// Appends one record.
    pub fn push(&mut self, payload: &[u64]) {
        assert!(payload.len() > crate::STEP);
        self.starts.push(self.words.len());
        self.words.extend_from_slice(payload);
    }

    /// Records logged, kept or dropped.
    pub fn len(&self) -> usize {
        self.first + self.starts.len()
    }

    /// Whether no record was logged.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The record at `index`, which must be kept.
    pub fn record(&self, index: usize) -> &[u64] {
        let local = index - self.first;
        let end = self.starts.get(local + 1).copied().unwrap_or(self.words.len());
        &self.words[self.starts[local]..end]
    }

    /// The step of the record at `index`.
    pub fn step(&self, index: usize) -> u64 {
        self.record(index)[crate::STEP]
    }

    /// The first record at or after `step`; `None` if dropped.
    pub fn first_at_or_after(&self, step: u64) -> Option<usize> {
        if step < self.first_step {
            return None;
        }
        let (mut lo, mut hi) = (self.first, self.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.step(mid) < step {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Some(lo)
    }

    /// Drops the records before `step`, a chunk's first.
    pub fn drop_before(&mut self, step: u64) {
        let Some(keep) = self.first_at_or_after(step) else { return };
        let local = keep - self.first;
        let offset = self.starts.get(local).copied().unwrap_or(self.words.len());
        self.words.drain(..offset);
        self.starts.drain(..local);
        self.starts.iter_mut().for_each(|start| *start -= offset);
        self.first = keep;
        self.first_step = step;
    }
}

/// The logs of one run, by operation type.
#[derive(Default)]
pub struct PrecompileLogs {
    logs: HashMap<u64, PrecompileLog>,
}

impl PrecompileLogs {
    /// Appends a bus payload to its operation type's log.
    pub fn push(&mut self, payload: &[u64]) {
        self.logs.entry(payload[crate::OP_TYPE]).or_default().push(payload);
    }

    /// The log of `op_type`, mutable.
    pub fn get_mut(&mut self, op_type: ZiskOperationType) -> Option<&mut PrecompileLog> {
        self.logs.get_mut(&(op_type as u64))
    }

    /// The log of `op_type`.
    pub fn get(&self, op_type: ZiskOperationType) -> Option<&PrecompileLog> {
        self.logs.get(&(op_type as u64))
    }
}

/// Every `capacity` records of `op_type` make one instance of the air.
#[derive(Clone, Copy, Debug)]
pub struct LogCut {
    /// Operation type.
    pub op_type: ZiskOperationType,
    /// Air group.
    pub airgroup_id: usize,
    /// Air.
    pub air_id: usize,
    /// Operations per instance.
    pub capacity: u64,
}
