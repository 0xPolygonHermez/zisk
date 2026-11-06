/* Poseidon precompiles: permutations of 16 field elements, in place.
 *
 * poseidon2 and poseidon1 over Goldilocks, one element per word, as
 * proofman-fields computes them (poseidon2_hash, poseidon1_hash): a word is
 * taken mod p, the result written canonical. poseidon2_koalabear over
 * KoalaBear, two elements per word, as zisk_precomp_helpers computes it
 * (Poseidon2Params::permute_words). The constants are generated:
 * model/zisk_poseidon_constants.sail. Each follows its source step by step,
 * Poseidon1 in the optimised form, with sparse matrices for the partial
 * rounds, that proofman-fields uses.
 */

let GOLDILOCKS_P : int = 18446744069414584321

val pow7 : int -> int
function pow7(x) = {
  let x2 = (x * x) % GOLDILOCKS_P;
  let x4 = (x2 * x2) % GOLDILOCKS_P;
  (((x4 * x2) % GOLDILOCKS_P) * x) % GOLDILOCKS_P
}

/* The 16 words at b, mod p. */
val read_goldilocks : bits(64) -> vector(16, int)
function read_goldilocks(b) = {
  var s : vector(16, int) = undefined;
  foreach (i from 0 to 15) s[i] = unsigned(read_mem(b + to_bits(64, 8 * i), 8)) % GOLDILOCKS_P;
  s
}

val write_goldilocks : (bits(64), vector(16, int)) -> unit
function write_goldilocks(b, s) =
  foreach (i from 0 to 15) write_mem(b + to_bits(64, 8 * i), 8, to_bits(64, s[i] % GOLDILOCKS_P))

/* --- Poseidon2 -------------------------------------------------------- */

/* The external layer: each block of 4 times Poseidon2's M4, then each
 * element plus the sum of its column across the blocks (matmul_external). */
val poseidon2_external : vector(16, int) -> vector(16, int)
function poseidon2_external(state) = {
  var s = state;
  foreach (k from 0 to 3) {
    let x0 = s[4 * k]; let x1 = s[4 * k + 1]; let x2 = s[4 * k + 2]; let x3 = s[4 * k + 3];
    let t0 = x0 + x1;
    let t1 = x2 + x3;
    let t2 = 2 * x1 + t1;
    let t3 = 2 * x3 + t0;
    let t4 = 4 * t1 + t3;
    let t5 = 4 * t0 + t2;
    s[4 * k]     = (t3 + t5) % GOLDILOCKS_P;
    s[4 * k + 1] = t5 % GOLDILOCKS_P;
    s[4 * k + 2] = (t2 + t4) % GOLDILOCKS_P;
    s[4 * k + 3] = t4 % GOLDILOCKS_P
  };
  var r = s;
  foreach (i from 0 to 15)
    r[i] = (s[i] + s[i % 4] + s[i % 4 + 4] + s[i % 4 + 8] + s[i % 4 + 12]) % GOLDILOCKS_P;
  r
}

val poseidon2_full_round : (vector(16, int), range(0, 134)) -> vector(16, int)
function poseidon2_full_round(state, base) = {
  var s = state;
  foreach (i from 0 to 15) s[i] = pow7(s[i] + unsigned(poseidon2_rc(base + i)));
  poseidon2_external(s)
}

val poseidon2_permute : vector(16, int) -> vector(16, int)
function poseidon2_permute(input) = {
  var s = poseidon2_external(input);
  foreach (r from 0 to 3) s = poseidon2_full_round(s, 16 * r);
  foreach (r from 0 to 21) {
    s[0] = pow7(s[0] + unsigned(poseidon2_rc(64 + r)));
    var sum : int = 0;
    foreach (i from 0 to 15) sum = sum + s[i];
    foreach (i from 0 to 15) s[i] = (s[i] * unsigned(poseidon2_diag(i)) + sum) % GOLDILOCKS_P
  };
  foreach (r from 0 to 3) s = poseidon2_full_round(s, 86 + 16 * r);
  s
}

/* poseidon2: the permutation of the 16 words at b, in place; c = 0. */
union clause zisk_op = ZOP_POSEIDON2 : unit
function clause zisk_op_of_code (0xeb) = Some(ZOP_POSEIDON2())
function clause zisk_exec_mem (ZOP_POSEIDON2(), _, b, _) = {
  check_aligned(b, "poseidon2");
  write_goldilocks(b, poseidon2_permute(read_goldilocks(b)));
  Some((zeros(), false))
}

/* --- Poseidon (Poseidon1) --------------------------------------------- */

/* The state times a 16x16 matrix, mat[j][i] at 16 j + i: new[i] is
 * sum_j old[j] * mat[j][i]. */
val poseidon1_matmul : (vector(16, int), bool) -> vector(16, int)
function poseidon1_matmul(old, use_p) = {
  var r : vector(16, int) = undefined;
  foreach (i from 0 to 15) {
    var sum : int = 0;
    foreach (j from 0 to 15)
      sum = sum + old[j] * unsigned(if use_p then poseidon1_p(16 * j + i) else poseidon1_m(16 * j + i));
    r[i] = sum % GOLDILOCKS_P
  };
  r
}

/* Every element to the 7th, plus the constants from base on. */
val poseidon1_sbox_add : (vector(16, int), range(0, 134)) -> vector(16, int)
function poseidon1_sbox_add(state, base) = {
  var s = state;
  foreach (i from 0 to 15) s[i] = (pow7(s[i]) + unsigned(poseidon1_c(base + i))) % GOLDILOCKS_P;
  s
}

