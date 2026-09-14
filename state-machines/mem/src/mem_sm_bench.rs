//! Full-size timing of the `Mem` fill, at the instance's real height.
//!
//! Ignored by default: it allocates the 2^22-row trace (3.5 GB at 104 columns) and up to 33M
//! operations. Run it with the phase timers on to see where the time goes:
//!
//! ```text
//! cargo test --release -p zisk-sm-mem --features witness_timers \
//!     bench_full_size_mem_fill -- --ignored --nocapture
//! ```
//!
//! `MEM_BENCH_FILLS` (default `5,30,60,100`) is the list of occupancies to run, in percent of the
//! instance's slots; `MEM_BENCH_RANGES` (default: the rayon pool size) the number of fill ranges.
//!
//! The operations are shaped like the collectors hand them over: in time order, so the addresses
//! are interleaved, split in chunk-sized vectors, with every address's operations in increasing
//! step. Each address gets one read followed by writes, which is the shape that takes exactly one
//! slot per operation, so the fill is `fill%` of the slots.

use super::*;
use proofman_fields::Goldilocks;
use std::time::Instant;

type Row = MemTraceRow<Goldilocks>;

fn env_list(name: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(name)
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| default.to_vec())
}

/// A segment with `used_slots` slots in use over `used_slots / ops_per_addr` addresses two qwords
/// apart, and its operations in time order split in `n_chunks` vectors.
fn build(used_slots: usize, ops_per_addr: usize, n_chunks: usize) -> (MemModuleSegmentCheckPoint, Vec<Vec<MemInput>>) {
    let mut n_addrs = (used_slots / ops_per_addr).max(1);
    // The address visited at time `t` is `(t * STRIDE) % n_addrs`, a permutation of the addresses
    // per round as long as the two are coprime.
    const STRIDE: usize = 1_000_003;
    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 {
            a
        } else {
            gcd(b, a % b)
        }
    }
    while gcd(n_addrs, STRIDE) != 1 {
        n_addrs -= 1;
    }
    let base = RAM_W_ADDR_INIT;
    let mut seg = MemModuleSegmentCheckPoint::default();
    for a in 0..n_addrs {
        seg.add_addr_offset(base + 2 * a as u32, (a * ops_per_addr + 1) as u32);
    }
    let total = n_addrs * ops_per_addr;
    let per_chunk = total.div_ceil(n_chunks).max(1);
    let mut chunks: Vec<Vec<MemInput>> = Vec::with_capacity(n_chunks);
    let mut cur: Vec<MemInput> = Vec::with_capacity(per_chunk);
    // `MEM_BENCH_ORDER=addr` hands the operations over address by address instead, which is the
    // best case for the fill's writes: consecutive operations land in consecutive slots.
    let by_addr = std::env::var("MEM_BENCH_ORDER").map(|v| v == "addr").unwrap_or(false);
    for t in 0..total {
        let (round, a) = if by_addr {
            (t % ops_per_addr, t / ops_per_addr)
        } else {
            (t / n_addrs, ((t % n_addrs) * STRIDE) % n_addrs)
        };
        cur.push(MemInput::new(
            base + 2 * a as u32,
            round > 0,
            // Time order: the step is the position; address order: still increasing per address.
            if by_addr { (a + round * n_addrs + 1) as u64 } else { (t + 1) as u64 },
            0xDEAD_0000_0000_0000 | ((a as u64) << 8) | round as u64,
        ));
        if cur.len() == per_chunk {
            chunks.push(std::mem::replace(&mut cur, Vec::with_capacity(per_chunk)));
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    (seg, chunks)
}

#[test]
#[ignore]
fn bench_full_size_mem_fill() {
    let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_test_writer().try_init();

    let lanes = Row::default().get_all_addr().len();
    let num_rows = MemTrace::<Row>::NUM_ROWS;
    let num_slots = num_rows * lanes;
    let fills = env_list("MEM_BENCH_FILLS", &[5, 30, 60, 100]);
    let ranges = env_list("MEM_BENCH_RANGES", &[rayon::current_num_threads()]);
    let ops_per_addr = 2usize;
    let n_chunks = 64usize;

    let t_alloc = Instant::now();
    let mut rows = vec![Row::default(); num_rows];
    // Touch every page so the first fill does not pay the kernel's lazy zeroing.
    rows.par_iter_mut().for_each(|r| *r = Row::default());
    eprintln!("trace: {num_rows} rows x {lanes} lanes = {num_slots} slots, {} MB, allocated+touched in {:.0} ms",
        num_rows * std::mem::size_of::<Row>() / (1024 * 1024), t_alloc.elapsed().as_secs_f64() * 1e3);

    let prev = MemPreviousSegment { addr: RAM_W_ADDR_INIT, step: 0, value: 0 };
    for &fill in &fills {
        let used_slots = num_slots * fill / 100;
        let t_build = Instant::now();
        let (seg, chunks) = build(used_slots, ops_per_addr, n_chunks);
        let n_ops: usize = chunks.iter().map(|c| c.len()).sum();
        eprintln!("--- fill {fill}%: {n_ops} ops over {} addresses ({} chunks), built in {:.0} ms",
            seg.addr_range_slots.div_ceil(2), chunks.len(), t_build.elapsed().as_secs_f64() * 1e3);
        for &n_ranges in &ranges {
            for rep in 0..2 {
                let t = Instant::now();
                let out = fill_mem_trace::<Goldilocks, Row>(
                    &mut rows,
                    MemOps::new(&chunks),
                    &seg,
                    &prev,
                    SegmentId(0),
                    true,
                    n_ranges,
                );
                let d_fill = t.elapsed();
                // Stand-in for what `range_check_ranged` does with the histograms: widen to u64
                // and walk every bucket once.
                let t = Instant::now();
                let muls22: Vec<u64> = out.range_22bits.iter().map(|&m| m as u64).collect();
                let muls16: Vec<u64> = out.range_16bits.iter().map(|&m| m as u64).collect();
                let touched = muls22.iter().chain(muls16.iter()).filter(|&&m| m != 0).count();
                let d_rc = t.elapsed();
                eprintln!(
                    "fill {fill:3}% ranges {n_ranges:2} rep {rep}: fill_mem_trace {:6.1} ms | histogram widen+walk {:5.1} ms ({touched} non-zero buckets) | last_addr 0x{:X}",
                    d_fill.as_secs_f64() * 1e3,
                    d_rc.as_secs_f64() * 1e3,
                    out.last_addr * 8
                );
            }
        }
    }
}

// The padding was also measured by write strategy (`par_iter_mut` one row at a time, chunked row
// assignment, `copy_from_slice` from a 256..16384-row block): every one of them ran at the same
// 28 GB/s on a 32-thread desktop, i.e. at the memory's write bandwidth. The 3.3 GB an empty
// instance pads is the floor of the padding cost on the CPU; only not writing it would beat it.
