//! DMA 64-bit loop occupancy analysis (`--dma-stats`).
//!
//! Every DMA operation (`memcpy`, `memset`, `memcmp`, `inputcpy`) is proven in three parts: the
//! unaligned head (*pre*), the unaligned tail (*post*), and in between a loop over whole 64-bit
//! words. This analyses that loop, which is where the bulk of the work is.
//!
//! The loop is proven in rows that pack several word operations each — `ops_x_row`, today 4 or 8
//! for the aligned machines and 1 for the unaligned one. A row belongs to a single DMA call, so
//! the last row of every call is padded: the wider the row, the fewer rows a long loop needs, but
//! the more slots a short loop wastes. This measures both sides for a list of candidate row widths
//! (`--dma-ops-x-row`), separately for the aligned and the unaligned loop, since they are proven by
//! different machines.
//!
//! The exact distribution of loop lengths is kept (one counter per length), so the rows a candidate
//! width needs are computed exactly afterwards rather than estimated.
//!
//! The loop length comes from the same computation the cost model uses (the `ops_dma_*` functions
//! of `zisk_core`). For a `memcmp` that found a difference and ends exactly on an 8-byte boundary
//! the planner moves one word of the loop into the tail, which this does not model, so such calls
//! are counted with one loop operation more than the planner would use.

use std::collections::HashMap;

use crate::StatsReport;

/// Number of DMA operation kinds tracked, indexed by the `DMA_LOOP_*` constants of `zisk_core`.
pub const DMA_LOOP_KINDS: usize = 4;

/// Name of each operation kind, in `DMA_LOOP_*` order.
const KIND_NAMES: [&str; DMA_LOOP_KINDS] = ["MEMCPY", "MEMSET", "MEMCMP", "INPUTCPY"];

/// Row widths measured when `--dma-ops-x-row` is not given.
pub const DEFAULT_DMA_OPS_X_ROW: &str = "2,4,8,12,16,24,32";

/// The loops of one operation kind with one alignment.
#[derive(Default, Debug, Clone)]
struct DmaLoopBucket {
    /// DMA calls that ran a loop of this shape.
    calls: u64,
    /// Word operations the rows have to hold, summed over every call.
    ops: u64,
    /// Calls by their number of word operations, so the rows any row width needs can be derived
    /// exactly. DMA calls are few enough for the exact distribution to be cheaper than useful.
    by_ops: HashMap<u32, u64>,
    /// Shortest and longest loop seen, `0` when nothing was seen.
    min: u32,
    max: u32,
}

impl DmaLoopBucket {
    fn add(&mut self, ops: u32) {
        self.calls += 1;
        self.ops += ops as u64;
        *self.by_ops.entry(ops).or_default() += 1;
        self.min = if self.min == 0 { ops } else { self.min.min(ops) };
        self.max = self.max.max(ops);
    }

    /// Merges `other` into this bucket, to total several operation kinds.
    fn merge(&mut self, other: &Self) {
        self.calls += other.calls;
        self.ops += other.ops;
        for (&ops, &calls) in &other.by_ops {
            *self.by_ops.entry(ops).or_default() += calls;
        }
        if other.min != 0 {
            self.min = if self.min == 0 { other.min } else { self.min.min(other.min) };
        }
        self.max = self.max.max(other.max);
    }

    /// Rows needed if every row holds `ops_x_row` word operations. A row never mixes two DMA
    /// calls, so each call pays for its own partial last row.
    fn rows(&self, ops_x_row: u32) -> u64 {
        self.by_ops.iter().map(|(&ops, &calls)| calls * ops.div_ceil(ops_x_row) as u64).sum()
    }
}

/// Occupancy of the DMA 64-bit loops, per operation kind and alignment.
#[derive(Debug)]
pub struct DmaLoopStats {
    /// Row widths to measure, ascending.
    ops_x_row: Vec<u32>,
    /// `buckets[kind][0]` = unaligned loops, `buckets[kind][1]` = aligned loops.
    buckets: [[DmaLoopBucket; 2]; DMA_LOOP_KINDS],
}

