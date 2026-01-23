//! Hints for ZisK Precompiles stream processing
//!
//! This module provides functionality for parsing precompile hints
//! that are received as a stream of `u64` values. Hints are used to provide preprocessed
//! data to precompile operations in the ZisK zkVM.
//!
//! # Hint Format
//!
//! Each hint consists of:
//! - A **header** (`u64`): Contains the hint type (upper 32 bits) and data length (lower 32 bits)
//! - **Data** (`[u64; length]`): The hint payload, where `length` is specified in the header
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         Header (u64)                        │
//! ├·····························································┤
//! │      Hint Code (32 bits)           Length (32 bits).        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                        Data[0] (u64)                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                        Data[1] (u64)                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                             ...                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │                     Data[length-1] (u64)                    │
//! └─────────────────────────────────────────────────────────────┘
//!
//! - Hint Code — Control code or Data Hint Type
//! - Length — Number of following u64 data words
//!
//! ## Hint Type Layout
//!
//! ### Control codes
//!
//! The following control codes are defined:
//! - `0x00` (START): Reset processor state and global sequence.
//! - `0x01` (END): Wait until completion of all pending hints.
//! - `0x02` (CANCEL): Cancel current stream and stop processing further hints.
//! - `0x03` (ERROR): Indicate an error has occurred; stop processing further hints.
//!
//! Control codes are for control only and do not have any associated data (Length should be zero).
//!
//! ### Data Hint Types:
//!
//! For data hints, the hint code (32 bits) is structured as follows:
//! - **Bit 31 (MSB)**: Indicates if the data is pass-through (1) or requires computation (0)
//! - **Bits 0-30**: Encode the built-in hint code as defined in the constants
//!   (e.g., `HINT_SHA256`, `HINT_BN254_G1_ADD`, `HINT_SECP256K1_ECRECOVER`, etc.)
//! ```

use std::fmt::Display;

use anyhow::Result;

// === CONTROL CODES ===
const CTRL_START: u32 = 0x0000;
const CTRL_END: u32 = 0x0001;
const CTRL_CANCEL: u32 = 0x0002;
const CTRL_ERROR: u32 = 0x0003;

// === BUILT-IN HINT CODES ===
// SHA256 hint codes
const HINT_SHA256: u32 = 0x0100;

// BN254 hint codes
const HINT_BN254_G1_ADD: u32 = 0x0200;
const HINT_BN254_G1_MUL: u32 = 0x0201;
const HINT_BN254_PAIRING_CHECK: u32 = 0x0205;

// Secp256k1 hint codes
const HINT_SECP256K1_ECRECOVER: u32 = 0x0300;
const HINT_SECP256R1_VERIFY_SIGNATURE: u32 = 0x0301;

// BLS12-381 hint codes
const HINT_BLS12_381_G1_ADD: u32 = 0x0400;
const HINT_BLS12_381_G1_MSM: u32 = 0x0401;
const HINT_BLS12_381_G2_ADD: u32 = 0x0405;
const HINT_BLS12_381_G2_MSM: u32 = 0x0406;
const HINT_BLS12_381_PAIRING_CHECK: u32 = 0x040A;
const HINT_BLS12_381_FP_TO_G1: u32 = 0x0410;
const HINT_BLS12_381_FP2_TO_G2: u32 = 0x0411;

// Modular exponentiation hint codes
const HINT_MODEXP: u32 = 0x0500;

// KZG hint codes
const HINT_VERIFY_KZG_PROOF: u32 = 0x0600;

// Keccak256 hint codes
const HINT_KECCAK256: u32 = 0x0700;

/// Control code variants for stream control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CtrlHint {
    /// Reset processor state and global sequence.
    Start = CTRL_START,
    /// Wait until completion of all pending hints.
    End = CTRL_END,
    /// Cancel current stream and stop processing.
    Cancel = CTRL_CANCEL,
    /// Signal error and stop processing.
    Error = CTRL_ERROR,
}

impl Display for CtrlHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            CtrlHint::Start => "CTRL_START",
            CtrlHint::End => "CTRL_END",
            CtrlHint::Cancel => "CTRL_CANCEL",
            CtrlHint::Error => "CTRL_ERROR",
        };
        write!(f, "{} ({:#x})", name, *self as u32)
    }
}

