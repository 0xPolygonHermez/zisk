//! zisk-patch-elliptic-curves / k256: field, scalar and curve arithmetic plus
//! ECDSA verification routed to the secp256k1 zisklib functions.

#![no_main]
ziskos::entrypoint!(main);

use k256::{
    ecdsa::{
        signature::hazmat::{PrehashSigner, PrehashVerifier},
        RecoveryId, Signature, SigningKey, VerifyingKey,
    },
    elliptic_curve::{
        ops::{LinearCombination, Reduce},
        sec1::ToEncodedPoint,
        Field, PrimeField,
    },
    schnorr, ProjectivePoint, Scalar, U256,
};
use patch_test_common::{hex, hex_array, Suite};
use sha3::{Digest, Keccak256};

fn eth_address(vk: &VerifyingKey) -> Vec<u8> {
    let uncompressed = vk.to_encoded_point(false);
    Keccak256::digest(&uncompressed.as_bytes()[1..])[12..].to_vec()
}

fn main() {
    let mut t = Suite::new("k256");

    // --- Group law ---------------------------------------------------------
    let g = ProjectivePoint::GENERATOR;
    t.eq_hex(
        "G compressed",
        g.to_affine().to_encoded_point(true).as_bytes(),
        "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
    );
    let three_g = g * Scalar::from(3u64);
    t.eq_hex(
        "3G compressed",
        three_g.to_affine().to_encoded_point(true).as_bytes(),
        "02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9",
    );
    t.check("G + G + G == 3G", g + g + g == three_g);
    t.check("2G == G.double()", g * Scalar::from(2u64) == g.double());
    t.check("(n-1)G == -G", g * (-Scalar::ONE) == -g);
    t.check("(n-1)G + G == O", g * (-Scalar::ONE) + g == ProjectivePoint::IDENTITY);
    let a = Scalar::from_repr(hex_array::<32>("c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721").into()).unwrap();
    let b = Scalar::from(0xdead_beef_u64);
    t.check("a * a^-1 == 1", a * a.invert().unwrap() == Scalar::ONE);
    t.check("(a+b)G == aG + bG", g * (a + b) == g * a + g * b);
    t.check("lincomb(G,a,3G,b) == aG + b3G", ProjectivePoint::lincomb(&g, &a, &three_g, &b) == g * a + three_g * b);
    t.check("lincomb(3G,b,G,a) == b3G + aG", ProjectivePoint::lincomb(&three_g, &b, &g, &a) == three_g * b + g * a);

    // --- SEC1 decoding (decompression goes through the patched sqrt/lift_x) --
    let vk_g = VerifyingKey::from_sec1_bytes(&hex("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"));
    t.check("decompress G", vk_g.map(|vk| *vk.as_affine() == g.to_affine()).unwrap_or(false));
    // x = p is not a canonical field element.
    t.check(
        "reject x >= p",
        VerifyingKey::from_sec1_bytes(&hex("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f")).is_err(),
    );

    // --- ECDSA: Ethereum ecrecover vector (go-ethereum / EIP-196 test suite) --
    let prehash = hex("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let sig = Signature::from_slice(&hex(
        "9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608\
         4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada",
    ))
    .unwrap();
    let recovered = VerifyingKey::recover_from_prehash(&prehash, &sig, RecoveryId::from_byte(1).unwrap());
    t.check("ecrecover succeeds", recovered.is_ok());
    if let Ok(vk) = recovered {
        t.eq_hex("ecrecover address", &eth_address(&vk), "7156526fbd7a3c72969b54f64e42c10fbb768c8a");
        t.check("verify valid signature", vk.verify_prehash(&prehash, &sig).is_ok());
        let mut bad = prehash.clone();
        bad[31] ^= 1;
        t.check("reject tampered message", vk.verify_prehash(&bad, &sig).is_err());
        t.check("reject signature under other key", VerifyingKey::from(&SigningKey::from_bytes(&a.to_repr()).unwrap()).verify_prehash(&prehash, &sig).is_err());
    }

    // --- ECDSA sign (RFC 6979, deterministic) + verify round trip ------------
    let sk = SigningKey::from_bytes(&a.to_repr()).unwrap();
    let digest = Keccak256::digest(b"zisk patch testing");
    let (sig2, recid): (Signature, RecoveryId) = sk.sign_prehash_recoverable(&digest).unwrap();
    let sig3: Signature = sk.sign_prehash(&digest).unwrap();
    t.eq("sign is deterministic", sig2, sig3);
    t.eq_hex(
        "RFC 6979 signature",
        &sig2.to_bytes(),
        "8e5ca177f812905afb50940b98dbd5e1967434b52c5d95ab52117b20198e1fb9\
         38d2405f0fb27b1f5873828ab3d868d0f7dbb062156810a666c3bad65ab8265f",
    );
    t.check("verify own signature", sk.verifying_key().verify_prehash(&digest, &sig2).is_ok());
    t.check(
        "recover own key",
        VerifyingKey::recover_from_prehash(&digest, &sig2, recid).map(|vk| vk == *sk.verifying_key()).unwrap_or(false),
    );
    // A message hash above the group order n must be reduced, not rejected.
    let high = [0xffu8; 32];
    let sig_high: Signature = sk.sign_prehash(&high).unwrap();
    t.eq_hex(
        "RFC 6979 signature (hash > n)",
        &sig_high.to_bytes(),
        "21972c07b7108adf973fdc16df6e995ee611c2e5d9b7dcef724be5ae7b3f58c2\
         668a276e1b584077277904956afbb0a2f615e11c6fd76ecb58bba40d530181f6",
    );
    t.check("verify hash > n", sk.verifying_key().verify_prehash(&high, &sig_high).is_ok());
    let mut high_bad = high;
    high_bad[0] = 0xfe;
    t.check("reject tampered hash > n", sk.verifying_key().verify_prehash(&high_bad, &sig_high).is_err());
    // Scalar reduction of a full 256-bit digest.
    let z = <Scalar as Reduce<U256>>::reduce_bytes(&digest);
    t.check("reduce(digest) != 0", z != Scalar::ZERO);

    // --- BIP-340 Schnorr, test vector 0 ------------------------------------
    let schnorr_vk = schnorr::VerifyingKey::from_bytes(&hex("f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9")).unwrap();
    let schnorr_sig = schnorr::Signature::try_from(
        hex(
            "e907831f80848d1069a5371b402410364bdf1c5f8307b0084c55f1ce2dca8215\
             25f66a4a85ea8b71e482a74f382d2ce5ebeee8fdb2172f477df4900d310536c0",
        )
        .as_slice(),
    )
    .unwrap();
    t.check("BIP-340 vector 0 verifies", schnorr_vk.verify_prehash(&[0u8; 32], &schnorr_sig).is_ok());
    t.check("BIP-340 vector 0 tampered fails", schnorr_vk.verify_prehash(&[1u8; 32], &schnorr_sig).is_err());

    t.finish();
}
