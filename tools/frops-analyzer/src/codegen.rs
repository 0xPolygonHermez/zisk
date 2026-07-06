//! Approach 2: regenerate the `*_frops.rs` source files for the proposed FROPS.
//!
//! The emitted files keep the exact public surface the rest of the workspace relies on
//! (`is_frequent_op`, `get_row`, `TABLE_ID`, `NO_FROPS`, `new`, `build_table`, `generate_cmd`, and the
//! offset-consistency test), so they are drop-in replacements consumed by the existing
//! `*_frops_fixed_gen.rs` generators and the arith/binary state machines.
//!
//! Layout invariant (must match `FrequentOpsHelpers`): rows are grouped per opcode in ascending
//! opcode order; within an opcode they follow region order, and within a region they are row-major
//! over `b`. `OP_TABLE_OFFSETS[op - START]` is the cumulative row count of all lower opcodes, exactly
//! what `generate_table_offsets()` recomputes — so the generated `test_table_offsets` passes.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use crate::ops::{classify, variant_ident, FropsTable, OpInfo};
use crate::optimize::{Config, Proposal};

/// One-line comment documenting the parameters a file was generated with.
fn params_comment(cfg: &Config) -> String {
    format!(
        "generated with: max-table={} partition-bits={} low-cap={} max-regions-per-op={} table-cost={} nodes={}",
        cfg.max_table, cfg.partition_bits, cfg.low_cap, cfg.max_regions_per_op, cfg.table_cost, cfg.nodes
    )
}
use crate::region::Region;

struct TableMeta {
    struct_name: &'static str,
    air_name: &'static str,
    table_id: usize,
    rel_path: &'static str,
}

fn meta(table: FropsTable) -> TableMeta {
    match table {
        FropsTable::Arith => TableMeta {
            struct_name: "ArithFrops",
            air_name: "ArithFrops",
            table_id: 5010,
            rel_path: "state-machines/arith/src/arith_frops.rs",
        },
        FropsTable::BinaryBasic => TableMeta {
            struct_name: "BinaryBasicFrops",
            air_name: "BinaryBasicFrops",
            table_id: 5011,
            rel_path: "state-machines/binary/src/binary_basic_frops.rs",
        },
        FropsTable::BinaryExt => TableMeta {
            struct_name: "BinaryExtensionFrops",
            air_name: "BinaryExtensionFrops",
            table_id: 5012,
            rel_path: "state-machines/binary/src/binary_extension_frops.rs",
        },
    }
}

/// One opcode's selected regions, in deterministic order, with the op metadata.
struct OpBlock {
    info: OpInfo,
    const_name: String,
    variant: String,
    regions: Vec<Region>,
}

/// Writes the three regenerated source files under `workspace_root`. Existing files are backed up to
/// `<file>.bak`. Returns the list of (path, backup_made) written.
pub fn generate(prop: &Proposal, workspace_root: &Path) -> std::io::Result<Vec<(String, bool)>> {
    let mut written = Vec::new();
    for table in FropsTable::all() {
        let m = meta(table);
        let blocks = op_blocks(prop, table);
        let src = emit_file(&m, &blocks, &prop.config);
        let path = workspace_root.join(m.rel_path);
        // Preserve the *first* original as `<file>.rs.bak`; never clobber it on repeated runs.
        let mut backed = false;
        let bak = path.with_extension("rs.bak");
        if path.exists() && !bak.exists() {
            fs::copy(&path, &bak)?;
            backed = true;
        }
        fs::write(&path, src)?;
        written.push((m.rel_path.to_string(), backed));
    }

    // Additionally emit the x86-64 multiplicity-count macros.
    let asm_rel = "emulator-asm/src/frops/frops.s";
    let asm_path = workspace_root.join(asm_rel);
    if let Some(dir) = asm_path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&asm_path, emit_asm(prop))?;
    written.push((asm_rel.to_string(), false));

    Ok(written)
}

// ============================================================================================
// x86-64 assembly generation: one macro per op that counts FROPS multiplicity.
// ============================================================================================

fn fam_labels(table: FropsTable) -> (&'static str, &'static str, &'static str) {
    match table {
        FropsTable::Arith => {
            ("frops_arith_mult", "frops_arith_overflow", "frops_arith_overflow_index")
        }
        FropsTable::BinaryBasic => (
            "frops_binary_basic_mult",
            "frops_binary_basic_overflow",
            "frops_binary_basic_overflow_index",
        ),
        FropsTable::BinaryExt => (
            "frops_binary_extension_mult",
            "frops_binary_extension_overflow",
            "frops_binary_extension_overflow_index",
        ),
    }
}

