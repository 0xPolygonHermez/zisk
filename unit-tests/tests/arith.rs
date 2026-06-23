//! Per-AIR unit test for the Arith state machine (mul/div/rem family). Its
//! input is the raw operation bus payload `[op, op_type, a, b]`; the SM
//! computes the result itself.

use zisk_core::zisk_ops::ZiskOp;
use zisk_prover_backend::{inputs::OperationData, testing::with_prover, ArithSm};

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn arith_honest_input_verifies() {
    with_prover(|prover| {
        let mul: OperationData<u64> = [ZiskOp::Mul.code() as u64, 0, 5, 7];
        let divu: OperationData<u64> = [ZiskOp::Divu.code() as u64, 0, 42, 6];

        let result = prover
            .input::<ArithSm>(mul)
            .input::<ArithSm>(divu)
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Arith inputs should satisfy all constraints");
    });
}