impl DmaLoopStats {
    /// Creates the analysis for the given row widths (a width of 0 is ignored; an empty list falls
    /// back to [`DEFAULT_DMA_OPS_X_ROW`]).
    pub fn new(ops_x_row: &[u32]) -> Self {
        let mut ops_x_row: Vec<u32> = ops_x_row.iter().copied().filter(|&r| r > 0).collect();
        ops_x_row.sort_unstable();
        ops_x_row.dedup();
        if ops_x_row.is_empty() {
            ops_x_row = parse_ops_x_row(DEFAULT_DMA_OPS_X_ROW).unwrap();
        }
        Self { ops_x_row, buckets: Default::default() }
    }

    /// Accounts the 64-bit loop of one DMA call: `kind` is a `DMA_LOOP_*` constant, `loop_count`
    /// the words the loop copies, sets or compares, and `aligned` whether destination and source
    /// share the same 8-byte offset.
    ///
    /// An aligned loop holds exactly those words. An unaligned one has to read one source word
    /// more than it writes, since every destination word is stitched from two source words, so it
    /// packs `loop_count + 1` operations — the same count the `dma_unaligned` planner uses today.
    pub fn on_loop(&mut self, kind: u8, loop_count: usize, aligned: bool) {
        debug_assert!((kind as usize) < DMA_LOOP_KINDS);
        if loop_count == 0 {
            return;
        }
        let ops = if aligned { loop_count } else { loop_count + 1 };
        self.buckets[kind as usize][aligned as usize].add(ops as u32);
    }

    /// Reports the occupancy of every row width, per operation kind and then totalled by
    /// alignment, followed by how the loop lengths are distributed. The narrowest width measured is
    /// the reference of the relative columns.
    pub fn report(&self, report: &mut StatsReport) {
        report.add(&format!(
            "\nDMA LOOP OCCUPANCY (row widths {})\n",
            self.ops_x_row.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")
        ));

        let mut totals = [DmaLoopBucket::default(), DmaLoopBucket::default()];
        let mut kinds_seen = [0usize; 2];
        for (kind, buckets) in self.buckets.iter().enumerate() {
            for (aligned, bucket) in buckets.iter().enumerate() {
                totals[aligned].merge(bucket);
                if bucket.calls > 0 {
                    kinds_seen[aligned] += 1;
                    self.report_bucket(report, KIND_NAMES[kind], aligned == 1, bucket);
                }
            }
        }
        // What a single machine per alignment would see. Skipped when one kind fed it on its own,
        // in which case it would only repeat that kind's table.
        for (aligned, bucket) in totals.iter().enumerate() {
            if kinds_seen[aligned] > 1 {
                self.report_bucket(report, "TOTAL", aligned == 1, bucket);
            }
        }
    }

    /// One occupancy table plus one loop-length distribution for a single bucket.
    fn report_bucket(
        &self,
        report: &mut StatsReport,
        name: &str,
        aligned: bool,
        bucket: &DmaLoopBucket,
    ) {
        let header = occupancy_row(&[
            "OPS/ROW".to_string(),
            "ROWS".to_string(),
            "SLOTS".to_string(),
            "OCCUP%".to_string(),
            "PAD".to_string(),
            "ROWS-%".to_string(),
            "SLOTS+%".to_string(),
        ]);
        let width = header.trim_end().len();

        report.add(&format!(
            "\n{} {} — {} calls, {} loop ops, {:.1} ops/call (min {}, max {})\n",
            name,
            if aligned { "ALIGNED" } else { "UNALIGNED" },
            report.format_number(bucket.calls),
            report.format_number(bucket.ops),
            bucket.ops as f64 / bucket.calls as f64,
            report.format_number(bucket.min as u64),
            report.format_number(bucket.max as u64),
        ));
        report.add(&header);
        report.add(&format!("{}\n", "-".repeat(width)));

        // Rows and slots of every candidate. The narrowest one is the reference of the two relative
        // columns: widening a row always cuts rows (`ROWS-%`, the fixed per-row cost it saves) and
        // always adds padding (`SLOTS+%`, the slots it wastes). Those two numbers are the whole
        // trade-off, since a row of R operations costs a fixed part plus R times a per-operation
        // part.
        let measures: Vec<(u32, u64, u64)> = self
            .ops_x_row
            .iter()
            .map(|&r| {
                let rows = bucket.rows(r);
                (r, rows, rows * r as u64)
            })
            .collect();
        let (ref_rows, ref_slots) =
            measures.first().map_or((0, 0), |&(_, rows, slots)| (rows, slots));

        for &(ops_x_row, rows, slots) in &measures {
            report.add(&occupancy_row(&[
                ops_x_row.to_string(),
                report.format_number(rows),
                report.format_number(slots),
                perc(bucket.ops, slots),
                report.format_number(slots - bucket.ops),
                relative(rows, ref_rows),
                relative(slots, ref_slots),
            ]));
        }
        self.report_distribution(report, bucket, width);
    }

