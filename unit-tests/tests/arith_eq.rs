//! Template / example: per-AIR unit tests for ArithEq, exercising the three
//! ways the unit-test framework can drive constraint verification:
//!
//! 1. honest typed input              → constraints hold     (Ok)
//! 2. honest input + a tampering hook → constraint violated  (Err)
//! 3. raw trace override (bypasses `compute_witness`)         (Err here,
//!    because the hand-authored trace is deliberately invalid)
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
/// ourselves. Here we hand back the freshly zeroed trace — a deliberately
/// invalid ArithEq witness — so verification must fail. This proves the
/// override's trace is what actually reaches the constraint check.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_override_authors_trace_directly() {
    with_prover(|prover| {
        let result = prover
            .verify_input()
            .input::<ArithEqSm>(honest_input())
            .trace_override::<ArithEqSm, ArithEqTrace<ArithEqTraceRow<Goldilocks>>>(
                |trace, _inputs| {
                    let row = &mut trace.buffer[0];
                    row.set_step_addr(1);
                    row.set_x1(7);
                    row.set_y1(11);
                    row.set_q0(123);
                    Ok(())
                },
            )
            .run();

        assert!(
            result.is_err(),
            "an all-zero (override-authored) trace must violate ArithEq constraints, \
             but verification passed — the override did not reach the constraint check"
        );
    });
}
