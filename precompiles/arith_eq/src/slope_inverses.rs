//! Batch inversion of the slope denominators a run of curve operations needs.
//!
//! # Why
//!
//! Every elliptic-curve add or double computes a slope `s = num / den`, and `Fp::div_assign` is
//! `mul_assign(&other.inverse().unwrap())` -- a full modular inversion, which on these fields costs
//! about 340 field multiplications and is two thirds of the whole executor. Montgomery's trick
//! inverts a whole run with a single inversion plus three multiplications per element, which
//! measured ~85x cheaper per inversion from ~512 elements on.
//!
//! # The two passes and the invariant between them
//!
//! Batching forces the fill to see each operation twice: once to collect its denominator, and again
//! to write its rows. Both passes walk the same operation sequence in the same order, so the `i`-th
//! curve operation of a field in the second pass is the `i`-th inverse this type hands out. That is
//! the invariant the cursors rest on, and an out-of-step fill panics on the indexing rather than
//! silently using another operation's inverse.
//!
//! One run per field, because the three curves are three distinct types: an operation takes its
//! inverse from the run for its own curve, and the runs advance independently.
//!
//! # Zero denominators
//!
//! `ark_ff::batch_inversion` *skips* zeros and leaves them zero, where the `/` this replaces
//! panicked on `inverse()` returning `None`. Silently turning a panic into a wrong witness is the
//! worse failure, so the collection asserts instead. A zero denominator means `x1 == x2` on an add
//! (the PIL's `x_are_different` column already forbids it) or `y1 == 0` on a double (no such point
//! on a prime-order curve).

use ark_ff::{batch_inversion, Field};

use crate::{executors, ArithEqInput};
pub use executors::{Bn254Field, Secp256k1Field, Secp256r1Field};

/// Operations per inversion run. Past ~512 the per-inversion cost has flattened, so the rest of the
/// budget goes to locality: at this size the denominators stay in L1/L2 and the second pass re-reads
/// inputs the first pass has just touched.
pub(crate) const INV_RUN_OPS: usize = 1024;

/// The inverted slope denominators for one run of operations, one queue per curve.
///
/// Reused across the runs of a batch ([`Self::reload`]) so the three buffers are allocated once and
/// keep their capacity.
#[derive(Default)]
pub(crate) struct SlopeInverses {
    secp256k1: Vec<Secp256k1Field>,
    secp256r1: Vec<Secp256r1Field>,
    bn254: Vec<Bn254Field>,
    next: [usize; 3],
}

/// Inverts a run in place, after ruling out the zeros `batch_inversion` would quietly pass through.
fn invert_run<F: Field>(den: &mut [F]) {
    assert!(
        !den.iter().any(|d| d.is_zero()),
        "slope denominator is zero: an add with x1 == x2, or a double with y1 == 0"
    );
    batch_inversion(den);
}

