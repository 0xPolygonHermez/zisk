//! Streaming ingestion of operation-trace binaries and bounded per-op aggregation.
//!
//! Each trace file is a flat sequence of 17-byte records produced by `ziskemu --store-op-output`:
//!   byte 0      : opcode (u8)
//!   bytes 1..9  : operand a (u64, little-endian)
//!   bytes 9..17 : operand b (u64, little-endian)
//!
//! We never hold the full record stream in memory. Instead, for every FROPS-candidate opcode we keep
//! bounded histograms over three disjoint `a` regions (low / mid / high), which is enough to propose
//! the box-shaped predicates the optimizer searches.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::ops::{classify, OpInfo};

pub const RECORD_SIZE: usize = 17;

/// `a` values `>= HIGH_FROM` are treated as a "high" cluster (near u64::MAX, e.g. negative / mask
/// patterns). The window is bounded so the resulting boxes stay enumerable.
pub const HIGH_WINDOW: u64 = 1 << 20;
pub const HIGH_FROM: u64 = u64::MAX - HIGH_WINDOW + 1;

/// In the high cluster only small `b` values are tracked (shift amounts, small constants).
pub const B_HIGH_CAP: u64 = 4096;
/// In the mid cluster (addresses) only small `b` values are tracked.
pub const B_MID_CAP: u64 = 4096;
/// Mid-region addresses are bucketed by page so contiguous ranges can be detected cheaply.
pub const PAGE_SHIFT: u32 = 12;

// Memory guards: once a per-op map reaches its cap, new keys spill into `other` and the op is flagged
// as truncated (reported, never silently dropped).
const LOW_CAP_ENTRIES: usize = 1 << 22; // up to ~4M distinct low pairs per op
const HIGH_CAP_ENTRIES: usize = 1 << 21;
const MID_CAP_ENTRIES: usize = 1 << 20;
const MID_PB_CAP_ENTRIES: usize = 1 << 21;

