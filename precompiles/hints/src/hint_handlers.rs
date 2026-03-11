use anyhow::Result;
use std::collections::HashMap;
use zisk_common::{BuiltInHint, HintCode, PrecompileHint};
use ziskos_hints::handlers::blake2b::blake2b_compress_hint;
use ziskos_hints::handlers::bls381::{
    bls12_381_fp2_to_g2_hint, bls12_381_fp_to_g1_hint, bls12_381_g1_add_hint,
    bls12_381_g1_msm_hint, bls12_381_g2_add_hint, bls12_381_g2_msm_hint,
    bls12_381_pairing_check_hint,
};
use ziskos_hints::handlers::bn254::{
    bn254_g1_add_hint, bn254_g1_mul_hint, bn254_pairing_check_hint,
};
use ziskos_hints::handlers::keccak256::keccak256_hint;
use ziskos_hints::handlers::kzg::verify_kzg_proof_hint;
use ziskos_hints::handlers::modexp::modexp_hint;
use ziskos_hints::handlers::secp256k1::{
    secp256k1_ecdsa_address_recover, secp256k1_ecdsa_verify_address_recover,
};
use ziskos_hints::handlers::secp256r1::secp256r1_ecdsa_verify_hint;
use ziskos_hints::handlers::sha256::sha256_hint;

/// Type alias for custom hint handler functions.
pub type CustomHintHandler = Box<dyn Fn(&[u64]) -> Result<Vec<u64>> + Send + Sync>;

/// Bundles built-in and custom hint dispatch logic.
///
/// This is the single table that maps hint codes to compute functions.
/// Passed via `Arc` to each Rayon worker for parallel, allocation-free dispatch.
#[derive(Default)]
pub struct HintHandlers {
    custom: HashMap<u32, CustomHintHandler>,
}

impl HintHandlers {
    /// Register a custom hint handler for the given hint code.
    pub fn register<F>(mut self, hint_code: u32, handler: F) -> Self
    where
        F: Fn(&[u64]) -> Result<Vec<u64>> + Send + Sync + 'static,
    {
        self.custom.insert(hint_code, Box::new(handler));
        self
    }

    pub fn has_custom_hint_code(&self, code: u32) -> bool {
        self.custom.contains_key(&code)
    }

    /// Dispatch a hint to the appropriate handler.
    ///
    /// Control hints and Input hints must be handled before calling this.
    #[inline]
    pub fn dispatch(&self, hint: PrecompileHint) -> Result<Vec<u64>> {
        match hint.hint_code {
            HintCode::BuiltIn(builtin) => {
                Self::dispatch_builtin(builtin, hint.data, hint.data_len_bytes)
            }
            HintCode::Custom(code) => self
                .custom
                .get(&code)
                .map(|handler| handler(&hint.data))
                .unwrap_or_else(|| Err(anyhow::anyhow!("Unknown custom hint"))),
            _ => unreachable!("Control hints handled before dispatch"),
        }
    }

    /// Dispatches built-in hints to their corresponding handler functions.
    /// The `data_len_bytes` parameter is used for hints that operate on byte arrays (e.g., SHA256, Keccak256)
    /// to indicate the actual length of the data in bytes, since the `data` field is a `Vec<u64>` and may contain padding.
    /// The BuiltInHint::Input is intentionally not handled here, as input hints require special handling and should be processed separately before dispatching to workers.
    #[inline]
    fn dispatch_builtin(
        hint: BuiltInHint,
        data: Vec<u64>,
        data_len_bytes: usize,
    ) -> Result<Vec<u64>> {
        match hint {
            // SHA256 Hint Codes
            BuiltInHint::Sha256 => sha256_hint(&data, data_len_bytes),

            // BN254 Hint Codes
            BuiltInHint::Bn254G1Add => bn254_g1_add_hint(&data),
            BuiltInHint::Bn254G1Mul => bn254_g1_mul_hint(&data),
            BuiltInHint::Bn254PairingCheck => bn254_pairing_check_hint(&data),

            // Secp256k1 Hints
            BuiltInHint::Secp256k1EcdsaAddressRecover => secp256k1_ecdsa_address_recover(&data),
            BuiltInHint::Secp256k1EcdsaVerifyAddressRecover => {
                secp256k1_ecdsa_verify_address_recover(&data)
            }

            // Secp256r1 Hints
            BuiltInHint::Secp256r1EcdsaVerify => secp256r1_ecdsa_verify_hint(&data),

            // BLS12-381 Hint Codes
            BuiltInHint::Bls12_381G1Add => bls12_381_g1_add_hint(&data),
            BuiltInHint::Bls12_381G1Msm => bls12_381_g1_msm_hint(&data),
            BuiltInHint::Bls12_381G2Add => bls12_381_g2_add_hint(&data),
            BuiltInHint::Bls12_381G2Msm => bls12_381_g2_msm_hint(&data),
            BuiltInHint::Bls12_381PairingCheck => bls12_381_pairing_check_hint(&data),
            BuiltInHint::Bls12_381FpToG1 => bls12_381_fp_to_g1_hint(&data),
            BuiltInHint::Bls12_381Fp2ToG2 => bls12_381_fp2_to_g2_hint(&data),

            // Modular Exponentiation Hint Codes
            BuiltInHint::ModExp => modexp_hint(&data),

            // KZG Hint Codes
            BuiltInHint::VerifyKzgProof => verify_kzg_proof_hint(&data),

            // Keccak256 Hint Codes
            BuiltInHint::Keccak256 => keccak256_hint(&data, data_len_bytes),

            // Blake2b Hint Codes
            BuiltInHint::Blake2bCompress => blake2b_compress_hint(&data),

            // Input Hint Codes
            BuiltInHint::Input => unreachable!(
                "Input hints should be handled separately and not dispatched to workers"
            ),
        }
    }
}
