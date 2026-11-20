//! Coverage guest for wasm constraint verification: a plain `wasm32-wasip1` Rust program that
//! exercises the code shapes stock rustc output produces — i32/i64/i128 arithmetic, sub-word
//! loads and stores, heap growth (`memory.grow`), bulk memory, indirect calls through trait
//! objects, recursion, and the std WASI runtime (`random_get` via `HashMap`, `clock_time_get`,
//! args/environ, stderr).
//!
//! Input: an 8-byte little-endian `u64` `n` on stdin. Every printed value is deterministic, so the
//! output matches a native build of the same program.

use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::io::{Read, Write};

trait Shape {
    fn area(&self) -> u64;
}

struct Square(u64);
struct Rect(u64, u64);

impl Shape for Square {
    fn area(&self) -> u64 {
        self.0 * self.0
    }
}

impl Shape for Rect {
    fn area(&self) -> u64 {
        self.0 * self.1
    }
}

fn ackermann(m: u64, n: u64) -> u64 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        ackermann(m - 1, 1)
    } else {
        ackermann(m - 1, ackermann(m, n - 1))
    }
}

fn main() {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).expect("read stdin");
    let n = u64::from_le_bytes(input[..8].try_into().expect("input must hold an 8-byte u64"));

    // Integer operators, signed and unsigned, kept alive through black_box.
    let mut acc: u64 = 0;
    for i in 1..=n {
        let a = black_box(i as i32).wrapping_mul(-7);
        let b = black_box(i as i64) * 0x1234_5678_9abc;
        acc = acc.wrapping_add((a / 3) as u64 ^ (a % 5) as u64);
        acc = acc.wrapping_add((b as u64 / 11) ^ (b as u64 % 13));
        acc = acc.wrapping_add(((b / -9) ^ (b % -4)) as u64);
        acc = acc.rotate_left((i % 63) as u32)
            ^ (acc.leading_zeros() + acc.trailing_zeros() + acc.count_ones()) as u64;
        acc ^= (black_box(i as u32).rotate_right(5) as u64) << 1;
        acc = acc.wrapping_add(((i as u128 * 0xdead_beef_cafe_babe) >> 17) as u64);
        acc ^= (black_box(i as i8) as i64 + black_box(i as i16) as i64) as u64;
    }

    // Heap growth, memcpy/memset, sorting, and 8/16-bit loads and stores.
    let mut v: Vec<u32> = (0..n as u32 * 1000).map(|x| x.wrapping_mul(2654435761) >> 7).collect();
    v.sort_unstable();
    let mut bytes = vec![0u8; v.len() * 4];
    for (chunk, x) in bytes.chunks_mut(4).zip(&v) {
        chunk.copy_from_slice(&x.to_le_bytes());
    }
    let mut copy = bytes.clone();
    copy.rotate_left(3);
    copy.fill(0xa5);
    let halves: Vec<u16> = bytes.iter().map(|&b| b as u16 * 3).collect();
    let sum8 = bytes.iter().map(|&b| b as i8 as i64 as u64).fold(0, u64::wrapping_add);
    let sum16 = halves.iter().map(|&h| h as i16 as i64 as u64).fold(0, u64::wrapping_add);

    // Indirect calls through trait objects.
    let shapes: Vec<Box<dyn Shape>> = (0..n)
        .map(|i| {
            if i % 2 == 0 {
                Box::new(Square(i)) as Box<dyn Shape>
            } else {
                Box::new(Rect(i, i + 1))
            }
        })
        .collect();
    let areas: u64 = shapes.iter().map(|s| s.area()).sum();

    // HashMap (seeded through random_get), BTreeMap, and string formatting.
    let mut hashed = HashMap::new();
    let mut ordered = BTreeMap::new();
    for i in 0..n * 10 {
        *hashed.entry(i % 17).or_insert(0u64) += i;
        ordered.insert(format!("k{:05}", i * 7919 % 1000), i);
    }
    let hashed_sum: u64 = hashed.values().sum();
    let first_key = ordered.keys().next().cloned().unwrap_or_default();

    // Recursion, the clock, args and environment.
    let ack = ackermann(2, black_box(n % 20));
    let t0 = std::time::Instant::now();
    let _ = std::env::args().count() + std::env::vars().count();
    let clock_moved = t0.elapsed().as_nanos() > 0;

    eprintln!("stderr works");
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "acc={acc:x} v={} s8={sum8:x} s16={sum16:x} areas={areas} h={hashed_sum} {first_key} \
         ack={ack} clock={clock_moved}",
        v.len()
    )
    .expect("write stdout");
}