impl TryFrom<u32> for CtrlHint {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        match value {
            CTRL_START => Ok(Self::Start),
            CTRL_END => Ok(Self::End),
            CTRL_CANCEL => Ok(Self::Cancel),
            CTRL_ERROR => Ok(Self::Error),
            _ => Err(anyhow::anyhow!("Invalid control code: {:#x}", value)),
        }
    }
}

/// Built-in hint type variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BuiltInHint {
    // SHA256 hint types.
    /// Compute SHA-256 hash
    Sha256 = HINT_SHA256,

    // BN254 hint types
    /// BN254 elliptic curve addition.
    Bn254G1Add = HINT_BN254_G1_ADD,
    /// BN254 elliptic curve scalar multiplication.
    Bn254G1Mul = HINT_BN254_G1_MUL,
    /// BN254 pairing check.
    Bn254PairingCheck = HINT_BN254_PAIRING_CHECK,

    // Secp256k1 hint types.
    /// secp256k1 ECDSA signature recovery.
    Secp256k1EcRecover = HINT_SECP256K1_ECRECOVER,
    /// secp256r1 (P-256) signature verification.
    Secp256r1VerifySignature = HINT_SECP256R1_VERIFY_SIGNATURE,

    // BLS12-381 hint types.
    /// BLS12-381 G1 addition (returns 96-byte unpadded G1 point)
    Bls12_381G1Add = HINT_BLS12_381_G1_ADD,
    /// BLS12-381 G1 multi-scalar multiplication (returns 96-byte unpadded G1 point)
    Bls12_381G1Msm = HINT_BLS12_381_G1_MSM,
    /// BLS12-381 G2 addition (returns 192-byte unpadded G2 point)
    Bls12_381G2Add = HINT_BLS12_381_G2_ADD,
    /// BLS12-381 G2 multi-scalar multiplication (returns 192-byte unpadded G2 point)
    Bls12_381G2Msm = HINT_BLS12_381_G2_MSM,
    /// BLS12-381 pairing check.
    Bls12_381PairingCheck = HINT_BLS12_381_PAIRING_CHECK,
    /// BLS12-381 map field element to G1.
    Bls12_381FpToG1 = HINT_BLS12_381_FP_TO_G1,
    /// BLS12-381 map field element to G2.
    Bls12_381Fp2ToG2 = HINT_BLS12_381_FP2_TO_G2,

    // Modular exponentiation hint types.
    /// Modular exponentiation.
    ModExp = HINT_MODEXP,

    // KZG hint types.
    /// Verify KZG proof.
    VerifyKzgProof = HINT_VERIFY_KZG_PROOF,

    // Keccak256 hint types.
    /// Compute Keccak-256 hash.
    Keccak256 = HINT_KECCAK256,
}

impl Display for BuiltInHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            // SHA256 hint types
            BuiltInHint::Sha256 => "SHA256",
            // BN254 Hints
            BuiltInHint::Bn254G1Add => "BN254_G1_ADD",
            BuiltInHint::Bn254G1Mul => "BN254_G1_MUL",
            BuiltInHint::Bn254PairingCheck => "BN254_PAIRING_CHECK",
            // Secp256k1 Hints
            BuiltInHint::Secp256k1EcRecover => "SECP256K1_ECRECOVER",
            BuiltInHint::Secp256r1VerifySignature => "SECP256R1_VERIFY_SIGNATURE",
            // BLS12-381 Hints
            BuiltInHint::Bls12_381G1Add => "BLS12_381_G1_ADD",
            BuiltInHint::Bls12_381G1Msm => "BLS12_381_G1_MSM",
            BuiltInHint::Bls12_381G2Add => "BLS12_381_G2_ADD",
            BuiltInHint::Bls12_381G2Msm => "BLS12_381_G2_MSM",
            BuiltInHint::Bls12_381PairingCheck => "BLS12_381_PAIRING_CHECK",
            BuiltInHint::Bls12_381FpToG1 => "BLS12_381_FP_TO_G1",
            BuiltInHint::Bls12_381Fp2ToG2 => "BLS12_381_FP2_TO_G2",
            // Modular Exponentiation Hint
            BuiltInHint::ModExp => "MODEXP",
            // KZG Hint
            BuiltInHint::VerifyKzgProof => "VERIFY_KZG_PROOF",
            // Keccak256 Hint
            BuiltInHint::Keccak256 => "KECCAK256",
        };

        write!(f, "{} ({:#x})", name, *self as u32)
    }
}

