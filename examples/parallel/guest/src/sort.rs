//! Sorts `len` pseudo-random values with `tasks` tasks, in two fork-join regions as wide as
//! `tasks`, then summarises the sorted values with `par::join`:
//!
//! 1. Each task generates a slice of the values and sorts it: `tasks` sorted runs. Each value
//!    depends on its index only, so a task generates its own slice.
//! 2. The `u64` values split into `tasks` buckets of equal width: bucket `k` holds the values from
//!    `k · 2^64 / tasks` up to `(k + 1) · 2^64 / tasks`. Each task gathers its bucket from every
//!    run, where two binary searches (`partition_point`) find it, and sorts it. The buckets, in
//!    order, are the sorted values, and as the values are uniform each holds about `len / tasks`.
//! 3. `par::join` runs two closures whose results have different types, which `par::run` cannot
//!    take together: an order-sensitive checksum of the sorted values, and their histogram.
//!
//! Step 2 does the work of a mergesort's merge, split by value range instead of into pairs of
//! runs. Merging in pairs takes a region per level, each half as wide as the one before, down to
//! a single merge of two runs: the later levels leave most cores idle, and every region has a
//! fixed cost per task (on the assembly emulator a process, in the proof a padding chunk). Split
//! by value, the whole merge is one region as wide as the first.
//!
//! A task's result may own memory the task allocated (here, its run or its bucket): arenas are
//! never reused, so it stays valid after the region. So the second region's tasks can read the
//! first one's runs, which are complete before it starts.

#![no_main]
ziskos::entrypoint!(main);

use parallel_common::{Params, bin_counts, bucket_range, checksum, split, values};
use ziskos::par;

/// The bytes of a value
const WORD: usize = size_of::<u64>();

/// The arena of each task of a region whose tasks allocate at most `bytes` each.
///
/// - **Heap: what the task allocates, and a page to spare.** The heap is a bump allocator: every
///   allocation counts against it, and freeing gives nothing back. So each task here allocates
///   its result once, at its final size: `values` collects an iterator of known length, and a
///   bucket is gathered into a `Vec::with_capacity` of its exact length. A `Vec` grown by `push`
///   instead reallocates as it doubles, and its old buffers stay allocated: several times its
///   final size in all. `sort_unstable` sorts in place, where the stable `sort` allocates
///   scratch space, up to as much again as the slice.
/// - **Stack: 16 KiB, ample for a release build.** `sort_unstable`'s recursion is the deepest
///   call chain here. Guests build with the host's profile, so `cargo run` without `--release`
///   builds a debug guest, whose stack frames are much larger and can overflow a stack sized for
///   release. A canary below each stack catches most overflows when the task returns, failing
///   the execution; a single frame larger than the canary can step over it.
///
/// Arenas are never reused, so the arenas of every region add up, within the guest's 512 MiB of
/// RAM. Here they take about twice the values' size: a run and a bucket per value.
fn config_for(bytes: usize) -> par::Config {
    par::Config { heap_bytes: bytes + (4 << 10), stack_bytes: 16 << 10 }
}

fn main() {
    let params = ziskos::io::read::<Params>();
    let (tasks, seed) = (params.tasks.max(1), params.seed);

    // 1. Generate and sort each slice: a task allocates its run, the size of its slice
    let ranges = split(params.len, tasks);
    let largest = ranges.iter().map(|range| range.len()).max().unwrap_or(0);
    let jobs: Vec<_> = ranges
        .into_iter()
        .map(|range| {
            move || {
                let mut run = values(range.start as u64..range.end as u64, seed);
                run.sort_unstable();
                run
            }
        })
        .collect();
    let runs = par::run(jobs, &config_for(largest * WORD));

    // 2. Gather and sort each bucket. The binary searches the tasks will make, made here before
    // the region, give each bucket's size: the arenas are sized from the data, and each task
    // allocates its bucket at its exact size
    let sizes: Vec<usize> = (0..tasks)
        .map(|k| runs.iter().map(|run| bucket_range(run, k, tasks).len()).sum())
        .collect();
    let largest = sizes.iter().copied().max().unwrap_or(0);
    let jobs: Vec<_> = (0..tasks)
        .zip(sizes)
        .map(|(k, size)| {
            let runs = &runs;
            move || {
                let mut bucket = Vec::with_capacity(size);
                for run in runs {
                    bucket.extend_from_slice(&run[bucket_range(run, k, tasks)]);
                }
                bucket.sort_unstable();
                bucket
            }
        })
        .collect();
    let buckets = par::run(jobs, &config_for(largest * WORD));

    // 3. Two closures with results of different types, over the buckets in order. They read the
    // buckets and allocate nothing
    let (sum, bins) = par::join(
        || checksum(buckets.iter().flatten().copied()),
        || bin_counts(buckets.iter().flatten().copied()),
        &config_for(0),
    );

    let len: usize = buckets.iter().map(Vec::len).sum();
    ziskos::io::commit(&(len as u64));
    ziskos::io::commit(&sum);
    ziskos::io::commit(&bins);
}
