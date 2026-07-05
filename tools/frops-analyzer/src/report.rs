//! Approach 1: emit compressed, AI-friendly artifacts (`proposal.json` + `report.md`) describing the
//! frequency analysis and the proposed FROPS, without touching any source file.

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::ingest::Aggregator;
use crate::optimize::{Areas, Proposal};

pub fn write_reports(agg: &Aggregator, prop: &Proposal, dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let json = build_json(agg, prop);
    fs::write(dir.join("proposal.json"), serde_json::to_string_pretty(&json).unwrap())?;
    fs::write(dir.join("report.md"), build_md(agg, prop))?;
    Ok(())
}

fn areas_json(a: &Areas) -> serde_json::Value {
    json!({ "instances": a.instances, "table": a.table, "total": a.total })
}

fn pct(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        100.0 * part as f64 / whole as f64
    }
}

fn build_json(agg: &Aggregator, prop: &Proposal) -> serde_json::Value {
    let cfg = &prop.config;

    // Per-op view, sorted by occurrences descending.
    let mut codes: Vec<u8> = prop.op_total.keys().copied().collect();
    codes.sort_by_key(|c| std::cmp::Reverse(prop.op_total[c]));
    let ops: Vec<serde_json::Value> = codes
        .iter()
        .map(|&code| {
            let info = prop.op_info[&code];
            let total = prop.op_total[&code];
            let covered = prop.op_covered.get(&code).copied().unwrap_or(0);
            let regions: Vec<serde_json::Value> = prop
                .selected
                .iter()
                .filter(|s| s.info.code == code)
                .map(|s| {
                    json!({
                        "kind": s.region.kind.as_str(),
                        "predicate": s.region.predicate(),
                        "a_lo": s.region.a_lo,
                        "a_count": s.region.a_count,
                        "b_lo": s.region.b_lo,
                        "b_count": s.region.b_count,
                        "rows": s.region.rows(),
                        "hits": s.hits,
                    })
                })
                .collect();
            let current_covered = prop.current_covered.get(&code).copied().unwrap_or(0);
            json!({
                "code": format!("{code:#04x}"),
                "name": info.name,
                "table": info.table.key(),
                "sm": info.sm.name(),
                "cost": info.cost,
                "occurrences": total,
                "covered": covered,
                "coverage_pct": pct(covered, total),
                "current_covered": current_covered,
                "current_coverage_pct": pct(current_covered, total),
                "regions": regions,
            })
        })
        .collect();

    let dropped: Vec<serde_json::Value> = prop
        .dropped
        .iter()
        .map(|d| {
            json!({
                "code": format!("{:#04x}", d.info.code),
                "name": d.info.name,
                "kind": d.region.kind.as_str(),
                "predicate": d.region.predicate(),
                "rows": d.region.rows(),
                "hits": d.hits,
            })
        })
        .collect();

    let paid = |t: crate::ops::FropsTable| prop.table_paid_rows.get(&t).copied().unwrap_or(0);
    let used = |t: crate::ops::FropsTable| prop.table_rows.get(&t).copied().unwrap_or(0);
    let tables: serde_json::Value = json!({
        "arith": { "used": used(crate::ops::FropsTable::Arith), "paid": paid(crate::ops::FropsTable::Arith) },
        "binary_basic": { "used": used(crate::ops::FropsTable::BinaryBasic), "paid": paid(crate::ops::FropsTable::BinaryBasic) },
        "binary_extension": { "used": used(crate::ops::FropsTable::BinaryExt), "paid": paid(crate::ops::FropsTable::BinaryExt) },
        "total_used": prop.total_table_rows,
        "total_paid": prop.total_paid_rows,
        "waste": prop.total_waste,
        "partition_bits": cfg.partition_bits,
        "partitions": prop.total_paid_rows >> cfg.partition_bits,
        "max_table": cfg.max_table,
    });

    json!({
        "config": {
            "max_table": cfg.max_table,
            "nodes": cfg.nodes,
            "padding": cfg.padding,
            "table_cost": cfg.table_cost,
            "low_cap": cfg.low_cap,
        },
        "input": {
            "files": agg.files,
            "records": agg.records,
            "frops_candidate_records": agg.records - agg.skipped_non_frops,
            "skipped_non_frops": agg.skipped_non_frops,
            "trailing_bytes": agg.trailing_bytes,
        },
        "tables": tables,
        "area": {
            "baseline_no_padding": areas_json(&prop.baseline_nopad),
            "proposed_no_padding": areas_json(&prop.proposed_nopad),
            "baseline_padding": areas_json(&prop.baseline_pad),
            "proposed_padding": areas_json(&prop.proposed_pad),
            "savings_pct_no_padding":
                pct(prop.baseline_nopad.total.saturating_sub(prop.proposed_nopad.total), prop.baseline_nopad.total),
            "savings_pct_padding":
                pct(prop.baseline_pad.total.saturating_sub(prop.proposed_pad.total), prop.baseline_pad.total),
        },
        "comparison_vs_current": comparison(prop),
        "ops": ops,
        "dropped": dropped,
    })
}

