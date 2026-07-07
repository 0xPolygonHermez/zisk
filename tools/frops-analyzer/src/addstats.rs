//! One-off analysis: per block (file), how many non-FROPS 64-bit ADDs have both operands' high 32
//! bits zero (so the high half need not be computed), as a share of the non-FROPS ADDs.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sm_arith::ArithFrops;
use sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};
use zisk_core::zisk_ops::ZiskOp;

use crate::ingest::RECORD_SIZE;
use crate::ops::classify;

const ADD: u8 = ZiskOp::ADD; // 0x0a
const EQ: u8 = ZiskOp::EQ; // 0x09

#[derive(Default, Clone, Copy)]
struct AddStats {
    add_total: u64,
    add_frop: u64,
    add_nonfrop: u64,
    /// non-FROPS adds with hi32(a)==0 && hi32(b)==0 (operands fit in 32 bits).
    hi0_operands: u64,
    /// subset where additionally a+b < 2^32 (no carry into the high half at all).
    hi0_result: u64,
}

impl AddStats {
    fn add(&mut self, other: &AddStats) {
        self.add_total += other.add_total;
        self.add_frop += other.add_frop;
        self.add_nonfrop += other.add_nonfrop;
        self.hi0_operands += other.hi0_operands;
        self.hi0_result += other.hi0_result;
    }
}

fn scan_file(path: &Path) -> std::io::Result<AddStats> {
    let mut s = AddStats::default();
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut buf = [0u8; RECORD_SIZE * 4096];
    let mut carry = 0usize;
    loop {
        let n = reader.read(&mut buf[carry..])?;
        if n == 0 {
            break;
        }
        let available = carry + n;
        let full = available / RECORD_SIZE;
        for i in 0..full {
            let off = i * RECORD_SIZE;
            if buf[off] != ADD {
                continue;
            }
            let a = u64::from_le_bytes(buf[off + 1..off + 9].try_into().unwrap());
            let b = u64::from_le_bytes(buf[off + 9..off + 17].try_into().unwrap());
            s.add_total += 1;
            if BinaryBasicFrops::is_frequent_op(ADD, a, b) {
                s.add_frop += 1;
                continue;
            }
            s.add_nonfrop += 1;
            if (a >> 32) == 0 && (b >> 32) == 0 {
                s.hi0_operands += 1;
                if a + b < (1u64 << 32) {
                    s.hi0_result += 1;
                }
            }
        }
        let consumed = full * RECORD_SIZE;
        carry = available - consumed;
        buf.copy_within(consumed..available, 0);
    }
    Ok(s)
}

fn pct(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        100.0 * part as f64 / whole as f64
    }
}

/// Rows per instance (BinaryAdd trace NUM_ROWS = 2^22).
const INSTANCE_ROWS: u64 = 1 << 22;

// ---------------------------------------------------------------------------------------------
// Distribution analysis of the hi0-no-carry subset.
// ---------------------------------------------------------------------------------------------

const TOP_CAP: usize = 1 << 21;

const A_PAGE_SHIFT: u32 = 12; // 4 KB pages for the address-range analysis

struct Dist {
    count: u64,
    a_bits: [u64; 33], // a_bits[k] = #values whose bit-length is k (k=0 means value 0)
    b_bits: [u64; 33],
    joint: [[u64; 5]; 5], // band(a) x band(b)
    top_a: std::collections::HashMap<u64, u64>,
    top_b: std::collections::HashMap<u64, u64>,
    top_ab: std::collections::HashMap<(u64, u64), u64>,
    a_pages: std::collections::HashMap<u64, u64>, // a >> A_PAGE_SHIFT -> count
}

impl Default for Dist {
    fn default() -> Self {
        Self {
            count: 0,
            a_bits: [0; 33],
            b_bits: [0; 33],
            joint: [[0; 5]; 5],
            top_a: std::collections::HashMap::new(),
            top_b: std::collections::HashMap::new(),
            top_ab: std::collections::HashMap::new(),
            a_pages: std::collections::HashMap::new(),
        }
    }
}

fn bit_len(v: u64) -> usize {
    (64 - v.leading_zeros()) as usize // 0 for v==0
}

/// Coarse magnitude band: 0 (zero), 1 (1-8 bits), 2 (9-16), 3 (17-24), 4 (25-32).
fn band(bits: usize) -> usize {
    match bits {
        0 => 0,
        1..=8 => 1,
        9..=16 => 2,
        17..=24 => 3,
        _ => 4,
    }
}

fn bump<K: std::hash::Hash + Eq>(m: &mut std::collections::HashMap<K, u64>, k: K) {
    if let Some(v) = m.get_mut(&k) {
        *v += 1;
    } else if m.len() < TOP_CAP {
        m.insert(k, 1);
    }
}

impl Dist {
    fn record(&mut self, a: u64, b: u64) {
        self.count += 1;
        let (ab, bb) = (bit_len(a), bit_len(b));
        self.a_bits[ab] += 1;
        self.b_bits[bb] += 1;
        self.joint[band(ab)][band(bb)] += 1;
        bump(&mut self.top_a, a);
        bump(&mut self.top_b, b);
        bump(&mut self.top_ab, (a, b));
        bump(&mut self.a_pages, a >> A_PAGE_SHIFT);
    }
}

