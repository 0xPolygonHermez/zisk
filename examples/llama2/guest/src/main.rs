//! llama2 guest: generates text with a TinyStories model inside the zkVM.
//!
//! Reads three raw slices from its standard input:
//! 1. the model blob (layout in `llama2_common::checkpoint`), used in place;
//! 2. the generation parameters (`llama2_common::params::GenParams`);
//! 3. guest options, one `u64` of flags: bit 0 ([`USE_DOT_F8`]) computes the
//!    dot products of the matrix multiplications with the experimental
//!    `dot_f8` precompile, which ZisK's emulator runs but cannot prove yet;
//!    bit 1 ([`FIXED_SCALES`]) applies the group scales in fixed point
//!    instead of `f32`.
//!
//! Prints the generated token ids on one line, `tokens: <id> <id> ...`, for
//! the host to decode, and commits `sha256(params || generated tokens)` as
//! the public output, so the printed tokens can be checked against it. The
//! public output is limited to 64 words, too small for the tokens themselves.

#![no_main]

ziskos::entrypoint!(main);

use core::fmt::Write;

use llama2_common::formats::Format;
use llama2_common::forward::{DotFn, MatmulMode, Scales, dot_soft};
use llama2_common::generate::generate_with;
use llama2_common::model::Model;
use llama2_common::params::GenParams;
use sha2::{Digest, Sha256};

use ziskos::syscalls::{SyscallDotF8Params, dot_f8_formats, syscall_dot_f8};

/// Guest option: use the `dot_f8` precompile.
const USE_DOT_F8: u64 = 1;

/// Guest option: fixed-point scales in the matrix multiplications.
const FIXED_SCALES: u64 = 2;

/// The dot products of one row with the `dot_f8` precompile: one call
/// computes the sums of all its groups and writes them to `out`. The
/// precompile needs 8-byte aligned vectors and groups of whole 64-bit words
/// (8 values, or 16 with E2M1); other rows (the groups of 4 of
/// `stories260K`'s `w2`) use the plain-Rust loop.
fn dot_precompile(
    w: &[u8],
    wf: Format,
    x: &[u8],
    xf: Format,
    n: usize,
    gs: usize,
    out: &mut [i64],
) {
    let per_word = 64 / wf.bits().min(xf.bits());
    if (w.as_ptr() as usize | x.as_ptr() as usize) % 8 != 0 || gs % per_word != 0 {
        return dot_soft(w, wf, x, xf, n, gs, out);
    }
    let params = SyscallDotF8Params {
        x: w.as_ptr(),
        y: x.as_ptr(),
        n: n as u64,
        formats: dot_f8_formats(wf.id(), xf.id()),
        block: gs as u64,
        out: out.as_mut_ptr(),
    };
    // SAFETY: both vectors hold the bytes of `n` values and `out` holds
    // `n / gs` sums; the emulator checks the alignment and the lengths.
    unsafe { syscall_dot_f8(&params) };
}

fn main() {
    let blob = ziskos::io::read_slice();
    let params_bytes = ziskos::io::read_slice();
    let options = ziskos::io::read_slice();
    let flags = u64::from_le_bytes(options[..8].try_into().expect("guest options: 8 bytes"));

    let model = Model::parse(&blob);
    let params = GenParams::from_bytes(&params_bytes);

    let scales = if flags & FIXED_SCALES != 0 { Scales::Fixed } else { Scales::Float };
    let mut mode = MatmulMode::soft(scales);
    if flags & USE_DOT_F8 != 0 {
        mode.dot = dot_precompile as DotFn;
    }
    let tokens = generate_with(&model, &params, mode, |_| {});

    let mut line = String::from("tokens:");
    for t in &tokens {
        write!(line, " {t}").unwrap();
    }
    println!("{line}");

    let mut hasher = Sha256::new();
    hasher.update(&*params_bytes);
    for t in &tokens {
        hasher.update(t.to_le_bytes());
    }
    ziskos::io::commit_slice(&hasher.finalize());
}
