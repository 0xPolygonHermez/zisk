//! Global per-column classifier for a `.fixed` file.
//!
//! Classifies every column over the whole table into one of:
//! CONSTANT, L1, LAST, L1_LAST, SEQUENTIAL, BYTE_CYCLE, BYTE2_CYCLE, CYCLE,
//! PARTITION, or OTHER. For low-cardinality leftovers it reports whether the
//! column is constant on equal power-of-two partitions.
//!
//! Usage: `fixed-classify <file.fixed> <cols> <rows> <num_uid_cols>`

use fixed_analysis::*;

enum Cat {
    Const(u64),
    L1(u64, u64),
    Last(u64, u64),
    L1Last(u64, u64, u64), // first, middle, last
    Seq(u64),
    ByteCycle { base: u64, step: u64, modulus: u64, period: u64 },
    Byte2Cycle { k: u64, base: u64, step: u64, modulus: u64, period: u64, off: u64 },
    CycleGen { period: u64, distinct: u64 },
    Partition { block_bits: u32, nparts: u64, values: Vec<u64>, distinct: u64 },
    OtherFew { distinct: u64, block_bits: u32, nparts: u64 },
    Other { distinct: u64 },
}

fn classify(c: &[u64]) -> Cat {
    let n = c.len();
    let first = c[0];
    if c.iter().all(|&x| x == first) {
        return Cat::Const(first);
    }
    if n >= 3 {
        let mid = c[1];
        let inner_eq = c[1..n - 1].iter().all(|&x| x == mid);
        if inner_eq {
            let first_diff = c[0] != mid;
            let last_diff = c[n - 1] != mid;
            if first_diff && last_diff { return Cat::L1Last(c[0], mid, c[n - 1]); }
            if first_diff && !last_diff { return Cat::L1(c[0], mid); }
            if !first_diff && last_diff { return Cat::Last(mid, c[n - 1]); }
        }
    }
    if let Some(d) = field_delta_const(c) {
        return Cat::Seq(d);
    }
    if let Some((base, step, modulus, period)) = try_counter(c) {
        return Cat::ByteCycle { base, step, modulus, period };
    }
    if let Some((k, base, step, modulus, period, off)) = try_reps(c) {
        return Cat::Byte2Cycle { k, base, step, modulus, period, off };
    }
    let (trans, distinct) = transitions_and_distinct(c, 200);
    if let Some(p) = pow2_period(c) {
        return Cat::CycleGen { period: p as u64, distinct };
    }
    // PARTITION: only for low-cardinality columns (<= 8 distinct values)
    if distinct <= 8 && !trans.is_empty() {
        // largest uniform block = 2^(min 2-adic valuation of the transitions)
        let block_bits = trans.iter().map(|&t| t.trailing_zeros()).min().unwrap();
        let nparts = (n as u64) >> block_bits;
        // clean partition only if it fits into few large equal partitions
        if block_bits >= 1 && nparts <= 256 {
            let block = 1usize << block_bits;
            let mut values = Vec::new();
            for p in 0..nparts as usize {
                values.push(c[p * block]);
            }
            return Cat::Partition { block_bits, nparts, values, distinct };
        }
        return Cat::OtherFew { distinct, block_bits, nparts };
    }
    Cat::Other { distinct }
}

fn describe(cat: &Cat, bits_total: u32) -> (&'static str, String) {
    match cat {
        Cat::Const(v) => ("CONSTANT", format!("value={}", signed(*v))),
        Cat::L1(a, b) => ("L1", format!("first={} differs, rest={}", signed(*a), signed(*b))),
        Cat::Last(a, b) => ("LAST", format!("last={} differs, rest={}", signed(*b), signed(*a))),
        Cat::L1Last(a, m, z) => ("L1_LAST", format!("first={}, last={}, rest={}", signed(*a), signed(*z), signed(*m))),
        Cat::Seq(d) => ("SEQUENTIAL", format!("delta {:+}", signed(*d))),
        Cat::ByteCycle { base, step, modulus, period } => (
            "BYTE_CYCLE",
            format!(
                "cycle 0..{}{}{} (period {})",
                modulus - 1,
                if *step != 1 { format!(" step {}", step) } else { String::new() },
                if *base != 0 { format!(" SHIFTED x={}", base) } else { String::new() },
                period
            ),
        ),
        Cat::Byte2Cycle { k, base, step, modulus, period, off } => (
            "BYTE2_CYCLE",
            format!(
                "0:{k},..,{}:{k}{}{} (period {})",
                modulus - 1,
                if *step != 1 { format!(" step {}", step) } else { String::new() },
                if *base != 0 || *off != 0 {
                    format!(" SHIFTED {} rows (val={} off={}/{})", base * k + off, base, off, k)
                } else {
                    String::new()
                },
                period,
                k = k
            ),
        ),
        Cat::CycleGen { period, distinct } => ("CYCLE", format!("period {} ({} values/period)", period, distinct)),
        Cat::Partition { block_bits, nparts, values, distinct } => {
            let vs: String = if values.is_empty() {
                "(many)".into()
            } else {
                values.iter().map(|v| signed(*v).to_string()).collect::<Vec<_>>().join(",")
            };
            (
                "PARTITION",
                format!(
                    "{} values; constant on blocks of 2^{} rows => {} partitions (2^{}); values=[{}]",
                    distinct, block_bits, nparts, bits_total - block_bits, vs
                ),
            )
        }
        Cat::OtherFew { distinct, block_bits, nparts } => (
            "OTHER",
            format!(
                "{} values; max uniform block 2^{} => {} partitions (boundaries NOT aligned to large 2^k partitions)",
                distinct, block_bits, nparts
            ),
        ),
        Cat::Other { distinct } => ("OTHER", format!("{} values, no pattern", distinct)),
    }
}

fn main() {
    let (path, cols, rows, nuid) = common_args(
        "  Global per-column classification. For VirtualTableZisk0 use cols=82 nuid=22; for Zisk1 use cols=72 nuid=8.",
    );
    let layout = Layout::new(cols, nuid);
    let bits_total = (rows as f64).log2() as u32;

    eprintln!("Reading {}...", path);
    let columns = load_columns(&path, cols, rows).expect("failed to read file");

    use std::sync::Arc;
    use std::thread;
    let columns = Arc::new(columns);
    let nthreads = 16.min(cols).max(1);
    let mut handles = Vec::new();
    for t in 0..nthreads {
        let cr = Arc::clone(&columns);
        handles.push(thread::spawn(move || {
            let mut out = Vec::new();
            let mut j = t;
            while j < cr.len() {
                out.push((j, classify(&cr[j])));
                j += nthreads;
            }
            out
        }));
    }
    let mut results: Vec<(usize, Cat)> = Vec::new();
    for h in handles {
        results.extend(h.join().unwrap());
    }
    results.sort_by_key(|(j, _)| *j);

    println!("\n===== {} =====", path);
    println!("{} columns x {} rows (2^{})\n", cols, rows, bits_total);
    println!("{:<4} {:<14} {:<12} {}", "idx", "name", "type", "detail");
    println!("{}", "-".repeat(120));
    use std::collections::BTreeMap;
    let mut summary: BTreeMap<String, u32> = BTreeMap::new();
    for (j, cat) in &results {
        let (kind, det) = describe(cat, bits_total);
        *summary.entry(kind.to_string()).or_insert(0) += 1;
        println!("{:<4} {:<14} {:<12} {}", j, layout.name(*j), kind, det);
    }
    println!("\nSummary:");
    for (t, n) in &summary {
        println!("  {:<12} {}", t, n);
    }
}
