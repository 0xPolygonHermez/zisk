/-
  Binary computes `ltu`, `eq` and `and`, byte by byte.

  The same eight byte lookups into the Binary table that compute an `add`
  (`Binary.lean`) compute the other modeled binary ops; only the table's rows,
  and how the carry chain is read, differ:

  * `and`: each byte is `a & b`, and nothing is carried;
  * `ltu`: no byte has a result, and the carry is "less than so far": 1 if the
    byte is below, the carry-in if equal, 0 if above, so the last carry is
    `[A < B]` — which Binary adds into `c` and sends as the flag;
  * `eq`: the carry is "different so far", inverted at the last byte, so the
    last carry is `[A = B]`.

  `OpRow` is what `check_tables.py` checks of a Binary-table row for these
  ops (`BinaryTableOps`); `cmp_decode` reads a lane's eight lookups into
  naturals, as `binary8` does for `add`; and the eight-byte chains `ltu8`,
  `eq8`, `and8` finish each op.
-/
import Proofs.Ops

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-! ### Eight-byte chains, in naturals -/
/-- One more byte on top of a prefix: the comparison is the top byte's, unless
the top bytes are equal. -/
theorem lt_step (M x y a b : Nat) (hx : x < M) (hy : y < M) :
    x + M * a < y + M * b ↔ a < b ∨ (a = b ∧ x < y) := by
  constructor
  · intro h
    rcases Nat.lt_trichotomy a b with hab | hab | hab
    · exact Or.inl hab
    · subst hab; exact Or.inr ⟨rfl, by omega⟩
    · exfalso
      have : M * (b + 1) ≤ M * a := Nat.mul_le_mul_left M hab
      rw [Nat.mul_add, Nat.mul_one] at this
      omega
  · rintro (hab | ⟨rfl, hxy⟩)
    · have : M * (a + 1) ≤ M * b := Nat.mul_le_mul_left M hab
      rw [Nat.mul_add, Nat.mul_one] at this
      omega
    · omega

/-- One more byte on top of a prefix: equal exactly when both parts are. -/
theorem eq_step (M x y a b : Nat) (hx : x < M) (hy : y < M) :
    x + M * a = y + M * b ↔ a = b ∧ x = y := by
  constructor
  · intro h
    have hM : 0 < M := by omega
    have ha : (x + M * a) / M = a := by
      rw [Nat.add_mul_div_left _ _ hM, Nat.div_eq_of_lt hx, Nat.zero_add]
    have hb : (y + M * b) / M = b := by
      rw [Nat.add_mul_div_left _ _ hM, Nat.div_eq_of_lt hy, Nat.zero_add]
    have hab : a = b := by rw [← ha, ← hb, h]
    subst hab; exact ⟨rfl, by omega⟩
  · rintro ⟨rfl, rfl⟩; rfl

/-- One more byte on top of a prefix: AND is bytewise. -/
theorem and_step (m x y a b : Nat) (hx : x < 2 ^ m) (hy : y < 2 ^ m) :
    (x + 2 ^ m * a) &&& (y + 2 ^ m * b) = (x &&& y) + 2 ^ m * (a &&& b) := by
  apply Nat.eq_of_testBit_eq
  intro i
  have hxy : x &&& y < 2 ^ m := Nat.and_lt_two_pow x hy
  rw [Nat.add_comm x, Nat.add_comm y, Nat.add_comm (x &&& y),
    Nat.testBit_and, Nat.testBit_two_pow_mul_add _ hx, Nat.testBit_two_pow_mul_add _ hy,
    Nat.testBit_two_pow_mul_add _ hxy, Nat.testBit_and, Nat.testBit_and]
  split <;> rfl

/-- The value of eight bytes, little-endian, as two 32-bit limbs. -/
def bytes8 (a : Nat → Nat) : Nat :=
  (a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3) +
    4294967296 * (a 4 + 256 * a 5 + 65536 * a 6 + 16777216 * a 7)

theorem bytes8_prefix (a : Nat → Nat) :
    bytes8 a = a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3 + 4294967296 * a 4 +
      1099511627776 * a 5 + 281474976710656 * a 6 + 72057594037927936 * a 7 := by
  unfold bytes8; omega

/-- The comparison carry, one byte at a time. -/
theorem lt_chain (M x y a b f g : Nat) (hx : x < M) (hy : y < M)
    (hf : f = if x < y then 1 else 0)
    (hg : g = if a < b then 1 else if a = b then f else 0) :
    g = if x + M * a < y + M * b then 1 else 0 := by
  have hs := lt_step M x y a b hx hy
  subst hf hg
  rcases Nat.lt_trichotomy a b with hab | rfl | hab
  · rw [if_pos hab, if_pos (hs.mpr (Or.inl hab))]
  · rw [if_neg (Nat.lt_irrefl _), if_pos rfl]
    by_cases hxy : x < y
    · rw [if_pos hxy, if_pos (hs.mpr (Or.inr ⟨rfl, hxy⟩))]
    · rw [if_neg hxy, if_neg (fun h => by rcases hs.mp h with h | ⟨-, h⟩ <;> omega)]
  · rw [if_neg (by omega), if_neg (by omega),
      if_neg (fun h => by rcases hs.mp h with h | ⟨h, -⟩ <;> omega)]

/-- `ltu` on eight bytes: the last carry of the chain is `[A < B]`. -/
theorem ltu8 (a b f : Nat → Nat) (la : ∀ i, i < 8 → a i < 256) (lb : ∀ i, i < 8 → b i < 256)
    (h0 : f 0 = if a 0 < b 0 then 1 else 0)
    (h : ∀ i, i < 7 → f (i + 1) = if a (i + 1) < b (i + 1) then 1
      else if a (i + 1) = b (i + 1) then f i else 0) :
    f 7 = if bytes8 a < bytes8 b then 1 else 0 := by
  have := la 0 (by decide); have := la 1 (by decide); have := la 2 (by decide); have := la 3 (by decide); have := la 4 (by decide); have := la 5 (by decide)
  have := la 6 (by decide); have := la 7 (by decide); have := lb 0 (by decide); have := lb 1 (by decide); have := lb 2 (by decide); have := lb 3 (by decide)
  have := lb 4 (by decide); have := lb 5 (by decide); have := lb 6 (by decide); have := lb 7 (by decide)
  have s1 := lt_chain 256 (a 0) (b 0) (a 1) (b 1) _ _ (by omega) (by omega) h0 (h 0 (by decide))
  have s2 := lt_chain 65536 _ _ (a 2) (b 2) _ _ (by omega) (by omega) s1 (h 1 (by decide))
  have s3 := lt_chain 16777216 _ _ (a 3) (b 3) _ _ (by omega) (by omega) s2 (h 2 (by decide))
  have s4 := lt_chain 4294967296 _ _ (a 4) (b 4) _ _ (by omega) (by omega) s3 (h 3 (by decide))
  have s5 := lt_chain 1099511627776 _ _ (a 5) (b 5) _ _ (by omega) (by omega) s4 (h 4 (by decide))
  have s6 := lt_chain 281474976710656 _ _ (a 6) (b 6) _ _ (by omega) (by omega) s5 (h 5 (by decide))
  have s7 := lt_chain 72057594037927936 _ _ (a 7) (b 7) _ _ (by omega) (by omega) s6 (h 6 (by decide))
  rw [bytes8_prefix, bytes8_prefix]
  exact s7

/-- The inequality flag, one byte at a time. -/
theorem neq_chain (M x y a b n m : Nat) (hx : x < M) (hy : y < M)
    (hn : n = if x = y then 0 else 1)
    (hm : m = if n = 0 ∧ a = b then 0 else 1) :
    m = if x + M * a = y + M * b then 0 else 1 := by
  have := eq_step M x y a b hx hy
  subst hn hm
  by_cases h1 : a = b <;> by_cases h3 : x = y <;> simp_all

