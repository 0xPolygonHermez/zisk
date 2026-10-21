/* Binary extension operations: shifts, rotations, sign extensions, bit
 * counts, bit manipulation and carry-less products (OpType::BinaryE).
 * Mirrors core/src/ops_core.rs; most match RISC-V's Zba, Zbb, Zbc, Zbkb,
 * Zbkx, Zbs and Zicond instructions.
 *
 * A shift or rotation takes its amount from the low 6 bits of b, or the low
 * 5 bits for the `_w` variants, which work on the low 32 bits of a and
 * sign-extend a 32-bit result.
 */

/* x >> s, arithmetically: through a 128-bit sign extension, since Sail's
 * sail_arith_shiftright is a rotation in the Lean backend. */
val sra_bits : (bits(64), int) -> bits(64)
function sra_bits(x, s) = (sign_extend(x, 128) >> s)[63 .. 0]

/* x rotated left by s, s in 0..'n-1. */
val rotl : forall 'n, 'n > 0. (bits('n), int) -> bits('n)
function rotl(x, s) = (x << s) | (x >> (length(x) - s))

/* --- Shifts ----------------------------------------------------------- */

union clause zisk_op = ZOP_SLL : unit
function clause zisk_exec (ZOP_SLL(), a, b) = (a << unsigned(b[5 .. 0]), false)

union clause zisk_op = ZOP_SRL : unit
function clause zisk_exec (ZOP_SRL(), a, b) = (a >> unsigned(b[5 .. 0]), false)

union clause zisk_op = ZOP_SRA : unit
function clause zisk_exec (ZOP_SRA(), a, b) = (sra_bits(a, unsigned(b[5 .. 0])), false)

union clause zisk_op = ZOP_SLL_W : unit
function clause zisk_exec (ZOP_SLL_W(), a, b) =
  (sign_extend(a[31 .. 0] << unsigned(b[4 .. 0]), 64), false)

union clause zisk_op = ZOP_SRL_W : unit
function clause zisk_exec (ZOP_SRL_W(), a, b) =
  (sign_extend(a[31 .. 0] >> unsigned(b[4 .. 0]), 64), false)

union clause zisk_op = ZOP_SRA_W : unit
function clause zisk_exec (ZOP_SRA_W(), a, b) =
  (sra_bits(sign_extend(a[31 .. 0], 64), unsigned(b[4 .. 0])), false)

/* sll_u_w: the low 32 bits of a, zero-extended, shifted left (Zba's slli.uw). */
union clause zisk_op = ZOP_SLL_U_W : unit
function clause zisk_exec (ZOP_SLL_U_W(), a, b) =
  (zero_extend(a[31 .. 0], 64) << unsigned(b[5 .. 0]), false)

/* --- Rotations -------------------------------------------------------- */

union clause zisk_op = ZOP_ROL : unit
function clause zisk_exec (ZOP_ROL(), a, b) = (rotl(a, unsigned(b[5 .. 0])), false)

union clause zisk_op = ZOP_ROR : unit
function clause zisk_exec (ZOP_ROR(), a, b) = {
  let s = unsigned(b[5 .. 0]);
  (rotl(a, if s == 0 then 0 else 64 - s), false)
}

union clause zisk_op = ZOP_ROL_W : unit
function clause zisk_exec (ZOP_ROL_W(), a, b) =
  (sign_extend(rotl(a[31 .. 0], unsigned(b[4 .. 0])), 64), false)

union clause zisk_op = ZOP_ROR_W : unit
function clause zisk_exec (ZOP_ROR_W(), a, b) = {
  let s = unsigned(b[4 .. 0]);
  (sign_extend(rotl(a[31 .. 0], if s == 0 then 0 else 32 - s), 64), false)
}

/* --- Sign extensions and packing --------------------------------------- */

union clause zisk_op = ZOP_SIGNEXTEND_B : unit
function clause zisk_exec (ZOP_SIGNEXTEND_B(), _, b) = (sign_extend(b[7 .. 0], 64), false)

union clause zisk_op = ZOP_SIGNEXTEND_H : unit
function clause zisk_exec (ZOP_SIGNEXTEND_H(), _, b) = (sign_extend(b[15 .. 0], 64), false)

union clause zisk_op = ZOP_SIGNEXTEND_W : unit
function clause zisk_exec (ZOP_SIGNEXTEND_W(), _, b) = (sign_extend(b[31 .. 0], 64), false)

/* pack, pack_h, pack_w: the low halves of a and b side by side (Zbkb). */
union clause zisk_op = ZOP_PACK : unit
function clause zisk_exec (ZOP_PACK(), a, b) = (b[31 .. 0] @ a[31 .. 0], false)

union clause zisk_op = ZOP_PACK_H : unit
function clause zisk_exec (ZOP_PACK_H(), a, b) = (zero_extend(b[7 .. 0] @ a[7 .. 0], 64), false)

union clause zisk_op = ZOP_PACK_W : unit
function clause zisk_exec (ZOP_PACK_W(), a, b) = (sign_extend(b[15 .. 0] @ a[15 .. 0], 64), false)

/* rev8: b with its bytes reversed. */
union clause zisk_op = ZOP_REV8 : unit
function clause zisk_exec (ZOP_REV8(), _, b) = {
  var r : bits(64) = zeros();
  foreach (k from 0 to 7) {
    r[(8 * k + 7) .. (8 * k)] = b[(63 - 8 * k) .. (56 - 8 * k)]
  };
  (r, false)
}

/* orc_b: each byte of b that is not zero becomes 0xff. */
union clause zisk_op = ZOP_ORC_B : unit
function clause zisk_exec (ZOP_ORC_B(), _, b) = {
  var r : bits(64) = zeros();
  foreach (k from 0 to 7) {
    if b[(8 * k + 7) .. (8 * k)] != 0x00 then r[(8 * k + 7) .. (8 * k)] = 0xff
  };
  (r, false)
}

/* --- Bit counts --------------------------------------------------------- */

/* The leading zeros of x. */
val clz_bits : forall 'n, 'n > 0. bits('n) -> int
function clz_bits(x) = {
  var n : int = 0;
  var done : bool = false;
  foreach (i from (length(x) - 1) downto 0) {
    if not_bool(done) then { if x[i] == bitone then done = true else n = n + 1 }
  };
  n
}

/* The trailing zeros of x. */
val ctz_bits : forall 'n, 'n > 0. bits('n) -> int
function ctz_bits(x) = {
  var n : int = 0;
  var done : bool = false;
  foreach (i from 0 to (length(x) - 1)) {
    if not_bool(done) then { if x[i] == bitone then done = true else n = n + 1 }
  };
  n
}

/* The ones of x. */
val cpop_bits : forall 'n, 'n > 0. bits('n) -> int
function cpop_bits(x) = {
  var n : int = 0;
  foreach (i from 0 to (length(x) - 1)) { if x[i] == bitone then n = n + 1 };
  n
}

union clause zisk_op = ZOP_CLZ : unit
function clause zisk_exec (ZOP_CLZ(), _, b) = (to_bits(64, clz_bits(b)), false)

union clause zisk_op = ZOP_CLZ_W : unit
function clause zisk_exec (ZOP_CLZ_W(), _, b) = (to_bits(64, clz_bits(b[31 .. 0])), false)

union clause zisk_op = ZOP_CTZ : unit
function clause zisk_exec (ZOP_CTZ(), _, b) = (to_bits(64, ctz_bits(b)), false)

union clause zisk_op = ZOP_CTZ_W : unit
function clause zisk_exec (ZOP_CTZ_W(), _, b) = (to_bits(64, ctz_bits(b[31 .. 0])), false)

union clause zisk_op = ZOP_CPOP : unit
function clause zisk_exec (ZOP_CPOP(), _, b) = (to_bits(64, cpop_bits(b)), false)

union clause zisk_op = ZOP_CPOP_W : unit
function clause zisk_exec (ZOP_CPOP_W(), _, b) = (to_bits(64, cpop_bits(b[31 .. 0])), false)

/* --- Single bits (Zbs) --------------------------------------------------- */

union clause zisk_op = ZOP_BCLR : unit
function clause zisk_exec (ZOP_BCLR(), a, b) = {
  var r = a;
  r[unsigned(b[5 .. 0])] = bitzero;
  (r, false)
}

union clause zisk_op = ZOP_BSET : unit
function clause zisk_exec (ZOP_BSET(), a, b) = {
  var r = a;
  r[unsigned(b[5 .. 0])] = bitone;
  (r, false)
}

union clause zisk_op = ZOP_BINV : unit
function clause zisk_exec (ZOP_BINV(), a, b) = {
  var r = a;
  let i = unsigned(b[5 .. 0]);
  r[i] = if a[i] == bitone then bitzero else bitone;
  (r, false)
}

union clause zisk_op = ZOP_BEXT : unit
function clause zisk_exec (ZOP_BEXT(), a, b) =
  (if a[unsigned(b[5 .. 0])] == bitone then zero_extend(0b1, 64) else zeros(), false)

/* --- Additions of a zero-extended word (Zba) ---------------------------- */

union clause zisk_op = ZOP_ADD_U_W : unit
function clause zisk_exec (ZOP_ADD_U_W(), a, b) = (b + zero_extend(a[31 .. 0], 64), false)

union clause zisk_op = ZOP_SH1ADD_U_W : unit
function clause zisk_exec (ZOP_SH1ADD_U_W(), a, b) = (b + (zero_extend(a[31 .. 0], 64) << 1), false)

union clause zisk_op = ZOP_SH2ADD_U_W : unit
function clause zisk_exec (ZOP_SH2ADD_U_W(), a, b) = (b + (zero_extend(a[31 .. 0], 64) << 2), false)

union clause zisk_op = ZOP_SH3ADD_U_W : unit
function clause zisk_exec (ZOP_SH3ADD_U_W(), a, b) = (b + (zero_extend(a[31 .. 0], 64) << 3), false)

/* --- Carry-less products (Zbc) ------------------------------------------- */

/* clmul: the low 64 bits of the carry-less product of a and b. */
union clause zisk_op = ZOP_CLMUL : unit
function clause zisk_exec (ZOP_CLMUL(), a, b) = {
  var r : bits(64) = zeros();
  foreach (i from 0 to 63) { if b[i] == bitone then r = r ^ (a << i) };
  (r, false)
}

/* clmul_h: its high 64 bits. */
union clause zisk_op = ZOP_CLMUL_H : unit
function clause zisk_exec (ZOP_CLMUL_H(), a, b) = {
  var r : bits(64) = zeros();
  foreach (i from 1 to 63) { if b[i] == bitone then r = r ^ (a >> (64 - i)) };
  (r, false)
}

/* clmul_r: bits 126..63 of the product. */
union clause zisk_op = ZOP_CLMUL_R : unit
function clause zisk_exec (ZOP_CLMUL_R(), a, b) = {
  var r : bits(64) = zeros();
  foreach (i from 0 to 63) { if b[i] == bitone then r = r ^ (a >> (63 - i)) };
  (r, false)
}

/* --- Crossbar permutations (Zbkx) ---------------------------------------- */

/* xperm4: each nibble of b indexes a nibble of a. */
union clause zisk_op = ZOP_XPERM4 : unit
function clause zisk_exec (ZOP_XPERM4(), a, b) = {
  var r : bits(64) = zeros();
  foreach (i from 0 to 15) {
    let idx = unsigned(b[(4 * i + 3) .. (4 * i)]);
    r[(4 * i + 3) .. (4 * i)] = slice(a, 4 * idx, 4)
  };
  (r, false)
}

/* xperm8: each byte of b indexes a byte of a, or gives 0 past the eighth. */
union clause zisk_op = ZOP_XPERM8 : unit
function clause zisk_exec (ZOP_XPERM8(), a, b) = {
  var r : bits(64) = zeros();
  foreach (i from 0 to 7) {
    let idx = unsigned(b[(8 * i + 7) .. (8 * i)]);
    if idx < 8 then r[(8 * i + 7) .. (8 * i)] = slice(a, 8 * idx, 8)
  };
  (r, false)
}

/* --- Conditional zeroing (Zicond) --------------------------------------- */

union clause zisk_op = ZOP_CZERO_EQZ : unit
function clause zisk_exec (ZOP_CZERO_EQZ(), a, b) = (if b == zeros() then zeros() else a, false)

union clause zisk_op = ZOP_CZERO_NEZ : unit
function clause zisk_exec (ZOP_CZERO_NEZ(), a, b) = (if b != zeros() then zeros() else a, false)
