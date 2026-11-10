//! A histogram of `len` pseudo-random values, counted by `tasks` tasks that run as one fork-join
//! region. Each value depends on its index only, so each task generates its own slice.
//!
//! A reduction: the tasks share nothing they write. Each one reduces its slice to a histogram of
//! its own and returns it, and the histograms are added up after the region, in task order. That
//! is the pattern that replaces shared counters and atomics, and the one CUDA's reduction samples
//! build with shared memory, which fork-join tasks do not have.
//!
//! A task must never write into memory built before the region: not a shared counter, not its
//! own disjoint part of a shared buffer (a `chunks_mut` slice), not a `Vec` moved into its closure
//! and changed in place. When the tasks run in parallel, each one writes into a private copy of
//! everything outside its arena, so those writes are lost: the guest goes on with what that
//! memory held before the region, and the proof fails. A task returns what it computes instead;
//! its result lives in its arena, which the guest reads after the region.

#![no_main]
ziskos::entrypoint!(main);

use parallel_common::{BINS, Params, histogram, split};
use ziskos::par;

fn main() {
    let params = ziskos::io::read::<Params>();
    let seed = params.seed;
    let ranges = split(params.len, params.tasks);
    // The tasks allocate nothing and call no deeper than `histogram`: a small arena each is enough
    let config = par::Config { heap_bytes: 4 << 10, stack_bytes: 16 << 10 };

    // Each task returns its own histogram
    let jobs: Vec<_> = ranges
        .into_iter()
        .map(|range| move || histogram(range.start as u64..range.end as u64, seed))
        .collect();
    let partial = par::run(jobs, &config);

    // WRONG, in place of the region above: each task writes its histogram into its own part of a
    // buffer built before the region. It compiles: no two tasks share a `&mut`, and a `&mut`
    // slice is `Send`. Natively and on the Rust emulator, which run the tasks one after another,
    // it even gives the right counts. On the assembly emulator the tasks run in parallel, each
    // writing into its private copy of `counts`, so after the region `counts` is still all zeros:
    // the guest commits zeros, and the proof fails, because the trace has the tasks' writes
    // followed by reads that do not see them.
    //
    //     let mut counts = vec![0u64; BINS * ranges.len()];
    //     let jobs: Vec<_> = counts
    //         .chunks_mut(BINS)
    //         .zip(ranges)
    //         .map(|(mine, range)| {
    //             move || mine.copy_from_slice(&histogram(range.start as u64..range.end as u64, seed))
    //         })
    //         .collect();
    //     par::run(jobs, &config);

    // Added up after the region, in task order, as a sequential loop would
    let mut counts = [0u64; BINS];
    for task_counts in &partial {
        for (count, n) in counts.iter_mut().zip(task_counts) {
            *count += n;
        }
    }
    ziskos::io::commit(&counts);
}