fn scan_file_dist(path: &Path, d: &mut Dist) -> std::io::Result<()> {
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut buf = [0u8; RECORD_SIZE * 4096];
    let mut carry = 0usize;
    loop {
        let n = reader.read(&mut buf[carry..])?;
        if n == 0 {
            break;
        }
        let available = carry + n;
        let full = available / RECORD_SIZE;
        for i in 0..full {
            let off = i * RECORD_SIZE;
            if buf[off] != ADD {
                continue;
            }
            let a = u64::from_le_bytes(buf[off + 1..off + 9].try_into().unwrap());
            let b = u64::from_le_bytes(buf[off + 9..off + 17].try_into().unwrap());
            if (a >> 32) != 0 || (b >> 32) != 0 || a + b >= (1u64 << 32) {
                continue;
            }
            if BinaryBasicFrops::is_frequent_op(ADD, a, b) {
                continue;
            }
            d.record(a, b);
        }
        let consumed = full * RECORD_SIZE;
        carry = available - consumed;
        buf.copy_within(consumed..available, 0);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Per-block distribution of non-FROPS operations, plus EQ specifics.
// ---------------------------------------------------------------------------------------------

struct BlockStats {
    name: String,
    op_total: Vec<u64>,   // indexed by opcode (256)
    op_nonfrop: Vec<u64>, // non-FROPS count per opcode
    op_hi0: Vec<u64>,     // non-FROPS count per opcode with hi32(a)=hi32(b)=0
    eq_nonfrop: u64,
    eq_equal: u64, // non-FROPS eq with a == b
    eq_hi0: u64,   // non-FROPS eq with hi32(a)=hi32(b)=0
}

impl BlockStats {
    fn new(name: String) -> Self {
        Self {
            name,
            op_total: vec![0; 256],
            op_nonfrop: vec![0; 256],
            op_hi0: vec![0; 256],
            eq_nonfrop: 0,
            eq_equal: 0,
            eq_hi0: 0,
        }
    }
    fn total_nonfrop(&self) -> u64 {
        self.op_nonfrop.iter().sum()
    }
}

fn scan_file_nonfrop(path: &Path) -> std::io::Result<BlockStats> {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let mut s = BlockStats::new(name);
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut buf = [0u8; RECORD_SIZE * 4096];
    let mut carry = 0usize;
    loop {
        let n = reader.read(&mut buf[carry..])?;
        if n == 0 {
            break;
        }
        let available = carry + n;
        let full = available / RECORD_SIZE;
        for i in 0..full {
            let off = i * RECORD_SIZE;
            let op = buf[off];
            let a = u64::from_le_bytes(buf[off + 1..off + 9].try_into().unwrap());
            let b = u64::from_le_bytes(buf[off + 9..off + 17].try_into().unwrap());
            let Some(info) = classify(op) else { continue };
            s.op_total[op as usize] += 1;
            if crate::current::is_frequent(info.table, op, a, b) {
                continue;
            }
            s.op_nonfrop[op as usize] += 1;
            let hi0 = (a >> 32) == 0 && (b >> 32) == 0;
            if hi0 {
                s.op_hi0[op as usize] += 1;
            }
            if op == EQ {
                s.eq_nonfrop += 1;
                if a == b {
                    s.eq_equal += 1;
                }
                if hi0 {
                    s.eq_hi0 += 1;
                }
            }
        }
        let consumed = full * RECORD_SIZE;
        carry = available - consumed;
        buf.copy_within(consumed..available, 0);
    }
    Ok(s)
}

fn op_name(op: u8) -> String {
    ZiskOp::try_from_code(op).map(|o| o.name().to_string()).unwrap_or_else(|_| format!("{op:#04x}"))
}

pub fn run_nonfrop(dir: &Path) -> std::io::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().map(|e| e == "bin").unwrap_or(false))
        .collect();
    paths.sort();
    let blocks: Vec<BlockStats> =
        paths.iter().map(|p| scan_file_nonfrop(p)).collect::<std::io::Result<_>>()?;

    // Global non-FROPS distribution by op.
    let mut g_total = vec![0u64; 256];
    let mut g_nonfrop = vec![0u64; 256];
    let mut g_hi0 = vec![0u64; 256];
    for bl in &blocks {
        for op in 0..256 {
            g_total[op] += bl.op_total[op];
            g_nonfrop[op] += bl.op_nonfrop[op];
            g_hi0[op] += bl.op_hi0[op];
        }
    }
    let all_nonfrop: u64 = g_nonfrop.iter().sum::<u64>().max(1);
    let mut ops: Vec<u8> = (0u8..=255).filter(|&op| g_nonfrop[op as usize] > 0).collect();
    ops.sort_by_key(|&op| std::cmp::Reverse(g_nonfrop[op as usize]));

    println!("Global non-FROPS distribution by op:");
    println!(
        "{:<14} {:>15} {:>15} {:>9} {:>10} {:>15} {:>8}",
        "op", "total", "non-FROPS", "nonF%", "share%", "hi0", "hi0%"
    );
    for &op in &ops {
        let t = g_total[op as usize];
        let nf = g_nonfrop[op as usize];
        let h = g_hi0[op as usize];
        println!(
            "{:<14} {:>15} {:>15} {:>8.1}% {:>9.2}% {:>15} {:>7.1}%",
            op_name(op),
            t,
            nf,
            100.0 * nf as f64 / t.max(1) as f64,
            100.0 * nf as f64 / all_nonfrop as f64,
            h,
            100.0 * h as f64 / nf.max(1) as f64,
        );
    }

    // Per-block non-FROPS for the top opcodes.
    let top: Vec<u8> = ops.iter().take(8).copied().collect();
    println!("\nNon-FROPS per block (top {} ops):", top.len());
    print!("{:<46} {:>13}", "block", "nonfrop_tot");
    for &op in &top {
        print!("{:>11}", op_name(op));
    }
    println!();
    for bl in &blocks {
        print!("{:<46} {:>13}", bl.name, bl.total_nonfrop());
        for &op in &top {
            print!("{:>11}", bl.op_nonfrop[op as usize]);
        }
        println!();
    }

    // EQ specifics per block.
    println!("\nEQ (non-FROPS) per block:");
    println!(
        "{:<46} {:>14} {:>14} {:>8} {:>14} {:>8}",
        "block", "eq_nonfrop", "eq_equal", "equal%", "eq_hi0", "hi0%"
    );
    let pct = |x: u64, w: u64| if w == 0 { 0.0 } else { 100.0 * x as f64 / w as f64 };
    let (mut te, mut teq, mut th) = (0u64, 0u64, 0u64);
    for bl in &blocks {
        te += bl.eq_nonfrop;
        teq += bl.eq_equal;
        th += bl.eq_hi0;
        println!(
            "{:<46} {:>14} {:>14} {:>7.2}% {:>14} {:>7.2}%",
            bl.name,
            bl.eq_nonfrop,
            bl.eq_equal,
            pct(bl.eq_equal, bl.eq_nonfrop),
            bl.eq_hi0,
            pct(bl.eq_hi0, bl.eq_nonfrop),
        );
    }
    println!(
        "{:<46} {:>14} {:>14} {:>7.2}% {:>14} {:>7.2}%",
        "TOTAL",
        te,
        teq,
        pct(teq, te),
        th,
        pct(th, te)
    );
    println!("\neq_equal = non-FROPS eq with a==b. eq_hi0 = non-FROPS eq with hi32(a)=hi32(b)=0.");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Per-file high-half classification of every operation (hi = top 32 bits).
//   Hi0   : hi(a)=hi(b)=hi(c)=0          (result also fits 32 bits)
//   Hi0+  : hi(a)=hi(b)=0               (operands fit 32 bits; c may carry)
//   HiFFA : hi(a)=0xFFFF_FFFF           (a is a sign-extended 32-bit negative)
//   HiFFB : hi(b)=0xFFFF_FFFF
//   HiFF0 : hi(a)=0xFFFF_FFFF AND hi(b)=0
//   Hi0FF : hi(a)=0          AND hi(b)=0xFFFF_FFFF
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct HiCounts {
    total: u64,
    hi0: u64,
    hi0p: u64,
    hiffa: u64,
    hiffb: u64,
    hiff0: u64,
    hi0ff: u64,
}

impl HiCounts {
    fn add(&mut self, o: &HiCounts) {
        self.total += o.total;
        self.hi0 += o.hi0;
        self.hi0p += o.hi0p;
        self.hiffa += o.hiffa;
        self.hiffb += o.hiffb;
        self.hiff0 += o.hiff0;
        self.hi0ff += o.hi0ff;
    }
}

const HI_FF: u64 = 0xFFFF_FFFF;

fn scan_file_hiclass(path: &Path, exclude_frops: bool) -> std::io::Result<Vec<HiCounts>> {
    let mut ops = vec![HiCounts::default(); 256];
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut buf = [0u8; RECORD_SIZE * 4096];
    let mut carry = 0usize;
    loop {
        let n = reader.read(&mut buf[carry..])?;
        if n == 0 {
            break;
        }
        let available = carry + n;
        let full = available / RECORD_SIZE;
        for i in 0..full {
            let off = i * RECORD_SIZE;
            let op = buf[off];
            let Ok(zop) = ZiskOp::try_from_code(op) else { continue };
            let a = u64::from_le_bytes(buf[off + 1..off + 9].try_into().unwrap());
            let b = u64::from_le_bytes(buf[off + 9..off + 17].try_into().unwrap());
            // By default skip operations already covered by FROPS (analysing what specific
            // machines would still have to handle).
            if exclude_frops {
                if let Some(info) = classify(op) {
                    if crate::current::is_frequent(info.table, op, a, b) {
                        continue;
                    }
                }
            }
            let (c, _) = zop.call_ab(a, b);
            let e = &mut ops[op as usize];
            e.total += 1;
            let (ahi, bhi, chi) = (a >> 32, b >> 32, c >> 32);
            if ahi == 0 && bhi == 0 {
                e.hi0p += 1;
                if chi == 0 {
                    e.hi0 += 1;
                }
            }
            if ahi == HI_FF {
                e.hiffa += 1;
                if bhi == 0 {
                    e.hiff0 += 1;
                }
            }
            if bhi == HI_FF {
                e.hiffb += 1;
                if ahi == 0 {
                    e.hi0ff += 1;
                }
            }
        }
        let consumed = full * RECORD_SIZE;
        carry = available - consumed;
        buf.copy_within(consumed..available, 0);
    }
    Ok(ops)
}

fn print_hiclass_table(ops: &[HiCounts]) {
    let pc = |x: u64, w: u64| if w == 0 { 0.0 } else { 100.0 * x as f64 / w as f64 };
    println!(
        "{:<13} {:>13} {:>12} {:>6} {:>12} {:>6} {:>11} {:>6} {:>11} {:>6} {:>11} {:>6} {:>11} {:>6}",
        "op", "total", "Hi0", "%", "Hi0+", "%", "HiFFA", "%", "HiFFB", "%", "HiFF0", "%", "Hi0FF", "%"
    );
    let mut order: Vec<u8> = (0u8..=255).filter(|&op| ops[op as usize].total > 0).collect();
    order.sort_by_key(|&op| std::cmp::Reverse(ops[op as usize].total));
    for op in order {
        let e = ops[op as usize];
        println!(
            "{:<13} {:>13} {:>12} {:>5.1}% {:>12} {:>5.1}% {:>11} {:>5.1}% {:>11} {:>5.1}% {:>11} {:>5.1}% {:>11} {:>5.1}%",
            op_name(op),
            e.total,
            e.hi0, pc(e.hi0, e.total),
            e.hi0p, pc(e.hi0p, e.total),
            e.hiffa, pc(e.hiffa, e.total),
            e.hiffb, pc(e.hiffb, e.total),
            e.hiff0, pc(e.hiff0, e.total),
            e.hi0ff, pc(e.hi0ff, e.total),
        );
    }
}

pub fn run_hiclass(dir: &Path, exclude_frops: bool) -> std::io::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().map(|e| e == "bin").unwrap_or(false))
        .collect();
    paths.sort();

    println!(
        "Counting {} operations.",
        if exclude_frops { "NON-FROPS only (FROPS excluded)" } else { "ALL (FROPS included)" }
    );
    let mut total = vec![HiCounts::default(); 256];
    for path in &paths {
        let ops = scan_file_hiclass(path, exclude_frops)?;
        for op in 0..256 {
            total[op].add(&ops[op]);
        }
        let name = path.file_name().unwrap().to_string_lossy();
        println!("\n=== {name} ===");
        print_hiclass_table(&ops);
    }
    println!("\n=== TOTAL (all blocks) ===");
    print_hiclass_table(&total);
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// FROPS table-entry analysis: of the materialised table rows, how many have hi32(a)=hi32(b)=hi32(c)=0
// and how many of those also have the flag set.
// ---------------------------------------------------------------------------------------------

