//! The row schedule the AIR proves: one 128-row block per call.
//!
//! Row 0 applies the initial external layer. Every round then takes four rows:
//! add the round constant, square, cube (multiply by the value saved on the square
//! row), and the linear layer (external for full rounds, internal for partial
//! rounds). Partial rounds only touch lane 0 in the first three rows; the fixed
//! `wide` selector switches the other lanes off. The 113 scheduled rows are
//! followed by hold rows, so the output sits on the last row of the block.
//!
//! Every row reduces once per lane: `lhs = next + p * q` with a canonical `next`
//! and a 32-bit `q`. The internal layer keeps the Montgomery factor `R = 2^32 mod p`
//! and a bias `B` that keeps its quotient non-negative:
//! `R * next + B * p = sum(state) + d_i * state_i + p * q`.

use std::fmt::Write;

use crate::{validate_state, Parameters, State, MODULUS, WIDTH};

/// Rows per call.
pub const CLOCKS: usize = 128;
/// `2^32 mod p`, the Montgomery factor carried by the internal layer.
pub const MONTGOMERY_R: u64 = (1_u64 << 32) % MODULUS as u64;

/// What a row computes; `Hold` rows carry the state unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    External,
    Internal,
    Add,
    Square,
    Cube,
    Hold,
}

/// One scheduled row: its phase, whether all lanes are active, and the round constants added.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub phase: Phase,
    pub wide: bool,
    pub constants: State,
}

/// Witness values of one row: the state entering the row, the cube base (nonzero only on
/// cube rows) and the per-lane reduction quotients.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub state: State,
    pub base: State,
    pub quotient: State,
}

/// Pinned parameters plus the derived schedule, internal diagonal and quotient bias.
pub struct Rounds {
    pub parameters: Parameters,
    /// `d_i` with `internal[i][i] * R = 1 + d_i (mod p)`, centred so lane 0 is `-2`.
    pub diagonal: [i64; WIDTH],
    /// `B = WIDTH + max(d_i)`; makes every internal-layer quotient non-negative.
    pub bias: u64,
    pub steps: [Step; CLOCKS],
}

impl Rounds {
    pub fn pinned() -> Self {
        let parameters = Parameters::pinned();
        let p = u64::from(MODULUS);
        let diagonal = std::array::from_fn(|i| {
            let d = (u64::from(parameters.internal[i][i]) * MONTGOMERY_R + p - 1) % p;
            if d > p / 2 {
                d as i64 - p as i64
            } else {
                d as i64
            }
        });
        for (i, row) in parameters.internal.iter().enumerate() {
            for (j, coefficient) in row.iter().enumerate() {
                let expected =
                    if i == j { (1 + diagonal[i]).rem_euclid(p as i64) as u64 } else { 1 };
                assert_eq!(u64::from(*coefficient) * MONTGOMERY_R % p, expected);
            }
        }
        for row in parameters.external {
            assert_eq!(row.into_iter().map(u64::from).sum::<u64>(), 35);
        }
        let bias = WIDTH as u64 + *diagonal.iter().max().unwrap() as u64;
        let mut schedule = vec![Step { phase: Phase::External, wide: true, constants: [0; WIDTH] }];
        for (wide, constants) in parameters
            .begin
            .into_iter()
            .map(|c| (true, c))
            .chain(parameters.partial.into_iter().map(|c| {
                let mut constants = [0; WIDTH];
                constants[0] = c;
                (false, constants)
            }))
            .chain(parameters.end.into_iter().map(|c| (true, c)))
        {
            for phase in [
                Phase::Add,
                Phase::Square,
                Phase::Cube,
                if wide { Phase::External } else { Phase::Internal },
            ] {
                schedule.push(Step {
                    phase,
                    wide,
                    constants: if phase == Phase::Add { constants } else { [0; WIDTH] },
                });
            }
        }
        assert!(schedule.len() < CLOCKS);
        schedule.resize(CLOCKS, Step { phase: Phase::Hold, wide: false, constants: [0; WIDTH] });
        Self { parameters, diagonal, bias, steps: schedule.try_into().unwrap() }
    }

    /// Runs the schedule, emitting `(clock, row)` for all `CLOCKS` rows, and returns the output.
    pub fn execute(
        &self,
        mut state: State,
        mut emit: impl FnMut(usize, Row),
    ) -> Result<State, String> {
        validate_state(&state)?;
        let p = u64::from(MODULUS);
        let mut base = [0; WIDTH];
        for (clock, step) in self.steps.iter().enumerate() {
            let mut next = state;
            let mut quotient = [0; WIDTH];
            let sum = state.iter().copied().map(u64::from).sum::<u64>();
            for i in 0..WIDTH {
                let a = u64::from(state[i]);
                if step.phase == Phase::Internal {
                    let mixed = i128::from(sum) + i128::from(self.diagonal[i]) * i128::from(a);
                    let scaled = mixed.rem_euclid(i128::from(p)) as u64;
                    // Every off-diagonal coefficient equals R^-1.
                    let inverse = u64::from(self.parameters.internal[0][1]);
                    next[i] = (scaled * inverse % p) as u32;
                    let numerator =
                        i128::from(MONTGOMERY_R * u64::from(next[i]) + self.bias * p) - mixed;
                    assert_eq!(numerator % i128::from(p), 0);
                    quotient[i] = u32::try_from(numerator / i128::from(p))
                        .expect("internal quotient out of range");
                    continue;
                }
                let wide = step.wide || i == 0;
                let value = match step.phase {
                    Phase::External => self.parameters.external[i]
                        .iter()
                        .zip(state)
                        .map(|(c, x)| u64::from(*c) * u64::from(x))
                        .sum(),
                    Phase::Add if wide => a + u64::from(step.constants[i]),
                    Phase::Square if wide => a * a,
                    Phase::Cube if wide => a * u64::from(base[i]),
                    _ => continue,
                };
                next[i] = (value % p) as u32;
                quotient[i] = (value / p) as u32;
            }
            emit(
                clock,
                Row {
                    state,
                    base: if step.phase == Phase::Cube { base } else { [0; WIDTH] },
                    quotient,
                },
            );
            if step.phase == Phase::Square {
                base = state;
            }
            state = next;
        }
        Ok(state)
    }
}