fn fits_i32(v: u64) -> bool {
    v <= i32::MAX as u64
}

fn imm_str(v: u64) -> String {
    if v < 65536 {
        format!("{v}")
    } else {
        format!("{v:#X}")
    }
}

/// Rough cycle weight of one instruction line (imul = 3, everything else = 1).
fn instr_cyc(line: &str) -> u32 {
    match line.split_whitespace().next().unwrap_or("") {
        "imul" => 3,
        _ => 1,
    }
}
fn cyc(lines: &[String]) -> u32 {
    lines.iter().map(|l| instr_cyc(l)).sum()
}

/// `cmp reg,imm ; jcc target`, via `tmp` for immediates that don't fit imm32. Pushed as lines.
fn push_cmp(v: &mut Vec<String>, reg: &str, imm: u64, jcc: &str, target: &str, tmp: &str) {
    if fits_i32(imm) {
        v.push(format!("cmp     {reg}, {}", imm_str(imm)));
    } else {
        v.push(format!("mov     {tmp}, {}", imm_str(imm)));
        v.push(format!("cmp     {reg}, {tmp}"));
    }
    v.push(format!("{jcc}     {target}"));
}
fn push_sub(v: &mut Vec<String>, reg: &str, imm: u64, tmp: &str) {
    if imm == 0 {
    } else if fits_i32(imm) {
        v.push(format!("sub     {reg}, {}", imm_str(imm)));
    } else {
        v.push(format!("mov     {tmp}, {}", imm_str(imm)));
        v.push(format!("sub     {reg}, {tmp}"));
    }
}

/// b-axis compare (`t1` holds b, no free temp). Large immediates spill `rax`, popped before the `jcc`
/// so the FLAGS from the `cmp` survive. Assumes the caller's `t0`/`t1` are not `rax`.
fn push_cmp_b(v: &mut Vec<String>, imm: u64, jcc: &str, target: &str) {
    if fits_i32(imm) {
        v.push(format!("cmp     \\t1, {}", imm_str(imm)));
    } else {
        v.push("push    rax".into());
        v.push(format!("mov     rax, {}", imm_str(imm)));
        v.push("cmp     \\t1, rax".into());
        v.push("pop     rax".into());
    }
    v.push(format!("{jcc}     {target}"));
}
fn push_sub_b(v: &mut Vec<String>, imm: u64) {
    if imm == 0 {
    } else if fits_i32(imm) {
        v.push(format!("sub     \\t1, {}", imm_str(imm)));
    } else {
        v.push("push    rax".into());
        v.push(format!("mov     rax, {}", imm_str(imm)));
        v.push("sub     \\t1, rax".into());
        v.push("pop     rax".into());
    }
}

