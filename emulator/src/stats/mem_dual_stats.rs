//! Memory dual-row occupancy analysis (`--mem-duals`).
//!
//! A *dual* row of the memory state machine holds, in a single row, several memory operations that
//! share the same address and the same value: one read or write followed by up to `D` reads (the
//! `D` of `--mem-duals`; `D = 1` is the standard dual, `D = 0` disables the mechanism). Only the
//! step of each operation differs, so a write — which changes the value — always opens a new row,
//! while a read can join the row already open for its address as long as a dual slot is free.
//!
//! Rows cannot span two chunks, because the count-and-plan phase works per chunk: every
//! 2^`--mem-dual-reset-bits` steps the open rows are dropped and the next operation on an address
//! has to open a new one. `RESET LOST` counts exactly the reads that would have been duals if that
//! boundary did not exist, and `FULL LOST` the ones that had to open a new row because every dual
//! slot of the open row was already taken (i.e. what a larger `D` would absorb).
//!
//! Registers are not tracked: they are proven by the main state machine, not by the memory one, so
//! only the three memory zones proven as memory take part — input (128M addresses), ROM data (16M)
//! and RAM (64M). The very first address of the input zone is the exception inside them: it is the
//! free-input address, whose reads are marked `is_free_read` by the input-data state machine because
//! each one yields a different value, so two of its operations can never share a row. Tracking state is one 8-byte entry per address, allocated in lazily zeroed pages
//! of 2^`PAGE_BITS` addresses, so only the pages a program really touches are reserved.

use crate::StatsReport;
use zisk_core::{
    FREE_INPUT_ADDR, INPUT_ADDR, MAX_INPUT_SIZE, RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_SIZE,
};

/// Addresses (8-byte words) per lazily allocated page: 2^18 addresses = 2 MiB of tracking state.
const PAGE_BITS: u32 = 18;
const PAGE_WORDS: usize = 1 << PAGE_BITS;

// Packed per-address entry. An all-zero entry means "no row open for this address", so a page can
// be allocated zeroed:
//   bits  0..=47  reset block of the open row, stored as `block + 1` (0 is the empty marker).
//   bits 48..=62  dual slots already consumed by the open row.
//   bit  63       set when the operation that opened the row was a write.
const BLOCK_MASK: u64 = (1 << 48) - 1;
const USED_SHIFT: u32 = 48;
const USED_MASK: u64 = 0x7FFF;
const WRITE_FLAG: u64 = 1 << 63;

/// Largest number of duals per row a packed entry can count.
pub const MEM_DUAL_MAX: u32 = USED_MASK as u32;

/// The memory zones accounted separately, in report order: name, first address, size, and the one
/// address of the zone that can never take part in a dual row (`u64::MAX` when it has none).
const ZONES: [(&str, u64, u64, u64); 3] = [
    ("INPUT", INPUT_ADDR, MAX_INPUT_SIZE, FREE_INPUT_ADDR),
    ("ROM", ROM_ADDR, ROM_SIZE, u64::MAX),
    ("RAM", RAM_ADDR, RAM_SIZE, u64::MAX),
];

/// Dual occupancy of one memory zone: the tracking state of every address of the zone plus its
/// counters.
struct ZoneDuals {
    /// First 8-byte address of the zone, the base of the page index.
    base_word: u64,
    /// Address (8-byte word) of this zone that can never take part in a dual row, `u64::MAX` when
    /// it has none. The free-input address is the only one: memory does not hold its value, the
    /// input provides a different one on every read, so no two of its operations share a row.
    no_dual_word: u64,
    /// One entry per address of the zone; a page is an empty vector until first touched.
    pages: Vec<Vec<u64>>,
    /// Aligned memory operations accounted, i.e. `rows` + duals used.
    ops: u64,
    reads: u64,
    writes: u64,
    /// Memory rows used, i.e. operations that could not join an already open row.
    rows: u64,
    /// `rows_by_duals[k]` = rows currently holding `k` duals, split by the kind of the operation
    /// that opened them: `[0]` opened by a read, `[1]` opened by a write. Kept up to date on every
    /// operation, so it is exact at any point without a final flush.
    rows_by_duals: Vec<[u64; 2]>,
    /// Reads that opened a new row although the row open for their address still had free dual
    /// slots, only because that row belonged to a previous reset block.
    reset_lost: u64,
    /// Reads that opened a new row because every dual slot of the open row was already taken.
    full_lost: u64,
    /// Distinct addresses touched.
    addresses: u64,
}

