use alloy_primitives::Address;
use anyhow::{anyhow, Result};
use core::convert::TryInto;
use k256::{
    ecdsa::{hazmat::bits2field, RecoveryId, Signature},
    elliptic_curve::{
        bigint::CheckedAdd,
        ff::PrimeFieldBits,
        ops::{Invert, Reduce},
        point::DecompressPoint,
        sec1::ToEncodedPoint,
        subtle::Choice,
        AffinePoint, Curve, FieldBytesEncoding, PrimeField, ProjectivePoint,
    },
    FieldElement, Scalar, Secp256k1,
};

// Define an alias for the integer type used in the Secp256k1 curve
type Secp256k1Uint = <Secp256k1 as Curve>::Uint;

pub fn ecrecover(sig: &[u8; 65], msg_hash: &[u8; 32]) -> Result<[u8; 64]> {
    // Extract the signature and recovery id
    let signature = Signature::from_slice(&sig[..64])?;
    let recid = sig[64];

    // Extract r and s values from the signature
    let (r, s) = signature.split_scalars();
    let recovery_id = RecoveryId::from_byte(recid).expect("invalid recovery id");

    // Attempt to restore r if it has been reduced
    let mut r_bytes = r.to_repr();
    if recovery_id.is_x_reduced() {
        match Option::<Secp256k1Uint>::from(
            Secp256k1Uint::decode_field_bytes(&r_bytes).checked_add(&Secp256k1::ORDER),
        ) {
            Some(restored) => r_bytes = restored.encode_field_bytes(),
            None => return Err(anyhow!("r was reduced but couldn't be restored")),
        };
    }

    // Recover p2 (r, y) from r and the parity bit of the recovery id
    let y_is_odd: Choice = u8::from(recovery_id.is_y_odd()).into();
    let p2 = AffinePoint::<Secp256k1>::decompress(&r_bytes, y_is_odd).unwrap();

    // Compute r inverse
    let r_inv: Scalar = *r.invert();

    // Compute hash as the reduction of msg_hash in the Secp256k1 curve
    let hash = <Scalar as Reduce<Secp256k1Uint>>::reduce_bytes(&bits2field::<Secp256k1>(msg_hash)?);

    // Compute k1 and k2 scalars
    let k1 = -(r_inv * hash);
    let k2 = r_inv * *s;

    // Set p1 point to compute the public key
    let p1_point = ProjectivePoint::<Secp256k1>::GENERATOR.to_encoded_point(false);
    let mut p1 = [0u8; 64];
    p1[..32].copy_from_slice(p1_point.x().unwrap().as_slice());
    p1[32..].copy_from_slice(p1_point.y().unwrap().as_slice());

    // Set p2 point to compute the public key
    let p2_point = ProjectivePoint::<Secp256k1>::from(p2).to_encoded_point(false);
    let mut p2 = [0u8; 64];
    p2[..32].copy_from_slice(p2_point.x().unwrap().as_slice());
    p2[32..].copy_from_slice(p2_point.y().unwrap().as_slice());

    // Compute the public key
    let pubkey = secp256k1_pubkey(&k1, &p1, &k2, &p2);

    Ok(pubkey)
}

fn syscall_secp256k1_double(p: &mut [u8; 64]) {
    // Extract x1 and y1 coordinates from p
    let x1 = FieldElement::from_bytes((&p[..32]).try_into().unwrap()).unwrap();
    let y1 = FieldElement::from_bytes((&p[32..]).try_into().unwrap()).unwrap();

    // Define constants for the formulas
    let three = FieldElement::from_u64(3);
    let two = FieldElement::from_u64(2);

    // Compute s = (3 * x1^2) / (2 * y1) mod p
    let s = three.mul(&x1.square()).mul(&two.mul(&y1).invert().unwrap());
    let s = s.normalize();

    // Compute x2 = s^2 - 2 * x1 mod p
    let x2 = s.square() - (&two.mul(&x1));
    let x2 = x2.normalize();

    // Compute y2 = s * (x1 - x2) - y1 mod p
    let y2 = s.mul(&(x1 - x2)) - y1;
    let y2 = y2.normalize();

    // Store the computed x2 and y2 back into p
    p[..32].copy_from_slice(&x2.to_bytes());
    p[32..].copy_from_slice(&y2.to_bytes());
}

