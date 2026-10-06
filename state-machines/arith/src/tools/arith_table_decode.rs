//! Decodes `ArithTable` into a canonical, encoding-independent text form.
//!
//! The raw table stores `[op, flags, range_ab, range_cd]`, where the flag bit layout and the range id
//! numbering are both implementation details that have changed before and can change again. Comparing
//! two versions of the raw table is therefore all noise. This module decodes it: the flags are named
//! and the range ids are expanded into the range **type** of each of the eight constrained chunks.
//! Two tables decoded this way are directly comparable with `diff`.
//!
//! The decoded form of the current table is committed at `docs/arith_table.txt`, so that the semantic
//! delta of a change shows up as a diff of that file in review. Regenerate it with
//!
//! ```text
//! cargo run --release --bin arith_table_decode_gen
//! ```
//!
//! and `committed_decoding_is_up_to_date` fails if you forget.
//!
//! To compare against another revision, dump its table and decode it with the layout it used:
//!
//! ```text
//! git show <rev>:state-machines/arith/src/arith_table_data.rs > /tmp/old.rs
//! cargo run --release --bin arith_table_decode_gen -- /tmp/old.rs --legacy > /tmp/old.txt
//! diff /tmp/old.txt state-machines/arith/docs/arith_table.txt
//! ```

use crate::{arith_range_table_helpers::RANGE_PATTERN, ARITH_TABLE};

/// Where the two "is zero" facts live in the packed flags.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ZeroFlagEncoding {
    /// Bit 8 is `div_overflow_mul_rz`: it means `div_overflow` on divisions and "the result is zero"
    /// on multiplications, and there is no way to say "the remainder is zero". The flags it cannot
    /// express are decoded as `?`.
    Overloaded,
    /// Bit 8 is `div_overflow`, bit 12 `result_is_zero`, bit 13 `remainder_is_zero`.
    Split,
}

/// Everything needed to decode a table: how the range ids index the slot sequence, and how the flags
/// are packed.
pub struct TableLayout<'a> {
    /// Slot sequence of `ArithRangeTable`: `F` = any 16 bits, `+` = `[0, 0x7FFF]`,
    /// `-` = `[0x8000, 0xFFFF]`.
    pub range_pattern: &'a str,
    /// Offsets at which a range id is read for the `(x3, x1, y3, y1)` views. See the
    /// `range_a3 / range_a1 / range_b3 / range_b1` definitions in `pil/arith.pil`.
    pub range_offsets: [usize; 4],
    pub zero_flags: ZeroFlagEncoding,
}

/// The layout the committed table uses.
pub const CURRENT_LAYOUT: TableLayout<'static> = TableLayout {
    range_pattern: RANGE_PATTERN,
    range_offsets: [0, 1, 2, 3],
    zero_flags: ZeroFlagEncoding::Split,
};

/// The layout in use before the range-id recompression and the flag split: 43 slots read at
/// `0 / 26 / 17 / 9`, with `div_overflow_mul_rz` overloaded.
pub const LEGACY_LAYOUT: TableLayout<'static> = TableLayout {
    range_pattern: "FFF+++---FFFFFFFFF+-F+-F+-FFFFFFFFFFF+++---",
    range_offsets: [0, 26, 17, 9],
    zero_flags: ZeroFlagEncoding::Overloaded,
};

fn op_name(op: u16) -> &'static str {
    match op {
        0xB0 => "mulu",
        0xB1 => "muluh",
        0xB3 => "mulsuh",
        0xB4 => "mul",
        0xB5 => "mulh",
        0xB6 => "mul_w",
        0xB8 => "divu",
        0xB9 => "remu",
        0xBA => "div",
        0xBB => "rem",
        0xBC => "divu_w",
        0xBD => "remu_w",
        0xBE => "div_w",
        0xBF => "rem_w",
        _ => panic!("unknown arith opcode 0x{op:x}"),
    }
}

/// Decodes one row. `coarse` drops the two "is zero" columns, which is the projection both encodings
/// can express; a diff of two coarse decodings shows only genuine semantic changes.
fn decode_row(row: &[u16; 4], layout: &TableLayout, coarse: bool) -> String {
    let (op, flags, ab, cd) = (row[0], row[1], row[2], row[3]);
    let bit = |i: u32| (flags >> i) & 1;
    let (m32, div) = (bit(0), bit(1));
    let (na, nb, np, nr) = (bit(2), bit(3), bit(4), bit(5));
    let (sext, dbz) = (bit(6), bit(7));
    let (main_mul, main_div, signed) = (bit(9), bit(10), bit(11));

    let (dov, rz, remz) = match layout.zero_flags {
        ZeroFlagEncoding::Overloaded if div == 1 => (bit(8), "?".to_string(), "?".to_string()),
        ZeroFlagEncoding::Overloaded => (0, bit(8).to_string(), "0".to_string()),
        ZeroFlagEncoding::Split => (bit(8), bit(12).to_string(), bit(13).to_string()),
    };

    let slots: Vec<char> = layout.range_pattern.chars().collect();
    let ty = |rid: u16, view: usize| slots[rid as usize + layout.range_offsets[view]];
    let (a3, a1, b3, b1) = (ty(ab, 0), ty(ab, 1), ty(ab, 2), ty(ab, 3));
    let (c3, c1, d3, d1) = (ty(cd, 0), ty(cd, 1), ty(cd, 2), ty(cd, 3));

    let zeros = if coarse { String::new() } else { format!(" rz={rz} remz={remz}") };
    format!(
        "{:<7} m32={m32} div={div} | na={na} nb={nb} np={np} nr={nr} | sext={sext} dbz={dbz} \
         dov={dov}{zeros} | mul={main_mul} dv={main_div} sg={signed} | \
         a3={a3} a1={a1} b3={b3} b1={b1}  c3={c3} c1={c1} d3={d3} d1={d1}",
        op_name(op)
    )
}