fn count_table_hi(name: &str, table: &[(u8, u64, u64, u64, bool)]) -> (u64, u64, u64) {
    let mut total = 0u64;
    let mut hi0 = 0u64;
    let mut hi0_flag = 0u64;
    for &(_op, a, b, c, flag) in table {
        total += 1;
        if (a >> 32) == 0 && (b >> 32) == 0 && (c >> 32) == 0 {
            hi0 += 1;
            if flag {
                hi0_flag += 1;
            }
        }
    }
    let pc = |x: u64, w: u64| if w == 0 { 0.0 } else { 100.0 * x as f64 / w as f64 };
    println!(
        "{name:18} entries {total:>10}  hi0 {hi0:>10} ({:.2}%)  hi0+flag {hi0_flag:>10} ({:.2}% of hi0)",
        pc(hi0, total),
        pc(hi0_flag, hi0)
    );
    (total, hi0, hi0_flag)
}

pub fn run_table_hi() {
    let mut arith = ArithFrops::new();
    arith.build_table();
    let mut basic = BinaryBasicFrops::new();
    basic.build_table();
    let mut ext = BinaryExtensionFrops::new();
    ext.build_table();

    println!("FROPS table entries with hi32(a)=hi32(b)=hi32(c)=0 (and of those, with flag set):\n");
    let a = count_table_hi("arith", &arith.generate_full_table());
    let b = count_table_hi("binary_basic", &basic.generate_full_table());
    let e = count_table_hi("binary_extension", &ext.generate_full_table());
    let total = a.0 + b.0 + e.0;
    let hi0 = a.1 + b.1 + e.1;
    let hi0_flag = a.2 + b.2 + e.2;
    let pc = |x: u64, w: u64| if w == 0 { 0.0 } else { 100.0 * x as f64 / w as f64 };
    println!(
        "{:18} entries {total:>10}  hi0 {hi0:>10} ({:.2}%)  hi0+flag {hi0_flag:>10} ({:.2}% of hi0)",
        "TOTAL",
        pc(hi0, total),
        pc(hi0_flag, hi0)
    );
}

