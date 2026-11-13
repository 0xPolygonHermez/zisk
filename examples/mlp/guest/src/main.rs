//! MLP guest program: runs one inference of an integer neural network inside
//! the zkVM, so the proof shows that this model, on this input, produced
//! these logits.
//!
//! The guest reads three raw slices from its standard input:
//! 1. the model blob (layout in [`mlp_common`]), used in place, with no copy;
//! 2. the input vector, one `i8` per byte;
//! 3. guest options, one `u64` of flags: bit 0 ([`USE_DOT_I8`]) computes the
//!    dot products with the experimental `dot_i8` precompile, which ZisK's
//!    emulator runs but cannot prove yet.
//!
//! It commits `(class, logits)` as public values.

#![no_main]

ziskos::entrypoint!(main);

use mlp_common::{DotFn, Model, argmax, dot_soft, infer_with};
use ziskos::syscalls::{SyscallDotI8Params, syscall_dot_i8};

/// Guest option: use the `dot_i8` precompile.
const USE_DOT_I8: u64 = 1;

/// The dot product with the `dot_i8` precompile, which needs 8-byte aligned
/// vectors and a length that is a multiple of 8. Other lengths use the
/// plain-Rust loop.
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
    let input = ziskos::io::read_slice();
    let options = ziskos::io::read_slice();
    let flags = u64::from_le_bytes(options[..8].try_into().expect("guest options: 8 bytes"));
    let dot: DotFn = if flags & USE_DOT_I8 != 0 { dot_precompile } else { dot_soft };

    let model = Model::parse(&blob);
    // SAFETY: i8 and u8 have the same size and alignment.
    let input: &[i8] = unsafe { core::slice::from_raw_parts(input.as_ptr().cast(), input.len()) };

    let logits = infer_with(&model, input, dot);
    let class = argmax(&logits);

    ziskos::io::commit(&(class, logits));

    println!("class {class}");
}
