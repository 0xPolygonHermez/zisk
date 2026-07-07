//! Shared layout + hashing for the `hash` example. Poseidon1 works over
//! Goldilocks field elements (u64); ZisK publics are u32 slots, so a 4-element
//! digest is committed as 8 u32 limbs and `NormalizePublics` reassembles it.

/// Poseidon1 (width-16) used as a sponge: 12-element rate + 4-element capacity,
/// digest = first 4 outputs.
pub const RATE: usize = 12;
pub const CAPACITY: usize = 4;
pub const WIDTH: usize = RATE + CAPACITY; // 16
pub const DIGEST: usize = 4;
pub const DIGEST_SLOTS: usize = DIGEST * 2; // 8

/// Reassemble a field element from its two little-endian u32 publics limbs.
pub fn field_from_limbs(low: u32, high: u32) -> u64 {
    (low as u64) | ((high as u64) << 32)
}

/// Poseidon1 of 12 inputs → 4-element digest. `fields::poseidon1_hash` uses the
/// precompile on the guest and native Rust on the host, so both agree.
pub fn hash12(input: &[u64; RATE]) -> [u64; DIGEST] {
    use fields::{poseidon1_hash, Goldilocks, Poseidon1_16, PrimeField64};
    let mut state = [Goldilocks::new(0); WIDTH];
    for i in 0..RATE {
        state[i] = Goldilocks::from_u64(input[i]);
    }
    let out = poseidon1_hash::<Goldilocks, Poseidon1_16, WIDTH>(&state);
    core::array::from_fn(|i| out[i].as_canonical_u64())
}

/// Element-wise sum in the Goldilocks field (mod p), matching the circom fold.
pub fn add_vecs(a: &[u64; RATE], b: &[u64; RATE]) -> [u64; RATE] {
    use fields::{Goldilocks, PrimeField64};
    core::array::from_fn(|i| {
        (Goldilocks::from_u64(a[i]) + Goldilocks::from_u64(b[i])).as_canonical_u64()
    })
}
