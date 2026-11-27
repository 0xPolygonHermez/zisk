//! A Llama-2 transformer with quantized weights, shared by the native
//! program, the ZisK guest and the ZisK host.
//!
//! This is a Rust port of the int8 path of Andrej Karpathy's llama2.c
//! (`runq.c`), for the TinyStories checkpoints:
//!
//! - every matrix multiplication uses quantized weights and activations, in
//!   groups with one scale each, and an exact integer dot product per group;
//!   int8 as llama2.c, or the FP8 and FP4 formats of [`formats`];
//! - the per-group scales use `f32`, as in llama2.c, or fixed point (see
//!   [`fixed`]), which is much cheaper on ZisK;
//! - RMSNorm, softmax, SiLU and RoPE use `f32`.
//!
//! All `f32` math goes through the `libm` crate and the basic IEEE operations,
//! which are correctly rounded both on x86-64 and in the software float
//! routines the ZisK target compiles to. The native run and the ZisK run
//! therefore produce the same tokens.
//!
//! Modules:
//! - [`checkpoint`]: reads a llama2.c `.bin` checkpoint and quantizes it into a
//!   model blob;
//! - [`model`]: parses the blob without copying the quantized weights;
//! - [`forward`]: the transformer forward pass;
//! - [`fixed`]: fixed-point scales for the matrix multiplications;
//! - [`formats`]: int8, FP8 and FP4 weights and activations;
//! - [`generate`]: the generation loop and the sampler;
//! - [`tokenizer`]: llama2.c's BPE tokenizer (host side only);
//! - [`params`]: the generation request sent to the guest.

#![no_std]

extern crate alloc;

pub mod checkpoint;
pub mod fixed;
pub mod formats;
pub mod forward;
pub mod generate;
pub mod model;
pub mod params;
pub mod tokenizer;

/// Token id of `<s>`, which starts every sequence.
pub const BOS: u32 = 1;

pub(crate) fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

pub(crate) fn read_f32(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(read_u32(bytes, at))
}
