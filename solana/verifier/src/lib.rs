#![no_std]
extern crate alloc;

use alloc::{vec, vec::Vec};
use ark_ff::BigInt;
use scalar::Fr;
mod curve;
mod scalar;
#[rustfmt::skip]
mod profile;

pub const PROOF_BYTES: usize = 6752;
pub const PUBLIC_BYTES: usize = 512;
const GOLDILOCKS: u64 = 0xffff_ffff_0000_0001;
const ROOT: [u64; 4] =
    [17146226293136250724, 16833344589786796689, 10321050178608571017, 2095991909911874349];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VerifyError {
    InvalidLength = 1,
    InvalidEncoding,
    InvalidProof,
    CurveOperation,
}

fn encode(value: Fr) -> [u8; 32] {
    let mut out = [0; 32];
    for (i, limb) in scalar::integer(value).0.iter().rev().enumerate() {
        out[i * 8..(i + 1) * 8].copy_from_slice(&limb.to_be_bytes());
    }
    out
}

fn scalar(bytes: &[u8]) -> Result<Fr, VerifyError> {
    let mut limbs = [0; 4];
    for (i, part) in bytes.chunks_exact(8).rev().enumerate() {
        limbs[i] = u64::from_be_bytes(part.try_into().unwrap());
    }
    scalar::canonical(BigInt(limbs)).ok_or(VerifyError::InvalidEncoding)
}

fn challenge(parts: &[&[u8]]) -> Fr {
    scalar::hash(solana_keccak_hasher::hashv(parts).to_bytes())
}

#[inline(never)]
fn prepare(guest: &[u8; 32], outputs: &[u8], proof: &[u8]) -> Result<(Vec<Fr>, Fr), VerifyError> {
    checkpoint("start");
    if proof.len() != PROOF_BYTES || outputs.len() != PUBLIC_BYTES {
        return Err(VerifyError::InvalidLength);
    }
    if guest.chunks_exact(8).any(|b| u64::from_be_bytes(b.try_into().unwrap()) >= GOLDILOCKS)
        || outputs.chunks_exact(8).any(|b| u64::from_le_bytes(b.try_into().unwrap()) >= GOLDILOCKS)
    {
        return Err(VerifyError::InvalidEncoding);
    }
    let evals: Vec<Fr> = proof[576..].chunks_exact(32).map(scalar).collect::<Result<_, _>>()?;
    for (i, point) in proof[..576].chunks_exact(64).enumerate() {
        curve::validate(point, i < 8)?;
    }
    checkpoint("decode");
    let mut root = [0; 32];
    for (i, limb) in ROOT.iter().enumerate() {
        root[i * 8..(i + 1) * 8].copy_from_slice(&limb.to_be_bytes());
    }
    let public = scalar::hash(solana_sha256_hasher::hashv(&[guest, outputs, &root]).to_bytes());
    let mut mem = vec![Fr::ZERO; 98];
    mem[4] =
        challenge(&[&encode(profile::DIGEST), &encode(Fr::ONE), &encode(public), &proof[..256]]);
    mem[5] = challenge(&[&encode(mem[4])]);
    mem[6] = challenge(&[&encode(mem[5]), &proof[256..384]]);
    mem[7] = challenge(&[&encode(mem[6]), &proof[384..448]]);
    mem[8] = challenge(&[&encode(mem[7]), &proof[576..6688]]);
    mem[9] = challenge(&[&encode(mem[8]), &proof[448..512]]);
    checkpoint("transcript");
    profile::compute_zh(&mut mem, &evals, public)?;
    checkpoint("zh");
    profile::compute_zi(&mut mem, &evals, public)?;
    checkpoint("zi");
    profile::compute_q(&mut mem, &evals, public)?;
    checkpoint("q");
    profile::compute_z(&mut mem, &evals, public)?;
    checkpoint("z");
    profile::compute_inversions(&mut mem, &evals, public)?;
    checkpoint("inversions");
    profile::compute_r(&mut mem, &evals, public)?;
    checkpoint("r");
    Ok((mem, public))
}

use scalar::mul;
#[inline(never)]
fn add(a: Fr, b: Fr) -> Fr {
    a + b
}

fn batch_inverse(values: &mut [Fr], mut inverse: Fr) -> Result<(), VerifyError> {
    let mut products = [Fr::ONE; 10];
    let mut product = Fr::ONE;
    for (i, value) in values.iter().enumerate() {
        product = mul(product, *value);
        products[i] = product;
    }
    if mul(product, inverse) != Fr::ONE {
        return Err(VerifyError::InvalidProof);
    }
    for i in (1..values.len()).rev() {
        let value = values[i];
        values[i] = mul(inverse, products[i - 1]);
        inverse = mul(inverse, value);
    }
    values[0] = inverse;
    Ok(())
}

struct Groups {
    f: [u8; 64],
    e: [u8; 64],
    j: [u8; 64],
    #[cfg(feature = "trace")]
    quotients: [Fr; 11],
}

