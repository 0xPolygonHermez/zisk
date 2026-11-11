//! Integer-only multilayer perceptron shared by the guest and the host.
//!
//! The model is a stack of fully connected layers. Each layer has `i8`
//! weights and `i32` biases. Activations are `i8`. Hidden layers apply
//! `relu(acc >> shift)` saturated to `0..=127`. The last layer returns
//! its raw `i32` accumulators (the logits). Only integer operations are
//! used, so the guest and the host compute bit-identical results.
//!
//! The model travels as one flat little-endian byte blob, so the guest
//! can use the weights in place without deserializing them:
//!
//! ```text
//! u32 n_dims            number of layer widths (layers + 1)
//! u32 dims[n_dims]      widths: input, hidden..., output
//! per layer l (dims[l] -> dims[l+1]):
//!   u32 shift           right shift applied before ReLU (unused on the last layer)
//!   i32 bias[out]
//!   i8  weight[out][in] row-major: one row per output neuron
//! ```

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

/// Seed used by the examples so every run builds the same model and input.
pub const DEFAULT_SEED: u64 = 0x5A15_4D4C_5000_0001;

/// One fully connected layer, borrowed from the model blob.
pub struct Layer<'a> {
    pub inputs: usize,
    pub outputs: usize,
    pub shift: u32,
    bias: &'a [u8],
    weights: &'a [u8],
}

impl Layer<'_> {
    fn bias(&self, j: usize) -> i32 {
        let b = &self.bias[4 * j..4 * j + 4];
        i32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    /// Dot product of output neuron `j`'s weight row with `x`, plus its bias.
    #[inline(always)]
    fn neuron(&self, j: usize, x: &[i8]) -> i32 {
        let row = &self.weights[j * self.inputs..(j + 1) * self.inputs];
        let mut acc = self.bias(j);
        for (&w, &a) in row.iter().zip(x) {
            acc += (w as i8 as i32) * (a as i32);
        }
        acc
    }
}

/// A model parsed from its byte blob, without copying the weights.
pub struct Model<'a> {
    pub layers: Vec<Layer<'a>>,
}

fn take<'a>(blob: &mut &'a [u8], n: usize) -> &'a [u8] {
    let (head, tail) = blob.split_at(n);
    *blob = tail;
    head
}

fn take_u32(blob: &mut &[u8]) -> u32 {
    let b = take(blob, 4);
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

impl<'a> Model<'a> {
    /// Parses a model blob. Panics if the blob is malformed.
    pub fn parse(mut blob: &'a [u8]) -> Self {
        let n_dims = take_u32(&mut blob) as usize;
        assert!(n_dims >= 2, "a model needs at least an input and an output width");
        let dims: Vec<usize> = (0..n_dims).map(|_| take_u32(&mut blob) as usize).collect();
        let mut layers = Vec::with_capacity(n_dims - 1);
        for w in dims.windows(2) {
            let (inputs, outputs) = (w[0], w[1]);
            let shift = take_u32(&mut blob);
            let bias = take(&mut blob, 4 * outputs);
            let weights = take(&mut blob, inputs * outputs);
            layers.push(Layer { inputs, outputs, shift, bias, weights });
        }
        assert!(blob.is_empty(), "trailing bytes after the last layer");
        Model { layers }
    }

    /// Number of multiply-accumulate operations in one inference.
    pub fn macs(&self) -> u64 {
        self.layers.iter().map(|l| (l.inputs * l.outputs) as u64).sum()
    }

    /// Number of weights and biases.
    pub fn params(&self) -> u64 {
        self.layers.iter().map(|l| (l.inputs * l.outputs + l.outputs) as u64).sum()
    }
}

/// Runs the network on `input` and returns the logits of the last layer.
pub fn infer(model: &Model, input: &[i8]) -> Vec<i32> {
    let first = model.layers.first().expect("model has no layers");
    assert_eq!(input.len(), first.inputs, "input width does not match the model");

    let mut x: Vec<i8> = input.to_vec();
    let last = model.layers.len() - 1;
    for (l, layer) in model.layers.iter().enumerate() {
        if l == last {
            return (0..layer.outputs).map(|j| layer.neuron(j, &x)).collect();
        }
        x = (0..layer.outputs)
            .map(|j| (layer.neuron(j, &x) >> layer.shift).clamp(0, 127) as i8)
            .collect();
    }
    unreachable!()
}

/// Index of the largest logit (the first one on ties).
pub fn argmax(logits: &[i32]) -> u32 {
    let mut best = 0;
    for (i, &v) in logits.iter().enumerate() {
        if v > logits[best] {
            best = i;
        }
    }
    best as u32
}

/// Deterministic xorshift64* generator, so the model needs no data files.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_i8(&mut self) -> i8 {
        (self.next_u64() >> 56) as u8 as i8
    }
}

/// Right shift that keeps a hidden layer's activations in a useful range.
/// A random dot product over `inputs` terms grows like `sqrt(inputs)`.
fn shift_for(inputs: usize) -> u32 {
    let log2_ceil = usize::BITS - (inputs.max(1) - 1).leading_zeros();
    8 + log2_ceil.div_ceil(2)
}

/// Builds a model blob with pseudo-random weights for the given widths.
pub fn generate_model(dims: &[u32], seed: u64) -> Vec<u8> {
    assert!(dims.len() >= 2, "a model needs at least an input and an output width");
    let mut rng = Rng::new(seed);
    let mut blob = Vec::new();
    blob.extend_from_slice(&(dims.len() as u32).to_le_bytes());
    for d in dims {
        blob.extend_from_slice(&d.to_le_bytes());
    }
    for w in dims.windows(2) {
        let (inputs, outputs) = (w[0] as usize, w[1] as usize);
        blob.extend_from_slice(&shift_for(inputs).to_le_bytes());
        for _ in 0..outputs {
            let bias = (rng.next_u64() >> 52) as i32 - 2048;
            blob.extend_from_slice(&bias.to_le_bytes());
        }
        for _ in 0..inputs * outputs {
            blob.push(rng.next_i8() as u8);
        }
    }
    blob
}

/// Builds a pseudo-random input vector with values in `0..=127`,
/// like a normalized grayscale image.
pub fn generate_input(len: usize, seed: u64) -> Vec<i8> {
    let mut rng = Rng::new(seed ^ 0xA5A5_A5A5_A5A5_A5A5);
    (0..len).map(|_| (rng.next_u64() >> 57) as i8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_the_generated_shape() {
        let blob = generate_model(&[784, 128, 10], DEFAULT_SEED);
        let model = Model::parse(&blob);
        assert_eq!(model.layers.len(), 2);
        assert_eq!(model.macs(), 784 * 128 + 128 * 10);
        assert_eq!(model.params(), 784 * 128 + 128 + 128 * 10 + 10);
    }

    #[test]
    fn inference_is_deterministic_and_hidden_layer_is_alive() {
        let blob = generate_model(&[784, 128, 10], DEFAULT_SEED);
        let model = Model::parse(&blob);
        let input = generate_input(784, DEFAULT_SEED);
        let a = infer(&model, &input);
        let b = infer(&model, &input);
        assert_eq!(a, b);
        assert_eq!(a.len(), 10);
        // Distinct logits mean the hidden layer is neither all zero nor all saturated.
        let mut sorted = a.clone();
        sorted.sort();
        sorted.dedup();
        assert!(sorted.len() > 5, "logits collapsed: {a:?}");
    }
}
