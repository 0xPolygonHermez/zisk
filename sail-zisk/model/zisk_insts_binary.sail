/* Binary operations: comparisons, additions, subtractions and logic
 * (OpType::Binary). Mirrors core/src/ops_core.rs.
 *
 * The `_w` variants work on the low 32 bits of a and b and sign-extend a
 * 32-bit result to 64 bits, as RISC-V's W instructions do.
 */

/* A comparison's result: c = 1 and flag set when it holds, else c = 0. */
val bool_c : bool -> (bits(64), bool)
function bool_c(b) = if b then (zero_extend(0b1, 64), true) else (zeros(), false)

/* The low 32 bits, sign-extended to 64. */
val sx32 : bits(64) -> bits(64)
function sx32(x) = sign_extend(x[31 .. 0], 64)

/* --- 64-bit comparisons --------------------------------------------- */

/* ltu: unsigned less-than. Whole of RISC-V BLTU and BGEU. */
union clause zisk_op = ZOP_LTU : unit
function clause zisk_op_of_code (0x06) = Some(ZOP_LTU())
function clause zisk_exec (ZOP_LTU(), a, b) = bool_c(a <_u b)

/* lt: signed less-than. */
union clause zisk_op = ZOP_LT : unit
function clause zisk_op_of_code (0x07) = Some(ZOP_LT())
function clause zisk_exec (ZOP_LT(), a, b) = bool_c(a <_s b)

/* leu: unsigned less-than-or-equal. */
union clause zisk_op = ZOP_LEU : unit
function clause zisk_op_of_code (0x0c) = Some(ZOP_LEU())
function clause zisk_exec (ZOP_LEU(), a, b) = bool_c(a <=_u b)

/* le: signed less-than-or-equal. */
union clause zisk_op = ZOP_LE : unit
function clause zisk_op_of_code (0x0d) = Some(ZOP_LE())
function clause zisk_exec (ZOP_LE(), a, b) = bool_c(a <=_s b)

/* eq: c = 1 and flag set when a == b. */
union clause zisk_op = ZOP_EQ : unit
function clause zisk_op_of_code (0x09) = Some(ZOP_EQ())
function clause zisk_exec (ZOP_EQ(), a, b) = bool_c(a == b)

/* minu, min, maxu, max: the lesser or greater of a and b; flag clear. */
union clause zisk_op = ZOP_MINU : unit
function clause zisk_op_of_code (0x02) = Some(ZOP_MINU())
function clause zisk_exec (ZOP_MINU(), a, b) = (if a <_u b then a else b, false)

union clause zisk_op = ZOP_MIN : unit
function clause zisk_op_of_code (0x03) = Some(ZOP_MIN())
function clause zisk_exec (ZOP_MIN(), a, b) = (if a <_s b then a else b, false)

union clause zisk_op = ZOP_MAXU : unit
function clause zisk_op_of_code (0x04) = Some(ZOP_MAXU())
function clause zisk_exec (ZOP_MAXU(), a, b) = (if b <_u a then a else b, false)

union clause zisk_op = ZOP_MAX : unit
function clause zisk_op_of_code (0x05) = Some(ZOP_MAX())
function clause zisk_exec (ZOP_MAX(), a, b) = (if b <_s a then a else b, false)

/* --- 32-bit comparisons --------------------------------------------- */

union clause zisk_op = ZOP_LTU_W : unit
function clause zisk_op_of_code (0x16) = Some(ZOP_LTU_W())
function clause zisk_exec (ZOP_LTU_W(), a, b) = bool_c(a[31 .. 0] <_u b[31 .. 0])

union clause zisk_op = ZOP_LT_W : unit
function clause zisk_op_of_code (0x17) = Some(ZOP_LT_W())
function clause zisk_exec (ZOP_LT_W(), a, b) = bool_c(a[31 .. 0] <_s b[31 .. 0])

union clause zisk_op = ZOP_LEU_W : unit
function clause zisk_op_of_code (0x1c) = Some(ZOP_LEU_W())
function clause zisk_exec (ZOP_LEU_W(), a, b) = bool_c(a[31 .. 0] <=_u b[31 .. 0])

union clause zisk_op = ZOP_LE_W : unit
function clause zisk_op_of_code (0x1d) = Some(ZOP_LE_W())
function clause zisk_exec (ZOP_LE_W(), a, b) = bool_c(a[31 .. 0] <=_s b[31 .. 0])

union clause zisk_op = ZOP_EQ_W : unit
function clause zisk_op_of_code (0x19) = Some(ZOP_EQ_W())
function clause zisk_exec (ZOP_EQ_W(), a, b) = bool_c(a[31 .. 0] == b[31 .. 0])

/* minu_w, min_w, maxu_w, max_w: the lesser or greater of the low 32 bits,
 * sign-extended. */
union clause zisk_op = ZOP_MINU_W : unit
function clause zisk_op_of_code (0x12) = Some(ZOP_MINU_W())
function clause zisk_exec (ZOP_MINU_W(), a, b) =
  (if a[31 .. 0] <_u b[31 .. 0] then sx32(a) else sx32(b), false)