#[inline(never)]
fn groups(mem: &[Fr], proof: &[u8]) -> Result<Groups, VerifyError> {
    let mut f = profile::FIXED[0];
    let mut e = mem[60];
    #[cfg(feature = "trace")]
    let mut quotients = [Fr::ZERO; 11];
    #[cfg(feature = "trace")]
    {
        quotients[0] = mem[23];
    }
    let mut power = Fr::ONE;
    for i in 1..11 {
        power = mul(power, mem[8]);
        let quotient = mul(mul(power, mem[23]), mem[49 + i]);
        #[cfg(feature = "trace")]
        {
            quotients[i] = quotient;
        }
        let point = if i < 4 { &profile::FIXED[i][..] } else { &proof[(i - 4) * 64..(i - 3) * 64] };
        f = curve::add(&f, &curve::mul(point, &encode(quotient))?)?;
        e += mul(quotient, mem[60 + i]);
    }
    let mut generator = [0; 64];
    generator[31] = 1;
    generator[63] = 2;
    Ok(Groups {
        f,
        e: curve::mul(&generator, &encode(e))?,
        j: curve::mul(&proof[448..512], &encode(mem[23]))?,
        #[cfg(feature = "trace")]
        quotients,
    })
}

/// Verify one pinned ZisK v1.4.0-alpha BLAKE3 leaf Pilfflonk proof.
/// The caller supplies its trusted guest key and the exact 512 output bytes it consumes.
#[inline(never)]
pub fn verify(
    expected_program_vk: &[u8; 32],
    public_values: &[u8],
    proof: &[u8],
) -> Result<(), VerifyError> {
    let (mem, _) = prepare(expected_program_vk, public_values, proof)?;
    let g = groups(&mem, proof)?;
    checkpoint("groups");
    let a = curve::add(&g.f, &curve::negate(g.e))?;
    let a = curve::add(&a, &curve::negate(g.j))?;
    let a = curve::add(&a, &curve::mul(&proof[512..576], &encode(mem[9]))?)?;
    checkpoint("before pairing");
    curve::pair(&a, &curve::negate(proof[512..576].try_into().unwrap()))
}

#[cfg(feature = "trace")]
pub struct Trace {
    pub digest: [u8; 32],
    pub challenges: [[u8; 32]; 6],
    pub xi: [u8; 32],
    pub q: [u8; 32],
    pub r: [[u8; 32]; 11],
    pub quotients: [[u8; 32]; 11],
    pub f: [u8; 64],
    pub e: [u8; 64],
    pub j: [u8; 64],
}

#[cfg(feature = "trace")]
pub fn trace(guest: &[u8; 32], outputs: &[u8], proof: &[u8]) -> Result<Trace, VerifyError> {
    let (mem, public) = prepare(guest, outputs, proof)?;
    let g = groups(&mem, proof)?;
    checkpoint("groups");
    Ok(Trace {
        digest: encode(public),
        challenges: core::array::from_fn(|i| encode(mem[4 + i])),
        xi: encode(mem[10]),
        q: encode(mem[14]),
        r: core::array::from_fn(|i| encode(mem[60 + i])),
        quotients: g.quotients.map(encode),
        f: g.f,
        e: g.e,
        j: g.j,
    })
}

#[inline(always)]
fn checkpoint(_stage: &str) {
    #[cfg(feature = "profile-cus")]
    {
        solana_program::msg!(_stage);
        solana_program::log::sol_log_compute_units();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exceptional_challenges_and_inverse_checks() {
        let mut mem = vec![Fr::ZERO; 98];
        let mut evals = vec![Fr::ZERO; 193];
        mem[7] = Fr::ONE;
        assert_eq!(profile::compute_zh(&mut mem, &evals, Fr::ZERO), Err(VerifyError::InvalidProof));
        evals[192] = Fr::ONE;
        assert_eq!(profile::compute_zh(&mut mem, &evals, Fr::ZERO), Err(VerifyError::InvalidProof));
        assert_eq!(
            profile::compute_inversions(&mut mem, &evals, Fr::ZERO),
            Err(VerifyError::InvalidProof)
        );
        let mut values = [Fr::ONE; 10];
        values[5] = Fr::ZERO;
        assert_eq!(batch_inverse(&mut values, Fr::ZERO), Err(VerifyError::InvalidProof));
        let mut values = [Fr::from(2u64); 10];
        assert_eq!(batch_inverse(&mut values, Fr::ONE), Err(VerifyError::InvalidProof));
        batch_inverse(
            &mut values,
            scalar::hash({
                use ark_ff::{BigInteger, Field, PrimeField};
                ark_bn254::Fr::from(1024u64)
                    .inverse()
                    .unwrap()
                    .into_bigint()
                    .to_bytes_be()
                    .try_into()
                    .unwrap()
            }),
        )
        .unwrap();
        assert!(values.iter().all(|v| *v * Fr::from(2u64) == Fr::ONE));
    }
}
