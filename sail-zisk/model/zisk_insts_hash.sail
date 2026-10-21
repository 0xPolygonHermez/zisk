/* Hash precompiles: the Keccak-f[1600] permutation, the SHA-256
 * compression, and the BLAKE2b round and the BLAKE2s and BLAKE3
 * permutations.
 *
 * Each works on fixed-size blocks of words in memory, 8-byte aligned
 * (core/src/zisk_ops.rs, core/src/helpers.rs; zisk_precomp_helpers). The
 * constant tables are listed in their usual order; Sail indexes a vector
 * literal from its last element (default Order dec), so each table is read
 * through a function that counts from the first.
 */

val rotr64 : (bits(64), int) -> bits(64)
function rotr64(x, n) = (x >> n) | (x << (64 - n))

val rotl64 : (bits(64), int) -> bits(64)
function rotl64(x, n) = (x << n) | (x >> (64 - n))

val rotr32 : (bits(32), int) -> bits(32)
function rotr32(x, n) = (x >> n) | (x << (32 - n))

/* --- Keccak-f[1600] (FIPS 202) --------------------------------------- */

let KECCAK_RC : vector(24, bits(64)) = [
  0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
  0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
  0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
  0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
  0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
  0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008
]

/* The rotation of lane x + 5 y. */
let KECCAK_ROT : vector(25, range(0, 63)) = [
  0, 1, 62, 28, 27,
  36, 44, 6, 55, 20,
  3, 10, 43, 25, 39,
  41, 45, 15, 21, 8,
  18, 2, 61, 56, 14
]

val keccak_rc : range(0, 23) -> bits(64)
function keccak_rc(i) = KECCAK_RC[23 - i]

val keccak_rot : range(0, 24) -> range(0, 63)
function keccak_rot(i) = KECCAK_ROT[24 - i]

/* The 24 rounds over the state, lane x + 5 y at index x + 5 y. */
val keccak_f : vector(25, bits(64)) -> vector(25, bits(64))
function keccak_f(state) = {
  var s = state;
  foreach (round from 0 to 23) {
    /* theta */
    var c : vector(5, bits(64)) = [zeros(), zeros(), zeros(), zeros(), zeros()];
    foreach (x from 0 to 4)
      c[x] = s[x] ^ s[x + 5] ^ s[x + 10] ^ s[x + 15] ^ s[x + 20];
    foreach (x from 0 to 4) {
      let dx = c[(x + 4) % 5] ^ rotl64(c[(x + 1) % 5], 1);
      foreach (y from 0 to 4) s[x + 5 * y] = s[x + 5 * y] ^ dx
    };
    /* rho and pi: lane (x, y) moves to (y, 2x + 3y) */
    var t = s;
    foreach (x from 0 to 4)
      foreach (y from 0 to 4)
        t[y + 5 * ((2 * x + 3 * y) % 5)] = rotl64(s[x + 5 * y], keccak_rot(x + 5 * y));
    /* chi */
    foreach (x from 0 to 4)
      foreach (y from 0 to 4)
        s[x + 5 * y] = t[x + 5 * y] ^ (~(t[(x + 1) % 5 + 5 * y]) & t[(x + 2) % 5 + 5 * y]);
    /* iota */
    s[0] = s[0] ^ keccak_rc(round)
  };
  s
}

/* keccak: Keccak-f over the 25 words at b, in place; c = 0. */
union clause zisk_op = ZOP_KECCAK : unit
function clause zisk_op_of_code (0xf1) = Some(ZOP_KECCAK())
function clause zisk_exec_mem (ZOP_KECCAK(), _, b, _) = {
  check_aligned(b, "keccak");
  var s : vector(25, bits(64)) = undefined;
  foreach (i from 0 to 24) s[i] = read_mem(b + to_bits(64, 8 * i), 8);
  let r = keccak_f(s);
  foreach (i from 0 to 24) write_mem(b + to_bits(64, 8 * i), 8, r[i]);
  Some((zeros(), false))
}

/* --- SHA-256 (FIPS 180-4) -------------------------------------------- */

let SHA256_K : vector(64, bits(32)) = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
]

val sha256_k : range(0, 63) -> bits(32)
function sha256_k(i) = SHA256_K[63 - i]

/* The compression of one 64-byte block into the state, with the
 * feed-forward: sha2's compress256. */