/// Per-op FROPS distribution: total entries and how many have a[1]=b[1]=c[1]=0 AND flag=0
/// (high 32 bits of a, b, c all zero and the flag clear).
pub fn run_table_by_op() {
    let mut arith = ArithFrops::new();
    arith.build_table();
    let mut basic = BinaryBasicFrops::new();
    basic.build_table();
    let mut ext = BinaryExtensionFrops::new();
    ext.build_table();

    let mut total = vec![0u64; 256];
    let mut allzero = vec![0u64; 256]; // a1=b1=c1=0 && !flag
    for t in
        [&arith.generate_full_table(), &basic.generate_full_table(), &ext.generate_full_table()]
    {
        for &(op, a, b, c, flag) in t {
            total[op as usize] += 1;
            if (a >> 32) == 0 && (b >> 32) == 0 && (c >> 32) == 0 && !flag {
                allzero[op as usize] += 1;
            }
        }
    }

    let pc = |x: u64, w: u64| if w == 0 { 0.0 } else { 100.0 * x as f64 / w as f64 };
    let mut ops: Vec<u8> = (0u8..=255).filter(|&op| total[op as usize] > 0).collect();
    ops.sort_by_key(|&op| std::cmp::Reverse(total[op as usize]));

    println!("FROPS per op: entries and those with a[1]=b[1]=c[1]=0 AND flag=0\n");
    println!("{:<14} {:>14} {:>18} {:>9}", "op", "frops", "a1=b1=c1=flag=0", "%");
    let (mut gt, mut gz) = (0u64, 0u64);
    for op in ops {
        let t = total[op as usize];
        let z = allzero[op as usize];
        gt += t;
        gz += z;
        println!("{:<14} {:>14} {:>18} {:>8.2}%", op_name(op), t, z, pc(z, t));
    }
    println!("{:<14} {:>14} {:>18} {:>8.2}%", "TOTAL", gt, gz, pc(gz, gt));
}

