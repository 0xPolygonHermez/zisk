//! Computes the exact range of the `carry` columns of the `Arith` AIR and checks that the range
//! table in `pil/arith_range_table.pil` covers it.
//!
//! cargo run --release --bin arith_carry_range
//!
//! # Method
//!
//! The carry chain in `pil/arith.pil` is
//!
//! ```text
//! eq[0]                 - carry[0] * M   === 0
//! eq[k] + carry[k-1]    - carry[k] * M   === 0     (0 < k < 7)
//! eq[7] + carry[6]                       === 0
//! ```
//!
//! Unrolling it gives a closed form that does not depend on the intermediate carries:
//!
//! ```text
//! carry[k] = (sum_{j<=k} eq[j] * M^j) / M^(k+1)
//! ```
//!
//! For fixed flags, every `eq[j]` is **multilinear** in the 16 chunk variables `a/b/c/d[0..3]`:
//! each term is either a single chunk or a product of two *distinct* chunks, so no variable ever
//! appears squared. A multilinear function attains its extrema on a box at a *vertex* of that box,
//! and `carry[k]` above is a positive multiple of a multilinear function. Therefore enumerating
//!
//! ```text
//! chunks in {0, 0xFFFF}^16   x   flags in {0,1}^6
//! ```
//!
//! yields the **exact** extremum over the full domain `[0, 0xFFFF]^16`, not just a bound.
//!
//! The chunks are range-checked to 16 bits (`ARITH_RANGE_16_BITS`, or the `POS`/`NEG` halves for
//! the odd indices, which are sub-intervals of `[0, 0xFFFF]`), and `na`/`nb`/`np`/`nr`/`div`/`m32`
//! are binary, so this domain is a superset of what any satisfying assignment can use — the result
//! is a sound bound for the range check, not merely the range the executor happens to produce.
//!
//! Two domains are reported:
//!
//! * **constraints only** — flags and all 16 chunks free. The safe bound.
//! * **+ table implications** — additionally applies `nr => div` (forced by `arith_table`, the
//!   `[CBT]` comments in the PIL) and the `m32 * bus_a1 === 0` / `m32 * bus_b1 === 0` zeroing.
//!
//! # Symmetry
//!
//! The range is symmetric **up to one unit**, and which side is wider depends on the domain:
//!
//! * Low indices (`carry[0..3]`) lean *negative* by 1 on the free domain, because the coefficient
//!   of `d[i]` is `(div - 2*nr)`, whose domain `{0, +1, -1, -2}` is lopsided. The `-2` needs
//!   `nr = 1, div = 0`, which `arith_table` forbids, so with the table implications applied these
//!   indices come out exactly symmetric.
//! * High indices (`carry[4..6]`) lean *positive* by 1, and this one is structural: `fab` and
//!   `na_fb`/`nb_fa` are coupled through `na`/`nb`, so only 4 of the 12 sign combinations occur —
//!   `fab = +1` forces `na_fb = nb_fa ∈ {0, -1}` while `fab = -1` forces one of them to `+1` and
//!   the other to `0`. That set is not closed under global negation. Zeroing just the
//!   `a[i] * nb_fa` / `b[i] * na_fb` terms makes every index symmetric, which confirms they are the
//!   sole cause; the sign-fixed constant terms contribute nothing to the skew.
//!
//! Either way the skew is 1 out of 2^16..2^18, so a symmetric range table costs nothing.
//!
//! # Keeping this in sync
//!
//! `chunk_exprs` below is a transcription of `eq[0..7]` in `pil/arith.pil`. If those expressions
//! change, update this file and re-run it.

use std::process::ExitCode;

/// Maximum value of a 16-bit chunk.
const K: i64 = 0xFFFF;
/// Chunk base, `CHUNK_SIZE` in the PIL.
const M: i64 = 0x10000;

/// Carry range currently provided by `ArithRangeTable` (`MIN_CARRY_RANGE`/`MAX_CARRY_RANGE`).
const TABLE_MIN_CARRY: i64 = -262142;
const TABLE_MAX_CARRY: i64 = 262142;

/// Rows the chunk-range slots take before the carry block: `12 * 2^16 + 8 * 2^15`.
const CHUNK_RANGE_ROWS: i64 = 0x100000;

/// Number of carry columns: `CHUNKS_OP - 1`.
const CARRIES: usize = 7;

#[derive(Clone, Copy)]
struct Assignment {
    a: [i64; 4],
    b: [i64; 4],
    c: [i64; 4],
    d: [i64; 4],
    na: i64,
    nb: i64,
    np: i64,
    nr: i64,
    div: i64,
    m32: i64,
}

impl std::fmt::Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "na={} nb={} np={} nr={} div={} m32={} a={:x?} b={:x?} c={:x?} d={:x?}",
            self.na, self.nb, self.np, self.nr, self.div, self.m32, self.a, self.b, self.c, self.d
        )
    }
}