impl TryFrom<u32> for BuiltInHint {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        match value {
            // SHA256 hint types
            HINT_SHA256 => Ok(Self::Sha256),
            // BN254 Hints
            HINT_BN254_G1_ADD => Ok(Self::Bn254G1Add),
            HINT_BN254_G1_MUL => Ok(Self::Bn254G1Mul),
            HINT_BN254_PAIRING_CHECK => Ok(Self::Bn254PairingCheck),
            // Secp256k1 Hints
            HINT_SECP256K1_ECRECOVER => Ok(Self::Secp256k1EcRecover),
            HINT_SECP256R1_VERIFY_SIGNATURE => Ok(Self::Secp256r1VerifySignature),
            // BLS12-381 Hints
            HINT_BLS12_381_G1_ADD => Ok(Self::Bls12_381G1Add),
            HINT_BLS12_381_G1_MSM => Ok(Self::Bls12_381G1Msm),
            HINT_BLS12_381_G2_ADD => Ok(Self::Bls12_381G2Add),
            HINT_BLS12_381_G2_MSM => Ok(Self::Bls12_381G2Msm),
            HINT_BLS12_381_PAIRING_CHECK => Ok(Self::Bls12_381PairingCheck),
            HINT_BLS12_381_FP_TO_G1 => Ok(Self::Bls12_381FpToG1),
            HINT_BLS12_381_FP2_TO_G2 => Ok(Self::Bls12_381Fp2ToG2),
            // Modular Exponentiation Hint
            HINT_MODEXP => Ok(Self::ModExp),
            // KZG Hint
            HINT_VERIFY_KZG_PROOF => Ok(Self::VerifyKzgProof),
            // Keccak256 Hint
            HINT_KECCAK256 => Ok(Self::Keccak256),
            _ => Err(anyhow::anyhow!("Invalid built-in hint code: {:#x}", value)),
        }
    }
}

/// Hint code representing either a control code or built-in hint type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum HintCode {
    /// Control code for stream management.
    Ctrl(CtrlHint),
    /// Built-in hint type.
    BuiltIn(BuiltInHint),
    /// Custom hint type
    Custom(u32),
}

impl Display for HintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HintCode::Ctrl(ctrl) => write!(f, "{}", ctrl),
            HintCode::BuiltIn(builtin) => write!(f, "{}", builtin),
            HintCode::Custom(code) => write!(f, "CUSTOM_HINT_{:#x}", code),
        }
    }
}

impl TryFrom<u32> for HintCode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        // Try CtrlCode first
        if let Ok(ctrl) = CtrlHint::try_from(value) {
            return Ok(HintCode::Ctrl(ctrl));
        }
        // Try BuiltInHint next
        if let Ok(builtin) = BuiltInHint::try_from(value) {
            return Ok(HintCode::BuiltIn(builtin));
        }
        // Unknown codes return error - custom codes handled separately
        Err(anyhow::anyhow!("Unknown hint code: {:#x}", value))
    }
}

