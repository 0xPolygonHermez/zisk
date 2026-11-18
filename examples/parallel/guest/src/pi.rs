//! π from a grid: counts the cells of an `r × r` grid over a quarter of the circle of radius `r`
//! whose centres lie inside it (about `π/4 · r²` of them), splitting the rows `0..r` into `tasks`
//! ranges that run as one fork-join region.
//!
//! `par::run` takes one closure per task and returns their results in order, whether the tasks
//! ran one after another (natively, and on the Rust emulator) or at the same time (on the
//! assembly emulator). Each closure only reads what it captured and returns what it computed;
//! nothing is shared between tasks.

#![no_main]
ziskos::entrypoint!(main);

use parallel_common::{Params, cells_inside, split};
use ziskos::par;

fn main() {
    let params = ziskos::io::read::<Params>();
    let radius = params.len;

    // One task per range of rows: each returns how many of its cells are inside
    let jobs: Vec<_> = split(radius, params.tasks)
        .into_iter()
        .map(|rows| move || cells_inside(rows.start as u64..rows.end as u64, radius))
        .collect();
    // The tasks allocate nothing and call no deeper than `cells_inside`: a small arena each is
    // enough
    let partial = par::run(jobs, &par::Config { heap_bytes: 4 << 10, stack_bytes: 16 << 10 });

    // The results come back in task order, so they combine as a sequential loop would
    let inside: u64 = partial.iter().sum();
    ziskos::io::commit(&inside);
}