/// Emits one `.macro FROP_<OP> a, b, t0, t1 ... .endm`. `op_base` is `OP_TABLE_OFFSETS[op]` (the op's
/// first row within its family table). Regions are pre-sorted (low/mid/high) as in the Rust codegen.
/// The macro is annotated with rough cycle costs (imul=3, others=1): worst case to reject a non-FROP,
/// and best/worst case to count a FROP (increment, assuming no overflow).
fn emit_op_macro(name: &str, regions: &[Region], op_base: u64, table: FropsTable) -> String {
    let up = name.to_uppercase();
    let (mult, ovf, idx) = fam_labels(table);

    // Per region: head = fail path (a-tests, offset_a, b-load, b-tests); tail = match-only finish.
    let n = regions.len();
    let mut heads: Vec<Vec<String>> = vec![Vec::new(); n];
    let mut tails: Vec<Vec<String>> = vec![Vec::new(); n];
    let mut region_base = 0u64;
    let mut c_of = vec![0u64; n];
    for (i, r) in regions.iter().enumerate() {
        c_of[i] = op_base + region_base;
        region_base += r.rows();
    }
    let emitted: Vec<usize> = (0..n).collect();
    if emitted.is_empty() {
        return format!(
            ".macro FROP_{up} a, b, t0, t1\n    # no FROPS for this op: does nothing (0 cyc)\n.endm\n\n"
        );
    }
    let label = |i: usize| format!(".Lfrop_{up}_n{i}_\\@");
    let done = format!(".Lfrop_{up}_done_\\@");
    let incr = format!(".Lfrop_{up}_incr_\\@");
    // `next` target for the k-th emitted region: the following emitted region's label, else done.
    let next_of = |k: usize| -> String {
        emitted.get(k + 1).map(|&j| label(j)).unwrap_or_else(|| done.clone())
    };

    for (k, &i) in emitted.iter().enumerate() {
        let r = &regions[i];
        let nx = next_of(k);
        let head = &mut heads[i];
        // a axis (t1 is a free temp until b is loaded)
        head.push("mov     \\t0, \\a".into());
        if r.a_lo != 0 {
            push_cmp(head, "\\t0", r.a_lo, "jb", &nx, "\\t1");
        }
        if !r.a_to_max() {
            let a_hi = r.a_lo.wrapping_add(r.a_count.wrapping_mul(r.a_stride));
            push_cmp(head, "\\t0", a_hi, "jae", &nx, "\\t1");
        }
        if r.a_stride > 1 {
            let mask = r.a_stride - 1;
            let rem = r.a_lo & mask;
            head.push("mov     \\t1, \\t0".into());
            head.push(format!("and     \\t1, {mask}"));
            head.push(format!("cmp     \\t1, {rem}"));
            head.push(format!("jne     {nx}"));
        }
        // offset_a = (a - a_lo) / stride * b_count
        push_sub(head, "\\t0", r.a_lo, "\\t1");
        if r.a_stride > 1 {
            head.push(format!("shr     \\t0, {}", r.a_stride.trailing_zeros()));
        }
        if r.b_count != 1 {
            head.push(format!("imul    \\t0, \\t0, {}", r.b_count));
        }
        // b axis (t1 holds b; large immediates spill rax inside the helpers)
        head.push("mov     \\t1, \\b".into());
        let tail = &mut tails[i];
        if r.b_count == 1 {
            push_cmp_b(head, r.b_lo, "jne", &nx);
        } else {
            if r.b_lo != 0 {
                push_cmp_b(head, r.b_lo, "jb", &nx);
            }
            if !r.b_to_max() {
                let b_hi = r.b_lo + r.b_count;
                push_cmp_b(head, b_hi, "jae", &nx);
            }
            push_sub_b(tail, r.b_lo);
            tail.push("add     \\t0, \\t1".into());
        }
        if c_of[i] != 0 {
            tail.push(format!("add     \\t0, {}", c_of[i]));
        }
        tail.push(format!("jmp     {incr}"));
    }

    // Increment block (no-overflow path = lea + inc + jnz). Overflow append spills rax/rcx/rdx.
    let incr_block_cyc = 3u32;

    // Cost model.
    let not_frop: u32 = emitted.iter().map(|&i| cyc(&heads[i])).sum();
    let frop_best = cyc(&heads[emitted[0]]) + cyc(&tails[emitted[0]]) + incr_block_cyc;
    let last = *emitted.last().unwrap();
    let prior_fail: u32 = emitted[..emitted.len() - 1].iter().map(|&i| cyc(&heads[i])).sum();
    let frop_worst = prior_fail + cyc(&heads[last]) + cyc(&tails[last]) + incr_block_cyc;

    // Assemble.
    let mut s = format!(".macro FROP_{up} a, b, t0, t1\n");
    s.push_str(&format!(
        "    # cost (cyc, imul=3 else=1): reject-non-FROP worst {not_frop} | FROP+incr best {frop_best} worst {frop_worst} (no overflow)\n"
    ));
    for (k, &i) in emitted.iter().enumerate() {
        if k > 0 {
            s.push_str(&format!("{}:\n", label(i)));
        }
        for ln in &heads[i] {
            s.push_str(&format!("    {ln}\n"));
        }
        for ln in &tails[i] {
            s.push_str(&format!("    {ln}\n"));
        }
    }
    s.push_str(&format!("{incr}:\n"));
    s.push_str(&format!("    lea     \\t1, [rip + {mult}]\n"));
    s.push_str("    inc     dword ptr [\\t1 + \\t0*4]\n");
    s.push_str(&format!("    jnz     {done}\n"));
    s.push_str("    push    rax\n    push    rcx\n    push    rdx\n");
    s.push_str("    mov     rdx, \\t0\n");
    s.push_str(&format!("    lea     rax, [rip + {ovf}]\n"));
    s.push_str(&format!("    mov     ecx, dword ptr [rip + {idx}]\n"));
    s.push_str("    mov     dword ptr [rax + rcx*4], edx\n");
    s.push_str(&format!("    inc     dword ptr [rip + {idx}]\n"));
    s.push_str("    pop     rdx\n    pop     rcx\n    pop     rax\n");
    s.push_str(&format!("{done}:\n"));
    s.push_str(".endm\n\n");
    s
}

