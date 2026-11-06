/* Big-integer precompiles: 256-bit addition, multiply-add, and
 * multiply-add modulo, at 256 and 384 bits.
 *
 * b holds a table of operand addresses, each 8-byte aligned, whose last
 * entries are where the results go (core/src/zisk_ops.rs, opc_add256 and
 * on; zisk_precomp_helpers). The table itself is 8-byte aligned.
 */

/* add256: [x, y, cin, z] at b, cin a value rather than an address:
 * z = x + y + cin, 4 words; c = the carry out. */
union clause zisk_op = ZOP_ADD256 : unit
function clause zisk_op_of_code (0xf0) = Some(ZOP_ADD256())
function clause zisk_exec_mem (ZOP_ADD256(), _, b, _) = {
  check_aligned(b, "add256");
  let px = param(b, 0); check_aligned(px, "add256");
  let py = param(b, 1); check_aligned(py, "add256");
  let cin = unsigned(param(b, 2));
  let pz = param(b, 3); check_aligned(pz, "add256");
  assert(cin <= 1, "add256: the carry in is not 0 or 1");
  let s = read_uint(px, 4) + read_uint(py, 4) + cin;
  write_uint(pz, 4, s);
  Some((to_bits(64, s / pow2(256)), false))
}

/* arith256: [x, y, z, dl, dh] at b: dh:dl = x * y + z, 4 words each. */
union clause zisk_op = ZOP_ARITH256 : unit
function clause zisk_op_of_code (0xf2) = Some(ZOP_ARITH256())
function clause zisk_exec_mem (ZOP_ARITH256(), _, b, _) = {
  check_aligned(b, "arith256");
  foreach (i from 0 to 4) check_aligned(param(b, i), "arith256");
  let pdl = param(b, 3);
  let pdh = param(b, 4);
  let r = read_uint(param(b, 0), 4) * read_uint(param(b, 1), 4) + read_uint(param(b, 2), 4);
  write_uint(pdl, 4, r);
  write_uint(pdh, 4, r / pow2(256));
  Some((zeros(), false))
}

/* (x * y + z) mod m of n words each, from the table [x, y, z, m, d] at b,
 * into d. m must not be 0. */
val arith_mod : (bits(64), int, string) -> unit
function arith_mod(b, n, what) = {
  check_aligned(b, what);
  foreach (i from 0 to 4) check_aligned(param(b, i), what);
  let m = read_uint(param(b, 3), n);
  assert(m != 0, concat_str(what, ": the modulus is 0"));
  let r = (read_uint(param(b, 0), n) * read_uint(param(b, 1), n) + read_uint(param(b, 2), n)) % m;
  write_uint(param(b, 4), n, r)
}

/* arith256_mod: d = (x * y + z) mod m, 4 words each. */
union clause zisk_op = ZOP_ARITH256_MOD : unit
function clause zisk_op_of_code (0xf3) = Some(ZOP_ARITH256_MOD())
function clause zisk_exec_mem (ZOP_ARITH256_MOD(), _, b, _) = {
  arith_mod(b, 4, "arith256_mod");
  Some((zeros(), false))
}

/* arith384_mod: the same, 6 words each. */
union clause zisk_op = ZOP_ARITH384_MOD : unit
function clause zisk_op_of_code (0xe2) = Some(ZOP_ARITH384_MOD())
function clause zisk_exec_mem (ZOP_ARITH384_MOD(), _, b, _) = {
  arith_mod(b, 6, "arith384_mod");
  Some((zeros(), false))
}
