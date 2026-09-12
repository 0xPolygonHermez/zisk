//! KoalaBear Poseidon2 for the emulator and hint paths.
//!
//! States are sixteen canonical `u32` lanes packed pairwise into eight `u64` words,
//! little-endian; Montgomery limbs are rejected, not reduced.

#[path = "koala_poseidon2_parameters.rs"]
#[rustfmt::skip]
mod parameters;

pub const MODULUS: u32 = 2_130_706_433;
pub const WIDTH: usize = 16;
pub const WORDS: usize = 8;

/// Index of the first lane at or above the modulus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoncanonicalInput(pub usize);

/// Checks every lane without touching the words.
pub fn validate_packed(words: &[u64; WORDS]) -> Result<(), NoncanonicalInput> {
    for lane in 0..WIDTH {
        if (words[lane / 2] >> (32 * (lane % 2))) as u32 >= MODULUS {
            return Err(NoncanonicalInput(lane));
        }
    }
    Ok(())
}

/// Permutes in place; on a noncanonical lane the words are left unchanged.
pub fn permute_packed(words: &mut [u64; WORDS]) -> Result<(), NoncanonicalInput> {
    let mut state = core::array::from_fn(|i| (words[i / 2] >> (32 * (i % 2))) as u32);
    permute(&mut state)?;
    for (i, word) in words.iter_mut().enumerate() {
        *word = u64::from(state[2 * i]) | (u64::from(state[2 * i + 1]) << 32);
    }
    Ok(())
}

pub fn permute(state: &mut [u32; WIDTH]) -> Result<(), NoncanonicalInput> {
    for (i, value) in state.iter().enumerate() {
        if *value >= MODULUS {
            return Err(NoncanonicalInput(i));
        }
    }
    matrix(state, &parameters::EXTERNAL);
    for constants in &parameters::BEGIN {
        full_round(state, constants);
    }
    for constant in parameters::PARTIAL {
        state[0] = cube(add(state[0], constant));
        matrix(state, &parameters::INTERNAL);
    }
    for constants in &parameters::END {
        full_round(state, constants);
    }
    Ok(())
}

fn full_round(state: &mut [u32; WIDTH], constants: &[u32; WIDTH]) {
    for i in 0..WIDTH {
        state[i] = cube(add(state[i], constants[i]));
    }
    matrix(state, &parameters::EXTERNAL);
}

fn matrix(state: &mut [u32; WIDTH], coefficients: &[[u32; WIDTH]; WIDTH]) {
    let input = *state;
    for i in 0..WIDTH {
        state[i] = 0;
        for j in 0..WIDTH {
            state[i] = add(state[i], mul(input[j], coefficients[i][j]));
        }
    }
}

fn add(a: u32, b: u32) -> u32 {
    ((u64::from(a) + u64::from(b)) % u64::from(MODULUS)) as u32
}
fn mul(a: u32, b: u32) -> u32 {
    (u64::from(a) * u64::from(b) % u64::from(MODULUS)) as u32
}
fn cube(a: u32) -> u32 {
    mul(mul(a, a), a)
}