/// Run-length encodes a fixed column in PIL literal syntax (`value:count`).
fn sequence(values: &[u64]) -> String {
    let mut runs = Vec::new();
    let mut first = 0;
    while first < values.len() {
        let value = values[first];
        let mut end = first + 1;
        while end < values.len() && values[end] == value {
            end += 1;
        }
        runs.push(if end - first == 1 {
            value.to_string()
        } else {
            format!("{value}:{}", end - first)
        });
        first = end;
    }
    runs.join(",")
}

/// Renders the AIR template with the fixed selector, constant and linear-layer columns.
pub fn vm_pil() -> String {
    let rounds = Rounds::pinned();
    let mut fixed = String::new();
    let mut column = |name: &str, values: Vec<u64>| {
        writeln!(fixed, "    col fixed {name} = [[{}]:BLOCKS];", sequence(&values)).unwrap();
    };
    column("first", (0..CLOCKS).map(|i| u64::from(i == 0)).collect());
    column("last", (0..CLOCKS).map(|i| u64::from(i == CLOCKS - 1)).collect());
    for (name, phase) in [
        ("external", Phase::External),
        ("internal", Phase::Internal),
        ("op_add", Phase::Add),
        ("square", Phase::Square),
        ("cube", Phase::Cube),
    ] {
        column(name, rounds.steps.iter().map(|s| u64::from(s.phase == phase)).collect());
    }
    column("wide", rounds.steps.iter().map(|s| u64::from(s.wide)).collect());
    for i in 0..WIDTH {
        column(&format!("rc{i}"), rounds.steps.iter().map(|s| u64::from(s.constants[i])).collect());
    }
    let mut linear = String::new();
    for i in 0..WIDTH {
        let sum = rounds.parameters.external[i]
            .iter()
            .enumerate()
            .map(|(j, c)| format!("{c} * state[{j}]"))
            .collect::<Vec<_>>()
            .join(" + ");
        writeln!(linear, "    ext[{i}] = {sum};").unwrap();
        writeln!(linear, "    diag[{i}] = {};", rounds.diagonal[i]).unwrap();
        writeln!(linear, "    rc[{i}] = rc{i};").unwrap();
    }
    include_str!("../pil/koala_poseidon2_rounds.pil.in")
        .replace("@P@", &MODULUS.to_string())
        .replace("@R@", &MONTGOMERY_R.to_string())
        .replace("@BIAS@", &rounds.bias.to_string())
        .replace("@ROWS@", &CLOCKS.to_string())
        .replace("@FIXED@", &fixed)
        .replace("@LINEAR@", &linear)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_rounds_match_independent_dense_permutation() {
        let rounds = Rounds::pinned();
        let mut seed = 0x1234_5678_9abc_def0_u64;
        for case in 0..104 {
            let input = std::array::from_fn(|i| match case {
                0 => 0,
                1 => MODULUS - 1,
                2 => i as u32,
                3 => {
                    if i % 2 == 0 {
                        0
                    } else {
                        MODULUS - 1
                    }
                }
                _ => {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    (seed % u64::from(MODULUS)) as u32
                }
            });
            let mut rows = Vec::new();
            let actual = rounds.execute(input, |_, row| rows.push(row)).unwrap();
            assert_eq!(actual, rounds.parameters.permute(input).unwrap());
            assert_eq!(rows.len(), CLOCKS);
            assert_eq!(rows[0].state, input);
            assert_eq!(rows[CLOCKS - 1].state, actual);
            for (clock, row) in rows.iter().enumerate().take(CLOCKS - 1) {
                if rounds.steps[clock].phase == Phase::Square {
                    assert_eq!(rows[clock + 1].base, row.state);
                }
                assert!(row.state.iter().all(|v| *v < MODULUS));
            }
        }
    }

    #[test]
    fn bounded_integer_relations_fit_goldilocks() {
        let rounds = Rounds::pinned();
        let p = u128::from(MODULUS);
        let g = u128::from(crate::GOLDILOCKS);
        let quotient = u128::from(u32::MAX);
        assert!((p - 1) * (p - 1) < g);
        assert!(quotient * p + p - 1 < g);
        assert!(u128::from(MONTGOMERY_R) * (p - 1) + u128::from(rounds.bias) * p + p - 1 < g);
        assert!(quotient * p + u128::from(rounds.bias) * (p - 1) < g);
        assert!(u128::from(rounds.bias) + u128::from(MONTGOMERY_R) < quotient);
        assert_eq!(rounds.diagonal.iter().min(), Some(&-2));
    }

    #[test]
    fn noncanonical_input_is_rejected_before_emission() {
        let rounds = Rounds::pinned();
        let mut input = [0; WIDTH];
        input[15] = MODULUS;
        assert!(rounds.execute(input, |_, _| panic!("unexpected row")).is_err());
    }
}
