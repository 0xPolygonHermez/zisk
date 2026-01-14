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
//! - `0x04` (`Noop`): Pass-through data
//! - `0x05` (`EcRecover`): ECRECOVER inputs (currently returns empty)
//! ```

use std::fmt::Display;

use anyhow::Result;

// === CONTROL CODES ===
const CTRL_START: u32 = 0x00;
const CTRL_END: u32 = 0x01;
const CTRL_CANCEL: u32 = 0x02;
const CTRL_ERROR: u32 = 0x03;

// === BUILT-IN HINT CODES ===
// Noop hint code
const HINT_NOOP: u32 = 0x04;
// Ecrecover hint code
const HINT_ECRECOVER: u32 = 0x05;

// Big integer arithmetic hint codes
const HINT_REDMOD256: u32 = 0x06;
const HINT_ADDMOD256: u32 = 0x07;
const HINT_MULMOD256: u32 = 0x08;
const HINT_DIVREM256: u32 = 0x09;
const HINT_WPOW256: u32 = 0x0A;
const HINT_OMUL256: u32 = 0x0B;
const HINT_WMUL256: u32 = 0x0C;

// Modular exponentiation hint code
const HINT_MODEXP: u32 = 0x0D;

// BN254 hint codes
const HINT_IS_ON_CURVE_BN254: u32 = 0x0E;
const HINT_TO_AFFINE_BN254: u32 = 0x0F;
const HINT_ADD_BN254: u32 = 0x10;
const HINT_MUL_BN254: u32 = 0x11;
const HINT_TO_AFFINE_TWIST_BN254: u32 = 0x12;
const HINT_IS_ON_CURVE_TWIST_BN254: u32 = 0x13;
const HINT_IS_ON_SUBGROUP_TWIST_BN254: u32 = 0x14;
const HINT_PAIRING_BATCH_BN254: u32 = 0x15;