    /// How the calls and the word operations spread over the loop lengths, in power-of-two buckets:
    /// this is what decides the padding of a given row width.
    fn report_distribution(&self, report: &mut StatsReport, bucket: &DmaLoopBucket, width: usize) {
        let header = distribution_row(&[
            "OPS/CALL".to_string(),
            "CALLS".to_string(),
            "%".to_string(),
            "LOOP OPS".to_string(),
            "%".to_string(),
        ]);
        report.add(&format!("{}\n", "-".repeat(width)));
        report.add(&header);

        // One bucket per power of two up to the longest loop seen.
        let mut low = 1u32;
        while low <= bucket.max {
            let high = low.saturating_mul(2) - 1;
            let (calls, ops) = bucket
                .by_ops
                .iter()
                .filter(|(&ops, _)| ops >= low && ops <= high)
                .fold((0u64, 0u64), |acc, (&ops, &calls)| {
                    (acc.0 + calls, acc.1 + calls * ops as u64)
                });
            if calls > 0 {
                let label = if low == high {
                    low.to_string()
                } else {
                    format!("{}-{}", low, high.min(bucket.max))
                };
                report.add(&distribution_row(&[
                    label,
                    report.format_number(calls),
                    perc(calls, bucket.calls),
                    report.format_number(ops),
                    perc(ops, bucket.ops),
                ]));
            }
            low = high + 1;
        }
    }
}

/// Parses a comma-separated list of row widths, e.g. `"2,4,8"`. Returns the offending item on
/// error.
pub fn parse_ops_x_row(list: &str) -> Result<Vec<u32>, String> {
    let mut widths = Vec::new();
    for item in list.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        match item.parse::<u32>() {
            Ok(0) => return Err(format!("'{item}' is not a valid row width, it must be >= 1")),
            Ok(width) => widths.push(width),
            Err(_) => return Err(format!("'{item}' is not a number")),
        }
    }
    if widths.is_empty() {
        return Err("no row width given".to_string());
    }
    Ok(widths)
}

/// `value` as a signed percentage change from `reference`, `-` when there is no reference.
fn relative(value: u64, reference: u64) -> String {
    if reference == 0 {
        "-".to_string()
    } else {
        format!("{:+.2}%", 100.0 * (value as f64 - reference as f64) / reference as f64)
    }
}

/// `value` as a percentage of `total`, `-` when there is nothing to compare against.
fn perc(value: u64, total: u64) -> String {
    if total == 0 {
        "-".to_string()
    } else {
        format!("{:.2}%", 100.0 * value as f64 / total as f64)
    }
}

/// One row of the occupancy table, already formatted.
fn occupancy_row(cells: &[String; 7]) -> String {
    format!(
        "{:<9} {:>14} {:>14} {:>9} {:>14} {:>10} {:>10}\n",
        cells[0], cells[1], cells[2], cells[3], cells[4], cells[5], cells[6]
    )
}

