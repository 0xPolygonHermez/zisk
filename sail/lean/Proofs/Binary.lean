/-
  Binary computes `add`, byte by byte.

  The general Binary AIR is the third kind of prover of an `add` (after the
  dedicated adders and the FROPS tables). It proves every binary op the same
  way: eight byte lookups into the Binary table, each `(pos, op, a, b, cin, c,
  flags)`, chained by the carry, which goes out of one byte as part of its
  flags and into the next as its `cin`. Whether that adds correctly is a fact
  about the table, so the argument has two halves:

  * `AddRow`: what one Binary-table row with op `add` is — bytes below 256, a
    carry-in of 0 or 1, and the byte computed: a middle byte has
    `c + 256·cout = a + b + cin` with the carry as its flags, the last byte
    `c = (a + b + cin) mod 256` with the carry discarded and only `c`'s sign
    bit in its flags. Every row also has an op other than −6. `check_tables.py`
    checks all of it against the table the setup commits (`BinaryTableAdds`).
  * `binary8`: from eight such rows, Binary's own constraints, and the operands
    being the limbs of `A` and `B`, the `c` Binary sends is `A + B`, and its
    flag — the last carry — is 0.

  Two things in Binary make the second half more than arithmetic. Its op is
  `b_op + 16·mode32`, so an op of 10 could also be `b_op = −6` in 32-bit mode;
  no table row has that op, so `mode32 = 0`. And its carry columns are not
  range-checked: a carry is pinned only as part of a byte's flags, which also
  hold `16·result_is_a + 32·use_first_byte + …`, and as the next byte's `cin`.
  Since an `add` row's carry-in is 0 or 1 and its flags at most 1, those extra
  flag bits must be 0 and each carry is the table's.
-/
import Proofs.Adders
import Pil.Binary
import Pil.BinaryLarge
import Pil.BinaryHuge
import Pil.VirtualTableZisk1

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

section Table

variable {F : Type} [PilField F]

/-- One Binary-table row, `(pos, op, a, b, cin, c, flags)`, as `check_tables.py`
checks it: its op is not −6, and if its op is `add` it computes one byte. -/
def AddRow (pos op a b cin c f : F) : Prop :=
  op ≠ -6 ∧
    (op = 10 → ∃ np na nb nci nc nf : Nat, na < 256 ∧ nb < 256 ∧ nc < 256 ∧ nci ≤ 1 ∧
      pos = (np : F) ∧ a = (na : F) ∧ b = (nb : F) ∧ cin = (nci : F) ∧ c = (nc : F) ∧ f = (nf : F) ∧
      ((np = 0 ∧ nf ≤ 1 ∧ nc + 256 * nf = na + nb + nci) ∨
        (np = 1 ∧ (nf = 0 ∨ nf = 64) ∧ nc = (na + nb + nci) % 256)))

