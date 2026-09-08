//! Shared primitives for analysing ZisK `.fixed` files.
//!
//! A `.fixed` file stores the preprocessed (fixed) polynomials of an AIR in
//! **row-major** order: for every row, all column cells are written
//! consecutively, each cell an 8-byte little-endian `u64` holding a Goldilocks
//! field element (p = 2^64 - 2^32 + 1). The total element count is
//! `rows * cols`.
//!
//! For the ZisK virtual tables the column layout is:
//! `UID[0..nuid]`, then the payload `column[0..]`, then a single trailing
//! `__L1__` selector column. See [`Layout`].

use std::convert::TryInto;
use std::fs::File;
use std::io::{BufReader, Read};

/// Goldilocks prime: 2^64 - 2^32 + 1.
pub const P: u128 = 18446744069414584321;

/// Field subtraction `a - b (mod p)` for `a, b` in `[0, p)`.
#[inline]
pub fn fsub(a: u64, b: u64) -> u64 {
    if a >= b { a - b } else { a.wrapping_sub(b).wrapping_sub(0xFFFF_FFFF) }
}

/// Field addition `a + b (mod p)`.
#[inline]
pub fn fadd(a: u64, b: u64) -> u64 {
    (((a as u128) + (b as u128)) % P) as u64
}

/// Field multiplication `a * b (mod p)`.
#[inline]
pub fn fmul(a: u64, b: u64) -> u64 {
    (((a as u128) * (b as u128)) % P) as u64
}

/// Field exponentiation `a^e (mod p)`.
pub fn fpow(mut a: u64, mut e: u128) -> u64 {
    let mut r = 1u64;
    while e > 0 {
        if e & 1 == 1 { r = fmul(r, a); }
        a = fmul(a, a);
        e >>= 1;
    }
    r
}

/// Field inverse via Fermat's little theorem (`a^(p-2)`). Undefined for `a == 0`.
pub fn finv(a: u64) -> u64 { fpow(a, P - 2) }

/// Greatest common divisor.
pub fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}

/// Signed interpretation of a field element: values above p/2 are shown as
/// negative, so `p - 1` prints as `-1`.
pub fn signed(v: u64) -> i128 {
    let vv = v as u128;
    if vv > P / 2 { vv as i128 - P as i128 } else { vv as i128 }
}

/// Column layout of a virtual-table `.fixed` file.
pub struct Layout {
    pub cols: usize,
    pub nuid: usize,
    pub ncol: usize,
}

impl Layout {
    /// Build a layout from the total column count and the number of UID columns.
    /// The payload columns are `cols - nuid - 1` (one trailing `__L1__`).
    pub fn new(cols: usize, nuid: usize) -> Layout {
        Layout { cols, nuid, ncol: cols.saturating_sub(nuid + 1) }
    }

    /// Human-readable name of column `j`.
    pub fn name(&self, j: usize) -> String {
        if j < self.nuid {
            format!("UID[{}]", j)
        } else if j < self.nuid + self.ncol {
            format!("column[{}]", j - self.nuid)
        } else {
            "__L1__".to_string()
        }
    }
}

/// Load a row-major `.fixed` file and transpose it into `cols` column vectors,
/// each of length `rows`.
pub fn load_columns(path: &str, cols: usize, rows: usize) -> std::io::Result<Vec<Vec<u64>>> {
    let f = File::open(path)?;
    let mut rdr = BufReader::with_capacity(1 << 24, f);
    let mut columns: Vec<Vec<u64>> = (0..cols).map(|_| Vec::with_capacity(rows)).collect();
    let mut buf = vec![0u8; cols * 8];
    for _ in 0..rows {
        rdr.read_exact(&mut buf)?;
        for j in 0..cols {
            columns[j].push(u64::from_le_bytes(buf[j * 8..j * 8 + 8].try_into().unwrap()));
        }
    }
    Ok(columns)
}

/// Run-length encode a column: returns `(values, lengths)`.
pub fn rle(c: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let mut vals = Vec::new();
    let mut lens = Vec::new();
    let mut cur = c[0];
    let mut len = 1u64;
    for &x in &c[1..] {
        if x == cur { len += 1; } else { vals.push(cur); lens.push(len); cur = x; len = 1; }
    }
    vals.push(cur);
    lens.push(len);
    (vals, lens)
}

/// Constant field delta between consecutive rows, if the column is arithmetic.
pub fn field_delta_const(v: &[u64]) -> Option<u64> {
    if v.len() < 2 { return Some(0); }
    let d = fsub(v[1], v[0]);
    for i in 1..v.len() - 1 {
        if fsub(v[i + 1], v[i]) != d { return None; }
    }
    Some(d)
}

