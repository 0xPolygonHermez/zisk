//! llama2 guest: generates text with a TinyStories model inside the zkVM.
//!
//! Reads two raw slices from its standard input:
//! 1. the model blob (layout in `llama2_common::checkpoint`), used in place;
//! 2. the generation parameters (`llama2_common::params::GenParams`).
//!
//! Prints the generated token ids on one line, `tokens: <id> <id> ...`, for
//! the host to decode, and commits `sha256(params || generated tokens)` as
//! the public output, so the printed tokens can be checked against it. The
//! public output is limited to 64 words, too small for the tokens themselves.

#![no_main]

ziskos::entrypoint!(main);

use core::fmt::Write;

use llama2_common::generate::generate;
use llama2_common::model::Model;
use llama2_common::params::GenParams;
use sha2::{Digest, Sha256};

fn main() {
    let blob = ziskos::io::read_slice();
    let params_bytes = ziskos::io::read_slice();

    let model = Model::parse(&blob);
    let params = GenParams::from_bytes(&params_bytes);

    let tokens = generate(&model, &params, |_| {});

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
