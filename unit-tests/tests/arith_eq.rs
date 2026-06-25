//! Template / example: per-AIR unit tests for ArithEq, covering the three
//! entry points — honest `input()`, `input()` + tampering `hook()`, and raw
//! trace authoring via `trace()`.
//!
//! `run()` returns a `ConstraintsVerificationResult`: constraint violations
//! are data (`result.valid`, per-instance `failed_constraints`), not an
//! `Err`. `Err` means the run itself broke (setup, planning, witness).
//!
//! Use `cargo-zisk-dev get-constraints --air <Name>` to list an AIR's
//! constraint ids/lines when pinning asserts. All tests are `#[ignore]`
//! because they need a proving key at `~/.zisk/provingKey`.

use zisk_prover_backend::{
    inputs::{Arith256ModInput, ArithEqInput},
    testing::with_prover,
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
        let result =
            prover.input::<ArithEqSm>(honest_input()).run().expect("verification run failed");

        assert!(result.valid, "honest Arith256Mod input should satisfy all constraints");
    });
}

/// `compute_witness` builds the correct trace, then a hook flips one column
/// to a wrong value — the constraints must catch it, and the typed result
/// must report the failure against the ArithEq AIR.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_hook_injection_is_caught() {
    with_prover(|prover| {
        let result = prover
            .input::<ArithEqSm>(honest_input())
            .hook::<ArithEqSm>(|input_idx, clock, row| {
                if input_idx == 0 && clock == 0 {
                    row.set_q0(4); // honest = 3
                }
            })
            .run()
            .expect("verification run failed");

        assert!(
            !result.valid,
            "expected constraint violation from injected q0[0], but verification passed"
        );

        let failing: Vec<_> = result.instances.iter().filter(|i| !i.valid()).collect();
        assert!(
            failing.iter().any(|i| i.air_name.contains("ArithEq")),
            "failure should be reported against the ArithEq AIR, got: {:?}",
            failing.iter().map(|i| i.air_name.as_str()).collect::<Vec<_>>()
        );

        let failed: Vec<_> = failing.iter().flat_map(|i| i.failed_constraints.iter()).collect();
        assert!(
            failed.iter().any(|f| f.line.contains("eq[0][0]") && f.rows.iter().any(|r| r.row == 0)),
            "expected the eq/carry chain constraint to fail at row 0, got: {:?}",
            failed.iter().map(|f| (f.constraint_id, &f.line)).collect::<Vec<_>>()
        );
    });
}

/// Multiple instances of one AIR: `traces(2, …)` plans two instances and
/// invokes the author once per instance with its index. Instance 0 stays
/// all-zero (a valid "unused" instance); instance 1 breaks the sel_op latch
/// — so the failure must be reported against instance 1 only.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_two_instances_report_per_instance() {
    with_prover(|prover| {
        let result = prover
            .traces::<ArithEqSm>(2, |instance_idx, trace, _std| {
                if instance_idx == 1 {
                    trace[1].set_sel_op(1, true);
                }
                Ok(())
            })
            .run()
            .expect("verification run failed");

        assert!(!result.valid, "instance 1 breaks the latch, so the run must be invalid");
        let failing: Vec<_> = result.instances.iter().filter(|i| !i.valid()).collect();
        assert!(
            !failing.is_empty() && failing.iter().all(|i| i.air_instance_id == 1),
            "only instance 1 should fail, got: {:?}",
            failing.iter().map(|i| (i.air_name.as_str(), i.air_instance_id)).collect::<Vec<_>>()
        );
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith256_mod_override_authors_trace_directly() {
    with_prover(|prover| {
        let result = prover
            .trace::<ArithEqSm>(|trace, _std| {
                // Arith256Mod selector on at row 1 only — breaks the latch.
                trace[1].set_sel_op(1, true);
                Ok(())
            })
            .run()
            .expect("verification run failed");

        assert!(
            !result.valid,
            "an authored trace that breaks the sel_op latch must violate \
             ArithEq constraints, but verification passed — the authored trace did \
             not reach the constraint check"
        );

        let failed: Vec<_> = result
            .instances
            .iter()
            .filter(|i| !i.valid())
            .flat_map(|i| i.failed_constraints.iter())
            .collect();
        assert!(
            failed
                .iter()
                .any(|f| f.line.contains("sel_op[1]-'sel_op[1]")
                    && f.rows.iter().any(|r| r.row == 1)),
            "expected the sel_op latch constraint to fail at row 1, got: {:?}",
            failed.iter().map(|f| (f.constraint_id, &f.line)).collect::<Vec<_>>()
        );
    });
}
