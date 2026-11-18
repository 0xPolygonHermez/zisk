//! Fork-join regions (`ziskos::par`): runs tasks of different lengths, each building, sorting
//! and folding a vector of its own, in two regions one after the other, then joins two closures
//! with results of different types, which read what the two regions returned; and checks in the
//! guest that all of it gives what running it one after another gives.
//!
//! Reads one input slice: the tasks' sizes, as little-endian u32. Commits the number of tasks
//! and a checksum of every result. Some sizes stand for tasks the tests need: `u32::MAX` panics, as
//! a guest that ends inside a task; `DEEP` and `DEEPER` recurse with 1 KiB of stack a level, 40
//! levels (most of the 64 KiB stack of the tasks) and 256 levels (overflowing it); `NESTED` opens a
//! region of its own, which inside a task runs inline; `READS_INPUT` reads the input, which a task
//! must not.

#![no_main]
ziskos::entrypoint!(main);

use ziskos::par;

/// A task that nearly fills its stack, and one that overflows it (see `deep`)
const DEEP: u32 = u32::MAX - 2;
const DEEPER: u32 = u32::MAX - 1;
/// A task that opens a region of its own, and one that reads the input
const NESTED: u32 = u32::MAX - 3;
const READS_INPUT: u32 = u32::MAX - 4;

/// Recurses `depth` levels with about 1 KiB of stack each, and folds what every level kept there
#[inline(never)]
fn deep(depth: u32) -> u64 {
    let mut frame = [0u8; 1024];
    frame[depth as usize % 1024] = depth as u8;
    // Keeps the frame on the stack, without a copy
    core::hint::black_box(&mut frame);
    let below = if depth == 0 { 0 } else { deep(depth - 1) };
    below.wrapping_mul(31).wrapping_add(u64::from(frame[depth as usize % 1024]))
}

/// Builds `len` xorshift values from `seed`, sorts them and folds them into one word: work that
/// allocates, moves memory, and takes about 20 steps per value plus the sort
fn work(seed: u64, len: u32) -> u64 {
    match len {
        u32::MAX => panic!("a task asked to fail"),
        DEEP => return deep(40),
        DEEPER => return deep(256),
        NESTED => {
            // A region of two tasks: inline inside a task, a region of its own outside one
            let jobs: Vec<_> = (0..2u64).map(|k| move || work(seed ^ k, 200)).collect();
            let config = par::Config { heap_bytes: 64 << 10, stack_bytes: 16 << 10 };
            return par::run(jobs, &config).iter().fold(0u64, |a, &v| a.wrapping_add(v));
        }
        READS_INPUT => return u64::from(ziskos::io::read::<u32>()),
        _ => {}
    }
    let mut x = seed | 1;
    let mut values = Vec::with_capacity(len as usize);
    for _ in 0..len {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        values.push(x);
    }
    values.sort_unstable();
    values
        .iter()
        .enumerate()
        .fold(0u64, |acc, (k, &v)| acc.wrapping_mul(31).wrapping_add(v ^ k as u64))
}

fn main() {
    let input = ziskos::io::read_slice();
    let sizes: Vec<u32> =
        input.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();

    // Built before the regions and only read by the tasks
    let seeds: Vec<u64> =
        (0..sizes.len() as u64).map(|i| 0x9e37_79b9_7f4a_7c15u64.wrapping_mul(i + 1)).collect();
    let config = par::Config { heap_bytes: 1 << 20, stack_bytes: 64 << 10 };

    // One task per size, each reading its seed from the shared vector
    let jobs: Vec<_> = (0..sizes.len())
        .map(|i| {
            let (seeds, len) = (&seeds, sizes[i]);
            move || work(seeds[i], len)
        })
        .collect();
    let first = par::run(jobs, &config);

    // The same work in reverse order, each result owning memory its task allocated
    let jobs: Vec<_> = (0..sizes.len())
        .rev()
        .map(|i| {
            let (seeds, len) = (&seeds, sizes[i]);
            move || {
                let w = work(seeds[i] ^ 0xff, len);
                vec![w, w.rotate_left(17)]
            }
        })
        .collect();
    let second = par::run(jobs, &config);

    // What running them one after another gives
    let expected_first: Vec<u64> = (0..sizes.len()).map(|i| work(seeds[i], sizes[i])).collect();
    let expected_second: Vec<Vec<u64>> = (0..sizes.len())
        .rev()
        .map(|i| {
            let w = work(seeds[i] ^ 0xff, sizes[i]);
            vec![w, w.rotate_left(17)]
        })
        .collect();
    assert_eq!(first, expected_first, "the first region's results");
    assert_eq!(second, expected_second, "the second region's results");

    // Two closures with results of different types, which read the results the regions left in
    // their tasks' arenas: a region of two tasks of its own
    let fold_first = || first.iter().fold(0u64, |a, &v| a.wrapping_mul(31).wrapping_add(v));
    let low_halves = || second.iter().map(|v| v[0] as u32).collect::<Vec<u32>>();
    let join_config = par::Config { heap_bytes: 64 << 10, stack_bytes: 16 << 10 };
    let (folded, halves) = par::join(fold_first, low_halves, &join_config);
    assert_eq!((folded, &halves), (fold_first(), &low_halves()), "the joined results");

    let checksum = first
        .iter()
        .chain(second.iter().flatten())
        .copied()
        .chain([folded])
        .chain(halves.iter().map(|&h| u64::from(h)))
        .fold(0u64, |a, v| a.rotate_left(5) ^ v);
    ziskos::io::commit(&(sizes.len() as u64));
    ziskos::io::commit(&checksum);
}