/// Detect a modular counter `c[i] = (base + step*i) mod modulus` (a `byte_cycle`
/// / sawtooth), tolerant to a phase offset and a partial final cycle.
///
/// Returns `(base, step, modulus, period)` where `period` is the number of rows
/// after which the sequence repeats.
pub fn try_counter(c: &[u64]) -> Option<(u64, u64, u64, u64)> {
    let n = c.len();
    if n < 2 || c[1] == c[0] { return None; }
    let inc = c[1] > c[0];
    // first wrap-around
    let mut w = 0usize;
    for i in 1..n {
        let broke = if inc { c[i] < c[i - 1] } else { c[i] > c[i - 1] };
        if broke { w = i; break; }
    }
    if w == 0 { return None; } // monotonic -> not a cycle
    let s: i128 = c[1] as i128 - c[0] as i128;
    let m_i = if inc { c[w - 1] as i128 + s - c[w] as i128 } else { c[w] as i128 - c[w - 1] as i128 - s };
    if m_i <= 0 { return None; }
    let modulus = m_i as u128;
    let s_mod = (((s % modulus as i128) + modulus as i128) as u128) % modulus;
    if s_mod == 0 { return None; }
    let mut acc = (c[0] as u128) % modulus;
    for i in 0..n {
        if (c[i] as u128) != acc { return None; }
        acc = (acc + s_mod) % modulus;
    }
    let g = gcd(s_mod, modulus);
    Some((c[0], s_mod as u64, modulus as u64, (modulus / g) as u64))
}

/// Detect a repeated-value staircase cycle (`byte2_cycle`): each value held for
/// `k` rows, values cycling modulo `modulus`. Tolerant to a displaced start,
/// i.e. partial first and last runs.
///
/// Returns `(k, base, step, modulus, period, off)` where `base` is the starting
/// value, `off` is how many rows of the first block were already consumed
/// before row 0, and `period = k * (modulus / gcd(step, modulus))`.
pub fn try_reps(c: &[u64]) -> Option<(u64, u64, u64, u64, u64, u64)> {
    let (rv, rl) = rle(c);
    let r = rv.len();
    if r < 3 { return None; }
    let k = *rl.iter().max().unwrap();
    if k <= 1 { return None; }
    for i in 1..r - 1 {
        if rl[i] != k { return None; } // interior runs must be full
    }
    if rl[0] > k || rl[r - 1] > k { return None; }
    let (base, step, modulus, period_rv) = try_counter(&rv)?;
    let off = (k - rl[0]) % k;
    Some((k, base, step, modulus, period_rv * k, off))
}

/// Smallest power-of-two period `p` such that `c[i] == c[i + p]` for all `i`,
/// if the column is exactly periodic with such a period.
pub fn pow2_period(c: &[u64]) -> Option<usize> {
    let n = c.len();
    let mut p = 1usize;
    while p < n {
        p <<= 1;
        if p >= n { break; }
        let mut ok = true;
        for i in 0..n - p {
            if c[i] != c[i + p] { ok = false; break; }
        }
        if ok { return Some(p); }
    }
    None
}

/// Transition indices (rows where the value changes) and the distinct-value
/// count (capped at `cap` for efficiency).
pub fn transitions_and_distinct(c: &[u64], cap: usize) -> (Vec<usize>, u64) {
    use std::collections::HashSet;
    let mut t = Vec::new();
    let mut set = HashSet::new();
    set.insert(c[0]);
    for i in 1..c.len() {
        if c[i] != c[i - 1] { t.push(i); }
        if set.len() <= cap { set.insert(c[i]); }
    }
    (t, set.len() as u64)
}

/// Distinct-value count, capped at `cap` (returns `cap + 1` if exceeded).
pub fn distinct(c: &[u64], cap: usize) -> u64 {
    use std::collections::HashSet;
    let mut s = HashSet::new();
    for &x in c {
        s.insert(x);
        if s.len() > cap { return (cap as u64) + 1; }
    }
    s.len() as u64
}

/// Parse the four common CLI arguments: `<path> <cols> <rows> <nuid>`.
/// Prints usage and exits on error.
pub fn common_args(usage: &str) -> (String, usize, usize, usize) {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        eprintln!("usage: {} <file.fixed> <cols> <rows> <num_uid_cols>", a.first().map(|s| s.as_str()).unwrap_or("tool"));
        eprintln!("{}", usage);
        std::process::exit(2);
    }
    let path = a[1].clone();
    let cols = a[2].parse().expect("cols must be an integer");
    let rows = a[3].parse().expect("rows must be an integer");
    let nuid = a[4].parse().expect("num_uid_cols must be an integer");
    (path, cols, rows, nuid)
}
