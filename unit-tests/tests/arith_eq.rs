//! Template / example: per-AIR unit tests for ArithEq, exercising the three
//! ways the unit-test framework can drive constraint verification:
//!
//! 1. honest typed input              → constraints hold     (Ok)
//! 2. honest input + a tampering hook → constraint violated  (Err)
//! 3. raw trace override (bypasses `compute_witness`)         (Err here,
//!    because the hand-authored trace trips the sel_op latch)
//!
//! Copy this file's structure to add tests for other SMs. All are `#[ignore]`
//! because they need a real proving key at `~/.zisk/provingKey`; `with_prover`
//! skips silently when it's absent.

use fields::Goldilocks;
use zisk_prover_backend::{
    inputs::{Arith256ModInput, ArithEqInput},
    rows::ArithEqTraceRow,
    testing::with_prover,
    traces::ArithEqTrace,
    ArithEqSm,
};

/// A valid `Arith256Mod` operation: 3 * 5 mod 5 == 0, so `c == 0`.
fn honest_input() -> ArithEqInput {
    ArithEqInput::Arith256Mod(Arith256ModInput {
        a: [3, 0, 0, 0],
        b: [5, 0, 0, 0],
        c: [0, 0, 0, 0],
        module: [5, 0, 0, 0],
        ..Default::default()
    })
}

/// Baseline: a valid input run through the normal `compute_witness` path
/// satisfies every constraint. This is the same check the CLI used to do.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover.verify_input().input::<ArithEqSm>(honest_input()).run();
        assert!(result.is_ok(), "honest Arith256Mod input should verify, got Err");
    });
}

/// Hook feature: `compute_witness` builds the correct trace, then a hook
/// flips one column to a wrong value — the constraints must catch it.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_hook_injection_is_caught() {
    with_prover(|prover| {
        let result = prover
            .verify_input()
            .input::<ArithEqSm>(honest_input())
            .hook::<ArithEqSm>(|input_idx, clock, row: &mut ArithEqTraceRow<_>| {
                if input_idx == 0 && clock == 0 {
                    row.set_q0(4); // honest = 3
                }
            })
            .run();

        assert!(
            result.is_err(),
            "expected constraint violation from injected q0[0], but verification passed"
        );
    });
}

/// Override feature: bypass `compute_witness` entirely and author the trace
/// ourselves, then check that what we wrote is what the constraints see.
///
/// A freshly zeroed ArithEq trace is *not* invalid — with every `sel_op`
/// selector at zero the whole instance is "unused", every selector-gated
/// arithmetic constraint is multiplied by 0, and verification passes. So to
/// prove the authored trace actually reaches the constraint check we must
/// author a trace that *trips* a constraint.
///
/// The cheapest, layout-robust violation is the selector latch
/// (`arith_eq.pil`): `(1 - CLK_0) * (sel_op[i] - 'sel_op[i]) === 0` requires a
/// selector to stay constant across an operation's clock cycle except on its
/// first row (where the fixed column `CLK_0 == 1`). `CLK_0` is `1` on rows
/// 0, 16, 32, ... and `0` elsewhere, so turning `sel_op[1]` on at row 1 only
/// — while row 0 keeps it off — makes the latch evaluate to `1 * (1 - 0) = 1`,
/// a guaranteed failure that needs no carries, ranges, or arithmetic.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_override_authors_trace_directly() {
    with_prover(|prover| {
        let result = prover
            .verify_input()
            .input::<ArithEqSm>(honest_input())
            .trace_override::<ArithEqSm, ArithEqTrace<ArithEqTraceRow<Goldilocks>>>(
                |trace, _inputs| {
                    // Enable the Arith256Mod selector (index 1) on row 1 only.
                    // Row 0 keeps every selector at 0, so the latch constraint
                    // `(1 - CLK_0) * (sel_op[1] - 'sel_op[1]) === 0` is broken
                    // at row 1 (CLK_0 == 0 there).
                    trace.buffer[1].set_sel_op(1, true);
                    Ok(())
                },
            )
            .run();

        assert!(
            result.is_err(),
            "an override-authored trace that breaks the sel_op latch must violate \
             ArithEq constraints, but verification passed — the override did not \
             reach the constraint check"
        );
    });
}