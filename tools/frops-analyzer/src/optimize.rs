//! Area model and budgeted selection of FROPS regions.
//!
//! Area of a proof block = area of the per-op instance rows + area of the (replicated) FROPS table:
//!   * FROPS table area = `table_rows * table_cost * nodes`   (every node recomputes the table).
//!   * instance area, no padding = `Σ_op (occurrences - covered) * cost(op)`.
//!   * instance area, with padding = `Σ_sm ceil(used_sm / NUM_ROWS_sm) * NUM_ROWS_sm * cost_sm`.
//!
//! Selection is a greedy knapsack: candidate regions (already net-positive, disjoint per op) are
//! taken by descending coverage efficiency until the table budget `max_table` is exhausted. This
//! yields a table strictly within the budget that minimises area well in practice (documented
//! heuristic, not a proven optimum — especially under padding's step costs).

use std::collections::HashMap;

use crate::ingest::Aggregator;
use crate::ops::{classify, FropsTable, OpInfo, Sm};
use crate::region::{candidate_groups, Candidate, Region};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub max_table: u64,
    pub nodes: u64,
    pub padding: bool,
    pub table_cost: u64,
    pub low_cap: u64,
    /// Cap on FROPS regions per opcode, bounding the cost of `is_frequent_op` / `get_row`.
    pub max_regions_per_op: usize,
    /// Table partition bits: each family's table is padded to a multiple of `2^partition_bits` rows
    /// (the recursion partition size). `max_table` bounds the total *paid* (padded) rows.
    pub partition_bits: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct SelectedRegion {
    pub info: OpInfo,
    pub region: Region,
    pub hits: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct DroppedRegion {
    pub info: OpInfo,
    pub region: Region,
    pub hits: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Areas {
    pub instances: u64,
    pub table: u64,
    pub total: u64,
}

pub struct Proposal {
    pub config: Config,
    pub selected: Vec<SelectedRegion>,
    pub dropped: Vec<DroppedRegion>,
    /// Per-op total occurrences and covered occurrences.
    pub op_total: HashMap<u8, u64>,
    pub op_covered: HashMap<u8, u64>,
    pub op_info: HashMap<u8, OpInfo>,
    pub table_rows: HashMap<FropsTable, u64>,
    pub total_table_rows: u64,
    /// Paid (padded to 2^k) rows per family and total, plus the wasted padding.
    pub table_paid_rows: HashMap<FropsTable, u64>,
    pub total_paid_rows: u64,
    pub total_waste: u64,
    pub baseline_nopad: Areas,
    pub proposed_nopad: Areas,
    pub baseline_pad: Areas,
    pub proposed_pad: Areas,
    /// Comparison against the FROPS implementation currently in the tree.
    pub current_covered: HashMap<u8, u64>,
    pub current_table_rows: HashMap<FropsTable, u64>,
    pub current_total_table_rows: u64,
    pub current_nopad: Areas,
    pub current_pad: Areas,
}

pub fn optimize(agg: &Aggregator, cfg: Config) -> Proposal {
    let weight = cfg.table_cost.saturating_mul(cfg.nodes);

    // 1. Build a Pareto frontier per (op, template) group.
    struct Group {
        info: OpInfo,
        options: Vec<Candidate>,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut op_total: HashMap<u8, u64> = HashMap::new();
    let mut op_info: HashMap<u8, OpInfo> = HashMap::new();
    for (&code, op_agg) in &agg.ops {
        let Some(info) = classify(code) else { continue };
        op_total.insert(code, op_agg.total);
        op_info.insert(code, info);
        for frontier in candidate_groups(op_agg, cfg.low_cap) {
            groups.push(Group { info, options: frontier });
        }
    }

    // 2. Convex-hull increments per group. The hull of (rows, gain=hits*cost) has decreasing marginal
    //    slope, so a single greedy pass over all increments sorted by slope is the standard
    //    multiple-choice-knapsack solution: each step upgrades one group to its next box size.
    struct Inc {
        group: usize,
        level: usize,
        d_rows: u64,
        d_gain: u128,
        region: Region,
        hits: u64,
    }
    let weight_u = weight as u128;
    let mut incs: Vec<Inc> = Vec::new();
    for (gi, g) in groups.iter().enumerate() {
        let cost = g.info.cost as u128;
        // Upper convex hull starting at the origin (the "select nothing" option).
        let mut hull: Vec<(u64, u128, Option<Candidate>)> = vec![(0, 0, None)];
        for c in &g.options {
            let r = c.region.rows();
            let gain = c.hits as u128 * cost;
            while hull.len() >= 2 {
                let (r1, g1, _) = hull[hull.len() - 2];
                let (r2, g2, _) = hull[hull.len() - 1];
                // Pop the middle point if it is not on the upper hull
                // (slope r2->new >= slope r1->r2).
                if (gain - g2) * (r2 - r1) as u128 >= (g2 - g1) * (r - r2) as u128 {
                    hull.pop();
                } else {
                    break;
                }
            }
            hull.push((r, gain, Some(*c)));
        }
        for w in 1..hull.len() {
            let (r0, g0, _) = hull[w - 1];
            let (r1, g1, c1) = hull[w];
            let c1 = c1.unwrap();
            incs.push(Inc {
                group: gi,
                level: w - 1,
                d_rows: r1 - r0,
                d_gain: g1 - g0,
                region: c1.region,
                hits: c1.hits,
            });
        }
    }

    // 3. Partition-aware greedy. Each family's table is padded to a multiple of 2^k rows, so table
    //    cost is charged per whole 2^k partition. Rows that fall inside an already-paid partition are
    //    free — so we fill that padding with extra coverage. To keep the membership test cheap, a
    //    *new* region is only opened when it pays for its rows at full cost (justifying the extra
    //    comparison); *growing* an already-selected region (same predicate, larger constants → zero
    //    extra comparisons) is taken whenever it fits, free inside a paid partition.
    incs.sort_by(|x, y| (y.d_gain * x.d_rows as u128).cmp(&(x.d_gain * y.d_rows as u128)));
    let s_part = 1u64 << cfg.partition_bits;
    let max_parts = cfg.max_table / s_part;
    let parts_of = |rows: u64| rows.div_ceil(s_part);
    let mut group_level = vec![0usize; groups.len()];
    let mut group_sel: Vec<Option<Candidate>> = vec![None; groups.len()];
    let mut fam_rows: HashMap<FropsTable, u64> = HashMap::new();
    let mut used_parts = 0u64;
    for inc in &incs {
        if group_level[inc.group] != inc.level {
            continue; // group frozen earlier; increments must apply in order
        }
        let fam = groups[inc.group].info.table;
        let cur = fam_rows.get(&fam).copied().unwrap_or(0);
        let new = cur + inc.d_rows;
        let d_part = parts_of(new) - parts_of(cur);
        if used_parts + d_part > max_parts {
            continue; // no partition budget for this step
        }
        let marg = d_part as u128 * s_part as u128 * weight_u; // table cost of new partitions
        let opens_region = inc.level == 0;
        let take = if opens_region {
            // A new region adds a comparison; require it to pay for its rows at full cost.
            inc.d_gain > weight_u * inc.d_rows as u128
        } else {
            // Growing an existing region: free inside a paid partition, else must pay the partition.
            inc.d_gain > marg
        };
        if take {
            fam_rows.insert(fam, new);
            used_parts += d_part;
            group_level[inc.group] += 1;
            group_sel[inc.group] = Some(Candidate { region: inc.region, hits: inc.hits });
        }
    }

    let mut selected: Vec<SelectedRegion> = Vec::new();
    let mut dropped: Vec<DroppedRegion> = Vec::new();
    for (gi, g) in groups.iter().enumerate() {
        let sel_hits = group_sel[gi].map(|c| c.hits).unwrap_or(0);
        if let Some(c) = group_sel[gi] {
            selected.push(SelectedRegion { info: g.info, region: c.region, hits: c.hits });
        }
        // Report the most-covering net-positive box we could not afford (what the budget left out).
        if let Some(ideal) = g
            .options
            .iter()
            .filter(|c| {
                (c.hits as u128) * (g.info.cost as u128) > weight_u * c.region.rows() as u128
            })
            .filter(|c| c.hits > sel_hits)
            .max_by_key(|c| c.hits)
        {
            dropped.push(DroppedRegion { info: g.info, region: ideal.region, hits: ideal.hits });
        }
    }

    // 3b. Cap regions per opcode so the membership test stays cheap: keep the highest-coverage
    //     regions per op, demote the rest to `dropped`.
    {
        let mut by_op: HashMap<u8, Vec<usize>> = HashMap::new();
        for (i, s) in selected.iter().enumerate() {
            by_op.entry(s.info.code).or_default().push(i);
        }
        let mut keep = vec![true; selected.len()];
        for idxs in by_op.values() {
            if idxs.len() > cfg.max_regions_per_op {
                let mut sorted = idxs.clone();
                sorted.sort_by_key(|&i| std::cmp::Reverse(selected[i].hits));
                for &i in &sorted[cfg.max_regions_per_op..] {
                    keep[i] = false;
                }
            }
        }
        let mut kept = Vec::new();
        for (i, s) in selected.into_iter().enumerate() {
            if keep[i] {
                kept.push(s);
            } else {
                dropped.push(DroppedRegion { info: s.info, region: s.region, hits: s.hits });
            }
        }
        selected = kept;
    }

    // 4. Per-op covered occurrences and per-table row totals.
    let mut op_covered: HashMap<u8, u64> = HashMap::new();
    let mut table_rows: HashMap<FropsTable, u64> = HashMap::new();
    let mut total_table_rows = 0u64;
    for s in &selected {
        *op_covered.entry(s.info.code).or_default() += s.hits;
        *table_rows.entry(s.info.table).or_default() += s.region.rows();
        total_table_rows += s.region.rows();
    }

    // 4b. Pad each family's table to a multiple of 2^k rows (the real, paid size).
    let pad = |rows: u64| rows.div_ceil(s_part) * s_part;
    let mut table_paid_rows: HashMap<FropsTable, u64> = HashMap::new();
    for t in FropsTable::all() {
        table_paid_rows.insert(t, pad(table_rows.get(&t).copied().unwrap_or(0)));
    }
    let total_paid_rows: u64 = table_paid_rows.values().sum();
    let total_waste = total_paid_rows - total_table_rows;

    // 5. Area model — the FROPS table area is charged on the *paid* (padded) rows.
    let table_area = (total_paid_rows as u128 * weight as u128).min(u64::MAX as u128) as u64;

    let proposed_nopad = Areas {
        instances: instances_area_nopad(&op_total, &op_covered, &op_info),
        table: table_area,
        total: 0,
    }
    .finalize();
    let baseline_nopad = Areas {
        instances: instances_area_nopad(&op_total, &HashMap::new(), &op_info),
        table: 0,
        total: 0,
    }
    .finalize();

    let proposed_pad = Areas {
        instances: instances_area_pad(&op_total, &op_covered, &op_info),
        table: table_area,
        total: 0,
    }
    .finalize();
    let baseline_pad = Areas {
        instances: instances_area_pad(&op_total, &HashMap::new(), &op_info),
        table: 0,
        total: 0,
    }
    .finalize();

    // 6. Comparison against the current FROPS implementation over the same data.
    let current_covered: HashMap<u8, u64> =
        agg.ops.iter().map(|(&code, a)| (code, a.current_covered)).collect();
    let current_table_rows = crate::current::table_rows();
    let current_total_table_rows: u64 = current_table_rows.values().sum();
    // Current tables are padded per family too, for a fair comparison.
    let current_paid: u64 = current_table_rows.values().map(|&r| pad(r)).sum();
    let current_table_area = (current_paid as u128 * weight as u128).min(u64::MAX as u128) as u64;
    let current_nopad = Areas {
        instances: instances_area_nopad(&op_total, &current_covered, &op_info),
        table: current_table_area,
        total: 0,
    }
    .finalize();
    let current_pad = Areas {
        instances: instances_area_pad(&op_total, &current_covered, &op_info),
        table: current_table_area,
        total: 0,
    }
    .finalize();

    Proposal {
        config: cfg,
        selected,
        dropped,
        op_total,
        op_covered,
        op_info,
        table_rows,
        total_table_rows,
        table_paid_rows,
        total_paid_rows,
        total_waste,
        baseline_nopad,
        proposed_nopad,
        baseline_pad,
        proposed_pad,
        current_covered,
        current_table_rows,
        current_total_table_rows,
        current_nopad,
        current_pad,
    }
}

impl Areas {
    fn finalize(mut self) -> Self {
        self.total = self.instances.saturating_add(self.table);
        self
    }
}

fn instances_area_nopad(
    total: &HashMap<u8, u64>,
    covered: &HashMap<u8, u64>,
    info: &HashMap<u8, OpInfo>,
) -> u64 {
    let mut area = 0u128;
    for (&code, &tot) in total {
        let cov = covered.get(&code).copied().unwrap_or(0).min(tot);
        let cost = info[&code].cost as u128;
        area += (tot - cov) as u128 * cost;
    }
    area.min(u64::MAX as u128) as u64
}

fn instances_area_pad(
    total: &HashMap<u8, u64>,
    covered: &HashMap<u8, u64>,
    info: &HashMap<u8, OpInfo>,
) -> u64 {
    // Used rows per state machine.
    let mut used: HashMap<Sm, u64> = HashMap::new();
    for (&code, &tot) in total {
        let cov = covered.get(&code).copied().unwrap_or(0).min(tot);
        *used.entry(info[&code].sm).or_default() += tot - cov;
    }
    let mut area = 0u128;
    for sm in Sm::all() {
        let u = used.get(&sm).copied().unwrap_or(0);
        if u == 0 {
            continue;
        }
        let num_rows = sm.num_rows();
        let instances = u.div_ceil(num_rows);
        area += instances as u128 * num_rows as u128 * sm.cost() as u128;
    }
    area.min(u64::MAX as u128) as u64
}