/// Transcription of `eq[0..7]` in `pil/arith.pil`.
///
/// `rustfmt::skip` keeps one term per line so this stays diffable against the PIL.
#[rustfmt::skip]
fn chunk_exprs(v: &Assignment) -> [i64; 8] {
    let (a, b, c, d) = (&v.a, &v.b, &v.c, &v.d);
    let (na, nb, np, nr, div, m32) = (v.na, v.nb, v.np, v.nr, v.div, v.m32);

    let fab = 1 - 2 * na - 2 * nb + 4 * na * nb;
    let na_fb = na * (1 - 2 * nb);
    let nb_fa = nb * (1 - 2 * na);

    let mut e = [0i64; 8];

    e[0] = fab * a[0] * b[0] - c[0] + 2 * np * c[0] + div * d[0] - 2 * nr * d[0];

    e[1] =
        fab * a[1] * b[0] + fab * a[0] * b[1] - c[1] + 2 * np * c[1] + div * d[1] - 2 * nr * d[1];

    e[2] = fab * a[2] * b[0]
        + fab * a[1] * b[1]
        + fab * a[0] * b[2]
        + a[0] * nb_fa * m32
        + b[0] * na_fb * m32
        - c[2]
        + 2 * np * c[2]
        + div * d[2]
        - 2 * nr * d[2]
        - np * div * m32
        + nr * m32;

    e[3] = fab * a[3] * b[0]
        + fab * a[2] * b[1]
        + fab * a[1] * b[2]
        + fab * a[0] * b[3]
        + a[1] * nb_fa * m32
        + b[1] * na_fb * m32
        - c[3]
        + 2 * np * c[3]
        + div * d[3]
        - 2 * nr * d[3];

    e[4] = fab * a[3] * b[1]
        + fab * a[2] * b[2]
        + fab * a[1] * b[3]
        + na * nb * m32
        + b[0] * na_fb * (1 - m32)
        + a[0] * nb_fa * (1 - m32)
        - np * m32 * (1 - div)
        - np * (1 - m32) * div
        + nr * (1 - m32)
        - d[0] * (1 - div)
        + 2 * np * d[0] * (1 - div);

    e[5] = fab * a[3] * b[2]
        + fab * a[2] * b[3]
        + a[1] * nb_fa * (1 - m32)
        + b[1] * na_fb * (1 - m32)
        - d[1] * (1 - div)
        + d[1] * 2 * np * (1 - div);

    e[6] = fab * a[3] * b[3]
        + a[2] * nb_fa * (1 - m32)
        + b[2] * na_fb * (1 - m32)
        - d[2] * (1 - div)
        + 2 * np * d[2] * (1 - div);

    e[7] = M * na * nb * (1 - m32)
        + a[3] * nb_fa * (1 - m32)
        + b[3] * na_fb * (1 - m32)
        - M * np * (1 - div) * (1 - m32)
        - d[3] * (1 - div)
        + 2 * np * d[3] * (1 - div);

    e
}

struct CarryRange {
    lo: i64,
    hi: i64,
    lo_at: Assignment,
    hi_at: Assignment,
    /// Extremes of the real-valued (unrounded) carry, to tell a genuine asymmetry apart from a
    /// floor/ceil artifact of a symmetric interval.
    real_lo: f64,
    real_hi: f64,
}

/// Enumerates every vertex of the chunk box for every flag combination and returns the exact
/// range of each carry column.
fn scan(apply_table_implications: bool) -> Vec<CarryRange> {
    let zero = Assignment {
        a: [0; 4],
        b: [0; 4],
        c: [0; 4],
        d: [0; 4],
        na: 0,
        nb: 0,
        np: 0,
        nr: 0,
        div: 0,
        m32: 0,
    };
    let mut out: Vec<CarryRange> = (0..CARRIES)
        .map(|_| CarryRange { lo: 0, hi: 0, lo_at: zero, hi_at: zero, real_lo: 0.0, real_hi: 0.0 })
        .collect();

    for flags in 0..64u32 {
        let na = (flags & 1) as i64;
        let nb = ((flags >> 1) & 1) as i64;
        let np = ((flags >> 2) & 1) as i64;
        let nr = ((flags >> 3) & 1) as i64;
        let div = ((flags >> 4) & 1) as i64;
        let m32 = ((flags >> 5) & 1) as i64;

        // the arith_table forces nr = 0 for non-division ops ([CBT] comments in the PIL)
        if apply_table_implications && nr == 1 && div == 0 {
            continue;
        }

        for bits in 0..(1u32 << 16) {
            let bit = |i: u32| if (bits >> i) & 1 == 1 { K } else { 0 };
            let mut v = Assignment {
                a: [bit(0), bit(1), bit(2), bit(3)],
                b: [bit(4), bit(5), bit(6), bit(7)],
                c: [bit(8), bit(9), bit(10), bit(11)],
                d: [bit(12), bit(13), bit(14), bit(15)],
                na,
                nb,
                np,
                nr,
                div,
                m32,
            };

            if apply_table_implications && m32 == 1 {
                // m32 * bus_b1 === 0  =>  b2 = b3 = 0
                v.b[2] = 0;
                v.b[3] = 0;
                // m32 * bus_a1 === 0  =>  div ? c2 = c3 = 0 : a2 = a3 = 0
                if div == 1 {
                    v.c[2] = 0;
                    v.c[3] = 0;
                } else {
                    v.a[2] = 0;
                    v.a[3] = 0;
                }
            }

            let e = chunk_exprs(&v);

            // Real-valued unrolling of the recurrence, so the result does not depend on any
            // particular assignment dividing exactly. f64 additions here are exact (all operands
            // below 2^53) and dividing by 2^16 only shifts the exponent; the residual error is
            // ~1e-11, far below the integer granularity that gets reported.
            let mut carry = 0.0f64;
            for (k, slot) in out.iter_mut().enumerate() {
                carry = (e[k] as f64 + carry) / M as f64;
                let hi = carry.floor() as i64;
                let lo = carry.ceil() as i64;
                if hi > slot.hi {
                    slot.hi = hi;
                    slot.hi_at = v;
                }
                if lo < slot.lo {
                    slot.lo = lo;
                    slot.lo_at = v;
                }
                slot.real_hi = slot.real_hi.max(carry);
                slot.real_lo = slot.real_lo.min(carry);
            }
        }
    }
    out
}

