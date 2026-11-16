use zisk_core::zisk_ops::ZiskOp;
use ziskemu::get_ops_costs;
#[test]
fn fp32_opcode_cost_cannot_report_a_placeholder_zero() {
    let mut counts = [0u64; 256];
    assert_eq!(get_ops_costs(&counts), (0, 0));
    counts[ZiskOp::FLOATDOT as usize] = 1;
    assert!(std::panic::catch_unwind(|| get_ops_costs(&counts)).is_err());
}