const HEADER: &str = "\
# ArithTable decoded to a canonical form. GENERATED - do not edit.
#
#   cargo run --release --bin arith_table_decode_gen
#
# The raw table stores [op, flags, range_ab, range_cd]. The flag bit layout and the range id
# numbering are implementation details, so this file decodes them away: flags are named and the range
# ids are expanded into the range TYPE of each of the eight constrained chunks. Two tables decoded
# this way are directly comparable with diff, which is the point: the semantic delta of a change to
# the table shows up here as a diff.
#
# Columns
#   m32, div            operation width and family
#   na, nb, np, nr      sign of a / b / (product | dividend) / remainder
#   sext                the 32-bit result drives sign extension
#   dbz                 div_by_zero
#   dov                 div_overflow
#   rz                  result_is_zero      (the product for mul, the quotient for div)
#   remz                remainder_is_zero
#   mul, dv, sg         main_mul, main_div, signed
#   a3 a1 b3 b1 c3 c1 d3 d1   range type of each constrained chunk:
#                             F = any 16 bits, + = [0, 0x7FFF], - = [0x8000, 0xFFFF]
#
# '?' marks a fact the encoding being decoded cannot express.
#
# Sorted canonically, not in table order: row indices are an implementation detail.
";

/// Decodes a whole table. `coarse` drops the two "is zero" columns and collapses the rows that then
/// become equal.
pub fn decode_table(rows: &[[u16; 4]], layout: &TableLayout, coarse: bool) -> String {
    let mut lines: Vec<String> = rows.iter().map(|r| decode_row(r, layout, coarse)).collect();
    lines.sort_unstable();
    if coarse {
        lines.dedup();
    }
    let kind = if coarse { " (coarse: the two is-zero columns projected out)" } else { "" };
    format!("{HEADER}# rows: {}{kind}\n\n{}\n", lines.len(), lines.join("\n"))
}

/// Decodes the table this build was compiled with.
pub fn decode_current_table(coarse: bool) -> String {
    decode_table(&ARITH_TABLE, &CURRENT_LAYOUT, coarse)
}

/// Extracts `[op, flags, range_ab, range_cd]` tuples from arbitrary text, so that a table dumped from
/// another revision can be decoded without depending on that revision's code.
pub fn parse_rows(text: &str) -> Vec<[u16; 4]> {
    let mut rows = Vec::new();
    for chunk in text.split('[').skip(1) {
        let Some(end) = chunk.find(']') else { continue };
        let fields: Vec<&str> = chunk[..end].split(',').map(str::trim).collect();
        if fields.len() != 4 {
            continue;
        }
        let parsed: Option<Vec<u16>> = fields.iter().map(|f| f.parse::<u16>().ok()).collect();
        if let Some(v) = parsed {
            // the opcode range is what distinguishes a table row from any other 4-tuple in the file
            if (0xB0..=0xBF).contains(&v[0]) {
                rows.push([v[0], v[1], v[2], v[3]]);
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The golden check: `docs/arith_table.txt` must be what the current table decodes to. Without
    /// this the committed file rots silently on the next change to the PIL filters, which is worse
    /// than having no file at all because it would still look authoritative.
    #[test]
    fn committed_decoding_is_up_to_date() {
        let committed = include_str!("../../docs/arith_table.txt");
        let actual = decode_current_table(false);
        assert_eq!(
            actual, committed,
            "docs/arith_table.txt is stale; regenerate it with \
             `cargo run --release --bin arith_table_decode_gen`"
        );
    }

    #[test]
    fn parse_rows_reads_the_generated_table() {
        let rows = parse_rows(include_str!("../arith_table_data.rs"));
        assert_eq!(rows.len(), ARITH_TABLE.len());
        assert_eq!(rows[0], ARITH_TABLE[0]);
    }

    /// The coarse projection is what makes a comparison across a renumbering meaningful, so check it
    /// really does collapse rows that differ only in the two is-zero flags.
    #[test]
    fn coarse_projection_collapses_the_zero_flags() {
        let full = decode_current_table(false);
        let coarse = decode_current_table(true);
        let count = |s: &str| s.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).count();
        assert_eq!(count(&full), ARITH_TABLE.len());
        assert!(
            count(&coarse) < count(&full),
            "the current table has rows differing only in rz/remz, so the projection must be smaller"
        );
        assert!(!coarse.contains(" rz="), "the coarse form must not carry the is-zero columns");
    }
}
