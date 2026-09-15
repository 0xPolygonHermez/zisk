/* ZisK operations: (c, flag) = op(a, b).
 *
 * One `union clause` + one `function clause` per opcode. Mirrors
 * core/src/zisk_ops.rs; the four here are those used by the
 * ziskasm doubler example.
 */

scattered union zisk_op
scattered function zisk_exec

val zisk_exec : (zisk_op, bits(64), bits(64)) -> (bits(64), bool)

/* copyb: c = b. Used for loads, stores and register moves. */
union clause zisk_op = ZOP_COPYB : unit
function clause zisk_exec (ZOP_COPYB(), a, b) = (b, false)

/* add: wrapping 64-bit addition. */
union clause zisk_op = ZOP_ADD : unit
function clause zisk_exec (ZOP_ADD(), a, b) = (a + b, false)

/* eq: c = 1 and flag set when a == b. */
union clause zisk_op = ZOP_EQ : unit
function clause zisk_exec (ZOP_EQ(), a, b) =
  if a == b then (zero_extend(0b1, 64), true) else (zeros(), false)

/* ltu: unsigned less-than. Whole of RISC-V BLTU and BGEU. */
union clause zisk_op = ZOP_LTU : unit
function clause zisk_exec (ZOP_LTU(), a, b) =
  if a <_u b then (zero_extend(0b1, 64), true) else (zeros(), false)

end zisk_op
end zisk_exec