/-- Every row the prover `p` serves on the Binary table's bus (125) is an
`AddRow`: the table's contents, checked by `check_tables.py`. -/
def BinaryTableAdds (p : Prover F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = 125 →
    AddRow (tupleAt p.arg.tuple p.x p.t j 0) (tupleAt p.arg.tuple p.x p.t j 1)
      (tupleAt p.arg.tuple p.x p.t j 2) (tupleAt p.arg.tuple p.x p.t j 3)
      (tupleAt p.arg.tuple p.x p.t j 4) (tupleAt p.arg.tuple p.x p.t j 5)
      (tupleAt p.arg.tuple p.x p.t j 6)

/-- The provers of the Binary table: the five arguments of the merged virtual
table `VirtualTableZisk1` that serve bus 125. -/
def binaryTable (xv : Pil.Zisk.VirtualTableZisk1.Ctx F) (tv : Pil.Zisk.VirtualTableZisk1.Trace F) :
    List (Prover F) :=
  [⟨Pil.Zisk.VirtualTableZisk1.bus19, xv, tv⟩, ⟨Pil.Zisk.VirtualTableZisk1.bus20, xv, tv⟩,
   ⟨Pil.Zisk.VirtualTableZisk1.bus21, xv, tv⟩, ⟨Pil.Zisk.VirtualTableZisk1.bus22, xv, tv⟩,
   ⟨Pil.Zisk.VirtualTableZisk1.bus23, xv, tv⟩]

/-- A byte Binary looks up in the table is an `AddRow`. -/
theorem addRow_of_lookup {C T : Type} (a : BusArg F C T) (x : C) (t : T) (i : Int)
    (ps : List (Prover F)) (hl : LookupHolds a x t ps) (hbus : ∀ x t i, a.bus x t i = 125)
    (hreps : a.reps x t i ≠ 0) (hd : ∀ p ∈ ps, BinaryTableAdds p) :
    AddRow (tupleAt a.tuple x t i 0) (tupleAt a.tuple x t i 1) (tupleAt a.tuple x t i 2)
      (tupleAt a.tuple x t i 3) (tupleAt a.tuple x t i 4) (tupleAt a.tuple x t i 5)
      (tupleAt a.tuple x t i 6) := by
  obtain ⟨-, -, -, -, hb⟩ := hl
  obtain ⟨p, hp, j, -, hpb, htup⟩ := hb i hreps
  rw [htup 0, htup 1, htup 2, htup 3, htup 4, htup 5, htup 6]
  exact hd p hp j (by rw [hpb, hbus])

end Table

section Algebra

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A bit, as a natural number 0 or 1. -/
theorem bit_nat {s : F} (h : s * (1 - s) = 0) : ∃ n : Nat, n ≤ 1 ∧ s = (n : F) := by
  obtain ⟨b, hb⟩ := bool_of_idem h
  refine ⟨if b then 1 else 0, by split <;> omega, ?_⟩
  rw [hb, bit_cast]

/-- Small naturals that are equal in the field are equal: the shape the byte
decoding below uses constantly. -/
theorem nat_eq {x y : Nat} (hx : x < 2 ^ 40) (hy : y < 2 ^ 40) (h : (x : F) = (y : F)) : x = y :=
  natCast_inj (by unfold goldilocks; omega) (by unfold goldilocks; omega) h

/-- An `add` row at a middle position (`pos` a small number other than 1). -/
theorem addRow_mid {pos a b cin c f : F} (h : AddRow pos 10 a b cin c f) (n : Nat) (hn : n < 4)
    (hn1 : n ≠ 1) (hpos : pos = (n : F)) :
    n = 0 ∧ ∃ na nb nci nc nf : Nat, na < 256 ∧ nb < 256 ∧ nc < 256 ∧ nci ≤ 1 ∧ nf ≤ 1 ∧
      a = (na : F) ∧ b = (nb : F) ∧ cin = (nci : F) ∧ c = (nc : F) ∧ f = (nf : F) ∧
      nc + 256 * nf = na + nb + nci := by
  obtain ⟨np, na, nb, nci, nc, nf, la, lb, lc, lci, hp, ha, hb, hci, hc, hf, hcase⟩ := h.2 rfl
  have hnp : n = np := nat_eq (by omega) (by rcases hcase with ⟨rfl, -⟩ | ⟨rfl, -⟩ <;> omega)
    (hpos.symm.trans hp)
  rcases hcase with ⟨rfl, hf1, hsum⟩ | ⟨rfl, -, -⟩
  · exact ⟨hnp, na, nb, nci, nc, nf, la, lb, lc, lci, hf1, ha, hb, hci, hc, hf, hsum⟩
  · omega

/-- An `add` row at the last position (`pos` 1). -/
theorem addRow_last {pos a b cin c f : F} (h : AddRow pos 10 a b cin c f) (hpos : pos = ((1 : Nat) : F)) :
    ∃ na nb nci nc nf : Nat, na < 256 ∧ nb < 256 ∧ nc < 256 ∧ nci ≤ 1 ∧ (nf = 0 ∨ nf = 64) ∧
      a = (na : F) ∧ b = (nb : F) ∧ cin = (nci : F) ∧ c = (nc : F) ∧ f = (nf : F) ∧
      nc = (na + nb + nci) % 256 := by
  obtain ⟨np, na, nb, nci, nc, nf, la, lb, lc, lci, hp, ha, hb, hci, hc, hf, hcase⟩ := h.2 rfl
  have hnp : 1 = np := nat_eq (by omega) (by rcases hcase with ⟨rfl, -⟩ | ⟨rfl, -⟩ <;> omega)
    (hpos.symm.trans hp)
  rcases hcase with ⟨rfl, -, -⟩ | ⟨rfl, hf64, hc'⟩
  · omega
  · exact ⟨na, nb, nci, nc, nf, la, lb, lc, lci, hf64, ha, hb, hci, hc, hf, hc'⟩

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A 32-bit limb from its four bytes, little-endian, in the field and in the
naturals. -/
theorem limb_cast (x0 x1 x2 x3 : Nat) :
    ((x0 + 256 * x1 + 65536 * x2 + 16777216 * x3 : Nat) : F) =
      (x0 : F) + 256 * (x1 : F) + 65536 * (x2 : F) + 16777216 * (x3 : F) := by
  simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
  rw [← natCast_ofNat' 256, ← natCast_ofNat' 65536, ← natCast_ofNat' 16777216]

/-! The arithmetic, in naturals. Kept in small lemmas: `omega` given the whole
chain with its bounds at once does not finish. -/

/-- Four bytes added with a carry chain: the limb, plus the carry out. -/
theorem chain4 (a0 a1 a2 a3 b0 b1 b2 b3 c0 c1 c2 c3 cin f0 f1 f2 f3 : Nat)
    (s0 : c0 + 256 * f0 = a0 + b0 + cin) (s1 : c1 + 256 * f1 = a1 + b1 + f0)
    (s2 : c2 + 256 * f2 = a2 + b2 + f1) (s3 : c3 + 256 * f3 = a3 + b3 + f2) :
    c0 + 256 * c1 + 65536 * c2 + 16777216 * c3 + 4294967296 * f3 =
      (a0 + 256 * a1 + 65536 * a2 + 16777216 * a3) + (b0 + 256 * b1 + 65536 * b2 + 16777216 * b3)
        + cin := by
  omega

/-- Four bytes make a limb below 2^32. -/
theorem limb_lt (c0 c1 c2 c3 : Nat) (h0 : c0 < 256) (h1 : c1 < 256) (h2 : c2 < 256)
    (h3 : c3 < 256) : c0 + 256 * c1 + 65536 * c2 + 16777216 * c3 < 4294967296 := by omega

/-- Two limbs and the carries between them are the sum modulo 2^64. -/
theorem limbs_sum (L H A0 A1 B0 B1 f q : Nat) (hL : L < 4294967296) (hH : H < 4294967296)
    (lo : L + 4294967296 * f = A0 + B0) (hi : H + 4294967296 * q = A1 + B1 + f) :
    L = (A0 + B0 + 4294967296 * (A1 + B1)) % 2 ^ 64 % 2 ^ 32 ∧
      H = (A0 + B0 + 4294967296 * (A1 + B1)) % 2 ^ 64 / 2 ^ 32 := by
  constructor <;> omega

/-- The top byte, its carry out discarded: the byte plus 256 times the carry. -/
theorem top_byte (a b f c : Nat) (h : c = (a + b + f) % 256) :
    c + 256 * ((a + b + f) / 256) = a + b + f := by omega

theorem lo_lt (A : BitVec 64) : A.toNat % 2 ^ 32 < 2 ^ 40 := by omega
theorem hi_lt (A : BitVec 64) : A.toNat / 2 ^ 32 < 2 ^ 40 := by have := A.isLt; omega

/-- The sum of two 64-bit values, in the shape `limbs_sum` concludes. -/
theorem sum_limbs (A B : BitVec 64) :
    (A + B).toNat = (A.toNat % 2 ^ 32 + B.toNat % 2 ^ 32 +
      4294967296 * (A.toNat / 2 ^ 32 + B.toNat / 2 ^ 32)) % 2 ^ 64 := by
  rw [BitVec.toNat_add]; omega

omit [Lean.Grind.IsCharP F goldilocks] in
theorem one_sub_zero : (1 : F) - 0 = ((1 : Nat) : F) := by
  rw [Lean.Grind.Semiring.natCast_one]; grind

theorem lt40_of_le1 {x : Nat} (h : x ≤ 1) : x < 2 ^ 40 := by omega
theorem lt40_of_lt256 {x : Nat} (h : x < 256) : x < 2 ^ 40 := by omega
theorem lt40_of_le64 {x : Nat} (h : x = 0 ∨ x = 64) : x < 2 ^ 40 := by omega

/-- 32-bit mode would need `b_op = −6`, which no table row has. -/
theorem mode32_zero {bop m32 : F} (hop : bop + 16 * m32 = 10) (hm32 : m32 * (1 - m32) = 0)
    (hne : bop ≠ -6) : m32 = 0 ∧ bop = 10 := by
  obtain ⟨nm, lm, em⟩ := bit_nat hm32
  rcases (by omega : nm = 0 ∨ nm = 1) with h | h
  · rw [h, Lean.Grind.Semiring.natCast_zero] at em
    subst em; exact ⟨rfl, by grind⟩
  · rw [h, Lean.Grind.Semiring.natCast_one] at em
    subst em; exact absurd (by grind) hne

/-- **The algebra of one Binary lane proving `add`.** Over the lane's columns
— the op parts, the flag bits, the eight bytes of `a`, `b`, `c` and the eight
carries — eight `add` rows of the table, one per byte with the lane's own
`pos`, carry-in and flags, give `c = A + B` with the last carry 0. -/
theorem binary8 (bop m32 bos mcs u r cs : F) (a b c k : Nat → F) (A B : BitVec 64)
    (hop : bop + 16 * m32 = 10)
    (hm32 : m32 * (1 - m32) = 0) (hk7 : k 7 * (1 - k 7) = 0) (hr : r * (1 - r) = 0)
    (hu : u * (1 - u) = 0) (hcs : cs * (1 - cs) = 0)
    (e5 : bos - (m32 * (cs + 512 - bop) + bop) = 0) (e6 : mcs - m32 * cs = 0)
    (t0 : AddRow (2 * u) bop (a 0) (b 0) 0 (c 0) (k 0 + 16 * r + 32 * u))
    (t1 : AddRow 0 bop (a 1) (b 1) (k 0) (c 1) (k 1 + 16 * r + 32 * u))
    (t2 : AddRow 0 bop (a 2) (b 2) (k 1) (c 2) (k 2 + 16 * r + 32 * u))
    (t3 : AddRow m32 bop (a 3) (b 3) (k 2) (c 3) (k 3 + 16 * r + 32 * u + 64 * mcs))
    (t4 : AddRow 0 bos (a 4) (b 4) (k 3) (c 4) (k 4 + 16 * r + 32 * u + 64 * mcs))
    (t5 : AddRow 0 bos (a 5) (b 5) (k 4) (c 5) (k 5 + 16 * r + 32 * u + 64 * mcs))
    (t6 : AddRow 0 bos (a 6) (b 6) (k 5) (c 6) (k 6 + 16 * r + 32 * u + 64 * mcs))
    (t7 : AddRow (1 - m32) bos (a 7) (b 7) (k 6) (c 7) (k 7 + 16 * r + 32 * u + 64 * cs))
    (ha0 : a 0 + 256 * a 1 + 65536 * a 2 + 16777216 * a 3 = lo A)
    (ha1 : a 4 + 256 * a 5 + 65536 * a 6 + 16777216 * a 7 = hi A)
    (hb0 : b 0 + 256 * b 1 + 65536 * b 2 + 16777216 * b 3 = lo B)
    (hb1 : b 4 + 256 * b 5 + 65536 * b 6 + 16777216 * b 7 = hi B) :
    c 0 + 256 * c 1 + 65536 * c 2 + 16777216 * c 3 + k 7 = lo (A + B) ∧
      c 4 + 256 * c 5 + 65536 * c 6 + 16777216 * c 7 = hi (A + B) ∧ k 7 = 0 := by
  -- 64-bit mode, and the op is `add` in every byte.
  obtain ⟨hm0, hbop⟩ := mode32_zero hop hm32 t0.1
  subst hm0 hbop
  have hbos : bos = 10 := by grind
  have hmcs : mcs = 0 := by grind
  subst hbos hmcs
  -- `use_first_byte` is 0: byte 0's position is `2·use_first_byte`, an `add` row's 0 or 1.
  obtain ⟨nu, lu, eu⟩ := bit_nat hu
  obtain ⟨hnu, na0, nb0, nci0, nc0, nf0, la0, lb0, lc0, lci0, lf0, ea0, eb0, eci0, ec0, ef0, s0⟩ :=
    addRow_mid t0 (2 * nu) (by omega) (by omega)
      (by rw [eu, Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 2])
  rw [(by omega : nu = 0), Lean.Grind.Semiring.natCast_zero] at eu
  subst eu
  have z : ((0 : Nat) : F) = 0 := Lean.Grind.Semiring.natCast_zero
  -- The middle bytes and the last.
  obtain ⟨-, na1, nb1, nci1, nc1, nf1, la1, lb1, lc1, lci1, lf1, ea1, eb1, eci1, ec1, ef1, s1⟩ :=
    addRow_mid t1 0 (by decide) (by decide) z.symm
  obtain ⟨-, na2, nb2, nci2, nc2, nf2, la2, lb2, lc2, lci2, lf2, ea2, eb2, eci2, ec2, ef2, s2⟩ :=
    addRow_mid t2 0 (by decide) (by decide) z.symm
  obtain ⟨-, na3, nb3, nci3, nc3, nf3, la3, lb3, lc3, lci3, lf3, ea3, eb3, eci3, ec3, ef3, s3⟩ :=
    addRow_mid t3 0 (by decide) (by decide) z.symm
  obtain ⟨-, na4, nb4, nci4, nc4, nf4, la4, lb4, lc4, lci4, lf4, ea4, eb4, eci4, ec4, ef4, s4⟩ :=
    addRow_mid t4 0 (by decide) (by decide) z.symm
  obtain ⟨-, na5, nb5, nci5, nc5, nf5, la5, lb5, lc5, lci5, lf5, ea5, eb5, eci5, ec5, ef5, s5⟩ :=
    addRow_mid t5 0 (by decide) (by decide) z.symm
  obtain ⟨-, na6, nb6, nci6, nc6, nf6, la6, lb6, lc6, lci6, lf6, ea6, eb6, eci6, ec6, ef6, s6⟩ :=
    addRow_mid t6 0 (by decide) (by decide) z.symm
  obtain ⟨na7, nb7, nci7, nc7, nf7, la7, lb7, lc7, lci7, lf7, ea7, eb7, eci7, ec7, ef7, s7⟩ :=
    addRow_last t7 one_sub_zero
  clear t0 t1 t2 t3 t4 t5 t6 t7
  -- Byte 0 carries in nothing.
  have hci0 : nci0 = 0 :=
    nat_eq (lt40_of_le1 lci0) (by decide) (by rw [← eci0, z])
  -- `result_is_a` is 0: byte 0's flags are its carry plus 16·`result_is_a`, at most 1,
  -- and its carry is byte 1's carry-in, 0 or 1.
  obtain ⟨nr, lr, er⟩ := bit_nat hr
  have h16 : ((nci1 + 16 * nr : Nat) : F) = (nf0 : F) := by
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 16,
      ← eci1, ← er, ← ef0, Lean.Grind.Semiring.mul_zero, Lean.Grind.Semiring.add_zero]
  have e16 := nat_eq (by have := lt40_of_le1 lci1; have := lt40_of_le1 lr; omega)
    (lt40_of_le1 lf0) h16
  have hnr : nr = 0 := by omega
  have l01 : nci1 = nf0 := by omega
  rw [hnr, z] at er
  subst er
  simp only [Lean.Grind.Semiring.mul_zero, Lean.Grind.Semiring.add_zero] at ef0 ef1 ef2 ef3 ef4 ef5 ef6 ef7
  -- Every other carry is the table's.
  have l12 : nci2 = nf1 := nat_eq (lt40_of_le1 lci2) (lt40_of_le1 lf1) (eci2.symm.trans ef1)
  have l23 : nci3 = nf2 := nat_eq (lt40_of_le1 lci3) (lt40_of_le1 lf2) (eci3.symm.trans ef2)
  have l34 : nci4 = nf3 := nat_eq (lt40_of_le1 lci4) (lt40_of_le1 lf3) (eci4.symm.trans ef3)
  have l45 : nci5 = nf4 := nat_eq (lt40_of_le1 lci5) (lt40_of_le1 lf4) (eci5.symm.trans ef4)
  have l56 : nci6 = nf5 := nat_eq (lt40_of_le1 lci6) (lt40_of_le1 lf5) (eci6.symm.trans ef5)
  have l67 : nci7 = nf6 := nat_eq (lt40_of_le1 lci7) (lt40_of_le1 lf6) (eci7.symm.trans ef6)
  -- The last carry is 0: the last byte's flags are it plus 64·`c_is_signed`, and 0 or 64.
  obtain ⟨nk, lk, ek⟩ := bit_nat hk7
  obtain ⟨ns, ls, es⟩ := bit_nat hcs
  have h64 : ((nk + 64 * ns : Nat) : F) = (nf7 : F) := by
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 64,
      ← ek, ← es, ← ef7]
  have e64 := nat_eq (by have := lt40_of_le1 lk; have := lt40_of_le1 ls; omega)
    (lt40_of_le64 lf7) h64
  have hk0 : nk = 0 := by omega
  rw [hk0, z] at ek
  -- The operands' limbs, as naturals.
  have hA := A.isLt
  have hB := B.isLt
  rw [ea0, ea1, ea2, ea3, ← limb_cast] at ha0
  rw [ea4, ea5, ea6, ea7, ← limb_cast] at ha1
  rw [eb0, eb1, eb2, eb3, ← limb_cast] at hb0
  rw [eb4, eb5, eb6, eb7, ← limb_cast] at hb1
  have nA0 := nat_eq (Nat.lt_trans (limb_lt _ _ _ _ la0 la1 la2 la3) (by decide))
    (lo_lt A) ha0
  have nA1 := nat_eq (Nat.lt_trans (limb_lt _ _ _ _ la4 la5 la6 la7) (by decide))
    (hi_lt A) ha1
  have nB0 := nat_eq (Nat.lt_trans (limb_lt _ _ _ _ lb0 lb1 lb2 lb3) (by decide))
    (lo_lt B) hb0
  have nB1 := nat_eq (Nat.lt_trans (limb_lt _ _ _ _ lb4 lb5 lb6 lb7) (by decide))
    (hi_lt B) hb1
  -- The carry chain.
  subst hci0 l01 l12 l23 l34 l45 l56 l67
  -- (`subst` kept the carry-ins: byte `i`'s carry out is now `nci(i+1)`.)
  have clo := chain4 na0 na1 na2 na3 nb0 nb1 nb2 nb3 nc0 nc1 nc2 nc3 0 nci1 nci2 nci3 nci4
    s0 s1 s2 s3
  have hq := top_byte na7 nb7 nci7 nc7 s7
  have chi := chain4 na4 na5 na6 na7 nb4 nb5 nb6 nb7 nc4 nc5 nc6 nc7 nci4 nci5 nci6 nci7
    ((na7 + nb7 + nci7) / 256) s4 s5 s6 hq
  rw [Nat.add_zero, nA0, nB0] at clo
  rw [nA1, nB1] at chi
  obtain ⟨m0, m1⟩ := limbs_sum _ _ _ _ _ _ _ _ (limb_lt _ _ _ _ lc0 lc1 lc2 lc3)
    (limb_lt _ _ _ _ lc4 lc5 lc6 lc7) clo chi
  rw [← sum_limbs] at m0 m1
  refine ⟨?_, ?_, ek⟩
  · rw [ek, Lean.Grind.Semiring.add_zero, ec0, ec1, ec2, ec3, ← limb_cast, lo, m0]
  · rw [ec4, ec5, ec6, ec7, ← limb_cast, hi, m1]

end Algebra

section Lanes

/-! ### One instance per lane

Each names its lane's op-bus argument, its eight byte lookups and its two
helper constraints (`b_op_or_sext`, `mode32_and_c_is_signed`). What is assumed:
the eight lookups (`LookupHolds`, against the five arguments that serve the
Binary table) and the table's `add` rows (`BinaryTableAdds`, checked by
`check_tables.py`). -/

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