/-- `eq` on eight bytes: the chain carries "not equal so far"; inverted at the
last byte, it is `[A = B]`. -/
theorem eq8 (a b n : Nat → Nat) (f7 : Nat) (la : ∀ i, i < 8 → a i < 256) (lb : ∀ i, i < 8 → b i < 256)
    (h0 : n 0 = if 0 = 0 ∧ a 0 = b 0 then 0 else 1)
    (h : ∀ i, i < 7 → n (i + 1) = if n i = 0 ∧ a (i + 1) = b (i + 1) then 0 else 1)
    (h7 : f7 = 1 - n 7) :
    f7 = if bytes8 a = bytes8 b then 1 else 0 := by
  have := la 0 (by decide); have := la 1 (by decide); have := la 2 (by decide); have := la 3 (by decide); have := la 4 (by decide); have := la 5 (by decide)
  have := la 6 (by decide); have := la 7 (by decide); have := lb 0 (by decide); have := lb 1 (by decide); have := lb 2 (by decide); have := lb 3 (by decide)
  have := lb 4 (by decide); have := lb 5 (by decide); have := lb 6 (by decide); have := lb 7 (by decide)
  have e0 : n 0 = if a 0 = b 0 then 0 else 1 := by rw [h0]; simp
  have s1 := neq_chain 256 (a 0) (b 0) (a 1) (b 1) _ _ (by omega) (by omega) e0 (h 0 (by decide))
  have s2 := neq_chain 65536 _ _ (a 2) (b 2) _ _ (by omega) (by omega) s1 (h 1 (by decide))
  have s3 := neq_chain 16777216 _ _ (a 3) (b 3) _ _ (by omega) (by omega) s2 (h 2 (by decide))
  have s4 := neq_chain 4294967296 _ _ (a 4) (b 4) _ _ (by omega) (by omega) s3 (h 3 (by decide))
  have s5 := neq_chain 1099511627776 _ _ (a 5) (b 5) _ _ (by omega) (by omega) s4 (h 4 (by decide))
  have s6 := neq_chain 281474976710656 _ _ (a 6) (b 6) _ _ (by omega) (by omega) s5 (h 5 (by decide))
  have s7 := neq_chain 72057594037927936 _ _ (a 7) (b 7) _ _ (by omega) (by omega) s6 (h 6 (by decide))
  rw [bytes8_prefix, bytes8_prefix, h7, s7]
  split <;> rfl

/-- `and` on eight bytes: the bytes of `A & B` are the bytes' ANDs. -/
theorem and8 (a b c : Nat → Nat) (la : ∀ i, i < 8 → a i < 256) (lb : ∀ i, i < 8 → b i < 256)
    (hc : ∀ i, i < 8 → c i = a i &&& b i) :
    bytes8 c = bytes8 a &&& bytes8 b := by
  have := la 0 (by decide); have := la 1 (by decide); have := la 2 (by decide); have := la 3 (by decide); have := la 4 (by decide); have := la 5 (by decide)
  have := la 6 (by decide); have := la 7 (by decide); have := lb 0 (by decide); have := lb 1 (by decide); have := lb 2 (by decide); have := lb 3 (by decide)
  have := lb 4 (by decide); have := lb 5 (by decide); have := lb 6 (by decide); have := lb 7 (by decide)
  rw [bytes8_prefix, bytes8_prefix, bytes8_prefix, hc 0 (by decide), hc 1 (by decide),
    hc 2 (by decide), hc 3 (by decide), hc 4 (by decide), hc 5 (by decide), hc 6 (by decide),
    hc 7 (by decide)]
  rw [show (256 : Nat) = 2 ^ 8 from rfl, show (65536 : Nat) = 2 ^ 16 from rfl,
    show (16777216 : Nat) = 2 ^ 24 from rfl, show (4294967296 : Nat) = 2 ^ 32 from rfl,
    show (1099511627776 : Nat) = 2 ^ 40 from rfl, show (281474976710656 : Nat) = 2 ^ 48 from rfl,
    show (72057594037927936 : Nat) = 2 ^ 56 from rfl]
  rw [and_step 56 _ _ (a 7) (b 7) (by omega) (by omega),
    and_step 48 _ _ (a 6) (b 6) (by omega) (by omega),
    and_step 40 _ _ (a 5) (b 5) (by omega) (by omega),
    and_step 32 _ _ (a 4) (b 4) (by omega) (by omega),
    and_step 24 _ _ (a 3) (b 3) (by omega) (by omega),
    and_step 16 _ _ (a 2) (b 2) (by omega) (by omega),
    and_step 8 _ _ (a 1) (b 1) (by omega) (by omega)]

/-! ### The table -/

/-- One Binary-table row of `ltu` (6), `eq` (9) or `and` (14), in naturals, as
`check_tables.py` checks it. -/
def CmpRowOk (code np na nb nci nc nf : Nat) : Prop :=
  na < 256 ∧ nb < 256 ∧ nc < 256 ∧ nci ≤ 1 ∧ (np = 0 ∨ np = 1) ∧
    (code = 6 → nc = 0 ∧ nf = if na < nb then 1 else if na = nb then nci else 0) ∧
    (code = 9 → nc = 0 ∧
      nf = if np = 1 then 1 - (if nci = 0 ∧ na = nb then 0 else 1)
        else (if nci = 0 ∧ na = nb then 0 else 1)) ∧
    (code = 14 → nc = na &&& nb ∧ nf = 0)

theorem cmpRow_nf_le {code np na nb nci nc nf : Nat} (hcode : code = 6 ∨ code = 9 ∨ code = 14)
    (h : CmpRowOk code np na nb nci nc nf) : nf ≤ 1 := by
  obtain ⟨-, -, -, hci, -, h6, h9, h14⟩ := h
  rcases hcode with rfl | rfl | rfl
  · obtain ⟨-, hf⟩ := h6 rfl; rw [hf]; split
    · omega
    · split <;> omega
  · obtain ⟨-, hf⟩ := h9 rfl; rw [hf]; split <;> split <;> omega
  · obtain ⟨-, hf⟩ := h14 rfl; omega

section Table

variable {F : Type} [PilField F]

/-- One Binary-table row, `(pos, op, a, b, cin, c, flags)`: its op is below
2^10, and if it is one of `ltu`, `eq`, `and` it computes its byte. -/
def OpRow (pos op a b cin c f : F) : Prop :=
  (∃ n : Nat, n < 1024 ∧ op = (n : F)) ∧
    ∀ code, (code = 6 ∨ code = 9 ∨ code = 14) → op = ((code : Nat) : F) →
      ∃ np na nb nci nc nf : Nat, pos = (np : F) ∧ a = (na : F) ∧ b = (nb : F) ∧
        cin = (nci : F) ∧ c = (nc : F) ∧ f = (nf : F) ∧ CmpRowOk code np na nb nci nc nf

