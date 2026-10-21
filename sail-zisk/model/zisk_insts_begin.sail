/* ZisK operations: (c, flag) = op(a, b).
 *
 * One `union clause` + one `function clause` per opcode, in a file per
 * family, as sail-riscv spreads its instructions over riscv_insts_*.sail.
 * Mirrors the table in core/src/zisk_ops.rs and the semantics in
 * core/src/ops_core.rs and core/src/operations/.
 */

scattered union zisk_op
scattered function zisk_exec

val zisk_exec : (zisk_op, bits(64), bits(64)) -> (bits(64), bool)