/// Side-by-side comparison of the proposal against the current in-tree FROPS, over the same data.
fn comparison(prop: &Proposal) -> serde_json::Value {
    let total_occ: u64 = prop.op_total.values().sum();
    let proposed_cov: u64 = prop.op_covered.values().sum();
    let current_cov: u64 = prop.current_covered.values().sum();
    json!({
        "current": {
            "table_rows": prop.current_total_table_rows,
            "covered_hits": current_cov,
            "coverage_pct": pct(current_cov, total_occ),
            "area_total_no_padding": prop.current_nopad.total,
            "area_total_padding": prop.current_pad.total,
        },
        "proposed": {
            "table_rows": prop.total_table_rows,
            "covered_hits": proposed_cov,
            "coverage_pct": pct(proposed_cov, total_occ),
            "area_total_no_padding": prop.proposed_nopad.total,
            "area_total_padding": prop.proposed_pad.total,
        },
        "delta": {
            "table_rows": prop.total_table_rows as i128 - prop.current_total_table_rows as i128,
            "coverage_pct_points": pct(proposed_cov, total_occ) - pct(current_cov, total_occ),
            "area_no_padding": prop.proposed_nopad.total as i128 - prop.current_nopad.total as i128,
            "area_padding": prop.proposed_pad.total as i128 - prop.current_pad.total as i128,
            "proposed_is_better_no_padding": prop.proposed_nopad.total <= prop.current_nopad.total,
            "proposed_is_better_padding": prop.proposed_pad.total <= prop.current_pad.total,
        },
    })
}

