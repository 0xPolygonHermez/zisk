//! Per-segment classifier for a virtual-table `.fixed` file.
//!
//! The virtual table packs many sub-tables into contiguous row ranges; the UID
//! columns identify which sub-table each row belongs to. This tool splits the
//! rows at every UID change and classifies each payload column *within* each
//! segment, where the byte cycles actually live. It detects shifted cycles and
//! reports the displacement.
//!
//! Usage: `fixed-segments <file.fixed> <cols> <rows> <num_uid_cols> [compact]`

use fixed_analysis::*;

#[derive(Clone)]
enum Cat {
    Const(u64),
    Seq(i128),
    L1(u64, u64),
    Last(u64, u64),
    ByteCycle { base: u64, step: u64, modulus: u64, period: u64 },
    Byte2Cycle { k: u64, base: u64, step: u64, modulus: u64, period: u64, off: u64 },
    Staircase { k: u64, delta: i128 },
    CycleGeneric { period: u64 },
    Other,
}

fn classify(c: &[u64]) -> Cat {
    let n = c.len();
    if n == 0 { return Cat::Other; }
    let first = c[0];
    if c.iter().all(|&x| x == first) { return Cat::Const(first); }
    if n >= 2 {
        // L1: first value differs, rest equal
        if c[0] != c[1] && c[2..].iter().all(|&x| x == c[1]) { return Cat::L1(c[0], c[1]); }
        // LAST: last value differs, rest equal
        if c[n - 1] != c[0] && c[..n - 1].iter().all(|&x| x == c[0]) { return Cat::Last(c[0], c[n - 1]); }
    }
    if let Some(d) = field_delta_const(c) { return Cat::Seq(signed(d)); }
    if let Some((base, step, modulus, period)) = try_counter(c) {
        return Cat::ByteCycle { base, step, modulus, period };
    }
    if let Some((k, base, step, modulus, period, off)) = try_reps(c) {
        return Cat::Byte2Cycle { k, base, step, modulus, period, off };
    }
    // non-cyclic staircase with uniform runs
    let (rv, rl) = rle(c);
    let r = rv.len();
    if r >= 2 {
        let k = rl[0];
        if k > 1 {
            let mut uniform = true;
            for i in 0..r - 1 { if rl[i] != k { uniform = false; break; } }
            if uniform && rl[r - 1] <= k {
                if let Some(d) = field_delta_const(&rv) {
                    if d != 0 { return Cat::Staircase { k, delta: signed(d) }; }
                }
            }
        }
    }
    if let Some(p) = pow2_period(c) { return Cat::CycleGeneric { period: p as u64 }; }
    Cat::Other
}

fn tag(cat: &Cat) -> String {
    match cat {
        Cat::Const(v) => format!("CONSTANT({})", signed(*v)),
        Cat::Seq(d) => format!("SEQUENTIAL(delta {:+})", d),
        Cat::L1(a, b) => format!("L1(first={}, rest={})", signed(*a), signed(*b)),
        Cat::Last(a, b) => format!("LAST(rest={}, last={})", signed(*a), signed(*b)),
        Cat::ByteCycle { base, step, modulus, period } => format!(
            "byte_cycle: cycle 0..{}{}{} (period {})",
            modulus - 1,
            if *step != 1 { format!(" step {}", step) } else { String::new() },
            if *base != 0 { format!(" SHIFTED x={}", base) } else { String::new() },
            period
        ),
        Cat::Byte2Cycle { k, base, modulus, step, period, off } => format!(
            "byte2_cycle: 0:{k},1:{k},..,{}:{k}{}{} (period {})",
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
        Cat::Staircase { k, delta } => format!("STAIRCASE(each value x{}, step {:+}, no cycle)", k, delta),
        Cat::CycleGeneric { period } => format!("CYCLE-GEN(period {}, non-linear block)", period),
        Cat::Other => "OTHER".to_string(),
    }
}

/// Coarse kind key used to group columns in the compact view.
fn kind_key(cat: &Cat) -> &'static str {
    match cat {
        Cat::Const(_) => "CONSTANT",
        Cat::Seq(_) => "SEQUENTIAL",
        Cat::L1(..) => "L1",
        Cat::Last(..) => "LAST",
        Cat::ByteCycle { .. } => "byte_cycle",
        Cat::Byte2Cycle { .. } => "byte2_cycle",
        Cat::Staircase { .. } => "STAIRCASE",
        Cat::CycleGeneric { .. } => "CYCLE-GEN",
        Cat::Other => "OTHER",
    }
}