theorem Binary_bus21_adds {x : Binary.Ctx F} {t : Binary.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, Binary.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds Binary.bus13 x t (binaryTable xv tv))
    (l1 : LookupHolds Binary.bus14 x t (binaryTable xv tv))
    (l2 : LookupHolds Binary.bus15 x t (binaryTable xv tv))
    (l3 : LookupHolds Binary.bus16 x t (binaryTable xv tv))
    (l4 : LookupHolds Binary.bus17 x t (binaryTable xv tv))
    (l5 : LookupHolds Binary.bus18 x t (binaryTable xv tv))
    (l6 : LookupHolds Binary.bus19 x t (binaryTable xv tv))
    (l7 : LookupHolds Binary.bus20 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨Binary.bus21, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, Binary.bus21, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup Binary.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup Binary.bus14 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup Binary.bus15 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup Binary.bus16 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup Binary.bus17 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup Binary.bus18 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup Binary.bus19 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup Binary.bus20 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, Binary.bus13, Binary.bus14, Binary.bus15, Binary.bus16, Binary.bus17, Binary.bus18, Binary.bus19, Binary.bus20, Binary.Binary_mode64_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c5
  have e6 := (h j).c6
  simp only [Binary.c5, Binary.c6] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
    ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
    ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
    (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
    (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
    (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

theorem BinaryLarge_bus21_adds {x : BinaryLarge.Ctx F} {t : BinaryLarge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryLarge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds BinaryLarge.bus13 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryLarge.bus14 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryLarge.bus15 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryLarge.bus16 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryLarge.bus17 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryLarge.bus18 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryLarge.bus19 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryLarge.bus20 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨BinaryLarge.bus21, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, BinaryLarge.bus21, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup BinaryLarge.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup BinaryLarge.bus14 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup BinaryLarge.bus15 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup BinaryLarge.bus16 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup BinaryLarge.bus17 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup BinaryLarge.bus18 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup BinaryLarge.bus19 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup BinaryLarge.bus20 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryLarge.bus13, BinaryLarge.bus14, BinaryLarge.bus15, BinaryLarge.bus16, BinaryLarge.bus17, BinaryLarge.bus18, BinaryLarge.bus19, BinaryLarge.bus20, BinaryLarge.BinaryLarge_mode64_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c5
  have e6 := (h j).c6
  simp only [BinaryLarge.c5, BinaryLarge.c6] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
    ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
    ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
    (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
    (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
    (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

theorem BinaryLarge_bus32_adds {x : BinaryLarge.Ctx F} {t : BinaryLarge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryLarge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds BinaryLarge.bus24 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryLarge.bus25 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryLarge.bus26 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryLarge.bus27 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryLarge.bus28 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryLarge.bus29 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryLarge.bus30 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryLarge.bus31 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨BinaryLarge.bus32, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, BinaryLarge.bus32, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup BinaryLarge.bus24 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup BinaryLarge.bus25 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup BinaryLarge.bus26 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup BinaryLarge.bus27 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup BinaryLarge.bus28 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup BinaryLarge.bus29 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup BinaryLarge.bus30 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup BinaryLarge.bus31 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryLarge.bus24, BinaryLarge.bus25, BinaryLarge.bus26, BinaryLarge.bus27, BinaryLarge.bus28, BinaryLarge.bus29, BinaryLarge.bus30, BinaryLarge.bus31, BinaryLarge.BinaryLarge_mode64_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c12
  have e6 := (h j).c13
  simp only [BinaryLarge.c12, BinaryLarge.c13] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
    ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
    ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
    (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
    (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
    (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

theorem BinaryHuge_bus21_adds {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds BinaryHuge.bus13 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus14 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus15 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus16 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus17 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus18 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus19 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus20 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨BinaryHuge.bus21, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, BinaryHuge.bus21, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup BinaryHuge.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup BinaryHuge.bus14 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup BinaryHuge.bus15 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup BinaryHuge.bus16 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup BinaryHuge.bus17 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup BinaryHuge.bus18 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup BinaryHuge.bus19 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup BinaryHuge.bus20 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus13, BinaryHuge.bus14, BinaryHuge.bus15, BinaryHuge.bus16, BinaryHuge.bus17, BinaryHuge.bus18, BinaryHuge.bus19, BinaryHuge.bus20, BinaryHuge.BinaryHuge_mode64_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c5
  have e6 := (h j).c6
  simp only [BinaryHuge.c5, BinaryHuge.c6] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 0) ((t j).mode32 0) ((t j).b_op_or_sext 0)
    ((t j).mode32_and_c_is_signed 0) ((t j).use_first_byte 0) ((t j).result_is_a 0)
    ((t j).c_is_signed 0) (fun k => (t j).free_in_a 0 k) (fun k => (t j).free_in_b 0 k)
    (fun k => (t j).free_in_c 0 k) (fun k => (t j).carry 0 k) A B hop
    (h j).bool_mode32_0 (h j).bool_carry_0_7 (h j).bool_result_is_a_0
    (h j).bool_use_first_byte_0 (h j).bool_c_is_signed_0 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

theorem BinaryHuge_bus32_adds {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds BinaryHuge.bus24 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus25 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus26 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus27 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus28 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus29 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus30 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus31 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨BinaryHuge.bus32, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, BinaryHuge.bus32, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup BinaryHuge.bus24 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup BinaryHuge.bus25 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup BinaryHuge.bus26 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup BinaryHuge.bus27 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup BinaryHuge.bus28 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup BinaryHuge.bus29 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup BinaryHuge.bus30 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup BinaryHuge.bus31 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus24, BinaryHuge.bus25, BinaryHuge.bus26, BinaryHuge.bus27, BinaryHuge.bus28, BinaryHuge.bus29, BinaryHuge.bus30, BinaryHuge.bus31, BinaryHuge.BinaryHuge_mode64_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c12
  have e6 := (h j).c13
  simp only [BinaryHuge.c12, BinaryHuge.c13] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 1) ((t j).mode32 1) ((t j).b_op_or_sext 1)
    ((t j).mode32_and_c_is_signed 1) ((t j).use_first_byte 1) ((t j).result_is_a 1)
    ((t j).c_is_signed 1) (fun k => (t j).free_in_a 1 k) (fun k => (t j).free_in_b 1 k)
    (fun k => (t j).free_in_c 1 k) (fun k => (t j).carry 1 k) A B hop
    (h j).bool_mode32_1 (h j).bool_carry_1_7 (h j).bool_result_is_a_1
    (h j).bool_use_first_byte_1 (h j).bool_c_is_signed_1 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

theorem BinaryHuge_bus43_adds {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds BinaryHuge.bus35 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus36 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus37 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus38 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus39 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus40 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus41 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus42 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨BinaryHuge.bus43, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, BinaryHuge.bus43, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup BinaryHuge.bus35 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup BinaryHuge.bus36 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup BinaryHuge.bus37 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup BinaryHuge.bus38 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup BinaryHuge.bus39 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup BinaryHuge.bus40 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup BinaryHuge.bus41 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup BinaryHuge.bus42 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus35, BinaryHuge.bus36, BinaryHuge.bus37, BinaryHuge.bus38, BinaryHuge.bus39, BinaryHuge.bus40, BinaryHuge.bus41, BinaryHuge.bus42, BinaryHuge.BinaryHuge_mode64_2, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c19
  have e6 := (h j).c20
  simp only [BinaryHuge.c19, BinaryHuge.c20] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 2) ((t j).mode32 2) ((t j).b_op_or_sext 2)
    ((t j).mode32_and_c_is_signed 2) ((t j).use_first_byte 2) ((t j).result_is_a 2)
    ((t j).c_is_signed 2) (fun k => (t j).free_in_a 2 k) (fun k => (t j).free_in_b 2 k)
    (fun k => (t j).free_in_c 2 k) (fun k => (t j).carry 2 k) A B hop
    (h j).bool_mode32_2 (h j).bool_carry_2_7 (h j).bool_result_is_a_2
    (h j).bool_use_first_byte_2 (h j).bool_c_is_signed_2 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

theorem BinaryHuge_bus54_adds {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p)
    (l0 : LookupHolds BinaryHuge.bus46 x t (binaryTable xv tv))
    (l1 : LookupHolds BinaryHuge.bus47 x t (binaryTable xv tv))
    (l2 : LookupHolds BinaryHuge.bus48 x t (binaryTable xv tv))
    (l3 : LookupHolds BinaryHuge.bus49 x t (binaryTable xv tv))
    (l4 : LookupHolds BinaryHuge.bus50 x t (binaryTable xv tv))
    (l5 : LookupHolds BinaryHuge.bus51 x t (binaryTable xv tv))
    (l6 : LookupHolds BinaryHuge.bus52 x t (binaryTable xv tv))
    (l7 : LookupHolds BinaryHuge.bus53 x t (binaryTable xv tv)) :
    ∀ j, AddsAt ⟨BinaryHuge.bus54, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  simp only [tupleAt, BinaryHuge.bus54, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have t0 := addRow_of_lookup BinaryHuge.bus46 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  have t1 := addRow_of_lookup BinaryHuge.bus47 x t j _ l1 (fun _ _ _ => rfl) one_ne_zero' hd
  have t2 := addRow_of_lookup BinaryHuge.bus48 x t j _ l2 (fun _ _ _ => rfl) one_ne_zero' hd
  have t3 := addRow_of_lookup BinaryHuge.bus49 x t j _ l3 (fun _ _ _ => rfl) one_ne_zero' hd
  have t4 := addRow_of_lookup BinaryHuge.bus50 x t j _ l4 (fun _ _ _ => rfl) one_ne_zero' hd
  have t5 := addRow_of_lookup BinaryHuge.bus51 x t j _ l5 (fun _ _ _ => rfl) one_ne_zero' hd
  have t6 := addRow_of_lookup BinaryHuge.bus52 x t j _ l6 (fun _ _ _ => rfl) one_ne_zero' hd
  have t7 := addRow_of_lookup BinaryHuge.bus53 x t j _ l7 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus46, BinaryHuge.bus47, BinaryHuge.bus48, BinaryHuge.bus49, BinaryHuge.bus50, BinaryHuge.bus51, BinaryHuge.bus52, BinaryHuge.bus53, BinaryHuge.BinaryHuge_mode64_3, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0 t1 t2 t3 t4 t5 t6 t7
  have e5 := (h j).c26
  have e6 := (h j).c27
  simp only [BinaryHuge.c26, BinaryHuge.c27] at e5 e6
  obtain ⟨c0, c1, c7⟩ := binary8 ((t j).b_op 3) ((t j).mode32 3) ((t j).b_op_or_sext 3)
    ((t j).mode32_and_c_is_signed 3) ((t j).use_first_byte 3) ((t j).result_is_a 3)
    ((t j).c_is_signed 3) (fun k => (t j).free_in_a 3 k) (fun k => (t j).free_in_b 3 k)
    (fun k => (t j).free_in_c 3 k) (fun k => (t j).carry 3 k) A B hop
    (h j).bool_mode32_3 (h j).bool_carry_3_7 (h j).bool_result_is_a_3
    (h j).bool_use_first_byte_3 (h j).bool_c_is_signed_3 e5 e6
    t0 t1 t2 t3 t4 t5 t6 t7 ha0 ha1 hb0 hb1
  exact ⟨c0, c1, c7⟩

end Lanes

end Correspondence