union clause zisk_op = ZOP_MIN_W : unit
function clause zisk_op_of_code (0x13) = Some(ZOP_MIN_W())
function clause zisk_exec (ZOP_MIN_W(), a, b) =
  (if a[31 .. 0] <_s b[31 .. 0] then sx32(a) else sx32(b), false)

union clause zisk_op = ZOP_MAXU_W : unit
function clause zisk_op_of_code (0x14) = Some(ZOP_MAXU_W())
function clause zisk_exec (ZOP_MAXU_W(), a, b) =
  (if b[31 .. 0] <_u a[31 .. 0] then sx32(a) else sx32(b), false)

union clause zisk_op = ZOP_MAX_W : unit
function clause zisk_op_of_code (0x15) = Some(ZOP_MAX_W())
function clause zisk_exec (ZOP_MAX_W(), a, b) =
  (if b[31 .. 0] <_s a[31 .. 0] then sx32(a) else sx32(b), false)

/* --- Additions and subtractions --------------------------------------- */

/* add: wrapping 64-bit addition. */
union clause zisk_op = ZOP_ADD : unit
function clause zisk_op_of_code (0x0a) = Some(ZOP_ADD())
function clause zisk_exec (ZOP_ADD(), a, b) = (a + b, false)

/* sub: wrapping 64-bit subtraction. */
union clause zisk_op = ZOP_SUB : unit
function clause zisk_op_of_code (0x0b) = Some(ZOP_SUB())
function clause zisk_exec (ZOP_SUB(), a, b) = (a - b, false)

/* add_w, sub_w: on the low 32 bits, the result sign-extended. */
union clause zisk_op = ZOP_ADD_W : unit
function clause zisk_op_of_code (0x1a) = Some(ZOP_ADD_W())
function clause zisk_exec (ZOP_ADD_W(), a, b) = (sx32(a + b), false)

union clause zisk_op = ZOP_SUB_W : unit
function clause zisk_op_of_code (0x1b) = Some(ZOP_SUB_W())
function clause zisk_exec (ZOP_SUB_W(), a, b) = (sx32(a - b), false)

/* sh1add, sh2add, sh3add: b + (a << 1, 2, 3), as RISC-V Zba's. */
union clause zisk_op = ZOP_SH1ADD : unit
function clause zisk_op_of_code (0x56) = Some(ZOP_SH1ADD())
function clause zisk_exec (ZOP_SH1ADD(), a, b) = (b + (a << 1), false)

union clause zisk_op = ZOP_SH2ADD : unit
function clause zisk_op_of_code (0x57) = Some(ZOP_SH2ADD())
function clause zisk_exec (ZOP_SH2ADD(), a, b) = (b + (a << 2), false)

union clause zisk_op = ZOP_SH3ADD : unit
function clause zisk_op_of_code (0x58) = Some(ZOP_SH3ADD())
function clause zisk_exec (ZOP_SH3ADD(), a, b) = (b + (a << 3), false)

/* --- Logic ------------------------------------------------------------ */

/* and: bitwise. `ret` uses it to clear bit 0 of the return address. */
union clause zisk_op = ZOP_AND : unit
function clause zisk_op_of_code (0x0e) = Some(ZOP_AND())
function clause zisk_exec (ZOP_AND(), a, b) = (a & b, false)

union clause zisk_op = ZOP_OR : unit
function clause zisk_op_of_code (0x0f) = Some(ZOP_OR())
function clause zisk_exec (ZOP_OR(), a, b) = (a | b, false)

union clause zisk_op = ZOP_XOR : unit
function clause zisk_op_of_code (0x10) = Some(ZOP_XOR())
function clause zisk_exec (ZOP_XOR(), a, b) = (a ^ b, false)

/* andn, orn, xnor: with b, or the result, inverted, as RISC-V Zbb's. */
union clause zisk_op = ZOP_ANDN : unit
function clause zisk_op_of_code (0x53) = Some(ZOP_ANDN())
function clause zisk_exec (ZOP_ANDN(), a, b) = (a & ~(b), false)

union clause zisk_op = ZOP_ORN : unit
function clause zisk_op_of_code (0x54) = Some(ZOP_ORN())
function clause zisk_exec (ZOP_ORN(), a, b) = (a | ~(b), false)

union clause zisk_op = ZOP_XNOR : unit
function clause zisk_op_of_code (0x55) = Some(ZOP_XNOR())
function clause zisk_exec (ZOP_XNOR(), a, b) = (~(a ^ b), false)

/* brev8: b with the bits of each byte reversed, as RISC-V Zbkb's. */
union clause zisk_op = ZOP_BREV8 : unit
function clause zisk_op_of_code (0x52) = Some(ZOP_BREV8())
function clause zisk_exec (ZOP_BREV8(), _, b) = {
  var r : bits(64) = zeros();
  foreach (k from 0 to 7) {
    foreach (j from 0 to 7) {
      r[8 * k + j] = b[8 * k + 7 - j]
    }
  };
  (r, false)
}

/* unal8: c = 1 and flag set when a or b is not a multiple of 8. It guards
 * the fast path of routines that need 8-byte aligned addresses and counts. */
union clause zisk_op = ZOP_UNAL8 : unit
function clause zisk_op_of_code (0x5a) = Some(ZOP_UNAL8())
function clause zisk_exec (ZOP_UNAL8(), a, b) = bool_c((a | b)[2 .. 0] != 0b000)