fn main() {
    let (path, cols, rows, nuid) = common_args(
        "  Per-segment classification. Append 'compact' as a 5th argument for the grouped view.",
    );
    let layout = Layout::new(cols, nuid);
    let compact = std::env::args().nth(5).map(|s| s == "compact").unwrap_or(false);

    eprintln!("Reading {}...", path);
    let columns = load_columns(&path, cols, rows).expect("failed to read file");
    eprintln!("Loaded. Segmenting by UID...");

    // segment boundaries: a row starts a new segment if any UID column changes
    let mut bounds = vec![0usize];
    for r in 1..rows {
        let mut changed = false;
        for j in 0..layout.nuid {
            if columns[j][r] != columns[j][r - 1] { changed = true; break; }
        }
        if changed { bounds.push(r); }
    }
    bounds.push(rows);
    let nseg = bounds.len() - 1;

    println!("\n===== {} =====", path);
    println!("{} rows, {} UID columns, {} payload columns", rows, layout.nuid, layout.ncol);
    println!("Segments (by UID change): {}", nseg);

    let mut bc_count = 0u64;
    let mut b2_count = 0u64;
    let mut per_col_bc = vec![0usize; layout.ncol];
    let mut per_col_b2 = vec![0usize; layout.ncol];

    for s in 0..nseg {
        let start = bounds[s];
        let end = bounds[s + 1];
        let len = end - start;

        let mut items: Vec<(usize, Cat)> = Vec::new();
        let mut padding: Vec<usize> = Vec::new();
        for jc in 0..layout.ncol {
            let j = layout.nuid + jc;
            let cat = classify(&columns[j][start..end]);
            match &cat {
                Cat::ByteCycle { .. } => { bc_count += 1; per_col_bc[jc] += 1; }
                Cat::Byte2Cycle { .. } => { b2_count += 1; per_col_b2[jc] += 1; }
                Cat::Const(v) if *v == 0 => { padding.push(jc); continue; } // unused padding
                _ => {}
            }
            items.push((jc, cat));
        }

        let uid: Vec<i128> = (0..layout.nuid).map(|j| signed(columns[j][start])).collect();
        println!("\n[segment {:2}] rows {}..{}  (len {})  UID={:?}", s, start, end, len, uid);

        if compact {
            print_compact(&items);
            if !padding.is_empty() {
                println!("    {:11}: {} columns", "padding", padding.len());
            }
        } else {
            for (jc, cat) in &items {
                println!("    column[{:2}] = {}", jc, tag(cat));
            }
            if !padding.is_empty() {
                let pc: Vec<String> = padding.iter().map(|x| x.to_string()).collect();
                println!("    (padding CONSTANT(0), {} cols: {})", padding.len(), pc.join(","));
            }
        }
    }

    println!("\n--- Byte-cycle summary ---");
    println!("Segments with BYTE_CYCLE : {}", bc_count);
    println!("Segments with BYTE2_CYCLE: {}", b2_count);
    println!("\nPer column (number of segments where each pattern appears):");
    for jc in 0..layout.ncol {
        if per_col_bc[jc] > 0 || per_col_b2[jc] > 0 {
            println!("  column[{:2}]  byte_cycle x{}  byte2_cycle x{}", jc, per_col_bc[jc], per_col_b2[jc]);
        }
    }
}

/// Grouped, one-line-per-type rendering of a segment's columns.
fn print_compact(items: &[(usize, Cat)]) {
    use std::collections::BTreeMap;
    const ORDER: [&str; 9] = [
        "SEQUENTIAL", "byte_cycle", "byte2_cycle", "STAIRCASE", "CYCLE-GEN", "CONSTANT", "L1", "LAST", "OTHER",
    ];
    // group by kind, then by exact detail string (so cycles with same modulus merge)
    let mut groups: BTreeMap<&'static str, BTreeMap<String, Vec<usize>>> = BTreeMap::new();
    for (jc, cat) in items {
        groups.entry(kind_key(cat)).or_default().entry(tag(cat)).or_default().push(*jc);
    }
    for k in ORDER {
        if let Some(by_detail) = groups.get(k) {
            for (detail, cols) in by_detail {
                let list = cols.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
                if matches!(k, "byte_cycle" | "byte2_cycle" | "STAIRCASE" | "CYCLE-GEN") {
                    println!("    {:11}: column[{}]  -> {}", k, list, detail);
                } else {
                    println!("    {:11}: column[{}]", k, list);
                }
            }
        }
    }
}