impl HintCode {
    /// Convert HintCode to its u32 discriminant value.
    #[inline]
    pub const fn to_u32(self) -> u32 {
        match self {
            // Control Codes
            HintCode::Ctrl(CtrlHint::Start) => CTRL_START,
            HintCode::Ctrl(CtrlHint::End) => CTRL_END,
            HintCode::Ctrl(CtrlHint::Cancel) => CTRL_CANCEL,
            HintCode::Ctrl(CtrlHint::Error) => CTRL_ERROR,

            // Built-In Hint Codes
            // SHA256 Hints
            HintCode::BuiltIn(BuiltInHint::Sha256) => HINT_SHA256,
            // BN254 Hints
            HintCode::BuiltIn(BuiltInHint::Bn254G1Add) => HINT_BN254_G1_ADD,
            HintCode::BuiltIn(BuiltInHint::Bn254G1Mul) => HINT_BN254_G1_MUL,
            HintCode::BuiltIn(BuiltInHint::Bn254PairingCheck) => HINT_BN254_PAIRING_CHECK,
            // Secp256k1 Hints
            HintCode::BuiltIn(BuiltInHint::Secp256k1EcRecover) => HINT_SECP256K1_ECRECOVER,
            HintCode::BuiltIn(BuiltInHint::Secp256r1VerifySignature) => {
                HINT_SECP256R1_VERIFY_SIGNATURE
            }
            // BLS12-381 Hints
            HintCode::BuiltIn(BuiltInHint::Bls12_381G1Add) => HINT_BLS12_381_G1_ADD,
            HintCode::BuiltIn(BuiltInHint::Bls12_381G1Msm) => HINT_BLS12_381_G1_MSM,
            HintCode::BuiltIn(BuiltInHint::Bls12_381G2Add) => HINT_BLS12_381_G2_ADD,
            HintCode::BuiltIn(BuiltInHint::Bls12_381G2Msm) => HINT_BLS12_381_G2_MSM,
            HintCode::BuiltIn(BuiltInHint::Bls12_381PairingCheck) => HINT_BLS12_381_PAIRING_CHECK,
            HintCode::BuiltIn(BuiltInHint::Bls12_381FpToG1) => HINT_BLS12_381_FP_TO_G1,
            HintCode::BuiltIn(BuiltInHint::Bls12_381Fp2ToG2) => HINT_BLS12_381_FP2_TO_G2,
            // Modular Exponentiation Hint
            HintCode::BuiltIn(BuiltInHint::ModExp) => HINT_MODEXP,
            // KZG Hint
            HintCode::BuiltIn(BuiltInHint::VerifyKzgProof) => HINT_VERIFY_KZG_PROOF,
            // Keccak256 Hint
            HintCode::BuiltIn(BuiltInHint::Keccak256) => HINT_KECCAK256,

            // Custom Hints
            HintCode::Custom(code) => code,
        }
    }
}

/// Represents a single precompile hint parsed from a `u64` slice.
///
/// A hint consists of a type identifier and associated data. The hint type
/// determines how the data should be processed by the [`PrecompileHintsProcessor`].
pub struct PrecompileHint {
    /// The type of hint, determining how the data should be processed.
    pub hint_code: HintCode,
    /// Whether this hint contains pass-through data (true) or requires computation (false).
    /// Determined by bit 31 (MSB) of the hint code.
    pub is_passthrough: bool,
    /// The hint payload data.
    pub data: Vec<u64>,
    /// Data length in bytes
    pub data_len_bytes: usize,
}

impl std::fmt::Debug for PrecompileHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data_display = if self.data.len() <= 10 {
            format!("{:x?}", self.data)
        } else {
            format!("{:x?}... ({} more)", &self.data[..10], self.data.len() - 10)
        };
        f.debug_struct("PrecompileHint")
            .field("hint_type", &self.hint_code)
            .field("is_passthrough", &self.is_passthrough)
            .field("data_len_bytes", &self.data_len_bytes)
            .field("data", &data_display)
            .finish()
    }
}

impl PrecompileHint {
    /// Parses a [`PrecompileHint`] from a slice of `u64` values at the given index.
    ///
    /// # Arguments
    ///
    /// * `slice` - The source slice containing concatenated hints
    /// * `idx` - The index where the hint header starts
    /// * `allow_custom` - If true, unknown codes create Custom variant; if false, return error
    ///
    /// # Returns
    ///
    /// * `Ok(PrecompileHint)` - Successfully parsed hint
    /// * `Err` - If the slice is too short or the index is out of bounds
    #[inline(always)]
    pub fn from_u64_slice(slice: &[u64], idx: usize, allow_custom: bool) -> Result<Self> {
        if slice.is_empty() || idx >= slice.len() {
            return Err(anyhow::anyhow!("Slice too short or index out of bounds"));
        }

        let header = slice[idx];

        // Extract length from lower 32 bits
        let length = header & 0xFFFFFFFF;

        // Calculate how many u64s are needed to hold length
        let num_u64s = ((length + 7) / 8) as usize;

        anyhow::ensure!(
            slice.len() >= idx + 1 + num_u64s,
            "Slice too short for hint data {}: expected {} u64s, got {}",
            (header >> 32) as u32,
            num_u64s,
            slice.len() - idx - 1
        );

        // Extract hint code from upper 32 bits
        let hint_code_32 = (header >> 32) as u32;
        // Extract pass-through flag from bit 31 (MSB) - shift is faster than mask
        let is_passthrough = hint_code_32 >> 31 != 0;
        // Extract the actual hint code from bits 0-30 - mask is optimal
        let hint_code_value = hint_code_32 & 0x7FFFFFFF;

        let hint_code = if allow_custom {
            HintCode::try_from(hint_code_value).unwrap_or(HintCode::Custom(hint_code_value))
        } else {
            HintCode::try_from(hint_code_value)?
        };

        // Create a new Vec with the hint data.
        let data = slice[idx + 1..idx + 1 + num_u64s].to_vec();

        Ok(PrecompileHint { hint_code, is_passthrough, data, data_len_bytes: length as usize })
    }

