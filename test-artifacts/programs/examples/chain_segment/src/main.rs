//! Minimal "chain segment" guest for the recurser-aggregator end-to-end test.
//!
//! Reads two u32s `(old, new)` from stdin and commits them as its two public
//! values: `publics[0] = old`, `publics[1] = new`. A proof therefore attests a
//! single transition `old -> new`.
//!
//! Folding two such proofs with the `chain` example's `AggregatePublics`
//! (which enforces `a.new == b.old` and emits `[a.old, b.new]`) stitches
//! contiguous segments into one — `[10,20] + [20,30]` collapses to `[10,30]`.
//! The guest itself does no hashing; the digest in publics `[2..6)` is added
//! in-circuit by the example's `NormalizePublics` (see
//! `programs/aggregations/chain.toml`).
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let old: u32 = ziskos::io::read();
    let new: u32 = ziskos::io::read();
    ziskos::io::commit_slice(&old.to_le_bytes());
    ziskos::io::commit_slice(&new.to_le_bytes());
}
