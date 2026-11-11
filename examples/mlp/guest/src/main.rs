//! MLP guest program: runs one inference of an integer neural network inside
//! the zkVM, so the proof shows that this model, on this input, produced
//! these logits.
//!
//! The guest reads two raw slices from its standard input:
//! 1. the model blob (layout in [`mlp_common`]), used in place, with no copy;
//! 2. the input vector, one `i8` per byte.
//!
//! It commits `(class, logits)` as public values.

#![no_main]

ziskos::entrypoint!(main);

use mlp_common::{Model, argmax, infer};

fn main() {
    let blob = ziskos::io::read_slice();
    let input = ziskos::io::read_slice();

    let model = Model::parse(&blob);
    // SAFETY: i8 and u8 have the same size and alignment.
    let input: &[i8] = unsafe { core::slice::from_raw_parts(input.as_ptr().cast(), input.len()) };

    let logits = infer(&model, input);
    let class = argmax(&logits);

    ziskos::io::commit(&(class, logits));

    println!("class {class}");
}