// ---------------------------------------------------------------------------------------------
// Constant-column optimization: split each FROPS table into 2^R-row partitions, order rows so the
// "zero group" (a1=b1=c1=0 && flag=0) comes first, ops by descending count. A partition that is
// entirely one op can drop the OP column; one entirely in the zero group can drop A1,B1,C1,FLAG.
// 8 columns total (op,a0,a1,b0,b1,c0,c1,flag); a0,b0,c0 are always needed.
// ---------------------------------------------------------------------------------------------

struct Seg {
    start: u64,
    end: u64,
    op: u8,
    g0: bool,
}

/// Returns (baseline_colrows, op_saved, g0_saved, parts, g0_rows) for one table at partition size 2^r.
/// `op_saved` is the OP-column drop (free: an op is already a contiguous same-op block, so get_row is
/// unchanged). `g0_saved` is the a1/b1/c1/flag drop (needs the zero-group reorder, which breaks the
/// closed-form per-op offset → has an offset-computation cost).
/// `reorder_g0 = false`: op-contiguous layout (ops by desc count, no zero-group split). Only the OP
/// column can be dropped — this is offset-preserving (get_row stays `base[op] + box_offset`, only the
/// per-op base constant changes). `reorder_g0 = true`: zero group first, enabling the a1/b1/c1/flag
/// drop too, at the cost of breaking the closed-form per-op offset.
fn partition_table(
    table: &[(u8, u64, u64, u64, bool)],
    r: u32,
    reorder_g0: bool,
) -> (u128, u128, u128, u64, u64) {
    let s = 1u64 << r;
    let n = table.len() as u64;

    let mut g0: std::collections::HashMap<u8, u64> = std::collections::HashMap::new();
    let mut g1: std::collections::HashMap<u8, u64> = std::collections::HashMap::new();
    let mut all: std::collections::HashMap<u8, u64> = std::collections::HashMap::new();
    for &(op, a, b, c, flag) in table {
        *all.entry(op).or_default() += 1;
        if (a >> 32) == 0 && (b >> 32) == 0 && (c >> 32) == 0 && !flag {
            *g0.entry(op).or_default() += 1;
        } else {
            *g1.entry(op).or_default() += 1;
        }
    }
    let mut segs: Vec<Seg> = Vec::new();
    let mut pos = 0u64;
    let push =
        |m: &std::collections::HashMap<u8, u64>, g0: bool, pos: &mut u64, segs: &mut Vec<Seg>| {
            let mut v: Vec<(u8, u64)> = m.iter().map(|(&o, &c)| (o, c)).collect();
            v.sort_by_key(|x| std::cmp::Reverse(x.1));
            for (op, cnt) in v {
                segs.push(Seg { start: *pos, end: *pos + cnt, op, g0 });
                *pos += cnt;
            }
        };
    let mut g0_rows = 0u64;
    if reorder_g0 {
        push(&g0, true, &mut pos, &mut segs);
        g0_rows = pos;
        push(&g1, false, &mut pos, &mut segs);
    } else {
        // Op-contiguous, ops by descending total count; zero group not separated.
        push(&all, false, &mut pos, &mut segs);
    }

    let num_parts = n.div_ceil(s);
    let mut op_saved: u128 = 0;
    let mut g0_saved: u128 = 0;
    for p in 0..num_parts {
        let lo = p * s;
        let hi = lo + s;
        let real_hi = hi.min(n);
        let has_pad = hi > n;
        let mut single_op = !has_pad;
        let mut all_g0 = !has_pad;
        let mut first_op: Option<u8> = None;
        for seg in &segs {
            if seg.end <= lo || seg.start >= real_hi {
                continue;
            }
            match first_op {
                None => first_op = Some(seg.op),
                Some(o) if o != seg.op => single_op = false,
                _ => {}
            }
            if !seg.g0 {
                all_g0 = false;
            }
        }
        if single_op {
            op_saved += s as u128; // 1 column (OP)
        }
        if all_g0 {
            g0_saved += s as u128 * 4; // a1,b1,c1,flag
        }
    }
    let baseline = num_parts as u128 * s as u128 * 8;
    (baseline, op_saved, g0_saved, num_parts, g0_rows)
}