// Compute the double of a point p in the Secp256k1 curve and store the result in p
fn secp256k1_double(p: &mut [u8; 64]) {
    // Extract x1 and y1 coordinates from p
    let y1 = FieldElement::from_bytes((&p[32..]).try_into().unwrap()).unwrap();

    // If y1 is zero, the result is the point at infinity (all zeros)
    if y1.is_zero().into() {
        *p = [0u8; 64];
        return;
    }

    syscall_secp256k1_double(p);
}

fn syscall_secp256k1_add(p1: &mut [u8; 64], p2: &[u8; 64]) {
    // Extract x and y coordinates from both points
    let x1 = FieldElement::from_bytes((&p1[..32]).try_into().unwrap()).unwrap();
    let y1 = FieldElement::from_bytes((&p1[32..]).try_into().unwrap()).unwrap();
    let x2 = FieldElement::from_bytes((&p2[..32]).try_into().unwrap()).unwrap();
    let y2 = FieldElement::from_bytes((&p2[32..]).try_into().unwrap()).unwrap();

    // Compute s = (y2 - y1) / (x2 - x1) mod p
    let s = (y2 - y1).mul(&(x2 - x1).invert().unwrap());
    let s = s.normalize();

    // Compute x3 = s^2 - x1 - x2 mod p
    let x3 = s.square() - (x1 + x2).normalize();
    let x3 = x3.normalize();

    // Compute y3 = s * (x1 - x3) - y1 mod p
    let y3 = s.mul(&(x1 - x3)) - y1;
    let y3 = y3.normalize();

    // Store the result in p1
    p1[..32].copy_from_slice(&x3.to_bytes());
    p1[32..].copy_from_slice(&y3.to_bytes());
}

// Compute the sum of two points p1 and p2 in the Secp256k1 curve and store the result in p1
fn secp256k1_add(p1: &mut [u8; 64], p2: &[u8; 64]) {
    // Extract x and y coordinates from both points
    let x1 = FieldElement::from_bytes((&p1[..32]).try_into().unwrap()).unwrap();
    let y1 = FieldElement::from_bytes((&p1[32..]).try_into().unwrap()).unwrap();
    let x2 = FieldElement::from_bytes((&p2[..32]).try_into().unwrap()).unwrap();
    let y2 = FieldElement::from_bytes((&p2[32..]).try_into().unwrap()).unwrap();

    if x1 == x2 {
        if y1 == y2 {
            // If the points are the same use secp256k1_double
            secp256k1_double(p1);
        } else {
            // If x1 == x2 but y1 != y2 result is the point at infinity (all zeros)
            *p1 = [0u8; 64];
        }
        return;
    }

    // If p1 is the point at infinity, replace it with p2
    if x1.is_zero().into() && y1.is_zero().into() {
        *p1 = *p2;
        return;
    }

    // If p2 is the point at infinity, keep p1 unchanged
    if x2.is_zero().into() && y2.is_zero().into() {
        return;
    }

    syscall_secp256k1_add(p1, p2);
}

fn secp256k1_pubkey(k1: &Scalar, p1: &[u8; 64], k2: &Scalar, p2: &[u8; 64]) -> [u8; 64] {
    let mut res: Option<[u8; 64]> = None;
    let mut temp_p1p2 = *p1;
    secp256k1_add(&mut temp_p1p2, p2);

    let k1_bits = k1.to_le_bits();
    let k2_bits = k2.to_le_bits();
    let total_bits = k1_bits.len();

    for i in (0..total_bits).rev() {
        let k1_bit = k1_bits[i];
        let k2_bit = k2_bits[i];

        if k1_bit && k2_bit {
            // Add temp_p1p2 to res
            match res.as_mut() {
                Some(res) => secp256k1_add(res, &temp_p1p2),
                None => res = Some(temp_p1p2),
            };
        } else if k1_bit {
            // Add p1 to res
            match res.as_mut() {
                Some(res) => secp256k1_add(res, p1),
                None => res = Some(*p1),
            };
        } else if k2_bit {
            // Add p2 to res
            match res.as_mut() {
                Some(res) => secp256k1_add(res, p2),
                None => res = Some(*p2),
            };
        }

        // Only perform secp256k1_double if NOT in the last bit and res has a value
        if i > 0 {
            if let Some(res) = res.as_mut() {
                secp256k1_double(res);
            }
        }
    }

    res.unwrap()
}

pub fn public_key_to_address(public_key: &[u8; 64]) -> Result<Address> {
    let hash: [u8; 32] = alloy_primitives::keccak256(&public_key).try_into()?;
    Ok(alloy_primitives::Address::from_slice(&hash[12..]))
}
