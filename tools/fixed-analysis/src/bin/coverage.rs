//! Byte-cycle coverage report: for each payload column, how many UID segments
//! (and what fraction of rows) it is a `byte_cycle` / `byte2_cycle`, compared to
//! its global classification. Shows why patterns that are common per-segment can
//! look irregular globally (they restart, shifted, at each sub-table boundary).
//!
//! Usage: `fixed-coverage <file.fixed> <cols> <rows> <num_uid_cols>`

use fixed_analysis::*;

/// Short global-type label for a whole column.
fn global_kind(c: &[u64]) -> &'static str {
    let first = c[0];
    if c.iter().all(|&x| x == first) { return "CONSTANT"; }
    if field_delta_const(c).is_some() { return "SEQUENTIAL"; }
    if try_counter(c).is_some() { return "BYTE_CYCLE"; }
    if try_reps(c).is_some() { return "BYTE2_CYCLE"; }
    if pow2_period(c).is_some() { return "CYCLE"; }
    "OTHER"
}

fn main() {
    let (path, cols, rows, nuid) = common_args(
        "  Per-column byte-cycle coverage across UID segments vs the global type.",
    );
    let layout = Layout::new(cols, nuid);

    eprintln!("Reading {}...", path);
    let columns = load_columns(&path, cols, rows).expect("failed to read file");

    // segment boundaries by UID change
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

    let mut bc_segs = vec![0usize; layout.ncol];
    let mut bc_rows = vec![0usize; layout.ncol];
    let mut b2_segs = vec![0usize; layout.ncol];
    let mut b2_rows = vec![0usize; layout.ncol];

    for s in 0..nseg {
        let start = bounds[s];
        let end = bounds[s + 1];
        let len = end - start;
        for jc in 0..layout.ncol {
            let seg = &columns[layout.nuid + jc][start..end];
            if try_counter(seg).is_some() {
                bc_segs[jc] += 1;
                bc_rows[jc] += len;
            } else if try_reps(seg).is_some() {
                b2_segs[jc] += 1;
                b2_rows[jc] += len;
            }
        }
    }

    println!("\n===== {} =====", path);
    println!("{} rows, {} segments\n", rows, nseg);
    println!("{:<12} {:<12} {:<24} {:<24}", "column", "global type", "byte_cycle", "byte2_cycle");
    println!("{}", "-".repeat(74));
    let mut bc_cols = 0;
    let mut b2_cols = 0;
    for jc in 0..layout.ncol {
        if bc_segs[jc] == 0 && b2_segs[jc] == 0 { continue; }
        let gk = global_kind(&columns[layout.nuid + jc]);
        let bc = if bc_segs[jc] > 0 {
            format!("{} seg / {}% rows", bc_segs[jc], 100 * bc_rows[jc] / rows)
        } else {
            "-".into()
        };
        let b2 = if b2_segs[jc] > 0 {
            format!("{} seg / {}% rows", b2_segs[jc], 100 * b2_rows[jc] / rows)
        } else {
            "-".into()
        };
        if bc_segs[jc] > 0 { bc_cols += 1; }
        if b2_segs[jc] > 0 { b2_cols += 1; }
        println!("{:<12} {:<12} {:<24} {:<24}", format!("column[{}]", jc), gk, bc, b2);
    }
    println!("{}", "-".repeat(74));
    println!("Columns with byte_cycle in >=1 segment : {}", bc_cols);
    println!("Columns with byte2_cycle in >=1 segment: {}", b2_cols);
    println!("(vs GLOBAL: byte_cycle/byte2_cycle only where the counter is continuous over the whole table)");
}
