/* Arith operations: multiplication, division and remainder
 * (OpType::ArithAm32 and ArithA32). Mirrors core/src/ops_core.rs; they match
 * RISC-V's M extension, but for the flag, which division and remainder set
 * when dividing by zero.
 *
 * Products are taken as integers and cut to 128 bits; quotients and
 * remainders round towards zero, as Rust's `/` and `%`. The `_w` variants
 * work on the low 32 bits and sign-extend a 32-bit result.
 */

/* The 128-bit product of x and y, as integers. */
val prod128 : (int, int) -> bits(128)
function prod128(x, y) = to_bits(128, x * y)

/* --- Multiplication ----------------------------------------------------- */

/* mul, mulu: the low 64 bits of the product, signed or not alike. */
union clause zisk_op = ZOP_MUL : unit
function clause zisk_exec (ZOP_MUL(), a, b) = (prod128(signed(a), signed(b))[63 .. 0], false)

union clause zisk_op = ZOP_MULU : unit
function clause zisk_exec (ZOP_MULU(), a, b) = (prod128(unsigned(a), unsigned(b))[63 .. 0], false)

/* mulh, mulsuh, muluh: the high 64 bits, of signed x signed, signed x
 * unsigned, unsigned x unsigned. */
union clause zisk_op = ZOP_MULH : unit
function clause zisk_exec (ZOP_MULH(), a, b) = (prod128(signed(a), signed(b))[127 .. 64], false)

union clause zisk_op = ZOP_MULSUH : unit
function clause zisk_exec (ZOP_MULSUH(), a, b) = (prod128(signed(a), unsigned(b))[127 .. 64], false)

union clause zisk_op = ZOP_MULUH : unit
function clause zisk_exec (ZOP_MULUH(), a, b) = (prod128(unsigned(a), unsigned(b))[127 .. 64], false)

/* mul_w: the low 32 bits of the product, sign-extended. */
union clause zisk_op = ZOP_MUL_W : unit
function clause zisk_exec (ZOP_MUL_W(), a, b) =
  (sign_extend(prod128(signed(a[31 .. 0]), signed(b[31 .. 0]))[31 .. 0], 64), false)

/* --- Division and remainder --------------------------------------------- *
 * Dividing by zero sets the flag: a quotient of all ones, a remainder of a.
 * The signed overflow, MIN / -1, gives MIN and a remainder of 0, which the
 * truncation to the width yields by itself.
 */

union clause zisk_op = ZOP_DIVU : unit
function clause zisk_exec (ZOP_DIVU(), a, b) =
  if b == zeros() then (sail_ones(64), true)
  else (to_bits(64, tdiv_int(unsigned(a), unsigned(b))), false)

union clause zisk_op = ZOP_DIV : unit
function clause zisk_exec (ZOP_DIV(), a, b) =
  if b == zeros() then (sail_ones(64), true)
  else (to_bits(64, tdiv_int(signed(a), signed(b))), false)

union clause zisk_op = ZOP_REMU : unit
function clause zisk_exec (ZOP_REMU(), a, b) =
  if b == zeros() then (a, true)
  else (to_bits(64, tmod_int(unsigned(a), unsigned(b))), false)

union clause zisk_op = ZOP_REM : unit
function clause zisk_exec (ZOP_REM(), a, b) =
  if b == zeros() then (a, true)
  else (to_bits(64, tmod_int(signed(a), signed(b))), false)

union clause zisk_op = ZOP_DIVU_W : unit
function clause zisk_exec (ZOP_DIVU_W(), a, b) =
  if b[31 .. 0] == zeros() then (sail_ones(64), true)
  else (sign_extend(to_bits(32, tdiv_int(unsigned(a[31 .. 0]), unsigned(b[31 .. 0]))), 64), false)

union clause zisk_op = ZOP_DIV_W : unit
function clause zisk_exec (ZOP_DIV_W(), a, b) =
  if b[31 .. 0] == zeros() then (sail_ones(64), true)
  else (sign_extend(to_bits(32, tdiv_int(signed(a[31 .. 0]), signed(b[31 .. 0]))), 64), false)

union clause zisk_op = ZOP_REMU_W : unit
function clause zisk_exec (ZOP_REMU_W(), a, b) =
  if b[31 .. 0] == zeros() then (sx32(a), true)
  else (sign_extend(to_bits(32, tmod_int(unsigned(a[31 .. 0]), unsigned(b[31 .. 0]))), 64), false)

union clause zisk_op = ZOP_REM_W : unit
function clause zisk_exec (ZOP_REM_W(), a, b) =
  if b[31 .. 0] == zeros() then (sx32(a), true)
  else (sign_extend(to_bits(32, tmod_int(signed(a[31 .. 0]), signed(b[31 .. 0]))), 64), false)
