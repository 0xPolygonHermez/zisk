# Scalar arithmetic and equivalence checks

The first arkworks-based SBF baseline consumed about 3.43M CUs with profiling
logs. The on-chain backend therefore uses a private `Fr([u32; 9])`, with radix
B = 2^29 and Montgomery factor R = B^9 = 2^261. Arkworks remains an independent
host oracle. Every stored residue is normalized and less than the BN254 scalar
modulus:

```
r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
```

There is no public API for constructing an unchecked internal residue. Sources
are checked scalar decoding, hash reduction, arithmetic on canonical values,
and generated constants. The generator maps integer c to cR mod r at build time.

## Bounds used by multiplication

Column-wise multiplication accumulates the coefficients of `a*b + m*r`. Each
of nine low digits chooses `m_i = column * (-r_0^-1) mod B`; adding `m_i*r_0`
cancels that column. Both input limbs and correction digits are at most B−1.
A column contains at most 18 products. With carry less than 18(B−1), its sum is
less than `18B(B−1) < 2^63`. Products and additions used to accumulate a column
therefore fit in u64.

Computing the correction digit is different: `column.wrapping_mul(INV)` may
overflow u64. Its subsequent `& (B−1)` keeps exactly the product modulo B,
because B = 2^29 divides 2^64. That intentional wrap discards no bits needed
for the correction digit.

The nine cancelled digits make the result divisible by R, and `0 <= m < R`.
For `a,b < r`, `(a*b + m*r)/R < r + r^2/R < 2r`. The result fits in nine
normalized limbs, and one conditional subtraction of r yields its canonical
representative. The final carry fits in u32 because `2r < 2^255`.

Addition also needs at most one subtraction: `a+b < 2r < 2^261`, so it cannot
lose a carry beyond R. Negation returns zero for zero and r−a otherwise. Modular
subtraction adds r only after a borrow; discarding the carry beyond R then gives
a−b+r. All results remain canonical.

Canonical integer decoding rejects values at or above r, then multiplies the
raw integer by `R^2 mod r`. Encoding multiplies by raw integer one. Only SHA-256
statement digests and Keccak challenges are reduced modulo r; proof scalars are
never silently reduced.

## Generated arithmetic

`tools/src/generate.rs` accepts exactly one SHA-256-pinned Solidity source.
It translates only six selected scalar stages and rejects unsupported syntax,
moduli and addresses. Q's memory stores are split into small functions to satisfy
SBF's stack/branch limits. Subtraction and squaring are specialized without
changing their field meanings. The y-power schedule shares y², y⁴, y⁸ and y¹⁶;
xi²¹⁶ reuses xi²¹⁵·xi. All inverse checks and zero-denominator checks remain.

Tests compare complete JS transcript/quotient/group traces, the release proof,
each proof-word mutation, and exceptional challenges. Arithmetic tests compare
100,000 seeded pairs, raw limb boundaries, encoding, hash reduction, subtraction,
squaring and powers against arkworks. Coverage-guided fuzzing checks arithmetic
equivalence and exercises the proof decoder. Separate SBF tests execute the real
ELF, including every proof-word mutation and late pairing failure.

These bounds explain the implementation; the tests are evidence, not a formal
proof of the entire verifier. Agent reviews are not an external cryptographic
audit. New keys, profiles, generator inputs or arithmetic changes require fresh
review and measurements.