// BLS12-381 hint codes
const HINT_MUL_FP12_BLS12_381: u32 = 0x16;
const HINT_DECOMPRESS_BLS12_381: u32 = 0x17;
const HINT_IS_ON_CURVE_BLS12_381: u32 = 0x18;
const HINT_IS_ON_SUBGROUP_BLS12_381: u32 = 0x19;
const HINT_ADD_BLS12_381: u32 = 0x1A;
const HINT_SCALAR_MUL_BLS12_381: u32 = 0x1B;
const HINT_DECOMPRESS_TWIST_BLS12_381: u32 = 0x1C;
const HINT_IS_ON_CURVE_TWIST_BLS12_381: u32 = 0x1D;
const HINT_IS_ON_SUBGROUP_TWIST_BLS12_381: u32 = 0x1E;
const HINT_ADD_TWIST_BLS12_381: u32 = 0x1F;
const HINT_SCALAR_MUL_TWIST_BLS12_381: u32 = 0x20;
const HINT_MILLER_LOOP_BLS12_381: u32 = 0x21;
const HINT_FINAL_EXP_BLS12_381: u32 = 0x22;

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
    /// Pass-through hint type.
    /// When a hint has this type, the processor simply passes through the data
    /// without any additional computation.
    Noop = HINT_NOOP,

    /// Ecrecover hint type.
    EcRecover = HINT_ECRECOVER,

    // Big Integer Arithmetic Hints
    ///  Modular reduction of a 256-bit integer hint type.
    RedMod256 = HINT_REDMOD256,
    /// Modular addition of 256-bit integers hint type.
    AddMod256 = HINT_ADDMOD256,
    /// Modular multiplication of 256-bit integers hint type.
    MulMod256 = HINT_MULMOD256,
    /// Division and remainder of 256-bit integers hint type.
    DivRem256 = HINT_DIVREM256,
    /// Wrapping exponentiation of 256-bit integers hint type.
    WPow256 = HINT_WPOW256,
    /// Overflowing multiplication of 256-bit integers hint type.
    OMul256 = HINT_OMUL256,
    /// Wrapping multiplication of 256-bit integers hint type.
    WMul256 = HINT_WMUL256,

    /// Modular exponentiation hint type.
    ModExp = HINT_MODEXP,

    // BN254 Precompile Hints
    /// Check if point is on curve hint type for BN254 curve.
    IsOnCurveBn254 = HINT_IS_ON_CURVE_BN254,
    /// Convert to affine coordinates hint type for BN254 curve.
    ToAffineBn254 = HINT_TO_AFFINE_BN254,
    /// Point addition hint type for BN254 curve.
    AddBn254 = HINT_ADD_BN254,
    /// Scalar multiplication hint type for BN254 curve.
    MulBn254 = HINT_MUL_BN254,
    /// Convert to affine coordinates hint type for BN254 twist.
    ToAffineTwistBn254 = HINT_TO_AFFINE_TWIST_BN254,
    /// Check if point is on curve hint type for BN254 twist.
    IsOnCurveTwistBn254 = HINT_IS_ON_CURVE_TWIST_BN254,
    /// Check if point is in subgroup hint type for BN254 twist.
    IsOnSubgroupTwistBn254 = HINT_IS_ON_SUBGROUP_TWIST_BN254,
    /// Pairing batch computation hint type for BN254 curve.
    PairingBatchBn254 = HINT_PAIRING_BATCH_BN254,

    // BLS12-381 Precompile Hints
    /// Multiplication in Fp12 hint type for BLS12-381 curve.
    MulFp12Bls12_381 = HINT_MUL_FP12_BLS12_381,
    /// Point decompression hint type for BLS12-381 curve.
    DecompressBls12_381 = HINT_DECOMPRESS_BLS12_381,
    /// Check if point is on curve hint type for BLS12-381 curve.
    IsOnCurveBls12_381 = HINT_IS_ON_CURVE_BLS12_381,
    /// Check if point is in subgroup hint type for BLS12-381 curve.
    IsOnSubgroupBls12_381 = HINT_IS_ON_SUBGROUP_BLS12_381,
    /// Point addition hint type for BLS12-381 curve.
    AddBls12_381 = HINT_ADD_BLS12_381,
    /// Scalar multiplication hint type for BLS12-381 curve.
    ScalarMulBls12_381 = HINT_SCALAR_MUL_BLS12_381,
    /// Point decompression hint type for BLS12-381 twist.
    DecompressTwistBls12_381 = HINT_DECOMPRESS_TWIST_BLS12_381,
    /// Check if point is on curve hint type for BLS12-381 twist.
    IsOnCurveTwistBls12_381 = HINT_IS_ON_CURVE_TWIST_BLS12_381,
    /// Check if point is in subgroup hint type for BLS12-381 twist.
    IsOnSubgroupTwistBls12_381 = HINT_IS_ON_SUBGROUP_TWIST_BLS12_381,
    /// Point addition hint type for BLS12-381 twist.
    AddTwistBls12_381 = HINT_ADD_TWIST_BLS12_381,
    /// Scalar multiplication hint type for BLS12-381 twist.
    ScalarMulTwistBls12_381 = HINT_SCALAR_MUL_TWIST_BLS12_381,
    /// Miller loop computation hint type for BLS12-381 curve.
    MillerLoopBls12_381 = HINT_MILLER_LOOP_BLS12_381,
    /// Final exponentiation computation hint type for BLS12-381 curve.
    FinalExpBls12_381 = HINT_FINAL_EXP_BLS12_381,
}

impl Display for BuiltInHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            // Noop Hint
            BuiltInHint::Noop => "NOOP",

            // Ecrecover Hint
            BuiltInHint::EcRecover => "ECRECOVER",

            // Big Integer Arithmetic Hints
            BuiltInHint::RedMod256 => "REDMOD256",
            BuiltInHint::AddMod256 => "ADDMOD256",
            BuiltInHint::MulMod256 => "MULMOD256",
            BuiltInHint::DivRem256 => "DIVREM256",
            BuiltInHint::WPow256 => "WPOW256",
            BuiltInHint::OMul256 => "OMUL256",
            BuiltInHint::WMul256 => "WMUL256",

            // Modular Exponentiation Hint
            BuiltInHint::ModExp => "MODEXP",

            // BN254 Hints
            BuiltInHint::IsOnCurveBn254 => "IS_ON_CURVE_BN254",
            BuiltInHint::ToAffineBn254 => "TO_AFFINE_BN254",
            BuiltInHint::AddBn254 => "ADD_BN254",
            BuiltInHint::MulBn254 => "MUL_BN254",
            BuiltInHint::ToAffineTwistBn254 => "TO_AFFINE_TWIST_BN254",
            BuiltInHint::IsOnCurveTwistBn254 => "IS_ON_CURVE_TWIST_BN254",
            BuiltInHint::IsOnSubgroupTwistBn254 => "IS_ON_SUBGROUP_TWIST_BN254",
            BuiltInHint::PairingBatchBn254 => "PAIRING_BATCH_BN254",

            // BLS12-381 Hints
            BuiltInHint::MulFp12Bls12_381 => "MUL_FP12_BLS12_381",
            BuiltInHint::DecompressBls12_381 => "DECOMPRESS_BLS12_381",
            BuiltInHint::IsOnCurveBls12_381 => "IS_ON_CURVE_BLS12_381",
            BuiltInHint::IsOnSubgroupBls12_381 => "IS_ON_SUBGROUP_BLS12_381",
            BuiltInHint::AddBls12_381 => "ADD_BLS12_381",
            BuiltInHint::ScalarMulBls12_381 => "SCALAR_MUL_BLS12_381",
            BuiltInHint::DecompressTwistBls12_381 => "DECOMPRESS_TWIST_BLS12_381",
            BuiltInHint::IsOnCurveTwistBls12_381 => "IS_ON_CURVE_TWIST_BLS12_381",
            BuiltInHint::IsOnSubgroupTwistBls12_381 => "IS_ON_SUBGROUP_TWIST_BLS12_381",
            BuiltInHint::AddTwistBls12_381 => "ADD_TWIST_BLS12_381",
            BuiltInHint::ScalarMulTwistBls12_381 => "SCALAR_MUL_TWIST_BLS12_381",
            BuiltInHint::MillerLoopBls12_381 => "MILLER_LOOP_BLS12_381",
            BuiltInHint::FinalExpBls12_381 => "FINAL_EXP_BLS12_381",
        };
        write!(f, "{} ({:#x})", name, *self as u32)
    }
}

