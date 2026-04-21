//! Modular exponentiation software fallback using aurora-engine-modexp (non-hints, non-zkVM builds only).
//! Returns exactly `mod_len` bytes per EIP-198 spec.
pub fn modexp(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
    aurora_engine_modexp::modexp(base, exp, modulus)
}