val poseidon1_permute : vector(16, int) -> vector(16, int)
function poseidon1_permute(input) = {
  var s = input;
  foreach (i from 0 to 15) s[i] = (s[i] + unsigned(poseidon1_c(i))) % GOLDILOCKS_P;
  foreach (r from 0 to 2) s = poseidon1_matmul(poseidon1_sbox_add(s, 16 * (r + 1)), false);
  s = poseidon1_matmul(poseidon1_sbox_add(s, 64), true);
  foreach (r from 0 to 21) {
    s[0] = (pow7(s[0]) + unsigned(poseidon1_c(80 + r))) % GOLDILOCKS_P;
    var s0 : int = 0;
    foreach (j from 0 to 15) s0 = s0 + s[j] * unsigned(poseidon1_s(31 * r + j));
    foreach (t from 1 to 15)
      s[t] = (s[t] + s[0] * unsigned(poseidon1_s(31 * r + 15 + t))) % GOLDILOCKS_P;
    s[0] = s0 % GOLDILOCKS_P
  };
  foreach (r from 0 to 2) s = poseidon1_matmul(poseidon1_sbox_add(s, 102 + 16 * r), false);
  foreach (i from 0 to 15) s[i] = pow7(s[i]);
  poseidon1_matmul(s, false)
}

/* poseidon1: the permutation of the 16 words at b, in place; c = 0. */
union clause zisk_op = ZOP_POSEIDON1 : unit
function clause zisk_op_of_code (0xec) = Some(ZOP_POSEIDON1())
function clause zisk_exec_mem (ZOP_POSEIDON1(), _, b, _) = {
  check_aligned(b, "poseidon1");
  write_goldilocks(b, poseidon1_permute(read_goldilocks(b)));
  Some((zeros(), false))
}

/* --- Poseidon2 over KoalaBear ----------------------------------------- */

val kb_mul : (int, int) -> int
function kb_mul(x, y) = (x * y) % KOALABEAR_P

/* Each block of 4 times M4, then each element plus its column's sum. */
val koalabear_external : vector(16, int) -> vector(16, int)
function koalabear_external(state) = {
  var y : vector(16, int) = undefined;
  foreach (k from 0 to 3)
    foreach (row from 0 to 3) {
      var sum : int = 0;
      foreach (col from 0 to 3) sum = sum + koalabear_m4(4 * row + col) * state[4 * k + col];
      y[4 * k + row] = sum % KOALABEAR_P
    };
  var r : vector(16, int) = undefined;
  foreach (i from 0 to 15)
    r[i] = (y[i] + y[i % 4] + y[i % 4 + 4] + y[i % 4 + 8] + y[i % 4 + 12]) % KOALABEAR_P;
  r
}

val koalabear_sbox : int -> int
function koalabear_sbox(x) = kb_mul(kb_mul(x, x), x)

val koalabear_permute : vector(16, int) -> vector(16, int)
function koalabear_permute(input) = {
  var s = koalabear_external(input);
  foreach (r from 0 to 3) {
    foreach (i from 0 to 15) s[i] = koalabear_sbox((s[i] + koalabear_rc_begin(16 * r + i)) % KOALABEAR_P);
    s = koalabear_external(s)
  };
  foreach (r from 0 to 19) {
    s[0] = koalabear_sbox((s[0] + koalabear_rc_partial(r)) % KOALABEAR_P);
    var sum : int = 0;
    foreach (i from 0 to 15) sum = sum + s[i];
    foreach (i from 0 to 15)
      s[i] = kb_mul((sum % KOALABEAR_P + koalabear_diag(i) * s[i]) % KOALABEAR_P, KOALABEAR_INTERNAL_SCALE)
  };
  foreach (r from 0 to 3) {
    foreach (i from 0 to 15) s[i] = koalabear_sbox((s[i] + koalabear_rc_end(16 * r + i)) % KOALABEAR_P);
    s = koalabear_external(s)
  };
  s
}

/* poseidon2_koalabear: the permutation of the 8 words at b, in place;
 * c = 0. Word i holds elements 2i (low half) and 2i + 1 (high half), each
 * scaled by KOALABEAR_IO_SCALE, Montgomery form, and canonical: an element
 * not below p is an error. */
union clause zisk_op = ZOP_POSEIDON2_KOALABEAR : unit
function clause zisk_op_of_code (0xc1) = Some(ZOP_POSEIDON2_KOALABEAR())
function clause zisk_exec_mem (ZOP_POSEIDON2_KOALABEAR(), _, b, _) = {
  check_aligned(b, "poseidon2_koalabear");
  let inv = pow_mod(KOALABEAR_IO_SCALE, KOALABEAR_P - 2, KOALABEAR_P);
  var s : vector(16, int) = undefined;
  foreach (w from 0 to 7) {
    let word = read_mem(b + to_bits(64, 8 * w), 8);
    let lo = unsigned(word[31 .. 0]);
    let hi = unsigned(word[63 .. 32]);
    assert(lo < KOALABEAR_P & hi < KOALABEAR_P, "poseidon2_koalabear: an element is not canonical");
    s[2 * w] = kb_mul(lo, inv);
    s[2 * w + 1] = kb_mul(hi, inv)
  };
  let r = koalabear_permute(s);
  foreach (w from 0 to 7)
    write_mem(b + to_bits(64, 8 * w), 8,
              to_bits(32, kb_mul(r[2 * w + 1], KOALABEAR_IO_SCALE)) @ to_bits(32, kb_mul(r[2 * w], KOALABEAR_IO_SCALE)));
  Some((zeros(), false))
}