fn report(label: &str, ranges: &[CarryRange]) -> i64 {
    println!("\n=== {label} ===");
    let mut worst = 0i64;
    let mut worst_k = 0;
    for (k, r) in ranges.iter().enumerate() {
        let mag = r.hi.max(-r.lo);
        // The integer range is floor/ceil of the real one, so a symmetric real interval always
        // yields a symmetric integer interval. Comparing the *unrounded* extremes is therefore what
        // separates a genuine asymmetry from a rounding artifact. Note the real extremes sit just
        // under a multiple of 2^16 (e.g. 65535.999985), which is why floor/ceil shave a unit off.
        let skew = r.real_hi + r.real_lo;
        let verdict = if skew.abs() < 1e-6 {
            "symmetric".to_string()
        } else {
            format!(
                "asymmetric, {} side wider by {:.0}",
                if skew > 0.0 { "+" } else { "-" },
                skew.abs()
            )
        };
        println!(
            "  carry[{k}]  int  [{:>8}, {:>8}]   |max| = {mag:>8} = {:.0} * 2^16",
            r.lo,
            r.hi,
            mag as f64 / M as f64
        );
        println!(
            "            real [{:>16.6}, {:>15.6}]   skew = {skew:+.6}  -> {verdict}",
            r.real_lo, r.real_hi
        );
        if mag > worst {
            worst = mag;
            worst_k = k;
        }
    }
    println!("  worst: carry[{worst_k}]  |carry| <= {worst}   (2^18 = {})", 1 << 18);
    println!("    max at: {}", ranges[worst_k].hi_at);
    println!("    min at: {}", ranges[worst_k].lo_at);
    worst
}

fn main() -> ExitCode {
    println!("Arith carry range");
    println!("  chunk max      : 0x{K:X}");
    println!("  chunk base     : 0x{M:X}");
    println!("  carry columns  : {CARRIES}");
    println!("  vertices/flags : {} x {}", 1u32 << 16, 64);

    // The safe bound is the one computed with flags and chunks free; the table implications only
    // narrow the domain, so they are reported for information.
    let free = scan(false);
    let bound = report("constraints only (flags and all 16 chunks free)", &free);
    let restricted = scan(true);
    report("+ arith_table / bus implications (nr => div, m32 zeroing)", &restricted);

    println!("\n=== range table check ===");
    println!("  required : [{:>8}, {:>8}]", -bound, bound);
    println!("  table    : [{TABLE_MIN_CARRY:>8}, {TABLE_MAX_CARRY:>8}]");

    let carry_rows = TABLE_MAX_CARRY - TABLE_MIN_CARRY + 1;
    let ok = -bound >= TABLE_MIN_CARRY && bound <= TABLE_MAX_CARRY;
    if !ok {
        println!("  FAIL: the range table does not cover the required carry range");
        return ExitCode::FAILURE;
    }
    let slack = carry_rows - (2 * bound + 1);
    let verdict =
        if slack == 0 { ", exactly tight".to_string() } else { format!(", {slack} rows of slack") };
    println!("  OK{verdict} (table is {carry_rows} rows, {} needed)", 2 * bound + 1);
    println!("\n=== ArithRangeTable size ===");
    let rows = CHUNK_RANGE_ROWS + carry_rows;
    println!(
        "  chunk ranges : {CHUNK_RANGE_ROWS:>9}  (12 * 2^16 + 8 * 2^15, the compressed layout)"
    );
    println!("  carry        : {carry_rows:>9}");
    println!("  total        : {rows:>9}  (2^21 = {}, 2^22 = {})", 1 << 21, 1 << 22);
    println!(
        "  ArithRangeTable is a `virtual` air, so it is sized to exactly this many rows.\n\
         \x20 For reference, the pre-compression layout needed {} rows of chunk ranges alone.",
        25 * 65536 + 18 * 32768
    );
    ExitCode::SUCCESS
}
