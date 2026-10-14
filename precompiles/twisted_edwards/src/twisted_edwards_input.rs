use zisk_common::OperationBabyJubJubAddData;

use crate::executors::TwistedEdwardsCurve;

#[derive(Debug)]
pub enum TwistedEdwardsInput {
    BabyJubJubAdd(TwistedEdwardsAddInput),
    Ed25519Add(TwistedEdwardsAddInput),
}

impl TwistedEdwardsInput {
    pub fn curve_and_add(&self) -> (TwistedEdwardsCurve, &TwistedEdwardsAddInput) {
        match self {
            Self::BabyJubJubAdd(input) => (TwistedEdwardsCurve::BabyJubJub, input),
            Self::Ed25519Add(input) => (TwistedEdwardsCurve::Ed25519, input),
        }
    }
}

/// Point-add input, the same for every curve: the two points, whose addresses travel in the main
/// operands a (p1, receives the result) and b (p2).
#[derive(Debug)]
pub struct TwistedEdwardsAddInput {
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl TwistedEdwardsAddInput {
    // `OperationEd25519AddData` is the same array type, so this serves both ops.
    pub fn from(values: &OperationBabyJubJubAddData<u64>) -> Self {
        Self {
            p1_addr: values[2] as u32,
            p2_addr: values[3] as u32,
            step: values[4],
            p1: values[5..13].try_into().unwrap(),
            p2: values[13..21].try_into().unwrap(),
        }
    }
}