/// No-reorder analysis: in the NATURAL table layout (the order get_row already uses), for each 2^R
/// partition, check which of the 5 droppable columns (op, a1, b1, c1, flag) are constant across it.
/// Offset-preserving: nothing is reordered, only constant columns are replaced by a constant.
/// Returns saved column-rows per column [op, a1, b1, c1, flag] and the padded baseline col-rows.
fn partition_natural(table: &[(u8, u64, u64, u64, bool)], r: u32) -> ([u128; 5], u128) {
    let s = 1u64 << r;
    let n = table.len() as u64;
    let num_parts = n.div_ceil(s);
    let mut saved = [0u128; 5];
    for p in 0..num_parts {
        let lo = (p * s) as usize;
        let hi = ((p + 1) * s).min(n) as usize;
        let full = (p + 1) * s <= n;
        // First row's column values.
        let v0 = |t: &(u8, u64, u64, u64, bool)| {
            [t.0 as u64, t.1 >> 32, t.2 >> 32, t.3 >> 32, t.4 as u64]
        };
        let f = v0(&table[lo]);
        let mut konst = [true; 5];
        for row in &table[lo..hi] {
            let v = v0(row);
            for k in 0..5 {
                if v[k] != f[k] {
                    konst[k] = false;
                }
            }
        }
        for k in 0..5 {
            // Over a padded partition, a column stays constant only if its value is the pad value (0).
            if konst[k] && (full || f[k] == 0) {
                saved[k] += s as u128;
            }
        }
    }
    (saved, num_parts as u128 * s as u128 * 8)
}

