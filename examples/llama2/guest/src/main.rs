//! llama2 guest: generates text with a TinyStories model inside the zkVM.
//!
//! Reads three raw slices from its standard input:
//! 1. the model blob (layout in `llama2_common::checkpoint`), used in place;
//! 2. the generation parameters (`llama2_common::params::GenParams`);
//! 3. guest options, one `u64` of flags: bit 0 ([`USE_DOT_I8`]) computes the
//!    dot products of the matrix multiplications with the experimental
//!    `dot_i8` precompile, which ZisK's emulator runs but cannot prove yet.
//!
//! Prints the generated token ids on one line, `tokens: <id> <id> ...`, for
//! the host to decode, and commits `sha256(params || generated tokens)` as
//! the public output, so the printed tokens can be checked against it. The
//! public output is limited to 64 words, too small for the tokens themselves.

#![no_main]

ziskos::entrypoint!(main);

use core::fmt::Write;

use llama2_common::forward::{DotFn, dot_soft};
use llama2_common::generate::generate_with;
use llama2_common::model::Model;
use llama2_common::params::GenParams;
use sha2::{Digest, Sha256};

use ziskos::syscalls::{SyscallDotI8Params, syscall_dot_i8};

/// Guest option: use the `dot_i8` precompile.
const USE_DOT_I8: u64 = 1;

/// The dot product with the `dot_i8` precompile, which needs 8-byte aligned
/// vectors and a length that is a multiple of 8. Groups of other sizes (4 in
/// `stories260K`'s `w2`) use the plain-Rust loop.
fn dot_precompile(x: &[i8], y: &[i8]) -> i32 {
    if x.len() % 8 != 0 {
        return dot_soft(x, y);
    }
    let params = SyscallDotI8Params { x: x.as_ptr(), y: y.as_ptr(), n: x.len() as u64 };
    // SAFETY: both slices hold `n` bytes; the emulator checks the alignment.
    unsafe { syscall_dot_i8(&params) }
}

fn main() {
    let blob = ziskos::io::read_slice();
    let params_bytes = ziskos::io::read_slice();
    let options = ziskos::io::read_slice();
    let flags = u64::from_le_bytes(options[..8].try_into().expect("guest options: 8 bytes"));

    let model = Model::parse(&blob);
    let params = GenParams::from_bytes(&params_bytes);

    let dot: DotFn = if flags & USE_DOT_I8 != 0 { dot_precompile } else { dot_soft };
    let tokens = generate_with(&model, &params, dot, |_| {});

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