impl SlopeInverses {
    /// Collects and inverts the denominators of `ops`, discarding whatever the previous run left.
    ///
    /// `ops` must be exactly the operations the fill is about to write, in the order it will write
    /// them; non-curve operations contribute nothing and are skipped.
    pub(crate) fn reload<'a>(&mut self, ops: impl Iterator<Item = &'a ArithEqInput>) {
        self.secp256k1.clear();
        self.secp256r1.clear();
        self.bn254.clear();
        self.next = [0; 3];

        for op in ops {
            match op {
                ArithEqInput::Secp256k1Add(i) => self
                    .secp256k1
                    .push(executors::Secp256k1::slope_denominator(false, &i.p1, &i.p2)),
                ArithEqInput::Secp256k1Dbl(i) => {
                    self.secp256k1.push(executors::Secp256k1::slope_denominator(true, &i.p1, &i.p1))
                }
                ArithEqInput::Secp256r1Add(i) => self
                    .secp256r1
                    .push(executors::Secp256r1::slope_denominator(false, &i.p1, &i.p2)),
                ArithEqInput::Secp256r1Dbl(i) => {
                    self.secp256r1.push(executors::Secp256r1::slope_denominator(true, &i.p1, &i.p1))
                }
                ArithEqInput::Bn254CurveAdd(i) => {
                    self.bn254.push(executors::Bn254Curve::slope_denominator(false, &i.p1, &i.p2))
                }
                ArithEqInput::Bn254CurveDbl(i) => {
                    self.bn254.push(executors::Bn254Curve::slope_denominator(true, &i.p1, &i.p1))
                }
                // Arith256, Arith256Mod and the three Fp2 operations never divide.
                _ => {}
            }
        }

        invert_run(&mut self.secp256k1);
        invert_run(&mut self.secp256r1);
        invert_run(&mut self.bn254);
    }

    /// The next secp256k1 inverse in the run. Panics if the fill has fallen out of step with the
    /// collection, which is the only way the index can run past the end.
    #[inline(always)]
    pub(crate) fn next_secp256k1(&mut self) -> Secp256k1Field {
        let v = self.secp256k1[self.next[0]];
        self.next[0] += 1;
        v
    }

    /// The next secp256r1 inverse in the run.
    #[inline(always)]
    pub(crate) fn next_secp256r1(&mut self) -> Secp256r1Field {
        let v = self.secp256r1[self.next[1]];
        self.next[1] += 1;
        v
    }

    /// The next BN254 curve inverse in the run.
    #[inline(always)]
    pub(crate) fn next_bn254(&mut self) -> Bn254Field {
        let v = self.bn254[self.next[2]];
        self.next[2] += 1;
        v
    }
}

