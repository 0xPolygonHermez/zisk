// Generator of random dot_bf16 cases for emulator-asm/tests/dot_bf16/run.sh.
// Writes random dot_bf16 cases with the reference results:
// per case: u64 flags, u64 n, u64 block, u64 xbytes, u64 sbytes, x bytes, scale bytes, n u16 y,
// u32 total, n/block u32 block results
use std::io::Write;
use zisk_definitions::{dot_bf16_fp4_block, dot_bf16_fp4_reference, dot_bf16_reference};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn value(r: &mut Rng, kind: u64) -> u16 {
    let s = ((r.next() & 1) as u16) << 15;
    let m = (r.next() & 0x7f) as u16;
    let e: u16 = match kind {
        0 => r.below(256) as u16,
        1 => 1 + r.below(254) as u16,
        2 => {
            if r.below(2) == 0 {
                0
            } else {
                1 + r.below(10) as u16
            }
        }
        3 => 120 + r.below(16) as u16,
        _ => 240 + r.below(15) as u16,
    };
    let e = if kind != 0 && e == 255 { 254 } else { e };
    s | e << 7 | m
}

fn main() {
    let cases: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut f =
        std::io::BufWriter::new(std::fs::File::create(std::env::args().nth(2).unwrap()).unwrap());
    let mut r = Rng(0x0123_4567_89ab_cdef);
    for c in 0..cases {
        let flags = (c % 3) as u64;
        let kind = ((c / 3) % 5) as u64;
        let unit = if flags == 0 { 4 } else { dot_bf16_fp4_block(flags) };
        let units = 1 + r.below(if c % 7 == 0 { 128 } else { 8 }) as usize;
        let n = unit * units;
        let divisors: Vec<usize> =
            (1..=units).filter(|d| units % d == 0).map(|d| unit * d).collect();
        let block = divisors[r.below(divisors.len() as u64) as usize];
        let y: Vec<u16> = (0..n).map(|_| value(&mut r, kind)).collect();
        let mut out = vec![0u32; n / block];
        let (x, scales, total);
        if flags == 0 {
            let xv: Vec<u16> = (0..n).map(|_| value(&mut r, kind)).collect();
            total = dot_bf16_reference(&xv, &y, n, block, Some(&mut out));
            x = xv.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
            scales = Vec::new();
        } else {
            x = (0..n / 2).map(|_| r.next() as u8).collect::<Vec<u8>>();
            let nb = n / unit;
            scales = (0..nb)
                .map(|_| match kind {
                    0 => r.next() as u8,            // NaN scales included
                    3 => (120 + r.below(16)) as u8, // narrow
                    _ => {
                        if flags == 1 {
                            r.below(255) as u8
                        } else {
                            (r.next() as u8) & 0xfe | 0
                        }
                    } // mostly finite
                })
                .collect::<Vec<u8>>();
            total = dot_bf16_fp4_reference(flags, &x, &scales, &y, n, block, Some(&mut out));
        }
        for v in [flags, n as u64, block as u64, x.len() as u64, scales.len() as u64] {
            f.write_all(&v.to_le_bytes()).unwrap();
        }
        f.write_all(&x).unwrap();
        f.write_all(&scales).unwrap();
        for v in &y {
            f.write_all(&v.to_le_bytes()).unwrap();
        }
        f.write_all(&total.to_le_bytes()).unwrap();
        for o in &out {
            f.write_all(&o.to_le_bytes()).unwrap();
        }
    }
}
