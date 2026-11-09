#![no_main]
use ark_bn254::Fr as Ark;
use ark_ff::PrimeField;
use libfuzzer_sys::fuzz_target;
#[allow(dead_code)]
#[path = "../../verifier/src/scalar.rs"]
mod scalar;

fuzz_target!(|data: &[u8]| {
    if data.len() >= 64 {
        let a = Ark::from_be_bytes_mod_order(&data[..32]);
        let b = Ark::from_be_bytes_mod_order(&data[32..64]);
        let x = scalar::canonical(a.into_bigint()).unwrap();
        let y = scalar::canonical(b.into_bigint()).unwrap();
        assert_eq!(scalar::integer(scalar::mul(x, y)), (a * b).into_bigint());
        assert_eq!(scalar::integer(x + y), (a + b).into_bigint());
        assert_eq!(scalar::integer(scalar::sub(x, y)), (a - b).into_bigint());
        assert_eq!(scalar::integer(-x), (-a).into_bigint());
    }
    if data.len() >= 544 {
        let _ = zisk_solana_verifier::verify(
            data[..32].try_into().unwrap(),
            &data[32..544],
            &data[544..],
        );
    }
});
