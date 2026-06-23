//! Per-AIR unit test for ArithEq384 — the 384-bit sibling of ArithEq (see
//! arith_eq.rs). Same shape, six 64-bit limbs.

use zisk_prover_backend::{
    inputs::{Arith384ModInput, ArithEq384Input},
    testing::with_prover,
    ArithEq384Sm,
};

/// A valid `Arith384Mod` operation: 3 * 5 mod 5 == 0, so `c == 0`.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith384_mod_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<ArithEq384Sm>(ArithEq384Input::Arith384Mod(Arith384ModInput {
                a: [3, 0, 0, 0, 0, 0],
                b: [5, 0, 0, 0, 0, 0],
                c: [0, 0, 0, 0, 0, 0],
                module: [5, 0, 0, 0, 0, 0],
                ..Default::default()
            }))
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Arith384Mod input should satisfy all constraints");
    });
}
