//! Shared layout + hashing for the `hash` example. The hash is BLAKE3 over
//! Goldilocks field elements (u64), the family of the default proving key; ZisK publics are u32 slots, so a 4-element
//! digest is committed as 8 u32 limbs and `NormalizePublics` reassembles it.

/// 12 secret elements in, a 4-element digest out.
pub const RATE: usize = 12;
pub const CAPACITY: usize = 4;
pub const WIDTH: usize = RATE + CAPACITY; // 16
pub const DIGEST: usize = 4;
pub const DIGEST_SLOTS: usize = DIGEST * 2; // 8

/// Reassemble a field element from its two little-endian u32 publics limbs.
pub fn field_from_limbs(low: u32, high: u32) -> u64 {
    (low as u64) | ((high as u64) << 32)
}

/// BLAKE3 of 12 inputs → 4-element digest: `Blake3_8::linear_hash`, the leaf hash the BLAKE3
/// STARK verifier uses, and `LinearHash(12, 2, 1)` in `aggregate.circom` computes the same.
pub fn hash12(input: &[u64; RATE]) -> [u64; DIGEST] {
    use proofman_fields::{Blake3_8, Goldilocks, Hash, PrimeField64};
    let elems: [Goldilocks; RATE] = core::array::from_fn(|i| Goldilocks::from_u64(input[i]));
    let out = <Blake3_8 as Hash<Goldilocks>>::linear_hash(&elems);
    core::array::from_fn(|i| out[i].as_canonical_u64())
}

/// Element-wise sum in the Goldilocks field (mod p), matching the circom fold.
pub fn add_vecs(a: &[u64; RATE], b: &[u64; RATE]) -> [u64; RATE] {
    use proofman_fields::{Goldilocks, PrimeField64};
    core::array::from_fn(|i| {
        (Goldilocks::from_u64(a[i]) + Goldilocks::from_u64(b[i])).as_canonical_u64()
    })
}

/// The three leaf secret vectors the example uses, so the host and the
/// `gen-inputs` binary pick identical values.
pub fn secret_vectors() -> [[u64; RATE]; 3] {
    [
        core::array::from_fn(|i| (i as u64) + 1),    // va: 1..12
        core::array::from_fn(|i| (i as u64) + 100),  // vb: 100..111
        core::array::from_fn(|i| (i as u64) + 1000), // vc: 1000..1011
    ]
}
