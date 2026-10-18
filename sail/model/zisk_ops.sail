/* ZisK operations: (c, flag) = op(a, b).
 *
 * One `union clause` + one `function clause` per opcode. Mirrors the
 * table in core/src/zisk_ops.rs and the semantics in core/src/ops_core.rs.
 * The ones here are those a run of ziskasm/examples/doubler-min executes,
 * BIOS and launcher included.
 */

scattered union zisk_op
scattered function zisk_exec

val zisk_exec : (zisk_op, bits(64), bits(64)) -> (bits(64), bool)

/* flag: c = 0 and flag set. `call` uses it to take its jump. */
union clause zisk_op = ZOP_FLAG : unit
function clause zisk_exec (ZOP_FLAG(), a, b) = (zeros(), true)

/* copyb: c = b. Used for loads, stores and register moves. */
union clause zisk_op = ZOP_COPYB : unit
function clause zisk_exec (ZOP_COPYB(), a, b) = (b, false)

/* add: wrapping 64-bit addition. */
union clause zisk_op = ZOP_ADD : unit
function clause zisk_exec (ZOP_ADD(), a, b) = (a + b, false)

/* and: bitwise. `ret` uses it to clear bit 0 of the return address. */
union clause zisk_op = ZOP_AND : unit
function clause zisk_exec (ZOP_AND(), a, b) = (a & b, false)

/* eq: c = 1 and flag set when a == b. */
union clause zisk_op = ZOP_EQ : unit
function clause zisk_exec (ZOP_EQ(), a, b) =
  if a == b then (zero_extend(0b1, 64), true) else (zeros(), false)

/* ltu: unsigned less-than. Whole of RISC-V BLTU and BGEU. */
union clause zisk_op = ZOP_LTU : unit
function clause zisk_exec (ZOP_LTU(), a, b) =
  if a <_u b then (zero_extend(0b1, 64), true) else (zeros(), false)

/* pubout: c = b. The BIOS uses it to publish an output word; the
 * emulator gives it no side effect beyond c, and neither does the model. */
union clause zisk_op = ZOP_PUBOUT : unit
function clause zisk_exec (ZOP_PUBOUT(), a, b) = (b, false)

/* halt: c = 0. The program ends in error; the instruction carries is_end,
 * and zisk_execute records the error and its code, since an op here is a
 * pure function of a and b (Emu: opc_halt). */
union clause zisk_op = ZOP_HALT : unit
function clause zisk_exec (ZOP_HALT(), a, b) = (zeros(), false)

end zisk_op
end zisk_exec
