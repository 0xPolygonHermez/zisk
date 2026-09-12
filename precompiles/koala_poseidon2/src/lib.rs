//! Pinned KoalaBear Poseidon2 reference permutation, round schedule and PIL generation.

pub mod rounds;

use serde::Serialize;
use slop_algebra::{AbstractField, PrimeField32};
use slop_koala_bear::{
    DiffusionMatrixKoalaBear, KoalaBear, KoalaBear_BEGIN_EXT_CONSTS, KoalaBear_END_EXT_CONSTS,
    KoalaBear_PARTIAL_CONSTS,
};
use slop_poseidon2::Poseidon2ExternalMatrixGeneral;
use slop_symmetric::Permutation;

pub const WIDTH: usize = 16;
pub const MODULUS: u32 = KoalaBear::ORDER_U32;
pub const GOLDILOCKS: u64 = 0xffff_ffff_0000_0001;
pub type State = [u32; WIDTH];
pub type Matrix = [[u32; WIDTH]; WIDTH];

/// Round constants and linear layers taken from the pinned upstream crates.
///
/// Both matrices are stored as they act on column vectors, so `internal` keeps the
/// `2^-32` Montgomery factor of `DiffusionMatrixKoalaBear`; that factor is part of the
/// permutation, not an artifact of the representation.
#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub upstream: &'static str,
    pub modulus: u32,
    pub exponent: u32,
    pub begin: [[u32; WIDTH]; 4],
    pub partial: [u32; 20],
    pub end: [[u32; WIDTH]; 4],
    pub external: Matrix,
    pub internal: Matrix,
}

impl Parameters {
    pub fn pinned() -> Self {
        Self {
            upstream: "slop-koala-bear=6.2.2;p3-koala-bear=0.4.3-succinct",
            modulus: MODULUS,
            exponent: 3,
            begin: KoalaBear_BEGIN_EXT_CONSTS.map(|round| round.map(|x| x.as_canonical_u32())),
            partial: KoalaBear_PARTIAL_CONSTS.map(|x| x.as_canonical_u32()),
            end: KoalaBear_END_EXT_CONSTS.map(|round| round.map(|x| x.as_canonical_u32())),
            external: matrix_from_basis(Poseidon2ExternalMatrixGeneral),
            internal: matrix_from_basis(DiffusionMatrixKoalaBear),
        }
    }

    /// Dense reference permutation over canonical lanes; the oracle every other path is tested against.
    pub fn permute(&self, mut state: State) -> Result<State, String> {
        validate_state(&state)?;
        state = apply_matrix(&self.external, state);
        for constants in self.begin {
            state = apply_matrix(&self.external, full_round(state, constants));
        }
        for constant in self.partial {
            state[0] = cube(add(state[0], constant));
            state = apply_matrix(&self.internal, state);
        }
        for constants in self.end {
            state = apply_matrix(&self.external, full_round(state, constants));
        }
        Ok(state)
    }
}

/// Recovers a linear layer as a matrix by applying it to each basis vector.
fn matrix_from_basis(permutation: impl Permutation<[KoalaBear; WIDTH]>) -> Matrix {
    let mut matrix = [[0; WIDTH]; WIDTH];
    for column in 0..WIDTH {
        let mut basis = [KoalaBear::zero(); WIDTH];
        basis[column] = KoalaBear::one();
        permutation.permute_mut(&mut basis);
        for row in 0..WIDTH {
            matrix[row][column] = basis[row].as_canonical_u32();
        }
    }
    matrix
}

fn full_round(mut state: State, constants: State) -> State {
    for i in 0..WIDTH {
        state[i] = cube(add(state[i], constants[i]));
    }
    state
}

fn apply_matrix(matrix: &Matrix, state: State) -> State {
    matrix.map(|row| {
        row.into_iter()
            .zip(state)
            .fold(0, |sum, (coefficient, input)| add(sum, multiply(coefficient, input)))
    })
}

/// Rejects any lane at or above the modulus; noncanonical lanes are never reduced.
pub fn validate_state(state: &State) -> Result<(), String> {
    for (i, value) in state.iter().enumerate() {
        if *value >= MODULUS {
            return Err(format!("noncanonical input lane {i}: {value}"));
        }
    }
    Ok(())
}

/// Sixteen canonical u32 field elements, little-endian; not raw Montgomery limbs.
pub fn decode_input(bytes: &[u8]) -> Result<State, String> {
    if bytes.len() != WIDTH * 4 {
        return Err(format!("expected {} input bytes, got {}", WIDTH * 4, bytes.len()));
    }
    let state =
        std::array::from_fn(|i| u32::from_le_bytes(bytes[4 * i..4 * (i + 1)].try_into().unwrap()));
    validate_state(&state)?;
    Ok(state)
}

pub fn encode_output(state: State) -> Result<[u8; WIDTH * 4], String> {
    validate_state(&state)?;
    let mut bytes = [0; WIDTH * 4];
    for (i, value) in state.into_iter().enumerate() {
        bytes[4 * i..4 * (i + 1)].copy_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}

fn add(a: u32, b: u32) -> u32 {
    ((u64::from(a) + u64::from(b)) % u64::from(MODULUS)) as u32
}

fn multiply(a: u32, b: u32) -> u32 {
    (u64::from(a) * u64::from(b) % u64::from(MODULUS)) as u32
}

fn cube(a: u32) -> u32 {
    multiply(multiply(a, a), a)
}