impl ZoneDuals {
    fn new(base: u64, size: u64, no_dual_addr: u64, duals: u32) -> Self {
        let words = (size / 8) as usize;
        Self {
            base_word: base >> 3,
            no_dual_word: if no_dual_addr == u64::MAX { u64::MAX } else { no_dual_addr >> 3 },
            pages: vec![Vec::new(); words.div_ceil(PAGE_WORDS)],
            ops: 0,
            reads: 0,
            writes: 0,
            rows: 0,
            rows_by_duals: vec![[0; 2]; duals as usize + 1],
            reset_lost: 0,
            full_lost: 0,
            addresses: 0,
        }
    }

    /// Accounts one aligned memory operation on `word` (an 8-byte address of this zone) belonging
    /// to reset `block` (already stored as `block + 1`).
    #[inline]
    fn account(&mut self, word: u64, is_write: bool, block: u64, duals: u32) {
        let index = (word - self.base_word) as usize;
        let page = index >> PAGE_BITS;
        let offset = index & (PAGE_WORDS - 1);

        self.ops += 1;
        if is_write {
            self.writes += 1;
        } else {
            self.reads += 1;
        }

        let entry = {
            let page = &mut self.pages[page];
            if page.is_empty() {
                *page = vec![0u64; PAGE_WORDS];
            }
            page[offset]
        };

        if entry == 0 {
            self.addresses += 1;
        } else if !is_write && word != self.no_dual_word {
            let used = ((entry >> USED_SHIFT) & USED_MASK) as u32;
            if used < duals {
                if (entry & BLOCK_MASK) == block {
                    // The read joins the row already open for this address: one operation more in
                    // the same row, so it moves from the `used` bucket to the next one.
                    let kind = ((entry & WRITE_FLAG) != 0) as usize;
                    self.rows_by_duals[used as usize][kind] -= 1;
                    self.rows_by_duals[used as usize + 1][kind] += 1;
                    self.pages[page][offset] = entry + (1 << USED_SHIFT);
                    return;
                }
                // The open row had room, but the chunk boundary closed it.
                self.reset_lost += 1;
            } else if (entry & BLOCK_MASK) == block {
                self.full_lost += 1;
            }
        }

        // A new row: writes always open one, reads only when they cannot join the open row.
        self.pages[page][offset] = block | if is_write { WRITE_FLAG } else { 0 };
        self.rows += 1;
        self.rows_by_duals[0][is_write as usize] += 1;
    }

    /// Dual slots offered by the rows used, i.e. `rows * (1 + D)`.
    fn capacity(&self, duals: u32) -> u64 {
        self.rows * (1 + duals as u64)
    }
}

/// Dual-row occupancy of the whole memory, one independent accounting per zone.
pub struct MemDualStats {
    /// Duals allowed per row (`D`): 0 disables the mechanism, 1 is the standard dual.
    duals: u32,
    /// A row cannot span two chunks of 2^`reset_bits` steps.
    reset_bits: u32,
    zones: [ZoneDuals; ZONES.len()],
}