pub fn run_table_partition(r: u32) {
    let mut arith = ArithFrops::new();
    arith.build_table();
    let mut basic = BinaryBasicFrops::new();
    basic.build_table();
    let mut ext = BinaryExtensionFrops::new();
    ext.build_table();
    type NamedTable = (&'static str, Vec<(u8, u64, u64, u64, bool)>);
    let tables: [NamedTable; 3] = [
        ("arith", arith.generate_full_table()),
        ("binary_basic", basic.generate_full_table()),
        ("binary_extension", ext.generate_full_table()),
    ];

    let pct = |x: u128, w: u128| if w == 0 { 0.0 } else { 100.0 * x as f64 / w as f64 };

    println!(
        "Constant-column optimization, R={r} (partition = {} rows). 8 cols; a0,b0,c0 always kept.",
        1u64 << r
    );
    println!(
        "  OP-drop  = single-op partitions  -> FREE (offset preserved: only base[op] changes)."
    );
    println!("  G0-drop  = a1,b1,c1,flag on zero-group partitions -> needs reorder, BREAKS per-op offset.\n");
    println!(
        "{:<18} {:>10} {:>10} {:>12} {:>12} {:>10}",
        "table", "rows", "g0_rows", "A:free%", "B:full%", "best%"
    );
    let (mut tb, mut t_a, mut t_b, mut t_best) = (0u128, 0u128, 0u128, 0u128);
    for (name, t) in &tables {
        // Layout A (offset-preserving): OP-drop only. Layout B (offset-breaking): OP + a1b1c1flag.
        let (base, a_saved, _, _, _) = partition_table(t, r, false);
        let (_, b_op, b_g0, _, g0r) = partition_table(t, r, true);
        let b_saved = b_op + b_g0;
        let best = a_saved.max(b_saved); // pick the better layout per table
        tb += base;
        t_a += a_saved;
        t_b += b_saved;
        t_best += best;
        println!(
            "{:<18} {:>10} {:>10} {:>11.2}% {:>11.2}% {:>9.2}%",
            name,
            t.len(),
            g0r,
            pct(a_saved, base),
            pct(b_saved, base),
            pct(best, base)
        );
    }
    println!(
        "{:<18} {:>10} {:>10} {:>11.2}% {:>11.2}% {:>9.2}%",
        "TOTAL",
        "",
        "",
        pct(t_a, tb),
        pct(t_b, tb),
        pct(t_best, tb)
    );
    println!(
        "\nA:free% = OP column dropped only — offset preserved (get_row = base[op] + box_offset)."
    );
    println!("B:full% = OP + a1/b1/c1/flag via zero-group reorder — breaks the closed-form per-op offset.");
    println!(
        "best%   = better of A/B per table (B can be worse than A when the reorder fragments ops)."
    );

    // No-reorder, per-column: drop whichever columns are already constant in the natural layout.
    println!("\nNO-REORDER (natural layout, offset preserved) — per-column constant savings:");
    println!(
        "{:<18} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9}",
        "table", "op%", "a1%", "b1%", "c1%", "flag%", "total%"
    );
    let (mut gb, mut gsum) = (0u128, [0u128; 5]);
    for (name, t) in &tables {
        let (sv, base) = partition_natural(t, r);
        gb += base;
        for k in 0..5 {
            gsum[k] += sv[k];
        }
        let tot: u128 = sv.iter().sum();
        println!(
            "{:<18} {:>7.2}% {:>7.2}% {:>7.2}% {:>7.2}% {:>7.2}% {:>8.2}%",
            name,
            pct(sv[0], base),
            pct(sv[1], base),
            pct(sv[2], base),
            pct(sv[3], base),
            pct(sv[4], base),
            pct(tot, base)
        );
    }
    let gtot: u128 = gsum.iter().sum();
    println!(
        "{:<18} {:>7.2}% {:>7.2}% {:>7.2}% {:>7.2}% {:>7.2}% {:>8.2}%",
        "TOTAL",
        pct(gsum[0], gb),
        pct(gsum[1], gb),
        pct(gsum[2], gb),
        pct(gsum[3], gb),
        pct(gsum[4], gb),
        pct(gtot, gb)
    );
    println!("(a1/b1 are constant *within each box*, so they drop with NO reorder; c1/flag vary inside\nsome boxes — they only drop where a partition happens to land where they're constant.)");
}

fn top_k<K: Copy>(m: &std::collections::HashMap<K, u64>, k: usize) -> Vec<(K, u64)> {
    let mut v: Vec<(K, u64)> = m.iter().map(|(&kk, &c)| (kk, c)).collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v.truncate(k);
    v
}

pub fn run_dist(dir: &Path) -> std::io::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().map(|e| e == "bin").unwrap_or(false))
        .collect();
    paths.sort();
    let mut d = Dist::default();
    for p in &paths {
        scan_file_dist(p, &mut d)?;
    }
    let n = d.count.max(1);
    let pc = |x: u64| 100.0 * x as f64 / n as f64;

    println!("hi0-no-carry non-FROPS adds analysed: {}\n", d.count);

    // Bit-length histograms.
    println!("Magnitude (bit-length) distribution:");
    println!("{:>6} {:>16} {:>8} {:>16} {:>8}", "bits", "a count", "a %", "b count", "b %");
    for k in 0..=32 {
        if d.a_bits[k] == 0 && d.b_bits[k] == 0 {
            continue;
        }
        println!(
            "{:>6} {:>16} {:>7.2}% {:>16} {:>7.2}%",
            k,
            d.a_bits[k],
            pc(d.a_bits[k]),
            d.b_bits[k],
            pc(d.b_bits[k])
        );
    }

    // Cumulative thresholds.
    println!("\nCumulative share with value < 2^k:");
    println!("{:>6} {:>10} {:>10}", "k", "a<2^k", "b<2^k");
    for k in [4u32, 8, 10, 12, 16, 20, 24, 28, 32] {
        let acum: u64 = d.a_bits[..=k as usize].iter().sum();
        let bcum: u64 = d.b_bits[..=k as usize].iter().sum();
        println!("{:>6} {:>9.2}% {:>9.2}%", k, pc(acum), pc(bcum));
    }

    // Joint band matrix.
    let labels = ["0", "1-8", "9-16", "17-24", "25-32"];
    println!("\nJoint magnitude matrix (rows=a bits, cols=b bits), % of subset:");
    print!("{:>8}", "a\\b");
    for l in labels {
        print!("{l:>10}");
    }
    println!();
    for (ai, row) in d.joint.iter().enumerate() {
        print!("{:>8}", labels[ai]);
        for &c in row {
            print!("{:>9.2}%", pc(c));
        }
        println!();
    }

    // Top values.
    let cap_note = if d.top_b.len() >= TOP_CAP { " (capped)" } else { "" };
    println!("\nTop 15 b values{cap_note}:");
    for (b, c) in top_k(&d.top_b, 15) {
        println!("  b={:<12} {:>14} {:>6.2}%", b, c, pc(c));
    }
    println!("\nTop 15 a values:");
    for (a, c) in top_k(&d.top_a, 15) {
        println!("  a={:<12} {:>14} {:>6.2}%", a, c, pc(c));
    }
    println!("\nTop 15 (a,b) pairs:");
    for ((a, b), c) in top_k(&d.top_ab, 15) {
        println!("  ({:<10},{:<10}) {:>14} {:>6.2}%", a, b, c, pc(c));
    }

    // ---- b concentration: cumulative coverage of the most frequent b values ----
    let mut bvals = top_k(&d.top_b, d.top_b.len());
    bvals.sort_by_key(|x| std::cmp::Reverse(x.1));
    let bcap = if d.top_b.len() >= TOP_CAP { " (b set capped)" } else { "" };
    println!("\nb concentration{bcap}: distinct b = {}", d.top_b.len());
    println!("{:>8} {:>12}", "top-N b", "cum %");
    let mut acc = 0u64;
    let mut ni = 0usize;
    for &n in &[1usize, 2, 3, 5, 10, 20, 50, 100, 500] {
        if n > bvals.len() {
            break;
        }
        while ni < n {
            acc += bvals[ni].1;
            ni += 1;
        }
        println!("{:>8} {:>11.2}%", n, pc(acc));
    }

    // ---- a address ranges: merge contiguous dense pages, rank by coverage ----
    let acap = if d.a_pages.len() >= TOP_CAP { " (pages capped)" } else { "" };
    let mut pages: Vec<(u64, u64)> = d.a_pages.iter().map(|(&p, &c)| (p, c)).collect();
    pages.sort_by_key(|x| x.0);
    // Merge pages separated by <= GAP empty pages into one range.
    const GAP: u64 = 16; // bridge gaps up to 64 KB
    let mut ranges: Vec<(u64, u64, u64)> = Vec::new(); // (page_lo, page_hi_incl, hits)
    for (p, c) in pages {
        match ranges.last_mut() {
            Some(r) if p <= r.1 + GAP => {
                r.1 = p;
                r.2 += c;
            }
            _ => ranges.push((p, p, c)),
        }
    }
    let n_ranges = ranges.len();
    ranges.sort_by_key(|x| std::cmp::Reverse(x.2));
    println!(
        "\na address ranges{acap}: {} distinct pages -> {} contiguous ranges",
        d.a_pages.len(),
        n_ranges
    );
    println!("{:>14} {:>14} {:>8} {:>14} {:>7}", "addr_lo", "addr_hi", "pages", "hits", "%");
    let mut cum = 0u64;
    for (plo, phi, hits) in ranges.iter().take(15) {
        cum += hits;
        println!(
            "{:>14X} {:>14X} {:>8} {:>14} {:>6.2}%",
            plo << A_PAGE_SHIFT,
            ((phi + 1) << A_PAGE_SHIFT) - 1,
            phi - plo + 1,
            hits,
            pc(*hits)
        );
    }
    let top_ranges_cov: u64 = ranges.iter().take(15).map(|r| r.2).sum();
    println!(
        "top-15 ranges cover {:.2}% of the subset (cum top shown above ~{:.2}%)",
        pc(top_ranges_cov),
        pc(cum)
    );
    Ok(())
}

