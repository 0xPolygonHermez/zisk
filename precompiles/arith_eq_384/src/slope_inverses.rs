//! Batch inversion of the slope denominators a run of BLS12-381 curve operations needs.
//!
//! The 384-bit counterpart of `zisk_precomp_arith_eq`'s module of the same name, which carries the
//! full rationale. The short version: `s = num / den` is a full modular inversion per operation,
//! and on this field that is ~340 field multiplications and *three quarters* of `execute_add` --
//! the widest field here, so the one with the most to gain. Montgomery's trick inverts a whole run
//! with one inversion plus three multiplications each, measured ~84x cheaper per inversion.
//!
//! Only one queue, because `ArithEq384` has a single curve; `Arith384Mod` and the three Fp2
//! operations never divide. The invariant is the same: the fill walks the operations in exactly
//! the order [`SlopeInverses::reload`] collected them, so the `i`-th curve operation takes the
//! `i`-th inverse, and a fill out of step panics on the indexing instead of silently using another
//! operation's value.

use ark_ff::batch_inversion;
use num_traits::Zero;

use crate::{executors, ArithEq384Input};
pub use executors::Bls12_381Field;

/// Operations per inversion run. Past ~512 the per-inversion cost has flattened, so the rest goes
/// to locality: the denominators stay in cache and the second pass re-reads inputs the first has
/// just touched.
pub(crate) const INV_RUN_OPS: usize = 1024;

/// The inverted slope denominators for one run of operations.
///
/// Reused across a batch's runs ([`Self::reload`]) so the buffer is allocated once and keeps its
/// capacity.
#[derive(Default)]
pub(crate) struct SlopeInverses {
    den: Vec<Bls12_381Field>,
    next: usize,
}

impl SlopeInverses {
    /// Collects and inverts the denominators of `ops`, discarding the previous run's.
    ///
    /// `ops` must be exactly the operations the fill is about to write, in the order it will write
    /// them; operations that do not divide contribute nothing.
    pub(crate) fn reload<'a>(&mut self, ops: impl Iterator<Item = &'a ArithEq384Input>) {
        self.den.clear();
        self.next = 0;

        for op in ops {
            match op {
                ArithEq384Input::Bls12_381CurveAdd(i) => {
                    self.den.push(executors::Bls12_381Curve::slope_denominator(false, &i.p1, &i.p2))
                }
                ArithEq384Input::Bls12_381CurveDbl(i) => {
                    self.den.push(executors::Bls12_381Curve::slope_denominator(true, &i.p1, &i.p1))
                }
                _ => {}
            }
        }

        // `batch_inversion` *skips* zeros and leaves them zero, where the `/` this replaces panicked
        // on `inverse()` returning `None`. Turning a panic into a silently wrong witness is the
        // worse failure, so rule them out here. A zero denominator means `x1 == x2` on an add (the
        // PIL's `x_are_different` column already forbids it) or `y1 == 0` on a double (no such point
        // on a prime-order curve).
        assert!(!self.den.iter().any(|d| d.is_zero()), "slope denominator is zero");
        batch_inversion(&mut self.den);
    }

    /// The next inverse in the run. Panics if the fill has fallen out of step with the collection,
    /// which is the only way the index can run past the end.
    #[inline(always)]
    pub(crate) fn next_inverse(&mut self) -> Bls12_381Field {
        let v = self.den[self.next];
        self.next += 1;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Bls12_381ComplexAddInput, Bls12_381CurveAddInput, Bls12_381CurveDblInput,
        ARITH_EQ_384_U64S, ARITH_EQ_384_U64S_DOUBLE,
    };
    use ark_ff::Field;

    /// A point from two small coordinates: small keeps every limb pattern below the modulus, which
    /// `Fq::from(BigInt)` requires, and the denominators only need to be distinct and non-zero.
    fn pt(x: u64, y: u64) -> [u64; ARITH_EQ_384_U64S_DOUBLE] {
        let mut p = [0u64; ARITH_EQ_384_U64S_DOUBLE];
        p[0] = x;
        p[ARITH_EQ_384_U64S] = y;
        p
    }

    fn mixed_ops() -> Vec<ArithEq384Input> {
        vec![
            ArithEq384Input::Bls12_381CurveAdd(Bls12_381CurveAddInput {
                addr: 0,
                p1_addr: 0,
                p2_addr: 0,
                step: 0,
                p1: pt(1, 7),
                p2: pt(2, 9),
            }),
            // never divides, and must not consume an inverse
            ArithEq384Input::Bls12_381ComplexAdd(Bls12_381ComplexAddInput {
                addr: 0,
                f1_addr: 0,
                f2_addr: 0,
                step: 0,
                f1: pt(1, 1),
                f2: pt(2, 2),
            }),
            ArithEq384Input::Bls12_381CurveDbl(Bls12_381CurveDblInput {
                addr: 0,
                step: 0,
                p1: pt(5, 11),
            }),
            ArithEq384Input::Bls12_381CurveAdd(Bls12_381CurveAddInput {
                addr: 0,
                p1_addr: 0,
                p2_addr: 0,
                step: 0,
                p1: pt(6, 7),
                p2: pt(40, 9),
            }),
        ]
    }

    /// The invariant the scheme rests on: the fill's `i`-th curve operation takes the `i`-th
    /// inverse. Walk the sequence exactly as the fill does and check each operation receives the
    /// inverse of *its own* denominator.
    #[test]
    fn each_operation_takes_the_inverse_of_its_own_denominator() {
        let ops = mixed_ops();
        let mut inverses = SlopeInverses::default();
        inverses.reload(ops.iter());

        for (i, op) in ops.iter().enumerate() {
            let want = match op {
                ArithEq384Input::Bls12_381CurveAdd(o) => {
                    executors::Bls12_381Curve::slope_denominator(false, &o.p1, &o.p2)
                }
                ArithEq384Input::Bls12_381CurveDbl(o) => {
                    executors::Bls12_381Curve::slope_denominator(true, &o.p1, &o.p1)
                }
                _ => continue,
            };
            assert_eq!(inverses.next_inverse(), want.inverse().unwrap(), "op #{i}");
        }
        assert_eq!(inverses.den.len(), inverses.next, "the fill must consume every inverse");
    }

    /// Reloading must leave nothing of the previous run behind: the buffer is reused across a
    /// batch's runs, so a stale tail would be handed out as a later operation's inverse.
    #[test]
    fn reload_starts_the_run_from_scratch() {
        let ops = mixed_ops();
        let mut inverses = SlopeInverses::default();
        inverses.reload(ops.iter());
        let first = inverses.next_inverse();

        inverses.reload(ops.iter());
        assert_eq!(inverses.next_inverse(), first, "a reload restarts at the run's first op");
        assert_eq!(inverses.den.len(), 3, "3 of the 4 operations divide");
        assert_eq!(inverses.next, 1);
    }

    /// `batch_inversion` leaves zeros as zeros, where the `/` this replaces panicked. An add whose
    /// two x-coordinates match must not slip through into a silently wrong witness.
    #[test]
    #[should_panic(expected = "slope denominator is zero")]
    fn a_zero_denominator_is_refused_rather_than_passed_through() {
        let ops = vec![ArithEq384Input::Bls12_381CurveAdd(Bls12_381CurveAddInput {
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