    /// Parses a [`PrecompileHint`] from a slice of `u64` values at the given byte index.
    ///
    /// # Arguments
    ///
    /// * `slice` - The source slice containing concatenated hints
    /// * `idx` - The **byte index** where the hint header starts
    /// * `allow_custom` - If true, unknown codes create Custom variant; if false, return error
    ///
    /// # Returns
    ///
    /// * `Ok(PrecompileHint)` - Successfully parsed hint
    /// * `Err` - If the slice is too short or the index is out of bounds
    #[inline(always)]
    pub fn from_unaligned_u64_slice(slice: &[u64], idx: usize, allow_custom: bool) -> Result<Self> {
        const HEADER_SIZE: usize = 8;

        // Convert u64 slice to byte slice
        let byte_slice =
            unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 8) };

        if idx + HEADER_SIZE > byte_slice.len() {
            return Err(anyhow::anyhow!("Slice too short for header at byte index {}", idx));
        }

        // Read 8-byte header as u64 (little-endian)
        let header = u64::from_le_bytes(byte_slice[idx..idx + HEADER_SIZE].try_into().unwrap());

        // Extract length from lower 32 bits (length is in bytes)
        let length = (header & 0xFFFFFFFF) as usize;

        // Calculate how many u64s are needed to hold length bytes
        let num_u64s = length.div_ceil(8);
        println!("Header: {:#x}, Length: {}, Num u64s: {}", header, length, num_u64s);
        anyhow::ensure!(
            idx + HEADER_SIZE + length <= byte_slice.len(),
            "Slice too short for hint data {}: expected {} bytes, got {}",
            (header >> 32) as u32,
            length,
            byte_slice.len() - idx - HEADER_SIZE
        );

        // Extract hint code from upper 32 bits
        let hint_code_32 = (header >> 32) as u32;
        // Extract pass-through flag from bit 31 (MSB) - shift is faster than mask
        let is_passthrough = hint_code_32 >> 31 != 0;
        // Extract the actual hint code from bits 0-30 - mask is optimal
        let hint_code_value = hint_code_32 & 0x7FFFFFFF;

        let hint_code = if allow_custom {
            HintCode::try_from(hint_code_value).unwrap_or(HintCode::Custom(hint_code_value))
        } else {
            HintCode::try_from(hint_code_value)?
        };

        // Extract data bytes and convert to u64 with optimal performance
        let data_bytes = &byte_slice[idx + HEADER_SIZE..idx + HEADER_SIZE + length];
        let mut data = Vec::with_capacity(num_u64s);

        let mut offset = 0;
        // Process full u64s with direct unaligned reads
        while offset + 8 <= data_bytes.len() {
            let value = unsafe { (data_bytes.as_ptr().add(offset) as *const u64).read_unaligned() };
            data.push(u64::from_le(value));
            offset += 8;
        }

        // Handle last partial u64 if any
        if offset < data_bytes.len() {
            let mut bytes = [0u8; 8];
            bytes[..data_bytes.len() - offset].copy_from_slice(&data_bytes[offset..]);
            data.push(u64::from_le_bytes(bytes));
        }

        Ok(PrecompileHint { hint_code, is_passthrough, data, data_len_bytes: length })
    }
}
