use zisk_prover_backend::{
    inputs::{Arith256ModInput, ArithEqInput},
    rows::ArithEqTraceRow,
    testing::with_prover,
    ArithEqSm,
};

#[test]
#[ignore]
fn arith256_mod_hook_injection_is_caught() {
    with_prover(|prover| {
        let honest_input = || {
            ArithEqInput::Arith256Mod(Arith256ModInput {
                a: [3, 0, 0, 0],
                b: [5, 0, 0, 0],
                c: [0, 0, 0, 0],
                module: [5, 0, 0, 0],
                ..Default::default()
            })
        };

        // 2. Hook injection.
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