impl TryFrom<u32> for BuiltInHint {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        match value {
            // Noop Hint
            HINT_NOOP => Ok(Self::Noop),

            // Ecrecover Hint
            HINT_ECRECOVER => Ok(Self::EcRecover),

            // Big Integer Arithmetic Hints
            HINT_REDMOD256 => Ok(Self::RedMod256),
            HINT_ADDMOD256 => Ok(Self::AddMod256),
            HINT_MULMOD256 => Ok(Self::MulMod256),
            HINT_DIVREM256 => Ok(Self::DivRem256),
            HINT_WPOW256 => Ok(Self::WPow256),
            HINT_OMUL256 => Ok(Self::OMul256),
            HINT_WMUL256 => Ok(Self::WMul256),

            // Modular Exponentiation Hint
            HINT_MODEXP => Ok(Self::ModExp),

            // BN254 Hints
            HINT_IS_ON_CURVE_BN254 => Ok(Self::IsOnCurveBn254),
            HINT_TO_AFFINE_BN254 => Ok(Self::ToAffineBn254),
            HINT_ADD_BN254 => Ok(Self::AddBn254),
            HINT_MUL_BN254 => Ok(Self::MulBn254),
            HINT_TO_AFFINE_TWIST_BN254 => Ok(Self::ToAffineTwistBn254),
            HINT_IS_ON_CURVE_TWIST_BN254 => Ok(Self::IsOnCurveTwistBn254),
            HINT_IS_ON_SUBGROUP_TWIST_BN254 => Ok(Self::IsOnSubgroupTwistBn254),
            HINT_PAIRING_BATCH_BN254 => Ok(Self::PairingBatchBn254),

            // BLS12-381 Hints
            HINT_MUL_FP12_BLS12_381 => Ok(Self::MulFp12Bls12_381),
            HINT_DECOMPRESS_BLS12_381 => Ok(Self::DecompressBls12_381),
            HINT_IS_ON_CURVE_BLS12_381 => Ok(Self::IsOnCurveBls12_381),
            HINT_IS_ON_SUBGROUP_BLS12_381 => Ok(Self::IsOnSubgroupBls12_381),
            HINT_ADD_BLS12_381 => Ok(Self::AddBls12_381),
            HINT_SCALAR_MUL_BLS12_381 => Ok(Self::ScalarMulBls12_381),
            HINT_DECOMPRESS_TWIST_BLS12_381 => Ok(Self::DecompressTwistBls12_381),
            HINT_IS_ON_CURVE_TWIST_BLS12_381 => Ok(Self::IsOnCurveTwistBls12_381),
            HINT_IS_ON_SUBGROUP_TWIST_BLS12_381 => Ok(Self::IsOnSubgroupTwistBls12_381),
            HINT_ADD_TWIST_BLS12_381 => Ok(Self::AddTwistBls12_381),
            HINT_SCALAR_MUL_TWIST_BLS12_381 => Ok(Self::ScalarMulTwistBls12_381),
            HINT_MILLER_LOOP_BLS12_381 => Ok(Self::MillerLoopBls12_381),
            HINT_FINAL_EXP_BLS12_381 => Ok(Self::FinalExpBls12_381),

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
            // Noop Hint
            HintCode::BuiltIn(BuiltInHint::Noop) => HINT_NOOP,

            // Ecrecover Hint
            HintCode::BuiltIn(BuiltInHint::EcRecover) => HINT_ECRECOVER,

