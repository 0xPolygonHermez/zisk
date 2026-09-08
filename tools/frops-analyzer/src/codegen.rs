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

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::ops::{variant_ident, FropsTable, OpInfo};
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

    // Additionally emit the box data that `zisk-core` compiles in: the predicates above and the
    // assembly that counts frequent operations are generated from the same selection.
    let regions_path = workspace_root.join(REGIONS_REL_PATH);
    // Same rule as for the sources above: keep the *first* original, never clobber it later.
    let mut regions_backed = false;
    let regions_bak = regions_path.with_extension("rs.bak");
    if regions_path.exists() && !regions_bak.exists() {
        fs::copy(&regions_path, &regions_bak)?;
        regions_backed = true;
    }
    fs::write(&regions_path, emit_regions(prop))?;
    written.push((REGIONS_REL_PATH.to_string(), regions_backed));

    Ok(written)
}

// ============================================================================================
// Box data for `zisk-core`: the single source of truth shared by the generated state-machine
// predicates above and the x86-64 counting code that `zisk_core::frops_asm` emits into the
// ROM-histogram assembly.
// ============================================================================================

/// Path of the generated box-data module, relative to the workspace root.
const REGIONS_REL_PATH: &str = "core/src/frops_regions.rs";

fn base_const(table: FropsTable) -> &'static str {
    match table {
        FropsTable::Arith => "FROPS_ARITH_BASE",
        FropsTable::BinaryBasic => "FROPS_BINARY_BASIC_BASE",
        FropsTable::BinaryExt => "FROPS_BINARY_EXT_BASE",
    }
}

fn family_rows(blocks: &[OpBlock]) -> u64 {
    blocks.iter().flat_map(|b| b.regions.iter()).map(|r| r.rows()).sum()
}

/// Emits `core/src/frops_regions.rs`: every selected box with the global table row it starts at.
///
/// The three family tables are concatenated in `FropsTable::all()` order, so a box's `base_row` is
/// its family base plus the rows of the lower opcodes and of the earlier boxes of its own opcode —
/// the same layout `OP_TABLE_OFFSETS` describes per family.
fn emit_regions(prop: &Proposal) -> String {
    let mut s = String::new();
    s.push_str("// @generated by frops-analyzer — do not edit by hand.\n");
    s.push_str(&format!("// {}\n", params_comment(&prop.config)));
    s.push_str("//\n");
    s.push_str(
        "// Box data for the FROPS tables. See `crate::frops` for the shape and the row layout\n",
    );
    s.push_str("// invariant, and `crate::frops_asm` for the assembly emitted from it.\n");
    s.push_str("use crate::frops::FropsRegion;\n\n");

    let families: Vec<(FropsTable, Vec<OpBlock>)> =
        FropsTable::all().into_iter().map(|t| (t, op_blocks(prop, t))).collect();
    let total: u64 = families.iter().map(|(_, b)| family_rows(b)).sum();

    s.push_str(
        "/// Total rows of the global FROPS table (the three family tables concatenated).\n",
    );
    s.push_str(&format!("pub const FROPS_TABLE_ROWS: u64 = {total};\n\n"));

    let mut bases = Vec::new();
    let mut base = 0u64;
    for (table, blocks) in &families {
        s.push_str(&format!(
            "/// Base row of the {} family table within the global table.\n",
            table.key().replace('_', " ")
        ));
        s.push_str(&format!("pub const {}: u64 = {base};\n", base_const(*table)));
        bases.push(base);
        base += family_rows(blocks);
    }
    s.push('\n');

    let mut idents: BTreeMap<u8, String> = BTreeMap::new();
    for ((_, blocks), base) in families.iter().zip(bases) {
        let mut row = base;
        for b in blocks {
            let ident = format!("OP_{:02X}", b.info.code);
            s.push_str(&format!("/// `{}` (0x{:02x})\n", b.info.name, b.info.code));
            // One box per line is far more readable than what rustfmt would do with it.
            s.push_str("#[rustfmt::skip]\n");
            s.push_str(&format!("static {ident}: [FropsRegion; {}] = [\n", b.regions.len()));
            for r in &b.regions {
                s.push_str(&format!("    // {}: {}\n", r.kind.as_str(), r.predicate()));
                s.push_str(&format!(
                    "    FropsRegion {{ a_lo: {:#x}, a_count: {}, a_stride: {}, b_lo: {:#x}, \
                     b_count: {}, base_row: {row} }},\n",
                    r.a_lo, r.a_count, r.a_stride, r.b_lo, r.b_count
                ));
                row += r.rows();
            }
            s.push_str("];\n\n");
            idents.insert(b.info.code, ident);
        }
    }

    s.push_str("/// Boxes per opcode, in test order. See [`crate::frops::frops_regions`].\n");
    s.push_str("#[rustfmt::skip]\n");
    s.push_str("pub static FROPS_REGIONS: [&[FropsRegion]; 256] = [\n");
    for first in (0..256u32).step_by(8) {
        let cells: Vec<String> = (first..first + 8)
            .map(|c| match idents.get(&(c as u8)) {
                Some(ident) => format!("&{ident},"),
                None => "&[],".to_string(),
            })
            .collect();
        s.push_str(&format!("    /* {first:#04x} */ {}\n", cells.join(" ")));
    }
    s.push_str("];\n");
    s
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
    s.push_str("use zisk_sm_frequent_ops::FrequentOpsHelpers;\n");
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