fn asm_header(title: &str, params: &str) -> String {
    let mut s = String::new();
    s.push_str(".intel_syntax noprefix\n.code64\n\n");
    s.push_str(&format!("# @generated by frops-analyzer — {title} (x86-64).\n"));
    if !params.is_empty() {
        s.push_str(&format!("# {params}\n"));
    }
    s.push_str("# One macro per operation:  FROP_<OP>  a, b, t0, t1\n");
    s.push_str("#   a, b  : operands (register or immediate, read-only).\n");
    s.push_str("#   t0,t1 : the only two registers the macro clobbers freely (plus FLAGS).\n");
    s.push_str("#           The overflow path additionally push/pops rax, rcx, rdx.\n");
    s.push_str(
        "# Behaviour: if (op,a,b) is a FROP, mult[row] += 1; on u32 wrap (counter hits 0) the\n",
    );
    s.push_str("# row offset is appended to the overflow vector. If not a FROP / op has no FROPS: nothing.\n\n");
    for table in FropsTable::all() {
        let (mult, ovf, idx) = fam_labels(table);
        s.push_str(&format!(".extern {mult}\n.extern {ovf}\n.extern {idx}\n"));
    }
    s.push('\n');
    s
}

fn emit_asm(prop: &Proposal) -> String {
    let mut s = asm_header("FROPS multiplicity-count macros", &params_comment(&prop.config));
    for table in FropsTable::all() {
        let blocks = op_blocks(prop, table);
        let (start, offsets) = table_offsets(&blocks);
        let mut have: HashSet<u8> = HashSet::new();
        s.push_str(&format!("# ---- {} ----\n", table.key()));
        for b in &blocks {
            let base =
                if offsets.is_empty() { 0 } else { offsets[b.info.code as usize - start] as u64 };
            s.push_str(&emit_op_macro(b.info.name, &b.regions, base, table));
            have.insert(b.info.code);
        }
        // Empty macros for the remaining candidate opcodes of this family (no FROPS -> do nothing).
        for code in 0u8..=255 {
            if let Some(info) = classify(code) {
                if info.table == table && !have.contains(&code) {
                    s.push_str(&emit_op_macro(info.name, &[], 0, table));
                }
            }
        }
    }
    s
}

// ============================================================================================
// Assembly for the ORIGINAL (hand-tuned) FROPS, for cycle comparison.
//
// The original predicates are modelled as box regions (a in [lo,hi) with optional stride, b in a
// range). A few hand-tuned conditions are NOT boxes and are approximated (noted in the file header):
//   * LT's coupled `a <= b && (b-a) distance` middle term is dropped (only low-rect + (a==0,b<0x10000)
//     are kept).
//   * AND / SUB_W b-side bit-masks (`(b&3)==0`, `b & MASK == addr`) are modelled as plain b ranges
//     (the mask test is dropped, ~1 instruction less).
// So the reject cost for LT / AND / SUB_W is a slight under-estimate; everything else is faithful.
// ============================================================================================

use crate::region::RegionKind;
use zisk_core::zisk_ops::ZiskOp;

const LOW: u64 = 386;

fn rg(a_lo: u64, a_count: u64, a_stride: u64, b_lo: u64, b_count: u64) -> Region {
    let kind = if a_lo == 0 && a_stride == 1 { RegionKind::LowRect } else { RegionKind::MidBox };
    Region { a_lo, a_count, a_stride, b_lo, b_count, kind }
}
fn low() -> Region {
    rg(0, LOW, 1, 0, LOW)
}