fn build_md(agg: &Aggregator, prop: &Proposal) -> String {
    let cfg = &prop.config;
    let mut s = String::new();
    s.push_str("# FROPS analysis report\n\n");
    s.push_str(&format!(
        "Config: max_table={}, nodes={}, padding={}, table_cost={}, low_cap={}\n\n",
        cfg.max_table, cfg.nodes, cfg.padding, cfg.table_cost, cfg.low_cap
    ));
    s.push_str(&format!(
        "Input: {} file(s), {} records ({} FROPS-candidate, {} skipped non-FROPS).\n\n",
        agg.files,
        agg.records,
        agg.records - agg.skipped_non_frops,
        agg.skipped_non_frops
    ));

    s.push_str(&format!(
        "## Table usage (padded to 2^{} = {} rows/partition)\n\n",
        cfg.partition_bits,
        1u64 << cfg.partition_bits
    ));
    s.push_str("| table | used rows | paid (padded) | waste |\n|---|---|---|---|\n");
    for t in crate::ops::FropsTable::all() {
        let used = prop.table_rows.get(&t).copied().unwrap_or(0);
        let paid = prop.table_paid_rows.get(&t).copied().unwrap_or(0);
        s.push_str(&format!("| {} | {} | {} | {} |\n", t.key(), used, paid, paid - used));
    }
    s.push_str(&format!(
        "| **total** | **{}** | **{} / {} ({} part.)** | **{}** |\n\n",
        prop.total_table_rows,
        prop.total_paid_rows,
        cfg.max_table,
        prop.total_paid_rows >> cfg.partition_bits,
        prop.total_waste
    ));

    s.push_str("## Area\n\n");
    s.push_str("| variant | baseline | proposed | savings |\n|---|---|---|---|\n");
    s.push_str(&format!(
        "| no padding | {} | {} | {:.2}% |\n",
        prop.baseline_nopad.total,
        prop.proposed_nopad.total,
        pct(
            prop.baseline_nopad.total.saturating_sub(prop.proposed_nopad.total),
            prop.baseline_nopad.total
        )
    ));
    s.push_str(&format!(
        "| padding | {} | {} | {:.2}% |\n\n",
        prop.baseline_pad.total,
        prop.proposed_pad.total,
        pct(
            prop.baseline_pad.total.saturating_sub(prop.proposed_pad.total),
            prop.baseline_pad.total
        )
    ));

    // Comparison against the current in-tree FROPS.
    let total_occ: u64 = prop.op_total.values().sum();
    let proposed_cov: u64 = prop.op_covered.values().sum();
    let current_cov: u64 = prop.current_covered.values().sum();
    s.push_str("## Proposed vs current FROPS (same data)\n\n");
    s.push_str("| metric | current | proposed |\n|---|---|---|\n");
    s.push_str(&format!(
        "| table rows | {} | {} |\n",
        prop.current_total_table_rows, prop.total_table_rows
    ));
    s.push_str(&format!(
        "| covered hits | {} ({:.2}%) | {} ({:.2}%) |\n",
        current_cov,
        pct(current_cov, total_occ),
        proposed_cov,
        pct(proposed_cov, total_occ)
    ));
    s.push_str(&format!(
        "| area (no padding) | {} | {} |\n",
        prop.current_nopad.total, prop.proposed_nopad.total
    ));
    s.push_str(&format!(
        "| area (padding) | {} | {} |\n\n",
        prop.current_pad.total, prop.proposed_pad.total
    ));
    let variant = if prop.config.padding {
        (prop.current_pad.total, prop.proposed_pad.total, "padding")
    } else {
        (prop.current_nopad.total, prop.proposed_nopad.total, "no padding")
    };
    let verdict = if variant.1 <= variant.0 {
        format!(
            "Proposed is **better** by {} area ({:.2}%) [{}].",
            variant.0.saturating_sub(variant.1),
            pct(variant.0.saturating_sub(variant.1), variant.0),
            variant.2
        )
    } else {
        format!(
            "Current is **better** by {} area ({:.2}%) [{}].",
            variant.1.saturating_sub(variant.0),
            pct(variant.1.saturating_sub(variant.0), variant.0),
            variant.2
        )
    };
    s.push_str(&verdict);
    s.push_str("\n\nPer-table rows (current → proposed):\n\n| table | current | proposed |\n|---|---|---|\n");
    for t in crate::ops::FropsTable::all() {
        s.push_str(&format!(
            "| {} | {} | {} |\n",
            t.key(),
            prop.current_table_rows.get(&t).copied().unwrap_or(0),
            prop.table_rows.get(&t).copied().unwrap_or(0)
        ));
    }
    s.push('\n');

    s.push_str("## Proposed FROPS by op (most frequent first)\n\n");
    let mut codes: Vec<u8> = prop.op_total.keys().copied().collect();
    codes.sort_by_key(|c| std::cmp::Reverse(prop.op_total[c]));
    s.push_str("| op | code | occ | covered | % | region (predicate -> rows, hits) |\n");
    s.push_str("|---|---|---|---|---|---|\n");
    for code in codes {
        let info = prop.op_info[&code];
        let total = prop.op_total[&code];
        let covered = prop.op_covered.get(&code).copied().unwrap_or(0);
        let regions: Vec<String> = prop
            .selected
            .iter()
            .filter(|s| s.info.code == code)
            .map(|s| {
                format!("`{}` -> {} rows, {} hits", s.region.predicate(), s.region.rows(), s.hits)
            })
            .collect();
        let region_txt = if regions.is_empty() { "-".to_string() } else { regions.join("<br>") };
        s.push_str(&format!(
            "| {} | {:#04x} | {} | {} | {:.1} | {} |\n",
            info.name,
            code,
            total,
            covered,
            pct(covered, total),
            region_txt
        ));
    }

    if !prop.dropped.is_empty() {
        s.push_str("\n## Dropped (did not fit the table budget)\n\n");
        s.push_str("| op | kind | rows | hits |\n|---|---|---|---|\n");
        for d in &prop.dropped {
            s.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                d.info.name,
                d.region.kind.as_str(),
                d.region.rows(),
                d.hits
            ));
        }
    }
    s
}
