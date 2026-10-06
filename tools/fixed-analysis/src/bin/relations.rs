//! Affine-relation finder for a `.fixed` file.
//!
//! Detects columns related by `b = f*a + k` over the Goldilocks field, so that
//! a whole group of columns can be expressed from a single representative.
//! Columns are bucketed by an affine-invariant normalized form and then each
//! bucket is split into verified affine-equivalence clusters.
//!
//! Usage: `fixed-relations <file.fixed> <cols> <rows> <num_uid_cols>`

use fixed_analysis::*;
use std::collections::HashMap;

/// First index where the column changes value.
fn first_change(c: &[u64]) -> Option<usize> {
    (1..c.len()).find(|&i| c[i] != c[0])
}

/// Hash of the affine-invariant normalized form:
/// `nv[i] = (c[i] - c[0]) * inv(c[i0] - c[0])`.
fn norm_hash(c: &[u64], i0: usize) -> u64 {
    let base = c[0];
    let dinv = finv(fsub(c[i0], base));
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in c {
        let nv = fmul(fsub(x, base), dinv);
        h ^= nv;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Verify `b = f*a + k` for all rows; returns `(f, k)` or `None`.
fn affine(a: &[u64], b: &[u64], i0: usize) -> Option<(u64, u64)> {
    let f = fmul(fsub(b[i0], b[0]), finv(fsub(a[i0], a[0])));
    let k = fsub(b[0], fmul(f, a[0]));
    // quick sample first
    for &i in &[0usize, i0, a.len() / 3, a.len() / 2, a.len() - 1] {
        if b[i] != fadd(fmul(f, a[i]), k) { return None; }
    }
    for i in 0..a.len() {
        if b[i] != fadd(fmul(f, a[i]), k) { return None; }
    }
    Some((f, k))
}

fn main() {
    let (path, cols, rows, nuid) = common_args(
        "  Finds affine relations b = f*a + k between columns (global, over all rows).",
    );
    let layout = Layout::new(cols, nuid);

    eprintln!("Reading {}...", path);
    let columns = load_columns(&path, cols, rows).expect("failed to read file");
    eprintln!("Loaded. Searching for affine relations...");

    // constants set aside, grouped by value
    let mut const_groups: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut nonconst: Vec<usize> = Vec::new();
    for j in 0..cols {
        match first_change(&columns[j]) {
            None => const_groups.entry(columns[j][0]).or_default().push(j),
            Some(_) => nonconst.push(j),
        }
    }

    // bucket non-constant columns by normalized-form hash
    let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut i0map: HashMap<usize, usize> = HashMap::new();
    for &j in &nonconst {
        let i0 = first_change(&columns[j]).unwrap();
        i0map.insert(j, i0);
        buckets.entry(norm_hash(&columns[j], i0)).or_default().push(j);
    }

    // within each bucket, split into verified affine-equivalence clusters
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for (_h, members) in &buckets {
        let mut reps: Vec<usize> = Vec::new();
        let mut cl: Vec<Vec<usize>> = Vec::new();
        for &j in members {
            let mut placed = false;
            for (ci, &rep) in reps.iter().enumerate() {
                if affine(&columns[rep], &columns[j], i0map[&rep]).is_some() {
                    cl[ci].push(j);
                    placed = true;
                    break;
                }
            }
            if !placed {
                reps.push(j);
                cl.push(vec![j]);
            }
        }
        clusters.extend(cl);
    }
    clusters.sort_by_key(|c| (usize::MAX - c.len(), c[0]));

    println!("\n===== {} =====", path);
    println!("{} columns ({} constant, {} non-constant)\n", cols, cols - nonconst.len(), nonconst.len());

    println!("### AFFINE groups  (b = f*a + k) — each group reducible to 1 column ###");
    let mut saved = 0;
    for grp in clusters.iter().filter(|g| g.len() >= 2) {
        let rep = grp[0];
        let i0 = i0map[&rep];
        let d = distinct(&columns[rep], 500);
        let dstr = if d > 500 { ">500".into() } else { d.to_string() };
        println!("\nGroup of {} columns ({} distinct values); representative {}:", grp.len(), dstr, layout.name(rep));
        for &j in &grp[1..] {
            let (f, k) = affine(&columns[rep], &columns[j], i0).unwrap();
            println!(
                "   {} = {}*{} {}{}",
                layout.name(j),
                signed(f),
                layout.name(rep),
                if signed(k) >= 0 { "+ " } else { "- " },
                signed(k).abs()
            );
        }
        saved += grp.len() - 1;
    }
    if saved == 0 {
        println!("  (none)");
    }

    let singles: Vec<usize> = clusters.iter().filter(|g| g.len() == 1).map(|g| g[0]).collect();
    println!("\n### Non-constant with NO affine relation: {} ###", singles.len());
    let names: Vec<String> = singles.iter().map(|&j| layout.name(j)).collect();
    println!("  {}", names.join(", "));

    println!("\n### CONSTANT columns (grouped by value) ###");
    let mut cg: Vec<(&u64, &Vec<usize>)> = const_groups.iter().collect();
    cg.sort_by_key(|(v, _)| **v);
    for (v, js) in cg {
        let names: Vec<String> = js.iter().map(|&j| layout.name(j)).collect();
        println!("  value {:<6} : {}", signed(*v), names.join(", "));
    }

    // group by number of distinct values (comparable candidates, even if not globally affine)
    println!("\n### Non-constant grouped by number of distinct values ###");
    let mut byd: HashMap<u64, Vec<usize>> = HashMap::new();
    for &j in &nonconst {
        byd.entry(distinct(&columns[j], 500)).or_default().push(j);
    }
    let mut keys: Vec<u64> = byd.keys().cloned().collect();
    keys.sort();
    for d in keys {
        let mut js = byd[&d].clone();
        js.sort();
        let names: Vec<String> = js.iter().map(|&j| layout.name(j)).collect();
        let dstr = if d > 500 { ">500".into() } else { d.to_string() };
        println!("  {:>5} values ({} cols): {}", dstr, js.len(), names.join(", "));
    }

    println!(
        "\nSummary: {} non-constant columns -> {} distinct affine groups (saves {} columns)",
        nonconst.len(),
        clusters.len(),
        saved
    );
}