#[cfg(test)]
impl SlopeInverses {
    /// Inverses collected but never handed out, per curve. Zero everywhere means the fill consumed
    /// the run exactly.
    fn unconsumed(&self) -> [usize; 3] {
        [
            self.secp256k1.len() - self.next[0],
            self.secp256r1.len() - self.next[1],
            self.bn254.len() - self.next[2],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Bn254ComplexAddInput, Bn254CurveAddInput, Bn254CurveDblInput, Secp256k1AddInput,
        Secp256k1DblInput, Secp256r1AddInput, Secp256r1DblInput,
    };

    /// A point from two small coordinates. Small keeps every limb pattern below all three moduli,
    /// which `Fq::from(BigInt)` requires; the denominators only have to be distinct and non-zero.
    fn pt(x: u64, y: u64) -> [u64; 8] {
        [x, 0, 0, 0, y, 0, 0, 0]
    }

    /// A sequence interleaving all three curves with operations that never divide, so the runs have
    /// to advance independently of each other and of the operation index.
    fn mixed_ops() -> Vec<ArithEqInput> {
        let add_k1 = |x1, x2| {
            ArithEqInput::Secp256k1Add(Secp256k1AddInput {
                addr: 0,
                p1_addr: 0,
                p2_addr: 0,
                step: 0,
                p1: pt(x1, 7),
                p2: pt(x2, 9),
            })
        };
        let add_r1 = |x1, x2| {
            ArithEqInput::Secp256r1Add(Secp256r1AddInput {
                addr: 0,
                p1_addr: 0,
                p2_addr: 0,
                step: 0,
                p1: pt(x1, 7),
                p2: pt(x2, 9),
            })
        };
        let add_bn = |x1, x2| {
            ArithEqInput::Bn254CurveAdd(Bn254CurveAddInput {
                addr: 0,
                p1_addr: 0,
                p2_addr: 0,
                step: 0,
                p1: pt(x1, 7),
                p2: pt(x2, 9),
            })
        };
        vec![
            add_k1(1, 2),
            ArithEqInput::Bn254ComplexAdd(Bn254ComplexAddInput {
                addr: 0,
                f1_addr: 0,
                f2_addr: 0,
                step: 0,
                f1: pt(1, 1),
                f2: pt(2, 2),
            }),
            add_bn(3, 10),
            ArithEqInput::Secp256k1Dbl(Secp256k1DblInput { addr: 0, step: 0, p1: pt(5, 11) }),
            add_r1(4, 30),
            add_k1(6, 40),
            ArithEqInput::Bn254CurveDbl(Bn254CurveDblInput { addr: 0, step: 0, p1: pt(8, 13) }),
            ArithEqInput::Secp256r1Dbl(Secp256r1DblInput { addr: 0, step: 0, p1: pt(9, 17) }),
            add_bn(12, 50),
            add_k1(14, 60),
        ]
    }

    /// The invariant the scheme rests on: the fill's `i`-th operation of a curve takes the `i`-th
    /// inverse of that curve's run. Walk the sequence exactly as the fill does and check every
    /// operation receives the inverse of *its own* denominator -- not a neighbour's.
    #[test]
    fn each_operation_takes_the_inverse_of_its_own_denominator() {
        let ops = mixed_ops();
        let mut inverses = SlopeInverses::default();
        inverses.reload(ops.iter());

        for (i, op) in ops.iter().enumerate() {
            let (got, want) = match op {
                ArithEqInput::Secp256k1Add(o) => (
                    inverses.next_secp256k1(),
                    executors::Secp256k1::slope_denominator(false, &o.p1, &o.p2),
                ),
                ArithEqInput::Secp256k1Dbl(o) => (
                    inverses.next_secp256k1(),
                    executors::Secp256k1::slope_denominator(true, &o.p1, &o.p1),
                ),
                ArithEqInput::Secp256r1Add(o) => {
                    let want = executors::Secp256r1::slope_denominator(false, &o.p1, &o.p2);
                    assert_eq!(inverses.next_secp256r1(), want.inverse().unwrap(), "op #{i}");
                    continue;
                }
                ArithEqInput::Secp256r1Dbl(o) => {
                    let want = executors::Secp256r1::slope_denominator(true, &o.p1, &o.p1);
                    assert_eq!(inverses.next_secp256r1(), want.inverse().unwrap(), "op #{i}");
                    continue;
                }
                ArithEqInput::Bn254CurveAdd(o) => {
                    let want = executors::Bn254Curve::slope_denominator(false, &o.p1, &o.p2);
                    assert_eq!(inverses.next_bn254(), want.inverse().unwrap(), "op #{i}");
                    continue;
                }
                ArithEqInput::Bn254CurveDbl(o) => {
                    let want = executors::Bn254Curve::slope_denominator(true, &o.p1, &o.p1);
                    assert_eq!(inverses.next_bn254(), want.inverse().unwrap(), "op #{i}");
                    continue;
                }
                // never divides, and must not consume an inverse
                _ => continue,
            };
            assert_eq!(got, want.inverse().unwrap(), "op #{i}");
        }
        assert_eq!(inverses.unconsumed(), [0; 3], "the fill must consume every collected inverse");
    }

    /// Reloading must leave nothing of the previous run behind: the buffers are reused across a
    /// batch's runs, so a stale tail would be handed out as a later operation's inverse.
    #[test]
    fn reload_starts_the_run_from_scratch() {
        let ops = mixed_ops();
        let mut inverses = SlopeInverses::default();
        inverses.reload(ops.iter());
        let first = inverses.next_secp256k1();

        inverses.reload(ops.iter());
        assert_eq!(inverses.next_secp256k1(), first, "a reload restarts at the run's first op");
        assert_eq!(inverses.unconsumed(), [3, 2, 3], "4 secp256k1 ops, one consumed");
    }

    /// `batch_inversion` leaves zeros as zeros, where the `/` this replaces panicked. An add whose
    /// two x-coordinates match must not slip through into a silently wrong witness.
    #[test]
    #[should_panic(expected = "slope denominator is zero")]
    fn a_zero_denominator_is_refused_rather_than_passed_through() {
        let ops = vec![ArithEqInput::Secp256k1Add(Secp256k1AddInput {
            addr: 0,
            p1_addr: 0,
            p2_addr: 0,
            step: 0,
            p1: pt(3, 7),
            p2: pt(3, 9),
        })];
        SlopeInverses::default().reload(ops.iter());
    }
}