val sha256_compress : (vector(8, bits(32)), vector(16, bits(32))) -> vector(8, bits(32))
function sha256_compress(h, m) = {
  var w : vector(64, bits(32)) = undefined;
  foreach (i from 0 to 15) w[i] = m[i];
  foreach (i from 16 to 63) {
    let s0 = rotr32(w[i - 15], 7) ^ rotr32(w[i - 15], 18) ^ (w[i - 15] >> 3);
    let s1 = rotr32(w[i - 2], 17) ^ rotr32(w[i - 2], 19) ^ (w[i - 2] >> 10);
    w[i] = w[i - 16] + s0 + w[i - 7] + s1
  };
  var v = h;
  foreach (i from 0 to 63) {
    let s1 = rotr32(v[4], 6) ^ rotr32(v[4], 11) ^ rotr32(v[4], 25);
    let ch = (v[4] & v[5]) ^ (~(v[4]) & v[6]);
    let t1 = v[7] + s1 + ch + sha256_k(i) + w[i];
    let s0 = rotr32(v[0], 2) ^ rotr32(v[0], 13) ^ rotr32(v[0], 22);
    let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
    let t2 = s0 + maj;
    v[7] = v[6]; v[6] = v[5]; v[5] = v[4]; v[4] = v[3] + t1;
    v[3] = v[2]; v[2] = v[1]; v[1] = v[0]; v[0] = t1 + t2
  };
  var r = h;
  foreach (i from 0 to 7) r[i] = h[i] + v[i];
  r
}

/* sha256: compress the block at b into the state at a; c = 0. The state is
 * 8 32-bit words, little-endian, as sha2 keeps them in memory; the block 64
 * bytes, whose words SHA-256 reads big-endian. */
union clause zisk_op = ZOP_SHA256 : unit
function clause zisk_op_of_code (0xf9) = Some(ZOP_SHA256())
function clause zisk_exec_mem (ZOP_SHA256(), a, b, _) = {
  check_aligned(a, "sha256");
  check_aligned(b, "sha256");
  var h : vector(8, bits(32)) = undefined;
  foreach (i from 0 to 7) h[i] = read_mem(a + to_bits(64, 4 * i), 4);
  var m : vector(16, bits(32)) = undefined;
  foreach (i from 0 to 15) {
    let x = read_mem(b + to_bits(64, 4 * i), 4);
    m[i] = x[7 .. 0] @ x[15 .. 8] @ x[23 .. 16] @ x[31 .. 24]
  };
  let r = sha256_compress(h, m);
  foreach (i from 0 to 7) write_mem(a + to_bits(64, 4 * i), 4, r[i]);
  Some((zeros(), false))
}

/* --- BLAKE2 (RFC 7693) and BLAKE3 ------------------------------------ */

/* The message schedules: BLAKE2's ten permutations, and BLAKE3's seven,
 * its one permutation applied 0..6 times. */
let BLAKE2_SIGMA : vector(160, range(0, 15)) = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
  14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3,
  11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4,
  7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8,
  9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13,
  2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9,
  12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11,
  13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10,
  6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5,
  10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0
]

let BLAKE3_SIGMA : vector(112, range(0, 15)) = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
  2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8,
  3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1,
  10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6,
  12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4,
  9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7,
  11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13
]

val blake2_sigma : (range(0, 9), range(0, 15)) -> range(0, 15)
function blake2_sigma(r, k) = BLAKE2_SIGMA[159 - (16 * r + k)]

val blake3_sigma : (range(0, 6), range(0, 15)) -> range(0, 15)
function blake3_sigma(r, k) = BLAKE3_SIGMA[111 - (16 * r + k)]

/* The columns, then the diagonals: the four lanes each G mixes. */
let BLAKE_LANES : vector(32, range(0, 15)) = [
  0, 4, 8, 12,   1, 5, 9, 13,   2, 6, 10, 14,   3, 7, 11, 15,
  0, 5, 10, 15,  1, 6, 11, 12,  2, 7, 8, 13,    3, 4, 9, 14
]

val blake_lane : (range(0, 7), range(0, 3)) -> range(0, 15)
function blake_lane(g, k) = BLAKE_LANES[31 - (4 * g + k)]

/* One BLAKE2b round over 64-bit words: rotations 32, 24, 16, 63. */
val blake2b_round : (vector(16, bits(64)), vector(16, bits(64)), range(0, 9)) -> vector(16, bits(64))
function blake2b_round(state, m, r) = {
  var v = state;
  foreach (g from 0 to 7) {
    let a = blake_lane(g, 0); let b = blake_lane(g, 1);
    let c = blake_lane(g, 2); let d = blake_lane(g, 3);
    let x = m[blake2_sigma(r, 2 * g)];
    let y = m[blake2_sigma(r, 2 * g + 1)];
    v[a] = v[a] + v[b] + x;  v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = v[c] + v[d];      v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = v[a] + v[b] + y;  v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = v[c] + v[d];      v[b] = rotr64(v[b] ^ v[c], 63)
  };
  v
}