/// Bounded statistics for a single opcode.
#[derive(Debug, Default)]
pub struct OpAgg {
    pub total: u64,
    /// Low region: key = a * low_cap + b, for a, b < low_cap.
    pub low: HashMap<u64, u64>,
    pub low_max_a: u64,
    pub low_max_b: u64,
    /// High region: (a, b) for a >= HIGH_FROM and b < B_HIGH_CAP.
    pub high: HashMap<(u64, u64), u64>,
    pub high_min_a: u64,
    pub high_max_b: u64,
    /// Mid region: page (a >> PAGE_SHIFT) -> stats for low_cap <= a < HIGH_FROM.
    pub mid: HashMap<u64, MidPage>,
    /// Joint (page, b) -> count for the mid region, used to split a box into the actual `b` clusters
    /// instead of one wide `[0, max_b]` range. Bounded; spilled pairs only lose `b` resolution (the
    /// page total in `mid` still counts them), so coverage estimates stay conservative.
    pub mid_pb: HashMap<(u64, u64), u64>,
    /// Occurrences that fell outside every tracked bucket (large `b`, spilled keys, etc.).
    pub other: u64,
    pub truncated: bool,
    /// Occurrences the *current* FROPS implementation would cover (for comparison).
    pub current_covered: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MidPage {
    pub count: u64,
    pub max_b: u64,
    /// Histogram of `a & 7` over this page, so strided (mod 4/8) candidate boxes can be evaluated
    /// even when the page mixes alignments (not only uniformly-aligned clusters).
    pub a_rem: [u64; 8],
}

impl OpAgg {
    fn record(&mut self, info: &OpInfo, a: u64, b: u64, low_cap: u64) {
        self.total += 1;
        if crate::current::is_frequent(info.table, info.code, a, b) {
            self.current_covered += 1;
        }

        if a < low_cap && b < low_cap {
            let key = a * low_cap + b;
            if insert_bounded(&mut self.low, key, LOW_CAP_ENTRIES) {
                self.low_max_a = self.low_max_a.max(a);
                self.low_max_b = self.low_max_b.max(b);
            } else {
                self.other += 1;
                self.truncated = true;
            }
        } else if a >= HIGH_FROM && b < B_HIGH_CAP {
            if insert_bounded(&mut self.high, (a, b), HIGH_CAP_ENTRIES) {
                self.high_min_a = if self.high_min_a == 0 { a } else { self.high_min_a.min(a) };
                self.high_max_b = self.high_max_b.max(b);
            } else {
                self.other += 1;
                self.truncated = true;
            }
        } else if b < B_MID_CAP && a >= low_cap && a < HIGH_FROM {
            let page = a >> PAGE_SHIFT;
            if self.mid.contains_key(&page) || self.mid.len() < MID_CAP_ENTRIES {
                let e = self.mid.entry(page).or_default();
                e.count += 1;
                e.max_b = e.max_b.max(b);
                e.a_rem[(a & 7) as usize] += 1;
                // Joint (page, b) for b-splitting (bounded; spills only lose b resolution).
                if self.mid_pb.contains_key(&(page, b)) || self.mid_pb.len() < MID_PB_CAP_ENTRIES {
                    *self.mid_pb.entry((page, b)).or_default() += 1;
                }
            } else {
                self.other += 1;
                self.truncated = true;
            }
        } else {
            self.other += 1;
        }
    }
}

/// Inserts/increments `key`. Returns false (without inserting) if the map is at `cap` and the key is
/// new, so the caller can account the occurrence elsewhere.
fn insert_bounded<K: std::hash::Hash + Eq>(map: &mut HashMap<K, u64>, key: K, cap: usize) -> bool {
    if let Some(v) = map.get_mut(&key) {
        *v += 1;
        true
    } else if map.len() < cap {
        map.insert(key, 1);
        true
    } else {
        false
    }
}

/// Aggregated statistics over a whole directory of trace files.
pub struct Aggregator {
    pub low_cap: u64,
    pub ops: HashMap<u8, OpAgg>,
    pub records: u64,
    pub skipped_non_frops: u64,
    pub files: usize,
    pub trailing_bytes: u64,
}

impl Aggregator {
    pub fn new(low_cap: u64) -> Self {
        Self {
            low_cap,
            ops: HashMap::new(),
            records: 0,
            skipped_non_frops: 0,
            files: 0,
            trailing_bytes: 0,
        }
    }

    /// Reads every `*.bin` file in `dir` (non-recursive) and folds it into the aggregation.
    pub fn ingest_dir(&mut self, dir: &Path) -> std::io::Result<()> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().map(|e| e == "bin").unwrap_or(false))
            .collect();
        paths.sort();
        if paths.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no .bin files found in {}", dir.display()),
            ));
        }
        for path in paths {
            self.ingest_file(&path)?;
        }
        Ok(())
    }

    pub fn ingest_file(&mut self, path: &Path) -> std::io::Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(1 << 20, file);
        let mut buf = [0u8; RECORD_SIZE * 4096];
        let mut carry = 0usize; // bytes of a partial record carried at the front of buf
        loop {
            let n = reader.read(&mut buf[carry..])?;
            if n == 0 {
                break;
            }
            let available = carry + n;
            let full = available / RECORD_SIZE;
            for i in 0..full {
                let off = i * RECORD_SIZE;
                let op = buf[off];
                let a = u64::from_le_bytes(buf[off + 1..off + 9].try_into().unwrap());
                let b = u64::from_le_bytes(buf[off + 9..off + 17].try_into().unwrap());
                self.record(op, a, b);
            }
            let consumed = full * RECORD_SIZE;
            carry = available - consumed;
            buf.copy_within(consumed..available, 0);
        }
        self.trailing_bytes += carry as u64;
        self.files += 1;
        Ok(())
    }

    #[inline]
    fn record(&mut self, op: u8, a: u64, b: u64) {
        self.records += 1;
        match classify(op) {
            Some(info) => {
                let low_cap = self.low_cap;
                let agg = self.ops.entry(op).or_default();
                agg.record(&info, a, b, low_cap);
            }
            None => self.skipped_non_frops += 1,
        }
    }
}