/-- Every row the prover `p` serves on the Binary table's bus is an `OpRow`. -/
def BinaryTableOps (p : Prover F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = 125 →
    OpRow (tupleAt p.arg.tuple p.x p.t j 0) (tupleAt p.arg.tuple p.x p.t j 1)
      (tupleAt p.arg.tuple p.x p.t j 2) (tupleAt p.arg.tuple p.x p.t j 3)
      (tupleAt p.arg.tuple p.x p.t j 4) (tupleAt p.arg.tuple p.x p.t j 5)
      (tupleAt p.arg.tuple p.x p.t j 6)

/-- A byte Binary looks up in the table is an `OpRow`. -/
theorem opRow_of_lookup {C T : Type} (a : BusArg F C T) (x : C) (t : T) (i : Int)
    (ps : List (Prover F)) (hl : LookupHolds a x t ps) (hbus : ∀ x t i, a.bus x t i = 125)
    (hreps : a.reps x t i ≠ 0) (hd : ∀ p ∈ ps, BinaryTableOps p) :
    OpRow (tupleAt a.tuple x t i 0) (tupleAt a.tuple x t i 1) (tupleAt a.tuple x t i 2)
      (tupleAt a.tuple x t i 3) (tupleAt a.tuple x t i 4) (tupleAt a.tuple x t i 5)
      (tupleAt a.tuple x t i 6) := by
  obtain ⟨-, -, -, -, hb⟩ := hl
  obtain ⟨p, hp, j, -, hpb, htup⟩ := hb i hreps
  rw [htup 0, htup 1, htup 2, htup 3, htup 4, htup 5, htup 6]
  exact hd p hp j (by rw [hpb, hbus])

end Table

section Decode

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

theorem lt40_16 {x y : Nat} (hx : x ≤ 1) (hy : y ≤ 1) : x + 16 * y < 2 ^ 40 := by omega
theorem lt40_64 {x y : Nat} (hx : x ≤ 1) (hy : y ≤ 1) : x + 64 * y < 2 ^ 40 := by omega

/-- A row of one of the three ops, at a middle position. -/
theorem opRow_at {pos a b cin c f : F} {code : Nat} (hcode : code = 6 ∨ code = 9 ∨ code = 14)
    (h : OpRow pos ((code : Nat) : F) a b cin c f) (n : Nat) (hn : n < 4) (hpos : pos = (n : F)) :
    ∃ na nb nci nc nf : Nat, (n = 0 ∨ n = 1) ∧ a = (na : F) ∧ b = (nb : F) ∧ cin = (nci : F) ∧
      c = (nc : F) ∧ f = (nf : F) ∧ CmpRowOk code n na nb nci nc nf := by
  obtain ⟨np, na, nb, nci, nc, nf, hp, ha, hb, hci, hc, hf, hok⟩ := h.2 code hcode rfl
  have hnp : n = np := nat_eq (by omega)
    (by rcases hok.2.2.2.2.1 with h | h <;> omega) (hpos.symm.trans hp)
  subst hnp
  exact ⟨na, nb, nci, nc, nf, hok.2.2.2.2.1, ha, hb, hci, hc, hf, hok⟩

/-- 32-bit mode would need a `b_op` of `code − 16`, below zero; no table row has it. -/
theorem mode32_zero' {bop m32 : F} {code : Nat} (hcode : code = 6 ∨ code = 9 ∨ code = 14)
    (hop : bop + 16 * m32 = ((code : Nat) : F)) (hm32 : m32 * (1 - m32) = 0)
    (hrange : ∃ n : Nat, n < 1024 ∧ bop = (n : F)) : m32 = 0 ∧ bop = ((code : Nat) : F) := by
  obtain ⟨nm, lm, em⟩ := bit_nat hm32
  obtain ⟨n, ln, en⟩ := hrange
  rcases (by omega : nm = 0 ∨ nm = 1) with h | h
  · rw [h, Lean.Grind.Semiring.natCast_zero] at em
    subst em; exact ⟨rfl, by grind⟩
  · exfalso
    rw [h, Lean.Grind.Semiring.natCast_one] at em
    subst em
    have : ((n + 16 : Nat) : F) = ((code : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_add, ← en, ← natCast_ofNat' 16]; grind
    have := nat_eq (by omega) (by omega) this
    omega

/-- **Reading a lane's eight lookups of `ltu`, `eq` or `and`.** As `binary8`
does for `add`: the lane is in 64-bit mode, its extra flag bits are 0, and
every carry is the table's flags, the last one included. What is left is the
eight rows, in naturals, chained by their carries. -/
theorem cmp_decode (code : Nat) (hcode : code = 6 ∨ code = 9 ∨ code = 14)
    (bop m32 bos mcs u r cs : F) (a b c k : Nat → F)
    (hop : bop + 16 * m32 = ((code : Nat) : F))
    (hm32 : m32 * (1 - m32) = 0) (hk7 : k 7 * (1 - k 7) = 0) (hr : r * (1 - r) = 0)
    (hu : u * (1 - u) = 0) (hcs : cs * (1 - cs) = 0)
    (e5 : bos - (m32 * (cs + 512 - bop) + bop) = 0) (e6 : mcs - m32 * cs = 0)
    (t0 : OpRow (2 * u) bop (a 0) (b 0) 0 (c 0) (k 0 + 16 * r + 32 * u))
    (t1 : OpRow 0 bop (a 1) (b 1) (k 0) (c 1) (k 1 + 16 * r + 32 * u))
    (t2 : OpRow 0 bop (a 2) (b 2) (k 1) (c 2) (k 2 + 16 * r + 32 * u))
    (t3 : OpRow m32 bop (a 3) (b 3) (k 2) (c 3) (k 3 + 16 * r + 32 * u + 64 * mcs))
    (t4 : OpRow 0 bos (a 4) (b 4) (k 3) (c 4) (k 4 + 16 * r + 32 * u + 64 * mcs))
    (t5 : OpRow 0 bos (a 5) (b 5) (k 4) (c 5) (k 5 + 16 * r + 32 * u + 64 * mcs))
    (t6 : OpRow 0 bos (a 6) (b 6) (k 5) (c 6) (k 6 + 16 * r + 32 * u + 64 * mcs))
    (t7 : OpRow (1 - m32) bos (a 7) (b 7) (k 6) (c 7) (k 7 + 16 * r + 32 * u + 64 * cs)) :
    ∃ na nb nc nf : Nat → Nat,
      (∀ i, i < 8 → a i = (na i : F) ∧ b i = (nb i : F) ∧ c i = (nc i : F)) ∧
      k 7 = (nf 7 : F) ∧
      CmpRowOk code 0 (na 0) (nb 0) 0 (nc 0) (nf 0) ∧
      (∀ i, i < 6 → CmpRowOk code 0 (na (i + 1)) (nb (i + 1)) (nf i) (nc (i + 1)) (nf (i + 1))) ∧
      CmpRowOk code 1 (na 7) (nb 7) (nf 6) (nc 7) (nf 7) := by
  obtain ⟨hm0, hbop⟩ := mode32_zero' hcode hop hm32 t0.1
  subst hm0 hbop
  have hbos : bos = ((code : Nat) : F) := by grind
  have hmcs : mcs = 0 := by grind
  subst hbos hmcs
  have z : ((0 : Nat) : F) = 0 := Lean.Grind.Semiring.natCast_zero
  -- `use_first_byte` is 0: byte 0's position is `2·use_first_byte`, a row's 0 or 1.
  obtain ⟨nu, lu, eu⟩ := bit_nat hu
  obtain ⟨na0, nb0, nci0, nc0, nf0, hp0, ea0, eb0, eci0, ec0, ef0, ok0⟩ :=
    opRow_at hcode t0 (2 * nu) (by omega)
      (by rw [eu, Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 2])
  have hnu : nu = 0 := by omega
  rw [hnu, z] at eu
  rw [hnu, Nat.mul_zero] at ok0
  subst eu
  obtain ⟨na1, nb1, nci1, nc1, nf1, -, ea1, eb1, eci1, ec1, ef1, ok1⟩ :=
    opRow_at hcode t1 0 (by decide) z.symm
  obtain ⟨na2, nb2, nci2, nc2, nf2, -, ea2, eb2, eci2, ec2, ef2, ok2⟩ :=
    opRow_at hcode t2 0 (by decide) z.symm
  obtain ⟨na3, nb3, nci3, nc3, nf3, -, ea3, eb3, eci3, ec3, ef3, ok3⟩ :=
    opRow_at hcode t3 0 (by decide) z.symm
  obtain ⟨na4, nb4, nci4, nc4, nf4, -, ea4, eb4, eci4, ec4, ef4, ok4⟩ :=
    opRow_at hcode t4 0 (by decide) z.symm
  obtain ⟨na5, nb5, nci5, nc5, nf5, -, ea5, eb5, eci5, ec5, ef5, ok5⟩ :=
    opRow_at hcode t5 0 (by decide) z.symm
  obtain ⟨na6, nb6, nci6, nc6, nf6, -, ea6, eb6, eci6, ec6, ef6, ok6⟩ :=
    opRow_at hcode t6 0 (by decide) z.symm
  obtain ⟨na7, nb7, nci7, nc7, nf7, -, ea7, eb7, eci7, ec7, ef7, ok7⟩ :=
    opRow_at hcode t7 1 (by decide) one_sub_zero
  clear t0 t1 t2 t3 t4 t5 t6 t7
  have hci0 : nci0 = 0 := nat_eq (lt40_of_le1 ok0.2.2.2.1) (by decide) (by rw [← eci0, z])
  have f0 := cmpRow_nf_le hcode ok0; have f1 := cmpRow_nf_le hcode ok1
  have f2 := cmpRow_nf_le hcode ok2; have f3 := cmpRow_nf_le hcode ok3
  have f4 := cmpRow_nf_le hcode ok4; have f5 := cmpRow_nf_le hcode ok5
  have f6 := cmpRow_nf_le hcode ok6; have f7 := cmpRow_nf_le hcode ok7
  -- `result_is_a` is 0.
  obtain ⟨nr, lr, er⟩ := bit_nat hr
  have h16 : ((nci1 + 16 * nr : Nat) : F) = (nf0 : F) := by
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 16,
      ← eci1, ← er, ← ef0, Lean.Grind.Semiring.mul_zero, Lean.Grind.Semiring.add_zero]
  have e16 := nat_eq (lt40_16 ok1.2.2.2.1 lr) (lt40_of_le1 f0) h16
  have hnr : nr = 0 := by omega
  have l01 : nci1 = nf0 := by omega
  rw [hnr, z] at er
  subst er
  simp only [Lean.Grind.Semiring.mul_zero, Lean.Grind.Semiring.add_zero] at ef0 ef1 ef2 ef3 ef4 ef5 ef6 ef7
  -- Every carry is the table's flags; the last, with `c_is_signed`, too.
  have l12 : nci2 = nf1 := nat_eq (lt40_of_le1 ok2.2.2.2.1) (lt40_of_le1 f1) (eci2.symm.trans ef1)
  have l23 : nci3 = nf2 := nat_eq (lt40_of_le1 ok3.2.2.2.1) (lt40_of_le1 f2) (eci3.symm.trans ef2)
  have l34 : nci4 = nf3 := nat_eq (lt40_of_le1 ok4.2.2.2.1) (lt40_of_le1 f3) (eci4.symm.trans ef3)
  have l45 : nci5 = nf4 := nat_eq (lt40_of_le1 ok5.2.2.2.1) (lt40_of_le1 f4) (eci5.symm.trans ef4)
  have l56 : nci6 = nf5 := nat_eq (lt40_of_le1 ok6.2.2.2.1) (lt40_of_le1 f5) (eci6.symm.trans ef5)
  have l67 : nci7 = nf6 := nat_eq (lt40_of_le1 ok7.2.2.2.1) (lt40_of_le1 f6) (eci7.symm.trans ef6)
  obtain ⟨nk, lk, ek⟩ := bit_nat hk7
  obtain ⟨ns, ls, es⟩ := bit_nat hcs
  have h64 : ((nk + 64 * ns : Nat) : F) = (nf7 : F) := by
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 64,
      ← ek, ← es, ← ef7]
  have e64 := nat_eq (lt40_64 lk ls) (lt40_of_le1 f7) h64
  have hk : nk = nf7 := by omega
  subst hci0 l01 l12 l23 l34 l45 l56 l67
  refine ⟨fun i => [na0, na1, na2, na3, na4, na5, na6, na7].getD i 0,
    fun i => [nb0, nb1, nb2, nb3, nb4, nb5, nb6, nb7].getD i 0,
    fun i => [nc0, nc1, nc2, nc3, nc4, nc5, nc6, nc7].getD i 0,
    fun i => [nci1, nci2, nci3, nci4, nci5, nci6, nci7, nf7].getD i 0, ?_, ?_, ok0, ?_, ok7⟩
  · intro i hi
    rcases (by omega : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 ∨ i = 7) with
      rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    · exact ⟨ea0, eb0, ec0⟩
    · exact ⟨ea1, eb1, ec1⟩
    · exact ⟨ea2, eb2, ec2⟩
    · exact ⟨ea3, eb3, ec3⟩
    · exact ⟨ea4, eb4, ec4⟩
    · exact ⟨ea5, eb5, ec5⟩
    · exact ⟨ea6, eb6, ec6⟩
    · exact ⟨ea7, eb7, ec7⟩
  · rw [ek, hk]; rfl
  · intro i hi
    rcases (by omega : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5) with
      rfl | rfl | rfl | rfl | rfl | rfl
    · exact ok1
    · exact ok2
    · exact ok3
    · exact ok4
    · exact ok5
    · exact ok6

end Decode

section Finish

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- A lane's eight byte columns, as the limbs of `A`, are `A`. -/
theorem bytes_of_limbs (a : Nat → F) (na : Nat → Nat) (A : BitVec 64)
    (ha : ∀ i, i < 8 → a i = (na i : F)) (la : ∀ i, i < 8 → na i < 256)
    (h0 : a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3 = lo A)
    (h1 : a 4 + 256 * a 5 + 65536 * a 6 + 16777216 * a 7 = hi A) :
    bytes8 na = A.toNat := by
  rw [(ha 0 (by decide)), (ha 1 (by decide)), (ha 2 (by decide)), (ha 3 (by decide)),
    ← limb_cast] at h0
  rw [(ha 4 (by decide)), (ha 5 (by decide)), (ha 6 (by decide)), (ha 7 (by decide)),
    ← limb_cast] at h1
  have hA := A.isLt
  have n0 := nat_eq (Nat.lt_trans (limb_lt _ _ _ _ (la 0 (by decide)) (la 1 (by decide))
    (la 2 (by decide)) (la 3 (by decide))) (by decide)) (lo_lt A) h0
  have n1 := nat_eq (Nat.lt_trans (limb_lt _ _ _ _ (la 4 (by decide)) (la 5 (by decide))
    (la 6 (by decide)) (la 7 (by decide))) (by decide)) (hi_lt A) h1
  unfold bytes8; omega

omit [Lean.Grind.IsCharP F goldilocks] in
/-- Result bytes and a last carry that are, in naturals, `X`'s limbs and the
flag bit, are so in the field. -/
theorem finish_cmp (c k : Nat → F) (nc : Nat → Nat) (nf7 : Nat) (X : BitVec 64) (fl : Bool)
    (hc : ∀ i, i < 8 → c i = (nc i : F)) (hk : k 7 = (nf7 : F))
    (n0 : nc 0 + 256 * nc 1 + 65536 * nc 2 + 16777216 * nc 3 + nf7 = X.toNat % 2 ^ 32)
    (n1 : nc 4 + 256 * nc 5 + 65536 * nc 6 + 16777216 * nc 7 = X.toNat / 2 ^ 32)
    (nfl : nf7 = if fl then 1 else 0) :
    c 0 + 256 * c 1 + 65536 * c 2 + 16777216 * c 3 + k 7 = lo X ∧
      c 4 + 256 * c 5 + 65536 * c 6 + 16777216 * c 7 = hi X ∧ k 7 = bit fl := by
  rw [hc 0 (by decide), hc 1 (by decide), hc 2 (by decide), hc 3 (by decide), hc 4 (by decide),
    hc 5 (by decide), hc 6 (by decide), hc 7 (by decide), hk, ← limb_cast, ← limb_cast,
    ← Lean.Grind.Semiring.natCast_add, lo, hi, n0, n1]
  refine ⟨rfl, rfl, ?_⟩
  rw [nfl]; cases fl <;> simp [bit, Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.natCast_one]

end Finish

section Ops

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The bytes of eight rows are bytes. -/
theorem rows_bounds {code : Nat} {na nb nc nf : Nat → Nat}
    (ok0 : CmpRowOk code 0 (na 0) (nb 0) 0 (nc 0) (nf 0))
    (okm : ∀ i, i < 6 → CmpRowOk code 0 (na (i + 1)) (nb (i + 1)) (nf i) (nc (i + 1)) (nf (i + 1)))
    (ok7 : CmpRowOk code 1 (na 7) (nb 7) (nf 6) (nc 7) (nf 7)) :
    ∀ i, i < 8 → na i < 256 ∧ nb i < 256 ∧ nc i < 256 := by
  intro i hi
  cases i with
  | zero => exact ⟨ok0.1, ok0.2.1, ok0.2.2.1⟩
  | succ j =>
    by_cases h7 : j = 6
    · subst h7; exact ⟨ok7.1, ok7.2.1, ok7.2.2.1⟩
    · have o := okm j (by omega); exact ⟨o.1, o.2.1, o.2.2.1⟩

/-- Each row's rule, for any of its fields that a rule `P` reads. -/
theorem rows_rule {code : Nat} {na nb nc nf : Nat → Nat} (P : Nat → Nat → Nat → Nat → Nat → Nat → Prop)
    (hP : ∀ np x y ci z f, CmpRowOk code np x y ci z f → P np x y ci z f)
    (ok0 : CmpRowOk code 0 (na 0) (nb 0) 0 (nc 0) (nf 0))
    (okm : ∀ i, i < 6 → CmpRowOk code 0 (na (i + 1)) (nb (i + 1)) (nf i) (nc (i + 1)) (nf (i + 1)))
    (ok7 : CmpRowOk code 1 (na 7) (nb 7) (nf 6) (nc 7) (nf 7)) :
    P 0 (na 0) (nb 0) 0 (nc 0) (nf 0) ∧
      (∀ i, i < 6 → P 0 (na (i + 1)) (nb (i + 1)) (nf i) (nc (i + 1)) (nf (i + 1))) ∧
      P 1 (na 7) (nb 7) (nf 6) (nc 7) (nf 7) :=
  ⟨hP _ _ _ _ _ _ ok0, fun i hi => hP _ _ _ _ _ _ (okm i hi), hP _ _ _ _ _ _ ok7⟩

/-- **Binary computes `ltu`.** -/
theorem binaryLtu8 (bop m32 bos mcs u r cs : F) (a b c k : Nat → F) (A B : BitVec 64)
    (hop : bop + 16 * m32 = ((6 : Nat) : F))
    (hm32 : m32 * (1 - m32) = 0) (hk7 : k 7 * (1 - k 7) = 0) (hr : r * (1 - r) = 0)
    (hu : u * (1 - u) = 0) (hcs : cs * (1 - cs) = 0)
    (e5 : bos - (m32 * (cs + 512 - bop) + bop) = 0) (e6 : mcs - m32 * cs = 0)
    (t0 : OpRow (2 * u) bop (a 0) (b 0) 0 (c 0) (k 0 + 16 * r + 32 * u))
    (t1 : OpRow 0 bop (a 1) (b 1) (k 0) (c 1) (k 1 + 16 * r + 32 * u))
    (t2 : OpRow 0 bop (a 2) (b 2) (k 1) (c 2) (k 2 + 16 * r + 32 * u))
    (t3 : OpRow m32 bop (a 3) (b 3) (k 2) (c 3) (k 3 + 16 * r + 32 * u + 64 * mcs))
    (t4 : OpRow 0 bos (a 4) (b 4) (k 3) (c 4) (k 4 + 16 * r + 32 * u + 64 * mcs))
    (t5 : OpRow 0 bos (a 5) (b 5) (k 4) (c 5) (k 5 + 16 * r + 32 * u + 64 * mcs))
    (t6 : OpRow 0 bos (a 6) (b 6) (k 5) (c 6) (k 6 + 16 * r + 32 * u + 64 * mcs))
    (t7 : OpRow (1 - m32) bos (a 7) (b 7) (k 6) (c 7) (k 7 + 16 * r + 32 * u + 64 * cs))
    (ha0 : a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3 = lo A)
    (ha1 : a 4 + 256 * a 5 + 65536 * a 6 + 16777216 * a 7 = hi A)
    (hb0 : b 0 + 256 * b 1 + 65536 * b 2 + 16777216 * b 3 = lo B)
    (hb1 : b 4 + 256 * b 5 + 65536 * b 6 + 16777216 * b 7 = hi B) :
    c 0 + 256 * c 1 + 65536 * c 2 + 16777216 * c 3 + k 7 = lo (ltuSem A B).1 ∧
      c 4 + 256 * c 5 + 65536 * c 6 + 16777216 * c 7 = hi (ltuSem A B).1 ∧
      k 7 = bit (ltuSem A B).2 := by
  obtain ⟨na, nb, nc, nf, hv, hk, ok0, okm, ok7⟩ :=
    cmp_decode 6 (by decide) bop m32 bos mcs u r cs a b c k hop hm32 hk7 hr hu hcs e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7
  have hb := rows_bounds ok0 okm ok7
  have hA := bytes_of_limbs a na A (fun i hi => (hv i hi).1) (fun i hi => (hb i hi).1) ha0 ha1
  have hB := bytes_of_limbs b nb B (fun i hi => (hv i hi).2.1) (fun i hi => (hb i hi).2.1) hb0 hb1
  obtain ⟨r0, rm, r7⟩ := rows_rule (code := 6)
    (fun _ x y ci z f => z = 0 ∧ f = if x < y then 1 else if x = y then ci else 0)
    (fun _ _ _ _ _ _ h => h.2.2.2.2.2.1 rfl) ok0 okm ok7
  have lt := ltu8 na nb nf (fun i hi => (hb i hi).1) (fun i hi => (hb i hi).2.1)
    (by rw [r0.2]; split <;> (try split) <;> omega)
    (fun i hi => by
      rcases (by omega : i < 6 ∨ i = 6) with h | rfl
      · exact (rm i h).2
      · exact r7.2)
  have z0 : nc 0 = 0 := r0.1
  have z1 : nc 1 = 0 := (rm 0 (by decide)).1
  have z2 : nc 2 = 0 := (rm 1 (by decide)).1
  have z3 : nc 3 = 0 := (rm 2 (by decide)).1
  have z4 : nc 4 = 0 := (rm 3 (by decide)).1
  have z5 : nc 5 = 0 := (rm 4 (by decide)).1
  have z6 : nc 6 = 0 := (rm 5 (by decide)).1
  have z7 : nc 7 = 0 := r7.1
  rw [hA, hB] at lt
  have n0 : nc 0 + 256 * nc 1 + 65536 * nc 2 + 16777216 * nc 3 + nf 7 =
      (ltuSem A B).1.toNat % 2 ^ 32 := by
    rw [z0, z1, z2, z3, lt]; unfold ltuSem; split <;> rfl
  have n1 : nc 4 + 256 * nc 5 + 65536 * nc 6 + 16777216 * nc 7 = (ltuSem A B).1.toNat / 2 ^ 32 := by
    rw [z4, z5, z6, z7]; unfold ltuSem; split <;> rfl
  have nfl : nf 7 = if (ltuSem A B).2 then 1 else 0 := by
    rw [lt]; unfold ltuSem; split <;> rfl
  exact finish_cmp c k nc (nf 7) _ _ (fun i hi => (hv i hi).2.2) hk n0 n1 nfl

/-- **Binary computes `eq`.** -/
theorem binaryEq8 (bop m32 bos mcs u r cs : F) (a b c k : Nat → F) (A B : BitVec 64)
    (hop : bop + 16 * m32 = ((9 : Nat) : F))
    (hm32 : m32 * (1 - m32) = 0) (hk7 : k 7 * (1 - k 7) = 0) (hr : r * (1 - r) = 0)
    (hu : u * (1 - u) = 0) (hcs : cs * (1 - cs) = 0)
    (e5 : bos - (m32 * (cs + 512 - bop) + bop) = 0) (e6 : mcs - m32 * cs = 0)
    (t0 : OpRow (2 * u) bop (a 0) (b 0) 0 (c 0) (k 0 + 16 * r + 32 * u))
    (t1 : OpRow 0 bop (a 1) (b 1) (k 0) (c 1) (k 1 + 16 * r + 32 * u))
    (t2 : OpRow 0 bop (a 2) (b 2) (k 1) (c 2) (k 2 + 16 * r + 32 * u))
    (t3 : OpRow m32 bop (a 3) (b 3) (k 2) (c 3) (k 3 + 16 * r + 32 * u + 64 * mcs))
    (t4 : OpRow 0 bos (a 4) (b 4) (k 3) (c 4) (k 4 + 16 * r + 32 * u + 64 * mcs))
    (t5 : OpRow 0 bos (a 5) (b 5) (k 4) (c 5) (k 5 + 16 * r + 32 * u + 64 * mcs))
    (t6 : OpRow 0 bos (a 6) (b 6) (k 5) (c 6) (k 6 + 16 * r + 32 * u + 64 * mcs))
    (t7 : OpRow (1 - m32) bos (a 7) (b 7) (k 6) (c 7) (k 7 + 16 * r + 32 * u + 64 * cs))
    (ha0 : a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3 = lo A)
    (ha1 : a 4 + 256 * a 5 + 65536 * a 6 + 16777216 * a 7 = hi A)
    (hb0 : b 0 + 256 * b 1 + 65536 * b 2 + 16777216 * b 3 = lo B)
    (hb1 : b 4 + 256 * b 5 + 65536 * b 6 + 16777216 * b 7 = hi B) :
    c 0 + 256 * c 1 + 65536 * c 2 + 16777216 * c 3 + k 7 = lo (eqSem A B).1 ∧
      c 4 + 256 * c 5 + 65536 * c 6 + 16777216 * c 7 = hi (eqSem A B).1 ∧
      k 7 = bit (eqSem A B).2 := by
  obtain ⟨na, nb, nc, nf, hv, hk, ok0, okm, ok7⟩ :=
    cmp_decode 9 (by decide) bop m32 bos mcs u r cs a b c k hop hm32 hk7 hr hu hcs e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7
  have hb := rows_bounds ok0 okm ok7
  have hA := bytes_of_limbs a na A (fun i hi => (hv i hi).1) (fun i hi => (hb i hi).1) ha0 ha1
  have hB := bytes_of_limbs b nb B (fun i hi => (hv i hi).2.1) (fun i hi => (hb i hi).2.1) hb0 hb1
  obtain ⟨r0, rm, r7⟩ := rows_rule (code := 9)
    (fun np x y ci z f => z = 0 ∧
      f = if np = 1 then 1 - (if ci = 0 ∧ x = y then 0 else 1) else (if ci = 0 ∧ x = y then 0 else 1))
    (fun _ _ _ _ _ _ h => h.2.2.2.2.2.2.1 rfl) ok0 okm ok7
  have eq := eq8 na nb (fun i => if i = 7 then (if nf 6 = 0 ∧ na 7 = nb 7 then 0 else 1) else nf i)
    (nf 7) (fun i hi => (hb i hi).1) (fun i hi => (hb i hi).2.1)
    (by simpa using r0.2)
    (fun i hi => by
      rcases (by omega : i < 6 ∨ i = 6) with h | rfl
      · have := (rm i h).2
        rw [if_neg (by decide : (0 : Nat) ≠ 1)] at this
        dsimp only
        rw [if_neg (show i + 1 ≠ 7 by omega), if_neg (show i ≠ 7 by omega)]
        exact this
      · simp)
    (by simpa using r7.2)
  have z0 : nc 0 = 0 := r0.1
  have z1 : nc 1 = 0 := (rm 0 (by decide)).1
  have z2 : nc 2 = 0 := (rm 1 (by decide)).1
  have z3 : nc 3 = 0 := (rm 2 (by decide)).1
  have z4 : nc 4 = 0 := (rm 3 (by decide)).1
  have z5 : nc 5 = 0 := (rm 4 (by decide)).1
  have z6 : nc 6 = 0 := (rm 5 (by decide)).1
  have z7 : nc 7 = 0 := r7.1
  rw [hA, hB] at eq
  have hiff : A.toNat = B.toNat ↔ A = B := ⟨BitVec.eq_of_toNat_eq, fun h => h ▸ rfl⟩
  replace eq : nf 7 = if A = B then 1 else 0 := by
    rw [eq]
    by_cases h : A = B
    · rw [if_pos (hiff.mpr h), if_pos h]
    · rw [if_neg (fun e => h (hiff.mp e)), if_neg h]
  have n0 : nc 0 + 256 * nc 1 + 65536 * nc 2 + 16777216 * nc 3 + nf 7 =
      (eqSem A B).1.toNat % 2 ^ 32 := by
    rw [z0, z1, z2, z3, eq]; unfold eqSem; split <;> rfl
  have n1 : nc 4 + 256 * nc 5 + 65536 * nc 6 + 16777216 * nc 7 = (eqSem A B).1.toNat / 2 ^ 32 := by
    rw [z4, z5, z6, z7]; unfold eqSem; split <;> rfl
  have nfl : nf 7 = if (eqSem A B).2 then 1 else 0 := by
    rw [eq]; unfold eqSem; split <;> rfl
  exact finish_cmp c k nc (nf 7) _ _ (fun i hi => (hv i hi).2.2) hk n0 n1 nfl

/-- **Binary computes `and`.** -/
theorem binaryAnd8 (bop m32 bos mcs u r cs : F) (a b c k : Nat → F) (A B : BitVec 64)
    (hop : bop + 16 * m32 = ((14 : Nat) : F))
    (hm32 : m32 * (1 - m32) = 0) (hk7 : k 7 * (1 - k 7) = 0) (hr : r * (1 - r) = 0)
    (hu : u * (1 - u) = 0) (hcs : cs * (1 - cs) = 0)
    (e5 : bos - (m32 * (cs + 512 - bop) + bop) = 0) (e6 : mcs - m32 * cs = 0)
    (t0 : OpRow (2 * u) bop (a 0) (b 0) 0 (c 0) (k 0 + 16 * r + 32 * u))
    (t1 : OpRow 0 bop (a 1) (b 1) (k 0) (c 1) (k 1 + 16 * r + 32 * u))
    (t2 : OpRow 0 bop (a 2) (b 2) (k 1) (c 2) (k 2 + 16 * r + 32 * u))
    (t3 : OpRow m32 bop (a 3) (b 3) (k 2) (c 3) (k 3 + 16 * r + 32 * u + 64 * mcs))
    (t4 : OpRow 0 bos (a 4) (b 4) (k 3) (c 4) (k 4 + 16 * r + 32 * u + 64 * mcs))
    (t5 : OpRow 0 bos (a 5) (b 5) (k 4) (c 5) (k 5 + 16 * r + 32 * u + 64 * mcs))
    (t6 : OpRow 0 bos (a 6) (b 6) (k 5) (c 6) (k 6 + 16 * r + 32 * u + 64 * mcs))
    (t7 : OpRow (1 - m32) bos (a 7) (b 7) (k 6) (c 7) (k 7 + 16 * r + 32 * u + 64 * cs))
    (ha0 : a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3 = lo A)
    (ha1 : a 4 + 256 * a 5 + 65536 * a 6 + 16777216 * a 7 = hi A)
    (hb0 : b 0 + 256 * b 1 + 65536 * b 2 + 16777216 * b 3 = lo B)
    (hb1 : b 4 + 256 * b 5 + 65536 * b 6 + 16777216 * b 7 = hi B) :
    c 0 + 256 * c 1 + 65536 * c 2 + 16777216 * c 3 + k 7 = lo (andSem A B).1 ∧
      c 4 + 256 * c 5 + 65536 * c 6 + 16777216 * c 7 = hi (andSem A B).1 ∧
      k 7 = bit (andSem A B).2 := by
  obtain ⟨na, nb, nc, nf, hv, hk, ok0, okm, ok7⟩ :=
    cmp_decode 14 (by decide) bop m32 bos mcs u r cs a b c k hop hm32 hk7 hr hu hcs e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7
  have hb := rows_bounds ok0 okm ok7
  have hA := bytes_of_limbs a na A (fun i hi => (hv i hi).1) (fun i hi => (hb i hi).1) ha0 ha1
  have hB := bytes_of_limbs b nb B (fun i hi => (hv i hi).2.1) (fun i hi => (hb i hi).2.1) hb0 hb1
  obtain ⟨r0, rm, r7⟩ := rows_rule (code := 14)
    (fun _ x y _ z f => z = x &&& y ∧ f = 0)
    (fun _ _ _ _ _ _ h => h.2.2.2.2.2.2.2 rfl) ok0 okm ok7
  have hand := and8 na nb nc (fun i hi => (hb i hi).1) (fun i hi => (hb i hi).2.1)
    (fun i hi => by
      rcases (by omega : i = 0 ∨ i = 7 ∨ (1 ≤ i ∧ i ≤ 6)) with rfl | rfl | ⟨h1, h6⟩
      · exact r0.1
      · exact r7.1
      · have := (rm (i - 1) (by omega)).1
        rwa [Nat.sub_add_cancel h1] at this)
  rw [hA, hB, ← BitVec.toNat_and] at hand
  have hl := limb_lt _ _ _ _ (hb 0 (by decide)).2.2 (hb 1 (by decide)).2.2 (hb 2 (by decide)).2.2
    (hb 3 (by decide)).2.2
  have hh := limb_lt _ _ _ _ (hb 4 (by decide)).2.2 (hb 5 (by decide)).2.2 (hb 6 (by decide)).2.2
    (hb 7 (by decide)).2.2
  have f7 : nf 7 = 0 := r7.2
  unfold bytes8 at hand
  have n0 : nc 0 + 256 * nc 1 + 65536 * nc 2 + 16777216 * nc 3 + nf 7 =
      (andSem A B).1.toNat % 2 ^ 32 := by
    simp only [andSem]; omega
  have n1 : nc 4 + 256 * nc 5 + 65536 * nc 6 + 16777216 * nc 7 = (andSem A B).1.toNat / 2 ^ 32 := by
    simp only [andSem]; omega
  have nfl : nf 7 = if (andSem A B).2 then 1 else 0 := by simp [andSem, f7]
  exact finish_cmp c k nc (nf 7) _ _ (fun i hi => (hv i hi).2.2) hk n0 n1 nfl

end Ops

section Lanes

/-! ### One instance per lane

The same lookups and helper constraints as the `add` instances in
`Binary.lean`; what is assumed of the table is `BinaryTableOps`, checked by
`check_tables.py`. Each lane proves all three ops at every row. -/

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

theorem Binary_bus21_cmp {x : Binary.Ctx F} {t : Binary.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, Binary.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds Binary.bus13 x t (binaryTable xv tv))
    (l1 : LookupHolds Binary.bus14 x t (binaryTable xv tv))
    (l2 : LookupHolds Binary.bus15 x t (binaryTable xv tv))
    (l3 : LookupHolds Binary.bus16 x t (binaryTable xv tv))
    (l4 : LookupHolds Binary.bus17 x t (binaryTable xv tv))
    (l5 : LookupHolds Binary.bus18 x t (binaryTable xv tv))
    (l6 : LookupHolds Binary.bus19 x t (binaryTable xv tv))
    (l7 : LookupHolds Binary.bus20 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨Binary.bus21, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨Binary.bus21, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨Binary.bus21, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup Binary.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup Binary.bus14 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup Binary.bus15 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup Binary.bus16 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup Binary.bus17 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup Binary.bus18 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup Binary.bus19 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup Binary.bus20 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, Binary.bus13, Binary.bus14, Binary.bus15, Binary.bus16, Binary.bus17, Binary.bus18, Binary.bus19, Binary.bus20, Binary.Binary_mode64_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c5
  have e6 := (h j).c6
  simp only [Binary.c5, Binary.c6] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, Binary.bus21, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

theorem BinaryLarge_bus21_cmp {x : BinaryLarge.Ctx F} {t : BinaryLarge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryLarge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds BinaryLarge.bus13 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryLarge.bus14 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryLarge.bus15 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryLarge.bus16 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryLarge.bus17 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryLarge.bus18 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryLarge.bus19 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryLarge.bus20 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨BinaryLarge.bus21, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨BinaryLarge.bus21, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨BinaryLarge.bus21, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup BinaryLarge.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup BinaryLarge.bus14 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup BinaryLarge.bus15 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup BinaryLarge.bus16 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup BinaryLarge.bus17 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup BinaryLarge.bus18 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup BinaryLarge.bus19 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup BinaryLarge.bus20 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryLarge.bus13, BinaryLarge.bus14, BinaryLarge.bus15, BinaryLarge.bus16, BinaryLarge.bus17, BinaryLarge.bus18, BinaryLarge.bus19, BinaryLarge.bus20, BinaryLarge.BinaryLarge_mode64_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c5
  have e6 := (h j).c6
  simp only [BinaryLarge.c5, BinaryLarge.c6] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, BinaryLarge.bus21, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

theorem BinaryLarge_bus32_cmp {x : BinaryLarge.Ctx F} {t : BinaryLarge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryLarge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds BinaryLarge.bus24 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryLarge.bus25 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryLarge.bus26 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryLarge.bus27 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryLarge.bus28 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryLarge.bus29 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryLarge.bus30 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryLarge.bus31 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨BinaryLarge.bus32, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨BinaryLarge.bus32, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨BinaryLarge.bus32, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup BinaryLarge.bus24 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup BinaryLarge.bus25 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup BinaryLarge.bus26 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup BinaryLarge.bus27 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup BinaryLarge.bus28 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup BinaryLarge.bus29 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup BinaryLarge.bus30 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup BinaryLarge.bus31 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryLarge.bus24, BinaryLarge.bus25, BinaryLarge.bus26, BinaryLarge.bus27, BinaryLarge.bus28, BinaryLarge.bus29, BinaryLarge.bus30, BinaryLarge.bus31, BinaryLarge.BinaryLarge_mode64_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c12
  have e6 := (h j).c13
  simp only [BinaryLarge.c12, BinaryLarge.c13] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, BinaryLarge.bus32, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
      ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
      ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
      (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
      (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
      (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
      ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
      ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
      (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
      (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
      (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
      ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
      ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
      (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
      (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
      (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

theorem BinaryHuge_bus21_cmp {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds BinaryHuge.bus13 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus14 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus15 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus16 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus17 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus18 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus19 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus20 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨BinaryHuge.bus21, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨BinaryHuge.bus21, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨BinaryHuge.bus21, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup BinaryHuge.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup BinaryHuge.bus14 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup BinaryHuge.bus15 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup BinaryHuge.bus16 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup BinaryHuge.bus17 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup BinaryHuge.bus18 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup BinaryHuge.bus19 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup BinaryHuge.bus20 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus13, BinaryHuge.bus14, BinaryHuge.bus15, BinaryHuge.bus16, BinaryHuge.bus17, BinaryHuge.bus18, BinaryHuge.bus19, BinaryHuge.bus20, BinaryHuge.BinaryHuge_mode64_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c5
  have e6 := (h j).c6
  simp only [BinaryHuge.c5, BinaryHuge.c6] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, BinaryHuge.bus21, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
      ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
      ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
      (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
      (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
      (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

theorem BinaryHuge_bus32_cmp {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds BinaryHuge.bus24 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus25 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus26 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus27 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus28 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus29 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus30 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus31 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨BinaryHuge.bus32, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨BinaryHuge.bus32, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨BinaryHuge.bus32, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup BinaryHuge.bus24 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup BinaryHuge.bus25 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup BinaryHuge.bus26 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup BinaryHuge.bus27 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup BinaryHuge.bus28 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup BinaryHuge.bus29 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup BinaryHuge.bus30 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup BinaryHuge.bus31 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus24, BinaryHuge.bus25, BinaryHuge.bus26, BinaryHuge.bus27, BinaryHuge.bus28, BinaryHuge.bus29, BinaryHuge.bus30, BinaryHuge.bus31, BinaryHuge.BinaryHuge_mode64_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c12
  have e6 := (h j).c13
  simp only [BinaryHuge.c12, BinaryHuge.c13] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, BinaryHuge.bus32, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
      ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
      ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
      (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
      (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
      (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
      ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
      ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
      (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
      (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
      (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
      ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
      ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
      (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
      (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
      (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

theorem BinaryHuge_bus43_cmp {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds BinaryHuge.bus35 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus36 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus37 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus38 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus39 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus40 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus41 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus42 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨BinaryHuge.bus43, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨BinaryHuge.bus43, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨BinaryHuge.bus43, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup BinaryHuge.bus35 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup BinaryHuge.bus36 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup BinaryHuge.bus37 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup BinaryHuge.bus38 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup BinaryHuge.bus39 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup BinaryHuge.bus40 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup BinaryHuge.bus41 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup BinaryHuge.bus42 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus35, BinaryHuge.bus36, BinaryHuge.bus37, BinaryHuge.bus38, BinaryHuge.bus39, BinaryHuge.bus40, BinaryHuge.bus41, BinaryHuge.bus42, BinaryHuge.BinaryHuge_mode64_2, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c19
  have e6 := (h j).c20
  simp only [BinaryHuge.c19, BinaryHuge.c20] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, BinaryHuge.bus43, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 2) ((t j).mode32 2) ((t j).b_op_or_sext 2)
      ((t j).mode32_and_c_is_signed 2) ((t j).use_first_byte 2) ((t j).result_is_a 2)
      ((t j).c_is_signed 2) (fun k => (t j).free_in_a 2 k) (fun k => (t j).free_in_b 2 k)
      (fun k => (t j).free_in_c 2 k) (fun k => (t j).carry 2 k) A B hop
      (h j).bool_mode32_2 (h j).bool_carry_2_7 (h j).bool_result_is_a_2
      (h j).bool_use_first_byte_2 (h j).bool_c_is_signed_2 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 2) ((t j).mode32 2) ((t j).b_op_or_sext 2)
      ((t j).mode32_and_c_is_signed 2) ((t j).use_first_byte 2) ((t j).result_is_a 2)
      ((t j).c_is_signed 2) (fun k => (t j).free_in_a 2 k) (fun k => (t j).free_in_b 2 k)
      (fun k => (t j).free_in_c 2 k) (fun k => (t j).carry 2 k) A B hop
      (h j).bool_mode32_2 (h j).bool_carry_2_7 (h j).bool_result_is_a_2
      (h j).bool_use_first_byte_2 (h j).bool_c_is_signed_2 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 2) ((t j).mode32 2) ((t j).b_op_or_sext 2)
      ((t j).mode32_and_c_is_signed 2) ((t j).use_first_byte 2) ((t j).result_is_a 2)
      ((t j).c_is_signed 2) (fun k => (t j).free_in_a 2 k) (fun k => (t j).free_in_b 2 k)
      (fun k => (t j).free_in_c 2 k) (fun k => (t j).carry 2 k) A B hop
      (h j).bool_mode32_2 (h j).bool_carry_2_7 (h j).bool_result_is_a_2
      (h j).bool_use_first_byte_2 (h j).bool_c_is_signed_2 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

theorem BinaryHuge_bus54_cmp {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableOps p)
    (l0 : LookupHolds BinaryHuge.bus46 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus47 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus48 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus49 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus50 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus51 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus52 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus53 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 6 ltuSem ⟨BinaryHuge.bus54, x, t⟩ j ∧ ProvesAt 9 eqSem ⟨BinaryHuge.bus54, x, t⟩ j ∧
      ProvesAt 14 andSem ⟨BinaryHuge.bus54, x, t⟩ j := by
  intro j
  have t0 := opRow_of_lookup BinaryHuge.bus46 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := opRow_of_lookup BinaryHuge.bus47 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := opRow_of_lookup BinaryHuge.bus48 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := opRow_of_lookup BinaryHuge.bus49 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := opRow_of_lookup BinaryHuge.bus50 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := opRow_of_lookup BinaryHuge.bus51 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := opRow_of_lookup BinaryHuge.bus52 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := opRow_of_lookup BinaryHuge.bus53 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus46, BinaryHuge.bus47, BinaryHuge.bus48, BinaryHuge.bus49, BinaryHuge.bus50, BinaryHuge.bus51, BinaryHuge.bus52, BinaryHuge.bus53, BinaryHuge.BinaryHuge_mode64_3, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c26
  have e6 := (h j).c27
  simp only [BinaryHuge.c26, BinaryHuge.c27] at e5 e6
  refine ⟨?_, ?_, ?_⟩ <;> intro hop A B ha0 ha1 hb0 hb1 <;>
    simp only [tupleAt, BinaryHuge.bus54, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  · exact binaryLtu8 ((t j).b_op 3) ((t j).mode32 3) ((t j).b_op_or_sext 3)
      ((t j).mode32_and_c_is_signed 3) ((t j).use_first_byte 3) ((t j).result_is_a 3)
      ((t j).c_is_signed 3) (fun k => (t j).free_in_a 3 k) (fun k => (t j).free_in_b 3 k)
      (fun k => (t j).free_in_c 3 k) (fun k => (t j).carry 3 k) A B hop
      (h j).bool_mode32_3 (h j).bool_carry_3_7 (h j).bool_result_is_a_3
      (h j).bool_use_first_byte_3 (h j).bool_c_is_signed_3 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryEq8 ((t j).b_op 3) ((t j).mode32 3) ((t j).b_op_or_sext 3)
      ((t j).mode32_and_c_is_signed 3) ((t j).use_first_byte 3) ((t j).result_is_a 3)
      ((t j).c_is_signed 3) (fun k => (t j).free_in_a 3 k) (fun k => (t j).free_in_b 3 k)
      (fun k => (t j).free_in_c 3 k) (fun k => (t j).carry 3 k) A B hop
      (h j).bool_mode32_3 (h j).bool_carry_3_7 (h j).bool_result_is_a_3
      (h j).bool_use_first_byte_3 (h j).bool_c_is_signed_3 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  · exact binaryAnd8 ((t j).b_op 3) ((t j).mode32 3) ((t j).b_op_or_sext 3)
      ((t j).mode32_and_c_is_signed 3) ((t j).use_first_byte 3) ((t j).result_is_a 3)
      ((t j).c_is_signed 3) (fun k => (t j).free_in_a 3 k) (fun k => (t j).free_in_b 3 k)
      (fun k => (t j).free_in_c 3 k) (fun k => (t j).carry 3 k) A B hop
      (h j).bool_mode32_3 (h j).bool_carry_3_7 (h j).bool_result_is_a_3
      (h j).bool_use_first_byte_3 (h j).bool_c_is_signed_3 e5 e6
      t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1

end Lanes

end Correspondence
