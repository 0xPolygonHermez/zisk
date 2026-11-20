//! The workloads shared by both guests. Each one does `n` units of work of one kind and returns a
//! checksum, so the two guests can be checked to compute the same thing.
//!
//! The precompile workload calls the Keccak-f permutation the guest passes in: the `ziskos`
//! syscall on RISC-V, the `env::keccak_f` import on wasm. Both lower to the same precompile.

use std::collections::BTreeMap;
use std::hint::black_box;

pub type KeccakF = fn(&mut [u64; 25]);

pub const NAMES: [&str; 10] =
    ["fib", "sort", "btree", "calls", "memcpy", "bigint", "keccak_sw", "keccak_pc", "float", "json"];

/// Runs workload `id` with size `n` and returns its checksum.
pub fn run(id: u64, n: u64, keccak_f: KeccakF) -> u64 {
    match id {
        0 => fib(n),
        1 => sort(n),
        2 => btree(n),
        3 => calls(n),
        4 => memcpy(n),
        5 => bigint(n),
        6 => keccak(n, keccak_f_soft),
        7 => keccak(n, keccak_f),
        8 => float(n),
        9 => json(n),
        _ => panic!("unknown workload {id}"),
    }
}

/// xorshift64: the pseudo-random source of every workload.
fn next(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

/// Integer arithmetic in registers: `n` Fibonacci steps modulo 2^64.
fn fib(n: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        (a, b) = (b, a.wrapping_add(b));
    }
    a
}

/// Memory-heavy: sort `n` pseudo-random `u32`s.
fn sort(n: u64) -> u64 {
    let mut seed = 0x9e37_79b9_7f4a_7c15;
    let mut v: Vec<u32> = (0..n).map(|_| next(&mut seed) as u32).collect();
    v.sort_unstable();
    v.iter().enumerate().fold(0u64, |acc, (i, &x)| acc.wrapping_mul(31).wrapping_add(x as u64 ^ i as u64))
}

/// Pointer-chasing and allocation: `n` inserts into a `BTreeMap`, then `n` lookups.
fn btree(n: u64) -> u64 {
    let mut seed = 0x2545_f491_4f6c_dd1d;
    let mut map = BTreeMap::new();
    for i in 0..n {
        map.insert(next(&mut seed) % (4 * n + 1), i);
    }
    let mut seed = 0x2545_f491_4f6c_dd1d;
    let mut acc = 0u64;
    for _ in 0..n {
        if let Some(v) = map.get(&(next(&mut seed) % (4 * n + 1))) {
            acc = acc.wrapping_add(*v);
        }
    }
    acc ^ map.len() as u64
}

#[inline(never)]
fn rec(depth: u32, x: u64) -> u64 {
    if depth == 0 {
        return x;
    }
    rec(depth - 1, x.wrapping_mul(3)) ^ rec(depth - 1, x.wrapping_add(depth as u64))
}

/// Calls: `n` times a recursion of 1,023 calls that are not inlined.
fn calls(n: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..n {
        acc = acc.wrapping_add(rec(black_box(9), i));
    }
    acc
}

/// Bulk copies: `n` copies of 4 KiB between two buffers.
fn memcpy(n: u64) -> u64 {
    let mut a = vec![0u8; 4096 + 64];
    let mut b = vec![0u8; 4096 + 64];
    for (i, x) in a.iter_mut().enumerate() {
        *x = i as u8;
    }
    for i in 0..n {
        let off = (i % 64) as usize;
        b[..4096].copy_from_slice(&a[off..off + 4096]);
        black_box(&mut b);
        std::mem::swap(&mut a, &mut b);
    }
    a.iter().fold(0u64, |acc, &x| acc.wrapping_mul(131).wrapping_add(x as u64))
}

/// 64 × 64 → 128-bit multiplications: `n` schoolbook products of 256-bit numbers.
fn bigint(n: u64) -> u64 {
    let mut seed = 0x1234_5678_9abc_def1;
    let mut x = [0u64; 4];
    let mut y = [0u64; 4];
    for i in 0..4 {
        x[i] = next(&mut seed);
        y[i] = next(&mut seed);
    }
    let mut acc = 0u64;
    for _ in 0..n {
        let mut r = [0u64; 8];
        for i in 0..4 {
            let mut carry = 0u128;
            for j in 0..4 {
                let t = x[i] as u128 * y[j] as u128 + r[i + j] as u128 + carry;
                r[i + j] = t as u64;
                carry = t >> 64;
            }
            r[i + 4] = carry as u64;
        }
        // Feed the low half back in so the products do not repeat.
        x.copy_from_slice(&r[..4]);
        x[0] |= 1;
        acc ^= r[7];
    }
    acc ^ x[0]
}

const RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];
const ROTC: [u32; 24] =
    [1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44];
const PILN: [usize; 24] =
    [10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1];

/// Keccak-f[1600] in plain Rust: 64-bit logic and rotations.
fn keccak_f_soft(a: &mut [u64; 25]) {
    for rc in RC {
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
        }
        for x in 0..5 {
            let d = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
            for y in 0..5 {
                a[x + 5 * y] ^= d;
            }
        }
        let mut t = a[1];
        for i in 0..24 {
            let j = PILN[i];
            let tmp = a[j];
            a[j] = t.rotate_left(ROTC[i]);
            t = tmp;
        }
        for y in 0..5 {
            let row = [a[5 * y], a[5 * y + 1], a[5 * y + 2], a[5 * y + 3], a[5 * y + 4]];
            for x in 0..5 {
                a[5 * y + x] = row[x] ^ (!row[(x + 1) % 5] & row[(x + 2) % 5]);
            }
        }
        a[0] ^= rc;
    }
}

/// `n` Keccak-f permutations of one state.
fn keccak(n: u64, f: KeccakF) -> u64 {
    let mut state = [0u64; 25];
    state[0] = n;
    for _ in 0..n {
        f(&mut state);
    }
    state.iter().fold(0, |acc, &x| acc ^ x)
}

/// Floating point: `n` steps of a damped oscillator in `f64`.
fn float(n: u64) -> u64 {
    let (mut x, mut v) = (1.0f64, 0.0f64);
    let dt = 0.001;
    for i in 0..n {
        let a = -x - 0.1 * v + 1.0 / (2.0 + (i % 7) as f64);
        v += a * dt;
        x += v * dt;
    }
    x.to_bits() ^ v.to_bits()
}

/// A realistic mix: build a JSON document of `n` records, serialize it, parse it back and
/// fold the values (allocation, formatting, parsing, hashing of strings into a map).
fn json(n: u64) -> u64 {
    let mut seed = 0x0bad_5eed_dead_beef;
    let records: Vec<serde_json::Value> = (0..n)
        .map(|i| {
            serde_json::json!({
                "id": i,
                "name": format!("item-{}", next(&mut seed) % 1000),
                "tags": [next(&mut seed) % 10, next(&mut seed) % 100],
                "ok": i % 3 == 0,
            })
        })
        .collect();
    let text = serde_json::to_string(&records).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let mut by_name: BTreeMap<String, u64> = BTreeMap::new();
    for r in &parsed {
        *by_name.entry(r["name"].as_str().unwrap().to_string()).or_default() +=
            r["tags"][1].as_u64().unwrap();
    }
    by_name.values().fold(text.len() as u64, |acc, &v| acc.wrapping_mul(31).wrapping_add(v))
}