impl std::fmt::Debug for MemDualStats {
    /// Elides the per-address tracking state, which is far too large to print, and shows only the
    /// configuration and the per-zone counters.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemDualStats")
            .field("duals", &self.duals)
            .field("reset_bits", &self.reset_bits)
            .field("ops", &self.zones.iter().map(|z| z.ops).collect::<Vec<_>>())
            .field("rows", &self.zones.iter().map(|z| z.rows).collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl MemDualStats {
    /// Creates the analysis for `duals` duals per row (clamped to [`MEM_DUAL_MAX`]) and a reset
    /// period of 2^`reset_bits` steps.
    pub fn new(duals: u32, reset_bits: u32) -> Self {
        let duals = duals.min(MEM_DUAL_MAX);
        Self {
            duals,
            reset_bits: reset_bits.min(47),
            zones: std::array::from_fn(|z| {
                ZoneDuals::new(ZONES[z].1, ZONES[z].2, ZONES[z].3, duals)
            }),
        }
    }

    /// Accounts the aligned memory operations generated by one program-level memory access of
    /// `width` bytes at `address`, executed at `step`. Accesses outside the three tracked zones
    /// (registers included) are ignored; an invalid address is already reported by the cost path.
    ///
    /// An aligned 8-byte access is a single memory operation. Any other access is resolved by the
    /// memory-align state machine, which reads every 8-byte address it touches (one, or two when
    /// the access crosses the boundary) and writes back the ones it modifies.
    #[inline]
    pub fn on_access(&mut self, address: u64, width: u64, is_write: bool, step: u64) {
        let Some(zone) = ZONES.iter().position(|&(_, base, size, _)| {
            address >= base && address - base < size && address - base + width <= size
        }) else {
            return;
        };
        debug_assert!((1..=8).contains(&width));

        let block = (step >> self.reset_bits) + 1;
        debug_assert!(block <= BLOCK_MASK, "step {step} overflows the reset block field");
        let zone = &mut self.zones[zone];
        let duals = self.duals;

        if width == 8 && (address & 0x7) == 0 {
            zone.account(address >> 3, is_write, block, duals);
            return;
        }
        for word in (address >> 3)..=((address + width - 1) >> 3) {
            zone.account(word, false, block, duals);
            if is_write {
                zone.account(word, true, block, duals);
            }
        }
    }

    /// Reports the dual occupancy: a per-zone summary of how many memory rows the operations needed
    /// and how full those rows are, followed by the distribution of the rows by the number of duals
    /// they ended up holding.
    pub fn report(&self, report: &mut StatsReport) {
        let duals = self.duals;
        let header = summary_row_str(&[
            "ZONE".to_string(),
            "OPS".to_string(),
            "READS".to_string(),
            "WRITES".to_string(),
            "ROWS".to_string(),
            "OCCUP%".to_string(),
            "SAVED%".to_string(),
            "RESET LOST".to_string(),
            "FULL LOST".to_string(),
        ]);
        let width = header.trim_end().len();

        report.add(&format!(
            "\nMEM DUAL OCCUPANCY ({} dual{} per row, {} ops per row, reset 2^{} = {} steps)\n",
            duals,
            if duals == 1 { "" } else { "s" },
            1 + duals,
            self.reset_bits,
            report.format_number(1u64 << self.reset_bits),
        ));
        report.add(&header);
        report.add(&format!("{}\n", "-".repeat(width)));

        for (z, zone) in self.zones.iter().enumerate() {
            if zone.ops == 0 {
                continue;
            }
            report.add(&summary_row(
                report,
                ZONES[z].0,
                [
                    zone.ops,
                    zone.reads,
                    zone.writes,
                    zone.rows,
                    zone.capacity(duals),
                    zone.reset_lost,
                    zone.full_lost,
                ],
            ));
        }
        let total = |f: &dyn Fn(&ZoneDuals) -> u64| -> u64 { self.zones.iter().map(f).sum() };
        report.add(&format!("{}\n", "-".repeat(width)));
        report.add(&summary_row(
            report,
            "TOTAL",
            [
                total(&|z| z.ops),
                total(&|z| z.reads),
                total(&|z| z.writes),
                total(&|z| z.rows),
                total(&|z| z.capacity(duals)),
                total(&|z| z.reset_lost),
                total(&|z| z.full_lost),
            ],
        ));

        self.report_distribution(report);
    }

    /// Reports, per zone, how many rows ended up holding each number of duals, split by whether the
    /// operation that opened the row was a read or a write.
    fn report_distribution(&self, report: &mut StatsReport) {
        let header = dist_row(&[
            "DUALS".to_string(),
            "ROWS".to_string(),
            "%ROWS".to_string(),
            "READ FIRST".to_string(),
            "%".to_string(),
            "WRITE FIRST".to_string(),
            "%".to_string(),
        ]);
        let width = header.trim_end().len();

        for (z, zone) in self.zones.iter().enumerate() {
            if zone.ops == 0 {
                continue;
            }
            report.add(&format!(
                "\nMEM DUAL DISTRIBUTION {} ({} addresses touched)\n",
                ZONES[z].0,
                report.format_number(zone.addresses),
            ));
            report.add(&header);
            report.add(&format!("{}\n", "-".repeat(width)));
            let (mut t_read, mut t_write) = (0u64, 0u64);
            for (used, &[read, write]) in zone.rows_by_duals.iter().enumerate() {
                report.add(&dist_row(&[
                    used.to_string(),
                    report.format_number(read + write),
                    perc(read + write, zone.rows),
                    report.format_number(read),
                    perc(read, read + write),
                    report.format_number(write),
                    perc(write, read + write),
                ]));
                t_read += read;
                t_write += write;
            }
            report.add(&format!("{}\n", "-".repeat(width)));
            report.add(&dist_row(&[
                "TOTAL".to_string(),
                report.format_number(t_read + t_write),
                perc(t_read + t_write, zone.rows),
                report.format_number(t_read),
                perc(t_read, t_read + t_write),
                report.format_number(t_write),
                perc(t_write, t_read + t_write),
            ]));
        }
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

/// One row of the per-zone summary table, already formatted.
fn summary_row_str(cells: &[String; 9]) -> String {
    format!(
        "{:<8} {:>15} {:>15} {:>15} {:>15} {:>8} {:>8} {:>15} {:>15}\n",
        cells[0], cells[1], cells[2], cells[3], cells[4], cells[5], cells[6], cells[7], cells[8]
    )
}

/// One data row of the per-zone summary table: `values` are ops, reads, writes, rows, capacity,
/// reset-lost and full-lost; the occupancy and saving percentages are derived from them.
fn summary_row(report: &StatsReport, name: &str, values: [u64; 7]) -> String {
    let [ops, reads, writes, rows, capacity, reset_lost, full_lost] = values;
    summary_row_str(&[
        name.to_string(),
        report.format_number(ops),
        report.format_number(reads),
        report.format_number(writes),
        report.format_number(rows),
        perc(ops, capacity),
        perc(ops - rows, ops),
        report.format_number(reset_lost),
        report.format_number(full_lost),
    ])
}

/// One row of the per-zone dual-distribution table, already formatted.
fn dist_row(cells: &[String; 7]) -> String {
    format!(
        "{:<8} {:>15} {:>8} {:>15} {:>8} {:>15} {:>8}\n",
        cells[0], cells[1], cells[2], cells[3], cells[4], cells[5], cells[6]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Aligned 8-byte access at `address`, the single-operation case.
    fn access(duals: &mut MemDualStats, address: u64, is_write: bool, step: u64) {
        duals.on_access(address, 8, is_write, step);
    }

    fn ram(duals: &MemDualStats) -> &ZoneDuals {
        &duals.zones[2]
    }

    #[test]
    fn reads_join_the_open_row_until_its_dual_slots_are_taken() {
        let mut duals = MemDualStats::new(1, 8);
        access(&mut duals, RAM_ADDR, true, 0); // opens a row, first operation a write
        access(&mut duals, RAM_ADDR, false, 1); // joins it as its only dual
        access(&mut duals, RAM_ADDR, false, 2); // no slot left: opens a row of its own
        access(&mut duals, RAM_ADDR, false, 3); // joins that one

        let ram = ram(&duals);
        assert_eq!((ram.ops, ram.reads, ram.writes, ram.rows), (4, 3, 1, 2));
        assert_eq!((ram.reset_lost, ram.full_lost, ram.addresses), (0, 1, 1));
        // Both rows ended up full: one opened by a read, one opened by a write.
        assert_eq!(ram.rows_by_duals, vec![[0, 0], [1, 1]]);
        assert_eq!(ram.capacity(1), 4);
    }

    #[test]
    fn a_write_always_opens_a_row_because_it_changes_the_value() {
        let mut duals = MemDualStats::new(3, 8);
        access(&mut duals, RAM_ADDR, false, 0);
        access(&mut duals, RAM_ADDR, true, 1);
        access(&mut duals, RAM_ADDR, true, 2);

        let ram = ram(&duals);
        assert_eq!((ram.ops, ram.rows, ram.full_lost), (3, 3, 0));
        // Three empty rows: the first opened by a read, the other two by a write.
        assert_eq!(ram.rows_by_duals, vec![[1, 2], [0, 0], [0, 0], [0, 0]]);
    }

    #[test]
    fn a_row_does_not_span_two_reset_blocks() {
        let mut duals = MemDualStats::new(1, 4); // blocks of 16 steps
        access(&mut duals, RAM_ADDR, true, 0); // opens a row with its dual slot free
        access(&mut duals, RAM_ADDR, false, 16); // next block: that free slot is unreachable
        access(&mut duals, RAM_ADDR, false, 17); // joins the row opened at step 16

        let ram = ram(&duals);
        assert_eq!((ram.ops, ram.rows), (3, 2));
        // Only the read at step 16 is lost to the boundary; the one at step 17 found a free slot.
        assert_eq!((ram.reset_lost, ram.full_lost), (1, 0));
    }

    #[test]
    fn a_read_that_would_not_have_fit_anyway_is_not_blamed_on_the_reset() {
        let mut duals = MemDualStats::new(1, 4);
        access(&mut duals, RAM_ADDR, true, 0);
        access(&mut duals, RAM_ADDR, false, 15); // fills the only dual slot of the row
        access(&mut duals, RAM_ADDR, false, 16); // next block, but the row was already full

        let ram = ram(&duals);
        assert_eq!((ram.ops, ram.rows), (3, 2));
        assert_eq!((ram.reset_lost, ram.full_lost), (0, 0));
    }

    #[test]
    fn without_duals_every_operation_needs_its_own_row() {
        let mut duals = MemDualStats::new(0, 8);
        for step in 0..3 {
            access(&mut duals, RAM_ADDR, false, step);
        }
        let ram = ram(&duals);
        assert_eq!((ram.ops, ram.rows), (3, 3));
        assert_eq!(ram.capacity(0), 3);
        // Nothing is lost to the reset boundary, and the two repeated reads report what a dual row
        // would have absorbed.
        assert_eq!((ram.reset_lost, ram.full_lost), (0, 2));
        assert_eq!(ram.rows_by_duals, vec![[3, 0]]);
    }

    #[test]
    fn an_unaligned_access_is_accounted_on_every_address_it_touches() {
        // A 4-byte write inside one address: memory-align reads the address and writes it back.
        let mut duals = MemDualStats::new(1, 8);
        duals.on_access(RAM_ADDR + 1, 4, true, 0);
        let zone = ram(&duals);
        assert_eq!((zone.ops, zone.reads, zone.writes, zone.rows, zone.addresses), (2, 1, 1, 2, 1));

        // The same write crossing the 8-byte boundary: both addresses are read and written.
        let mut duals = MemDualStats::new(1, 8);
        duals.on_access(RAM_ADDR + 6, 4, true, 0);
        let zone = ram(&duals);
        assert_eq!((zone.ops, zone.reads, zone.writes, zone.rows, zone.addresses), (4, 2, 2, 4, 2));

        // An unaligned read crossing it is just two reads.
        let mut duals = MemDualStats::new(1, 8);
        duals.on_access(RAM_ADDR + 7, 2, false, 0);
        let zone = ram(&duals);
        assert_eq!((zone.ops, zone.reads, zone.writes, zone.rows, zone.addresses), (2, 2, 0, 2, 2));
    }

    #[test]
    fn the_free_input_address_never_takes_part_in_a_dual() {
        let mut duals = MemDualStats::new(1, 8);
        for step in 0..4 {
            access(&mut duals, FREE_INPUT_ADDR, false, step);
        }
        // The next input address duals like any other, so the zone is not excluded as a whole.
        access(&mut duals, FREE_INPUT_ADDR + 8, false, 4);
        access(&mut duals, FREE_INPUT_ADDR + 8, false, 5);

        let input = &duals.zones[0];
        assert_eq!((input.ops, input.rows, input.addresses), (6, 5, 2));
        // Nothing is reported as lost: no width of row could have packed those free reads.
        assert_eq!((input.reset_lost, input.full_lost), (0, 0));
        assert_eq!(input.rows_by_duals, vec![[4, 0], [1, 0]]);
    }

    #[test]
    fn every_zone_is_accounted_on_its_own_and_registers_are_left_out() {
        let mut duals = MemDualStats::new(1, 8);
        access(&mut duals, INPUT_ADDR + 8, false, 0);
        access(&mut duals, INPUT_ADDR + 8, false, 1);
        access(&mut duals, ROM_ADDR, false, 2);
        access(&mut duals, RAM_ADDR, true, 3);
        access(&mut duals, 0x1000, false, 4); // BIOS area: not proven as memory, ignored

        let ops: Vec<u64> = duals.zones.iter().map(|z| z.ops).collect();
        let rows: Vec<u64> = duals.zones.iter().map(|z| z.rows).collect();
        assert_eq!(ops, vec![2, 1, 1]);
        assert_eq!(rows, vec![1, 1, 1]);
    }

    #[test]
    fn addresses_of_the_same_zone_do_not_interfere() {
        let mut duals = MemDualStats::new(1, 8);
        // Two addresses far enough apart to land on different tracking pages.
        let other = RAM_ADDR + (PAGE_WORDS as u64 + 1) * 8;
        access(&mut duals, RAM_ADDR, true, 0);
        access(&mut duals, other, true, 1);
        access(&mut duals, RAM_ADDR, false, 2); // joins the row of its own address
        access(&mut duals, other, false, 3);

        let ram = ram(&duals);
        assert_eq!((ram.ops, ram.rows, ram.addresses), (4, 2, 2));
        assert_eq!(ram.rows_by_duals, vec![[0, 0], [0, 2]]);
    }
}