pub fn run(dir: &Path) -> std::io::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().map(|e| e == "bin").unwrap_or(false))
        .collect();
    paths.sort();

    let inst = |n: u64| n as f64 / INSTANCE_ROWS as f64;
    println!(
        "{:<46} {:>13} {:>13} {:>13} {:>8} {:>13} {:>10}",
        "block", "add_total", "add_frop", "nonfrop", "hi0%", "hi0_nocarry", "inst 2^22"
    );
    let mut total = AddStats::default();
    for p in &paths {
        let s = scan_file(p)?;
        total.add(&s);
        let name = p.file_name().unwrap().to_string_lossy();
        println!(
            "{:<46} {:>13} {:>13} {:>13} {:>7.2}% {:>13} {:>10.2}",
            name,
            s.add_total,
            s.add_frop,
            s.add_nonfrop,
            pct(s.hi0_result, s.add_nonfrop),
            s.hi0_result,
            inst(s.hi0_result),
        );
    }
    println!(
        "{:<46} {:>13} {:>13} {:>13} {:>7.2}% {:>13} {:>10.2}",
        "TOTAL",
        total.add_total,
        total.add_frop,
        total.add_nonfrop,
        pct(total.hi0_result, total.add_nonfrop),
        total.hi0_result,
        inst(total.hi0_result),
    );
    println!(
        "\nhi0_nocarry = non-FROPS adds with hi32(a)=hi32(b)=0 AND a+b < 2^32 (no carry: the whole high"
    );
    println!("half is 0 and need not be computed). hi0% is over non-FROPS adds.");
    println!(
        "inst 2^22 = hi0_nocarry / {INSTANCE_ROWS} (instances of 2^22 rows these ops would fill)."
    );
    println!(
        "(operands-fit-32-bits without the no-carry condition is within <0.01% of hi0_nocarry.)"
    );
    Ok(())
}