/// One row of the loop-length distribution table, already formatted.
fn distribution_row(cells: &[String; 5]) -> String {
    format!("{:<15} {:>14} {:>9} {:>14} {:>9}\n", cells[0], cells[1], cells[2], cells[3], cells[4])
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_core::zisk_ops::{DMA_LOOP_MEMCPY, DMA_LOOP_MEMSET};

    fn stats(widths: &[u32]) -> DmaLoopStats {
        DmaLoopStats::new(widths)
    }

    #[test]
    fn rows_are_counted_per_call_so_every_call_pays_its_partial_last_row() {
        let mut dma = stats(&[4]);
        // Three aligned loops of 5 words: each needs two rows of 4, the second one 3/4 empty.
        for _ in 0..3 {
            dma.on_loop(DMA_LOOP_MEMCPY, 5, true);
        }
        let bucket = &dma.buckets[DMA_LOOP_MEMCPY as usize][1];
        assert_eq!((bucket.calls, bucket.ops), (3, 15));
        assert_eq!(bucket.rows(4), 6);
        // A single loop of 15 words would have needed 4 rows instead of 6 for the same 15 words.
        let mut dma = stats(&[4]);
        dma.on_loop(DMA_LOOP_MEMCPY, 15, true);
        assert_eq!(dma.buckets[DMA_LOOP_MEMCPY as usize][1].rows(4), 4);
    }

    #[test]
    fn an_unaligned_loop_packs_one_operation_more_than_it_writes() {
        let mut dma = stats(&[2]);
        dma.on_loop(DMA_LOOP_MEMCPY, 4, false);
        let unaligned = &dma.buckets[DMA_LOOP_MEMCPY as usize][0];
        // Four destination words, five source reads.
        assert_eq!((unaligned.calls, unaligned.ops), (1, 5));
        assert_eq!(unaligned.rows(1), 5); // what the dma_unaligned planner counts today
        assert_eq!(unaligned.rows(2), 3);
        // The aligned bucket of the same operation is untouched.
        assert_eq!(dma.buckets[DMA_LOOP_MEMCPY as usize][1].calls, 0);
    }

    #[test]
    fn a_call_without_a_loop_is_not_accounted() {
        let mut dma = stats(&[4]);
        dma.on_loop(DMA_LOOP_MEMCPY, 0, true);
        dma.on_loop(DMA_LOOP_MEMCPY, 0, false);
        assert_eq!(dma.buckets[DMA_LOOP_MEMCPY as usize][1].calls, 0);
        assert_eq!(dma.buckets[DMA_LOOP_MEMCPY as usize][0].calls, 0);
    }

    #[test]
    fn wider_rows_need_fewer_rows_but_offer_more_slots() {
        let mut dma = stats(&[2, 4, 8]);
        // Loops of 4 words: they fill a row of 4 exactly and leave a row of 8 half empty.
        for _ in 0..10 {
            dma.on_loop(DMA_LOOP_MEMSET, 4, true);
        }
        let bucket = &dma.buckets[DMA_LOOP_MEMSET as usize][1];
        assert_eq!(bucket.ops, 40);
        assert_eq!((bucket.rows(2), bucket.rows(4), bucket.rows(8)), (20, 10, 10));
        // Slots: 40, 40 and 80 — a row of 8 doubles the trace for the same work.
        assert_eq!(bucket.rows(8) * 8, 80);
    }

    #[test]
    fn the_length_range_and_the_totals_cover_every_kind() {
        let mut dma = stats(&[4]);
        dma.on_loop(DMA_LOOP_MEMCPY, 3, true);
        dma.on_loop(DMA_LOOP_MEMCPY, 20, true);
        dma.on_loop(DMA_LOOP_MEMSET, 7, true);

        let memcpy = &dma.buckets[DMA_LOOP_MEMCPY as usize][1];
        assert_eq!((memcpy.min, memcpy.max), (3, 20));

        let mut total = DmaLoopBucket::default();
        for buckets in dma.buckets.iter() {
            total.merge(&buckets[1]);
        }
        assert_eq!((total.calls, total.ops, total.min, total.max), (3, 30, 3, 20));
        // 3 -> 1 row, 20 -> 5 rows, 7 -> 2 rows.
        assert_eq!(total.rows(4), 8);
    }

    #[test]
    fn row_widths_are_normalized_and_the_list_is_parsed() {
        assert_eq!(parse_ops_x_row("2, 4,8").unwrap(), vec![2, 4, 8]);
        assert!(parse_ops_x_row("2,0").is_err());
        assert!(parse_ops_x_row("2,x").is_err());
        assert!(parse_ops_x_row("").is_err());
        assert_eq!(parse_ops_x_row(DEFAULT_DMA_OPS_X_ROW).unwrap(), vec![2, 4, 8, 12, 16, 24, 32]);

        // Duplicates and zeros are dropped, and the list is sorted.
        assert_eq!(stats(&[8, 2, 0, 8]).ops_x_row, vec![2, 8]);
        // An empty list falls back to the default.
        assert_eq!(stats(&[]).ops_x_row, vec![2, 4, 8, 12, 16, 24, 32]);
    }
}
