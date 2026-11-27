//! MLP guest program: runs one inference of an integer neural network inside
//! the zkVM, so the proof shows that this model, on this input, produced
//! these logits.
//!
//! The guest reads three raw slices from its standard input:
//! 1. the model blob (layout in [`mlp_common`]), used in place, with no copy;
//! 2. the input vector, one `i8` per byte;
//! 3. guest options, one `u64` of flags: bit 0 ([`USE_DOT_F8`]) computes the
//!    dot products with the experimental `dot_f8` precompile (int8 x int8),
//!    which ZisK's emulator runs but cannot prove yet.
//!
//! It commits `(class, logits)` as public values.

#![no_main]

ziskos::entrypoint!(main);

use mlp_common::{DotFn, Model, argmax, dot_soft, infer_with};
use ziskos::syscalls::{DOT_F8_I8, SyscallDotF8Params, dot_f8_formats, syscall_dot_f8};

/// Guest option: use the `dot_f8` precompile.
const USE_DOT_F8: u64 = 1;

/// The dot product with the `dot_f8` precompile, on two int8 vectors. It
/// needs 8-byte aligned vectors and a length that is a multiple of 8; other
/// lengths use the plain-Rust loop. The exact `i64` result saturates to `i32`
/// like [`dot_soft`].
fn dot_precompile(x: &[i8], y: &[i8]) -> i32 {
    if x.len() % 8 != 0 {
        return dot_soft(x, y);
    }
    let params = SyscallDotF8Params {
        x: x.as_ptr().cast(),
        y: y.as_ptr().cast(),
        n: x.len() as u64,
        formats: dot_f8_formats(DOT_F8_I8, DOT_F8_I8),
        block: 0,
        out: core::ptr::null_mut(),
    };
    // SAFETY: both slices hold `n` bytes; the emulator checks the alignment.
    let dot = unsafe { syscall_dot_f8(&params) };
    dot.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn main() {
    let blob = ziskos::io::read_slice();
    let input = ziskos::io::read_slice();
    let options = ziskos::io::read_slice();
    let flags = u64::from_le_bytes(options[..8].try_into().expect("guest options: 8 bytes"));
    let dot: DotFn = if flags & USE_DOT_F8 != 0 { dot_precompile } else { dot_soft };

    let model = Model::parse(&blob);
    // SAFETY: i8 and u8 have the same size and alignment.
    let input: &[i8] = unsafe { core::slice::from_raw_parts(input.as_ptr().cast(), input.len()) };

    let logits = infer_with(&model, input, dot);
    let class = argmax(&logits);

    ziskos::io::commit(&(class, logits));

    println!("class {class}");
}