/// Box regions reproducing each original op's `is_frequent_op` predicate (see module note).
fn original_regions(code: u8) -> Vec<Region> {
    let op = match ZiskOp::try_from_code(code) {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    const MINUS_ONE: u64 = u64::MAX;
    match op {
        // --- arith: all low rect ---
        ZiskOp::Mulu
        | ZiskOp::Muluh
        | ZiskOp::Mulsuh
        | ZiskOp::Mul
        | ZiskOp::Mulh
        | ZiskOp::MulW
        | ZiskOp::Divu
        | ZiskOp::Remu
        | ZiskOp::Div
        | ZiskOp::Rem
        | ZiskOp::DivuW
        | ZiskOp::RemuW
        | ZiskOp::DivW
        | ZiskOp::RemW => vec![low()],

        // --- binary extension ---
        ZiskOp::SignExtendB
        | ZiskOp::SignExtendH
        | ZiskOp::SignExtendW
        | ZiskOp::Sll
        | ZiskOp::SllW
        | ZiskOp::Sra
        | ZiskOp::SraW
        | ZiskOp::SrlW => vec![low()],
        ZiskOp::Srl => vec![
            low(),
            // a >= 0xFFFF_FFFF_FFFF_F000 && b <= 64
            rg(0xFFFF_FFFF_FFFF_F000, 0x1000, 1, 0, 65),
        ],

        // --- binary basic: simple low rect ---
        ZiskOp::AddW
        | ZiskOp::EqW
        | ZiskOp::LtuW
        | ZiskOp::LtW
        | ZiskOp::Leu
        | ZiskOp::Le
        | ZiskOp::LeuW
        | ZiskOp::LeW => vec![low()],

        // EQ: (b==0 && a<=0xFFFFF) || low
        ZiskOp::Eq => vec![rg(0, 0x10_0000, 1, 0, 1), low()],
        // LTU: low || (b==1 && a>=0xFFFF_FFFF_FFFF_FF80)
        ZiskOp::Ltu => vec![low(), rg(0xFFFF_FFFF_FFFF_FF80, 0x80, 1, 1, 1)],
        // OR: low || (a<0x1000 && b<=16)
        ZiskOp::Or => vec![low(), rg(0, 0x1000, 1, 0, 17)],
        // SUB: low || (a<4192 && b<=8)
        ZiskOp::Sub => vec![low(), rg(0, 4192, 1, 0, 9)],
        // XOR: low || (a<2 && b==MAX_U64)
        ZiskOp::Xor => vec![low(), rg(0, 2, 1, MINUS_ONE, 1)],
        // ADD: low || several b-specific address bands
        ZiskOp::Add => vec![
            low(),
            rg(0xA010_0000, 0x10_0000 / 8, 8, 0, 1), // b==0, 8-aligned addr
            rg(0xA010_0000, 0x10_0000, 1, 1, 1),     // b==1, addr range
            rg(0xA010_0000, 0x10_0000 / 8, 8, 8, 1), // b==8, data addr 8-aligned
            rg(0x8000_0000, 0x80_0000 / 8, 8, 8, 1), // b==8, code addr 8-aligned
            rg(0, 24628, 1, MINUS_ONE, 1),           // b==-1, a<24628
            rg(0, 1024, 1, MINUS_ONE - 8, 9),        // b in [-9,-1], a<1024
        ],
        // AND: low || a==MASK(b range) || (b==0xFF..F8 && a<1024) || (b==7 && addr 8-aligned)
        ZiskOp::And => vec![
            low(),
            rg(0xFFFF_FFFF_FFFF_FFFC, 1, 1, 0x8000_0000, 0x90_0000), // a==MASK, b in [..) (b&3 dropped)
            rg(0, 1024, 1, 0xFFFF_FFFF_FFFF_FFF8, 1),                // b==0xFF..F8
            rg(0xA010_0000, 0x10_0000 / 8, 8, 7, 1),                 // b==7, 8-aligned addr
        ],
        // LT: low || (a==0 && b<0x10000)  [coupled distance term dropped]
        ZiskOp::Lt => vec![low(), rg(0, 1, 1, 0, 0x1_0000)],
        // SUB_W: low || (a==0 && b<386)   [b&MASK==addr term dropped]
        ZiskOp::SubW => vec![low(), rg(0, 1, 1, 0, LOW)],

        _ => vec![],
    }
}

/// Generates the x86-64 macros for the ORIGINAL hand-tuned FROPS (cycle-comparison aid).
pub fn generate_original_asm(out: &Path) -> std::io::Result<()> {
    let mut s =
        asm_header("ORIGINAL FROPS macros (cycle comparison; offsets not table-accurate)", "");
    s.push_str(
        "# NOTE: LT / AND / SUB_W contain non-box hand-tuned terms; see codegen.rs. Large b\n",
    );
    s.push_str("# immediates spill rax, so t0/t1 must not be rax here.\n\n");
    for table in FropsTable::all() {
        s.push_str(&format!("# ---- {} ----\n", table.key()));
        for code in 0u8..=255 {
            if let Some(info) = classify(code) {
                if info.table == table {
                    s.push_str(&emit_op_macro(info.name, &original_regions(code), 0, table));
                }
            }
        }
    }
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(out, s)
}

fn op_blocks(prop: &Proposal, table: FropsTable) -> Vec<OpBlock> {
    // Group selected regions by opcode for this table.
    let mut by_op: BTreeMap<u8, Vec<Region>> = BTreeMap::new();
    for s in &prop.selected {
        if s.info.table == table {
            by_op.entry(s.info.code).or_default().push(s.region);
        }
    }
    by_op
        .into_iter()
        .map(|(code, mut regions)| {
            // Deterministic region order: low_rect, mid_box, high_box.
            regions.sort_by_key(region_order);
            let info = prop.op_info[&code];
            let variant = variant_ident(code).unwrap_or_else(|| format!("Op{code:#04x}"));
            let const_name = format!("OP_{}", variant.to_uppercase());
            OpBlock { info, const_name, variant, regions }
        })
        .collect()
}

fn region_order(r: &Region) -> usize {
    match r.kind {
        crate::region::RegionKind::LowRect => 0,
        crate::region::RegionKind::MidBox => 1,
        crate::region::RegionKind::HighBox => 2,
    }
}

/// True if `e` is a single parenthesised group, e.g. `(a - 5)` (so it needs no extra wrap for a cast).
fn fully_parenthesized(e: &str) -> bool {
    if !e.starts_with('(') || !e.ends_with(')') {
        return false;
    }
    let mut depth = 0i32;
    for (i, ch) in e.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return i == e.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}

/// Emits the offset expression `(a - a_lo) / stride * b_count + (b - b_lo)) as usize + base`, with
/// minimal parentheses (`as` binds tighter than the arithmetic ops, so the value needs exactly one
/// wrapping layer unless it is a bare identifier or already a single parenthesised group).
fn offset_expr(r: &Region, base: u64) -> String {
    let mut a_term = if r.a_lo == 0 { "a".to_string() } else { format!("(a - {:#X})", r.a_lo) };
    if r.a_stride > 1 {
        a_term = format!("({a_term} / {})", r.a_stride);
    }
    let a_scaled = if r.b_count == 1 { a_term } else { format!("{a_term} * {}", r.b_count) };
    let b_term = if r.b_count == 1 {
        String::new()
    } else if r.b_lo == 0 {
        " + b".to_string()
    } else {
        format!(" + (b - {:#X})", r.b_lo)
    };
    let rel = format!("{a_scaled}{b_term}");
    let cast = if rel == "a" || rel == "b" || fully_parenthesized(&rel) {
        format!("{rel} as usize")
    } else {
        format!("({rel}) as usize")
    };
    if base == 0 {
        cast
    } else {
        format!("{cast} + {base}")
    }
}

/// Emits the `build_table` enumeration loops for one region.
fn enum_loops(r: &Region) -> String {
    let a_loop = if r.a_to_max() {
        format!("        for a in {:#X}..=u64::MAX {{\n", r.a_lo)
    } else if r.a_stride > 1 {
        let a_hi = r.a_lo + r.a_count * r.a_stride;
        format!("        for a in ({:#X}..{:#X}).step_by({}) {{\n", r.a_lo, a_hi, r.a_stride)
    } else if r.a_lo == 0 {
        format!("        for a in 0..{} {{\n", r.a_count)
    } else {
        format!("        for a in {:#X}..{:#X} {{\n", r.a_lo, r.a_lo + r.a_count)
    };
    let b_loop = if r.b_to_max() {
        format!("            for b in {:#X}..=u64::MAX {{\n", r.b_lo)
    } else if r.b_lo == 0 {
        format!("            for b in 0..{} {{\n", r.b_count)
    } else {
        format!("            for b in {:#X}..{:#X} {{\n", r.b_lo, r.b_lo + r.b_count)
    };
    format!("{a_loop}{b_loop}                ops.push([a, b]);\n            }}\n        }}\n")
}

fn emit_file(m: &TableMeta, blocks: &[OpBlock], cfg: &Config) -> String {
    let mut s = String::new();

    s.push_str("#![allow(dead_code)]\n");
    // The box predicates are emitted as `a >= LO && a < HI`; clippy would rewrite them to
    // `Range::contains` and factor common terms, but this generated form is intentional.
    s.push_str("#![allow(clippy::manual_range_contains, clippy::nonminimal_bool)]\n");
    s.push_str("// @generated by frops-analyzer — do not edit by hand.\n");
    s.push_str(&format!("// {}\n", params_comment(cfg)));
    s.push_str("use sm_frequent_ops::FrequentOpsHelpers;\n");
    s.push_str("use std::error::Error;\n");
    s.push_str("use zisk_core::zisk_ops::ZiskOp;\n\n");

    // Opcode constants.
    for b in blocks {
        s.push_str(&format!("const {}: u8 = ZiskOp::{}.code();\n", b.const_name, b.variant));
    }
    s.push('\n');

    // OP_TABLE_OFFSETS.
    let (start, offsets) = table_offsets(blocks);
    if offsets.is_empty() {
        s.push_str("const OP_TABLE_OFFSETS_START: usize = 256;\n");
        s.push_str("const OP_TABLE_OFFSETS: [usize; 0] = [];\n\n");
    } else {
        s.push_str(&format!("const OP_TABLE_OFFSETS_START: usize = {start};\n"));
        s.push_str(&format!(
            "const OP_TABLE_OFFSETS: [usize; {}] = {:?};\n\n",
            offsets.len(),
            offsets
        ));
    }

    // Struct + impl scaffolding.
    s.push_str("#[derive(Debug, Clone)]\n");
    s.push_str(&format!("pub struct {} {{\n    table: FrequentOpsHelpers,\n}}\n\n", m.struct_name));
    s.push_str("const FREQUENT_OP_EMPTY: usize = 256;\n\n");
    s.push_str(&format!(
        "impl Default for {0} {{\n    fn default() -> Self {{\n        Self::new()\n    }}\n}}\n\n",
        m.struct_name
    ));
    s.push_str(&format!("impl {} {{\n", m.struct_name));
    s.push_str(&format!("    pub const TABLE_ID: usize = {};\n", m.table_id));
    s.push_str("    pub const NO_FROPS: usize = FrequentOpsHelpers::NO_FROPS;\n");
    s.push_str(
        "    pub fn new() -> Self {\n        Self { table: FrequentOpsHelpers::new() }\n    }\n\n",
    );

    // build_table
    s.push_str("    pub fn build_table(&mut self) {\n");
    if blocks.is_empty() {
        s.push_str("        // No frequent operations proposed.\n");
    }
    for b in blocks {
        s.push_str(&format!("        // op {}\n", b.info.name));
        s.push_str("        {\n");
        s.push_str("        let mut ops: Vec<[u64; 2]> = Vec::new();\n");
        for r in &b.regions {
            s.push_str(&format!("        // {}: {}\n", r.kind.as_str(), r.predicate()));
            s.push_str(&enum_loops(r));
        }
        s.push_str(&format!("        self.table.add_ops({}, &mut ops, true);\n", b.const_name));
        s.push_str("        }\n");
    }
    s.push_str("    }\n\n");

    // is_frequent_op
    s.push_str("    #[inline(always)]\n");
    if blocks.is_empty() {
        s.push_str("    pub fn is_frequent_op(_op: u8, _a: u64, _b: u64) -> bool {\n");
        s.push_str("        false\n    }\n\n");
    } else {
        s.push_str("    pub fn is_frequent_op(op: u8, a: u64, b: u64) -> bool {\n");
        s.push_str("        match op {\n");
        for b in blocks {
            // OR-join the region predicates. Each predicate is an `&&`-chain and `&&` binds tighter
            // than `||`, so no wrapping parens are needed (avoids the unused_parens lint).
            let arm = b.regions.iter().map(|r| r.predicate()).collect::<Vec<_>>().join(" || ");
            s.push_str(&format!("            {} => {},\n", b.const_name, arm));
        }
        s.push_str("            _ => false,\n");
        s.push_str("        }\n    }\n\n");
    }

    // get_row
    s.push_str("    #[inline(always)]\n");
    if blocks.is_empty() {
        s.push_str("    pub fn get_row(_op: u8, _a: u64, _b: u64) -> usize {\n");
        s.push_str("        Self::NO_FROPS\n    }\n\n");
    } else {
        s.push_str("    pub fn get_row(op: u8, a: u64, b: u64) -> usize {\n");
        s.push_str("        let relative_offset = match op {\n");
        for b in blocks {
            s.push_str(&format!("            {} => {{\n", b.const_name));
            let mut base = 0u64;
            for (i, r) in b.regions.iter().enumerate() {
                let kw = if i == 0 { "if" } else { "} else if" };
                s.push_str(&format!("                {} {} {{\n", kw, r.predicate()));
                s.push_str(&format!("                    {}\n", offset_expr(r, base)));
                base += r.rows();
            }
            s.push_str("                } else {\n");
            s.push_str("                    Self::NO_FROPS\n");
            s.push_str("                }\n");
            s.push_str("            }\n");
        }
        s.push_str("            _ => return Self::NO_FROPS,\n");
        s.push_str("        };\n");
        s.push_str("        if relative_offset == Self::NO_FROPS {\n");
        s.push_str("            Self::NO_FROPS\n");
        s.push_str("        } else {\n");
        s.push_str(
            "            relative_offset + OP_TABLE_OFFSETS[op as usize - OP_TABLE_OFFSETS_START]\n",
        );
        s.push_str("        }\n");
        s.push_str("    }\n\n");
    }

    // Boilerplate tail (identical in behaviour to the hand-written files).
    s.push_str(&tail_methods(m.air_name));
    s.push_str("}\n\n");

    // Offset-consistency test. Kept private so the glob re-export in lib.rs is unambiguous.
    s.push_str("#[test]\n");
    s.push_str("fn test_table_offsets() {\n");
    s.push_str(&format!("    let mut fops = {}::new();\n", m.struct_name));
    s.push_str("    fops.test_table_offsets();\n");
    s.push_str("}\n\n");

    // Accessibility test: every materialised pair must be found by get_row / is_frequent_op.
    s.push_str("#[test]\n");
    s.push_str("fn test_all_accessible_values() {\n");
    s.push_str(&format!("    let mut fops = {}::new();\n", m.struct_name));
    s.push_str("    fops.build_table();\n");
    s.push_str("    let table = fops.generate_full_table();\n");
    s.push_str("    FrequentOpsHelpers::test_all_accessible_values(\n");
    s.push_str("        &table,\n");
    s.push_str(&format!("        {}::is_frequent_op,\n", m.struct_name));
    s.push_str(&format!("        {}::get_row,\n", m.struct_name));
    s.push_str("    );\n");
    s.push_str("}\n");

    s
}

fn tail_methods(air_name: &str) -> String {
    format!(
        r#"    #[inline(always)]
    pub fn count(&self) -> usize {{
        self.table.count()
    }}

    #[cfg(test)]
    pub fn test_table_offsets(&mut self) {{
        self.build_table();
        let (start, offsets) = self.table.generate_table_offsets();
        if (start != OP_TABLE_OFFSETS_START) || (offsets != OP_TABLE_OFFSETS) {{
            self.table.print_table_offsets();
            panic!("Table offsets do not match expected values");
        }}
        assert_eq!(start, OP_TABLE_OFFSETS_START);
        assert_eq!(offsets, OP_TABLE_OFFSETS);
    }}

    #[inline(always)]
    pub fn generate_full_table(&self) -> Vec<(u8, u64, u64, u64, bool)> {{
        self.table.generate_full_table()
    }}

    #[inline(always)]
    pub fn generate_table(&self) -> Vec<(u8, u64, u64)> {{
        self.table.generate_table()
    }}

    #[inline(always)]
    pub fn generate_cmd(
        &mut self,
        cmd_name: &'static str,
        default_file: &'static str,
    ) -> Result<(), Box<dyn Error>> {{
        self.build_table();
        let full_table = self.generate_full_table();
        let full_table_count = full_table.len();
        self.table.generate_cmd(
            "Zisk",
            "{air_name}",
            cmd_name,
            default_file,
            full_table,
            full_table_count,
        )
    }}
"#
    )
}

/// Computes `(START, offsets)` exactly as `FrequentOpsHelpers::generate_table_offsets` would for the
/// rows implied by `blocks`.
fn table_offsets(blocks: &[OpBlock]) -> (usize, Vec<usize>) {
    if blocks.is_empty() {
        return (256, Vec::new());
    }
    let rows_by_op: BTreeMap<u8, u64> =
        blocks.iter().map(|b| (b.info.code, b.regions.iter().map(|r| r.rows()).sum())).collect();
    let start = *rows_by_op.keys().next().unwrap() as usize;
    let end = *rows_by_op.keys().next_back().unwrap() as usize;
    let mut offsets = vec![0usize; end - start + 1];
    let mut running = 0usize;
    for code in start..=end {
        if let Some(&rows) = rows_by_op.get(&(code as u8)) {
            offsets[code - start] = running;
            running += rows as usize;
        }
    }
    (start, offsets)
}
