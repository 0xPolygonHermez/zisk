//! Canonical production ZisK library manifest and assembly entry point.

use zisk_core::{ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR};

use crate::{assemble_library_sources, ZiskLibrary};

/// Ordered library sources used by production linking and proof-artifact generation.
pub const ZISK_LIBRARY: &[(&str, &str)] = &[
    ("mem", include_str!("../zisklib/mem.zisk")),
    ("fcall", include_str!("../zisklib/fcall.zisk")),
    ("add", include_str!("../zisklib/add.zisk")),
    ("keccak", include_str!("../zisklib/keccak.zisk")),
    ("sha256", include_str!("../zisklib/sha256.zisk")),
    ("blake2b", include_str!("../zisklib/blake2b.zisk")),
    ("zkvm_io", include_str!("../zisklib/zkvm_io.zisk")),
    // EF zkVM-accelerator C ABI (zkvm_accelerators.h): native `ziskasm_zkvm_*`
    // entrypoints, redirect targets of the standard `zkvm_*` symbols.
    ("zkvm/marshal", include_str!("../zisklib/zkvm/marshal.zisk")),
    ("zkvm/keccak", include_str!("../zisklib/zkvm/keccak.zisk")),
    ("zkvm/sha256", include_str!("../zisklib/zkvm/sha256.zisk")),
    ("zkvm/secp256k1", include_str!("../zisklib/zkvm/secp256k1.zisk")),
    ("zkvm/secp256r1", include_str!("../zisklib/zkvm/secp256r1.zisk")),
    ("zkvm/blake2f", include_str!("../zisklib/zkvm/blake2f.zisk")),
    ("zkvm/modexp", include_str!("../zisklib/zkvm/modexp.zisk")),
    ("zkvm/bn254", include_str!("../zisklib/zkvm/bn254.zisk")),
    ("zkvm/bls12_381", include_str!("../zisklib/zkvm/bls12_381.zisk")),
    ("zkvm/ripemd160", include_str!("../zisklib/zkvm/ripemd160.zisk")),
    ("uint256/common", include_str!("../zisklib/uint256/common.zisk")),
    ("uint256/add", include_str!("../zisklib/uint256/add.zisk")),
    ("uint256/mul", include_str!("../zisklib/uint256/mul.zisk")),
    ("uint256/div", include_str!("../zisklib/uint256/div.zisk")),
    ("uint256/modular", include_str!("../zisklib/uint256/modular.zisk")),
    ("uint256/pow", include_str!("../zisklib/uint256/pow.zisk")),
    ("secp256k1/constants", include_str!("../zisklib/secp256k1/constants.zisk")),
    ("secp256k1/field", include_str!("../zisklib/secp256k1/field.zisk")),
    ("secp256k1/scalar", include_str!("../zisklib/secp256k1/scalar.zisk")),
    ("secp256k1/curve", include_str!("../zisklib/secp256k1/curve.zisk")),
    ("secp256k1/ecdsa", include_str!("../zisklib/secp256k1/ecdsa.zisk")),
    ("secp256k1/glv", include_str!("../zisklib/secp256k1/glv.zisk")),
    ("secp256k1/schnorr", include_str!("../zisklib/secp256k1/schnorr.zisk")),
    ("secp256r1/constants", include_str!("../zisklib/secp256r1/constants.zisk")),
    ("secp256r1/field", include_str!("../zisklib/secp256r1/field.zisk")),
    ("secp256r1/scalar", include_str!("../zisklib/secp256r1/scalar.zisk")),
    ("secp256r1/curve", include_str!("../zisklib/secp256r1/curve.zisk")),
    ("secp256r1/ecdsa", include_str!("../zisklib/secp256r1/ecdsa.zisk")),
    ("bn254/constants", include_str!("../zisklib/bn254/constants.zisk")),
    ("bn254/util", include_str!("../zisklib/bn254/util.zisk")),
    ("bn254/fp", include_str!("../zisklib/bn254/fp.zisk")),
    ("bn254/fr", include_str!("../zisklib/bn254/fr.zisk")),
    ("bn254/fp2", include_str!("../zisklib/bn254/fp2.zisk")),
    ("bn254/curve", include_str!("../zisklib/bn254/curve.zisk")),
    ("bn254/fp6", include_str!("../zisklib/bn254/fp6.zisk")),
    ("bn254/fp12", include_str!("../zisklib/bn254/fp12.zisk")),
    ("bn254/twist", include_str!("../zisklib/bn254/twist.zisk")),
    ("bn254/cyclotomic", include_str!("../zisklib/bn254/cyclotomic.zisk")),
    ("bn254/miller_loop", include_str!("../zisklib/bn254/miller_loop.zisk")),
    ("bn254/final_exp", include_str!("../zisklib/bn254/final_exp.zisk")),
    ("bn254/pairing", include_str!("../zisklib/bn254/pairing.zisk")),
    ("bls12_381/constants", include_str!("../zisklib/bls12_381/constants.zisk")),
    ("bls12_381/fp", include_str!("../zisklib/bls12_381/fp.zisk")),
    ("bls12_381/fp2", include_str!("../zisklib/bls12_381/fp2.zisk")),
    ("bls12_381/curve", include_str!("../zisklib/bls12_381/curve.zisk")),
    ("bls12_381/fp6", include_str!("../zisklib/bls12_381/fp6.zisk")),
    ("bls12_381/fp12", include_str!("../zisklib/bls12_381/fp12.zisk")),
    ("bls12_381/twist", include_str!("../zisklib/bls12_381/twist.zisk")),
    ("bls12_381/cyclotomic", include_str!("../zisklib/bls12_381/cyclotomic.zisk")),
    ("bls12_381/miller_loop", include_str!("../zisklib/bls12_381/miller_loop.zisk")),
    ("bls12_381/final_exp", include_str!("../zisklib/bls12_381/final_exp.zisk")),
    ("bls12_381/subgroup", include_str!("../zisklib/bls12_381/subgroup.zisk")),
    ("bls12_381/pairing", include_str!("../zisklib/bls12_381/pairing.zisk")),
    ("bls12_381/map", include_str!("../zisklib/bls12_381/map.zisk")),
    ("bls12_381/map_g2", include_str!("../zisklib/bls12_381/map_g2.zisk")),
    ("bls12_381/hash", include_str!("../zisklib/bls12_381/hash.zisk")),
    ("bls12_381/verify", include_str!("../zisklib/bls12_381/verify.zisk")),
    ("bls12_381/kzg", include_str!("../zisklib/bls12_381/kzg.zisk")),
    ("bigint/common", include_str!("../zisklib/bigint/common.zisk")),
    ("bigint/add_short", include_str!("../zisklib/bigint/add_short.zisk")),
    ("bigint/add_agtb", include_str!("../zisklib/bigint/add_agtb.zisk")),
    ("bigint/mul_short", include_str!("../zisklib/bigint/mul_short.zisk")),
    ("bigint/mul_long", include_str!("../zisklib/bigint/mul_long.zisk")),
    ("bigint/rem_short", include_str!("../zisklib/bigint/rem_short.zisk")),
    ("bigint/rem_long", include_str!("../zisklib/bigint/rem_long.zisk")),
    ("bigint/modexp", include_str!("../zisklib/bigint/modexp.zisk")),
    ("bls12_381/fr", include_str!("../zisklib/bls12_381/fr.zisk")),
];

/// Assemble the exact library artifact merged by `elf2rom`.
pub fn assemble_zisk_library() -> Result<ZiskLibrary, String> {
    assemble_library_sources(ZISK_LIBRARY, ZISKLIB_ROM_ADDR, ZISKLIB_RAM_ADDR)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn production_manifest_is_unique_and_assembles_at_reserved_bases() {
        let mut names = HashSet::new();
        for (name, source) in ZISK_LIBRARY {
            assert!(names.insert(*name), "duplicate production source name `{name}`");
            assert!(!source.is_empty(), "empty production source `{name}`");
        }

        let library = assemble_zisk_library().expect("assemble production zisklib");
        assert_eq!(library.insts.first_key_value().map(|(addr, _)| *addr), Some(ZISKLIB_ROM_ADDR));
        assert_eq!(library.symbols.get("zisklib_add"), Some(&ZISKLIB_ROM_ADDR));
        assert!(library.rw_data.first().is_some_and(|section| section.addr == ZISKLIB_RAM_ADDR));
    }
}
