//! Minimal "chain segment" guest for the recurser-aggregator end-to-end test.
//!
//! Reads two u32s `(old, new)` from stdin and commits them as its two public
//! values: `publics[0] = old`, `publics[1] = new`. A proof therefore attests a
//! single transition `old -> new`.
//!
//! Folding two such proofs with an `AggregatePublics` that enforces
//! `a.new == b.old` and emits `[a.old, b.new]` stitches contiguous segments
//! into one — `[10,20] + [20,30]` collapses to `[10,30]`. The `chain_simple`
//! example keeps slots `[2..64)` zero; the richer `chain` example fills
//! `[2..6)` with a NormalizePublics digest and zeroes the rest (see
//! `programs/aggregations/chain.toml` / `chain_simple.toml`).
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let old: u32 = ziskos::io::read();
    let new: u32 = ziskos::io::read();
    ziskos::io::commit_slice(&old.to_le_bytes());
    ziskos::io::commit_slice(&new.to_le_bytes());
}