/* One round over 32-bit words, BLAKE2s's and BLAKE3's G: rotations 16, 12,
 * 8, 7, with message words x and y for each G as sigma picks them. */
val blake32_round : (vector(16, bits(32)), vector(16, bits(32)), bool, range(0, 9)) -> vector(16, bits(32))
function blake32_round(state, m, is_blake3, r) = {
  var v = state;
  foreach (g from 0 to 7) {
    let a = blake_lane(g, 0); let b = blake_lane(g, 1);
    let c = blake_lane(g, 2); let d = blake_lane(g, 3);
    let (x, y) : (bits(32), bits(32)) =
      if is_blake3 then { assert(r <= 6); (m[blake3_sigma(r, 2 * g)], m[blake3_sigma(r, 2 * g + 1)]) }
      else (m[blake2_sigma(r, 2 * g)], m[blake2_sigma(r, 2 * g + 1)]);
    v[a] = v[a] + v[b] + x;  v[d] = rotr32(v[d] ^ v[a], 16);
    v[c] = v[c] + v[d];      v[b] = rotr32(v[b] ^ v[c], 12);
    v[a] = v[a] + v[b] + y;  v[d] = rotr32(v[d] ^ v[a], 8);
    v[c] = v[c] + v[d];      v[b] = rotr32(v[b] ^ v[c], 7)
  };
  v
}

/* blake2b: BLAKE2b's round (the extended argument mod 10, of its low 32
 * bits) of the 16-word state at a with the 16-word block at b, over a;
 * c = 0. The round only: no initialisation, no feed-forward. */
union clause zisk_op = ZOP_BLAKE2B : unit
function clause zisk_op_of_code (0xea) = Some(ZOP_BLAKE2B())
function clause zisk_exec_mem (ZOP_BLAKE2B(), a, b, ext) = {
  check_aligned(a, "blake2b");
  check_aligned(b, "blake2b");
  var v : vector(16, bits(64)) = undefined;
  var m : vector(16, bits(64)) = undefined;
  foreach (i from 0 to 15) {
    v[i] = read_mem(a + to_bits(64, 8 * i), 8);
    m[i] = read_mem(b + to_bits(64, 8 * i), 8)
  };
  let r = blake2b_round(v, m, unsigned(ext[31 .. 0]) % 10);
  foreach (i from 0 to 15) write_mem(a + to_bits(64, 8 * i), 8, r[i]);
  Some((zeros(), false))
}

/* The 32-bit words of a 64-byte block at addr, little-endian. */
val read_block32 : bits(64) -> vector(16, bits(32))
function read_block32(addr) = {
  var m : vector(16, bits(32)) = undefined;
  foreach (i from 0 to 15) m[i] = read_mem(addr + to_bits(64, 4 * i), 4);
  m
}

val write_block32 : (bits(64), vector(16, bits(32))) -> unit
function write_block32(addr, v) =
  foreach (i from 0 to 15) write_mem(addr + to_bits(64, 4 * i), 4, v[i])

/* blake2s: BLAKE2s's ten rounds of the 16-word state at a with the block at
 * b, over a; c = 0. No initialisation, no feed-forward. */
union clause zisk_op = ZOP_BLAKE2S : unit
function clause zisk_op_of_code (0xef) = Some(ZOP_BLAKE2S())
function clause zisk_exec_mem (ZOP_BLAKE2S(), a, b, _) = {
  check_aligned(a, "blake2s");
  check_aligned(b, "blake2s");
  var v = read_block32(a);
  let m = read_block32(b);
  foreach (r from 0 to 9) v = blake32_round(v, m, false, r);
  write_block32(a, v);
  Some((zeros(), false))
}

/* blake3: BLAKE3's seven rounds of the 16-word state at a with the block at
 * b, over a; c = 0. No feed-forward. */
union clause zisk_op = ZOP_BLAKE3 : unit
function clause zisk_op_of_code (0xee) = Some(ZOP_BLAKE3())
function clause zisk_exec_mem (ZOP_BLAKE3(), a, b, _) = {
  check_aligned(a, "blake3");
  check_aligned(b, "blake3");
  var v = read_block32(a);
  let m = read_block32(b);
  foreach (r from 0 to 6) v = blake32_round(v, m, true, r);
  write_block32(a, v);
  Some((zeros(), false))
}
