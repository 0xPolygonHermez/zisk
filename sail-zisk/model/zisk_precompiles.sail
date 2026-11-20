/* What the precompiles share: big integers in memory, and arithmetic modulo
 * a prime.
 *
 * A precompile works on operands in memory, at the addresses in a and b or
 * in a table of addresses at b, and writes its result back to memory; its c
 * is 0 but for add256's carry, and its flag false (core/src/zisk_ops.rs). It
 * reads all its operands before it writes anything, so a result may land on
 * an operand. An operand of n words is an unsigned integer, its words in
 * little-endian order, each 8-byte aligned.
 *
 * The emulator computes the field operations with arkworks (the
 * zisk_precomp_helpers crate), which panics on what is not a field
 * operation: an operand that is not a canonical element, below the modulus,
 * or a division by zero. The model asserts the same. ed25519_add is the
 * exception: it reduces its operands, so the model does too.
 */

/* An operand's address: 8-byte aligned, or the emulator panics. */
val check_aligned : (bits(64), string) -> unit
function check_aligned(addr, what) =
  assert(addr[2 .. 0] == 0b000, concat_str(what, ": an operand address is not 8-byte aligned"))

/* The n words at addr, as an unsigned integer, least significant first. */
val read_uint : (bits(64), int) -> int
function read_uint(addr, n) = {
  var x : int = 0;
  foreach (i from (n - 1) downto 0)
    x = x * 18446744073709551616 + unsigned(read_mem(addr + to_bits(64, 8 * i), 8));
  x
}

/* x modulo 2^(64n), as n words at addr. */
val write_uint : (bits(64), int, int) -> unit
function write_uint(addr, n, x) =
  foreach (i from 0 to (n - 1))
    write_mem(addr + to_bits(64, 8 * i), 8, get_slice_int(64, x, 64 * i))

/* The i-th word of a table of operand addresses at b. */
val param : (bits(64), int) -> bits(64)
function param(b, i) = read_mem(b + to_bits(64, 8 * i), 8)

/* --- Arithmetic modulo a prime p ------------------------------------- */

let SECP256K1_P : int = 115792089237316195423570985008687907853269984665640564039457584007908834671663
let SECP256R1_P : int = 115792089210356248762697446949407573530086143415290314195533631308867097853951
let BN254_P     : int = 21888242871839275222246405745257275088696311157297823662689037894645226208583
let BN254_R     : int = 21888242871839275222246405745257275088548364400416034343698204186575808495617
let BLS12_381_P : int = 4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787
let ED25519_P   : int = 57896044618658097711785492504343953926634992332820282019728792003956564819949

/* x, which must be a canonical element: below p. */
val canonical : (int, int) -> int
function canonical(x, p) = {
  assert(x < p, "a precompile operand is not a canonical field element");
  x
}

/* b^e mod m, by square and multiply, for e below 2^384. */
val pow_mod : (int, int, int) -> int
function pow_mod(b, e, m) = {
  var r : int = 1;
  var s : int = b % m;
  var k : int = e;
  foreach (i from 0 to 383) {
    if k % 2 == 1 then r = (r * s) % m;
    s = (s * s) % m;
    k = k / 2
  };
  r
}

/* x / y mod p; y must not be 0 mod p. */
val fdiv : (int, int, int) -> int
function fdiv(x, y, p) = {
  assert(y % p != 0, "a precompile divides by zero");
  (x * pow_mod(y, p - 2, p)) % p
}

/* Two field elements of n words each at addr, canonical: a point's (x, y),
 * or a complex number's (re, im). */
val read_pair : (bits(64), int, int) -> (int, int)
function read_pair(addr, n, p) =
  (canonical(read_uint(addr, n), p), canonical(read_uint(addr + to_bits(64, 8 * n), n), p))

val write_pair : (bits(64), int, (int, int)) -> unit
function write_pair(addr, n, (x, y)) = {
  write_uint(addr, n, x);
  write_uint(addr + to_bits(64, 8 * n), n, y)
}
