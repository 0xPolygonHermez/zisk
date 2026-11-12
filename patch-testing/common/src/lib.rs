//! Minimal check harness shared by the patch-testing guests.
//!
//! Every guest builds a [`Suite`], records checks against it and calls
//! [`Suite::finish`], which panics if any check failed. A panic makes `ziskemu`
//! exit non-zero, which is what `run.sh` keys on. The same guests also run
//! natively on the host, where the patched crates fall back to their upstream
//! code paths, so a host run validates the known-answer vectors themselves.

use std::fmt::Debug;

pub struct Suite {
    name: &'static str,
    passed: u32,
    failed: u32,
}

impl Suite {
    pub fn new(name: &'static str) -> Self {
        println!("[{name}] start");
        Self { name, passed: 0, failed: 0 }
    }

    /// Records a boolean check.
    pub fn check(&mut self, what: &str, ok: bool) {
        if ok {
            self.passed += 1;
        } else {
            self.failed += 1;
            println!("[{}] FAIL {what}", self.name);
        }
    }

    /// Records an equality check, printing both sides on mismatch.
    pub fn eq<T: PartialEq + Debug>(&mut self, what: &str, got: T, expected: T) {
        if got == expected {
            self.passed += 1;
        } else {
            self.failed += 1;
            println!("[{}] FAIL {what}\n  got:      {got:?}\n  expected: {expected:?}", self.name);
        }
    }

    /// Records an equality check of a byte string against a hex literal.
    pub fn eq_hex(&mut self, what: &str, got: &[u8], expected_hex: &str) {
        let expected = hex(expected_hex);
        if got == expected.as_slice() {
            self.passed += 1;
        } else {
            self.failed += 1;
            println!(
                "[{}] FAIL {what}\n  got:      {}\n  expected: {}",
                self.name,
                to_hex(got),
                expected_hex
            );
        }
    }

    /// Prints the summary and panics if any check failed.
    pub fn finish(self) {
        println!("[{}] {} passed, {} failed", self.name, self.passed, self.failed);
        assert!(self.failed == 0, "[{}] {} check(s) failed", self.name, self.failed);
        assert!(self.passed > 0, "[{}] no checks were run", self.name);
        println!("[{}] PASS", self.name);
    }
}

/// Decodes a hex string, ignoring an optional `0x` prefix and any whitespace.
pub fn hex(s: &str) -> Vec<u8> {
    let s: String = s.trim_start_matches("0x").chars().filter(|c| !c.is_whitespace()).collect();
    assert!(s.len() % 2 == 0, "odd-length hex literal");
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("bad hex")).collect()
}

/// Decodes a hex string into a fixed-size array.
pub fn hex_array<const N: usize>(s: &str) -> [u8; N] {
    hex(s).try_into().expect("hex literal has the wrong length")
}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Deterministic xorshift64* generator, so every run exercises the same inputs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}