            // Big Integer Arithmetic Hints
            HintCode::BuiltIn(BuiltInHint::RedMod256) => HINT_REDMOD256,
            HintCode::BuiltIn(BuiltInHint::AddMod256) => HINT_ADDMOD256,
            HintCode::BuiltIn(BuiltInHint::MulMod256) => HINT_MULMOD256,
            HintCode::BuiltIn(BuiltInHint::DivRem256) => HINT_DIVREM256,
            HintCode::BuiltIn(BuiltInHint::WPow256) => HINT_WPOW256,
            HintCode::BuiltIn(BuiltInHint::OMul256) => HINT_OMUL256,
            HintCode::BuiltIn(BuiltInHint::WMul256) => HINT_WMUL256,

            // Modular Exponentiation Hint
            HintCode::BuiltIn(BuiltInHint::ModExp) => HINT_MODEXP,

            // BN254 Hints
            HintCode::BuiltIn(BuiltInHint::IsOnCurveBn254) => HINT_IS_ON_CURVE_BN254,
            HintCode::BuiltIn(BuiltInHint::ToAffineBn254) => HINT_TO_AFFINE_BN254,
            HintCode::BuiltIn(BuiltInHint::AddBn254) => HINT_ADD_BN254,
            HintCode::BuiltIn(BuiltInHint::MulBn254) => HINT_MUL_BN254,
            HintCode::BuiltIn(BuiltInHint::ToAffineTwistBn254) => HINT_TO_AFFINE_TWIST_BN254,
            HintCode::BuiltIn(BuiltInHint::IsOnCurveTwistBn254) => HINT_IS_ON_CURVE_TWIST_BN254,
            HintCode::BuiltIn(BuiltInHint::IsOnSubgroupTwistBn254) => {
                HINT_IS_ON_SUBGROUP_TWIST_BN254
            }
            HintCode::BuiltIn(BuiltInHint::PairingBatchBn254) => HINT_PAIRING_BATCH_BN254,

            // BLS12-381 Hints
            HintCode::BuiltIn(BuiltInHint::MulFp12Bls12_381) => HINT_MUL_FP12_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::DecompressBls12_381) => HINT_DECOMPRESS_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::IsOnCurveBls12_381) => HINT_IS_ON_CURVE_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::IsOnSubgroupBls12_381) => HINT_IS_ON_SUBGROUP_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::AddBls12_381) => HINT_ADD_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::ScalarMulBls12_381) => HINT_SCALAR_MUL_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::DecompressTwistBls12_381) => {
                HINT_DECOMPRESS_TWIST_BLS12_381
            }
            HintCode::BuiltIn(BuiltInHint::IsOnCurveTwistBls12_381) => {
                HINT_IS_ON_CURVE_TWIST_BLS12_381
            }
            HintCode::BuiltIn(BuiltInHint::IsOnSubgroupTwistBls12_381) => {
                HINT_IS_ON_SUBGROUP_TWIST_BLS12_381
            }
            HintCode::BuiltIn(BuiltInHint::AddTwistBls12_381) => HINT_ADD_TWIST_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::ScalarMulTwistBls12_381) => {
                HINT_SCALAR_MUL_TWIST_BLS12_381
            }
            HintCode::BuiltIn(BuiltInHint::MillerLoopBls12_381) => HINT_MILLER_LOOP_BLS12_381,
            HintCode::BuiltIn(BuiltInHint::FinalExpBls12_381) => HINT_FINAL_EXP_BLS12_381,

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
    /// The hint payload data.
    pub data: Vec<u64>,
}

impl std::fmt::Debug for PrecompileHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data_display = if self.data.len() <= 10 {
            format!("{:?}", self.data)
        } else {
            format!("{:?}... ({} more)", &self.data[..10], self.data.len() - 10)
        };
        f.debug_struct("PrecompileHint")
            .field("hint_type", &self.hint_code)
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
        let length = (header & 0xFFFFFFFF) as u32;

        if slice.len() < idx + length as usize + 1 {
            return Err(anyhow::anyhow!(
                "Slice too short for hint data: expected {}, got {}",
                length,
                slice.len() - idx - 1
            ));
        }

        let hint_code_32 = (header >> 32) as u32;
        let hint_code = if allow_custom {
            HintCode::try_from(hint_code_32).unwrap_or(HintCode::Custom(hint_code_32))
        } else {
            HintCode::try_from(hint_code_32)?
        };

        // Create a new Vec with the hint data.
        let data = slice[idx + 1..idx + length as usize + 1].to_vec();

        Ok(PrecompileHint { hint_code, data })
    }
}
