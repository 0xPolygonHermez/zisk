/-
  Reading n bytes of a byte map, little-endian: what both interpreters'
  `readBytes` compute — lean-sail's sequential one under sail-riscv and its
  concurrency-interface one under sail-zisk — when the bytes are there.
  Stated once, so that a load reads the same value on both sides from the
  same memory.
-/
import Equiv.Riscv
import Equiv.Zisk

namespace Equiv

abbrev Mem := Std.ExtHashMap Nat (BitVec 8)

/-- The n bytes at a, little-endian, if they are all there. -/
def memRead (m : Mem) : (n : Nat) → Nat → Option (BitVec (8 * n))
  | 0, _ => some default
  | 1, a => m.get? a
  | n + 2, a => do
    let b ← m.get? a
    let bytes ← memRead m (n + 1) (a + 1)
    have h : 8 * (n + 1) + 8 = 8 * (n + 2) := by omega
    pure (h ▸ bytes.append b)

theorem riscv_readBytes (s : Equiv.Riscv.RVState) :
    ∀ (n a : Nat) (v : BitVec (8 * n)), memRead s.mem n a = some v →
      (Sail.ArchSem.readBytes n a : Sail.ArchSem.ArchSemSeqM LeanZiskRV.exception _).run s = .ok v s
  | 0, a, v, h => by simp [memRead] at h; subst h; rfl
  | 1, a, v, h => by
    simp only [memRead, Std.ExtHashMap.get?_eq_getElem?] at h
    simp [Sail.ArchSem.readBytes, Sail.ArchSem.readByte, h]
  | n + 2, a, v, h => by
    simp only [memRead, Std.ExtHashMap.get?_eq_getElem?] at h
    cases hb : s.mem[a]? with
    | none => simp [hb] at h
    | some b =>
      cases hr : memRead s.mem (n + 1) (a + 1) with
      | none => simp [hb, hr] at h
      | some bytes =>
        simp [hb, hr] at h
        have ih := riscv_readBytes s (n + 1) (a + 1) bytes hr
        simp only [Sail.ArchSem.readBytes, Sail.ArchSem.readByte]
        simp [hb]
        rw [ih]; subst h; rfl

theorem zisk_readBytes (z : Equiv.Zisk.ZState) :
    ∀ (n a : Nat) (v : BitVec (8 * n)), memRead z.mem n a = some v →
      (Sail.ConcurrencyInterfaceV1.PreSail.readBytes n a : _root_.Zisk.SailM _).run z = .ok (v, none) z
  | 0, a, v, h => by simp [memRead] at h; subst h; rfl
  | 1, a, v, h => by
    simp only [memRead, Std.ExtHashMap.get?_eq_getElem?] at h
    simp [Sail.ConcurrencyInterfaceV1.PreSail.readBytes, Sail.ConcurrencyInterfaceV1.PreSail.readByte, h]
  | n + 2, a, v, h => by
    simp only [memRead, Std.ExtHashMap.get?_eq_getElem?] at h
    cases hb : z.mem[a]? with
    | none => simp [hb] at h
    | some b =>
      cases hr : memRead z.mem (n + 1) (a + 1) with
      | none => simp [hb, hr] at h
      | some bytes =>
        simp [hb, hr] at h
        have ih := zisk_readBytes z (n + 1) (a + 1) bytes hr
        simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes, Sail.ConcurrencyInterfaceV1.PreSail.readByte]
        simp [hb]
        rw [ih]; subst h; rfl

/-- A cast of a bit vector, as `▸` writes it. -/
theorem rec_cast {w w' : Nat} (e : w = w') (x : BitVec w) : e ▸ x = x.cast e := by subst e; rfl

/-- `memRead`'s equation for two bytes or more. -/
theorem memRead_succ (m : Mem) (n a : Nat) :
    memRead m (n + 2) a = (do
      let b ← m.get? a
      let bytes ← memRead m (n + 1) (a + 1)
      pure ((show 8 * (n + 1) + 8 = 8 * (n + 2) by omega) ▸ bytes.append b)) := rfl

set_option maxHeartbeats 2000000 in
/-- A read's bytes, one by one. -/
theorem memRead_get (m : Mem) : ∀ (n a : Nat) (v : BitVec (8 * n)), memRead m n a = some v →
    ∀ i, i < n → m.get? (a + i) = some (v.extractLsb' (8 * i) 8)
  | 0, _, _, _, i, hi => absurd hi (by omega)
  | 1, a, v, h, i, hi => by
    have : i = 0 := by omega
    subst this
    simp only [memRead] at h
    rw [Nat.add_zero, h]; congr 1
    apply BitVec.eq_of_getLsbD_eq; intro j hj; simp [hj]
  | n + 2, a, v, h, i, hi => by
    rw [memRead_succ] at h
    cases hb : m.get? a with
    | none => rw [hb] at h; cases h
    | some b =>
      cases hr : memRead m (n + 1) (a + 1) with
      | none => rw [hb] at h; simp only [Option.bind_eq_bind, Option.bind_some, hr] at h; cases h
      | some bytes =>
        rw [hb] at h
        simp only [hr, Option.bind_eq_bind, Option.bind_some, Option.pure_def, Option.some.injEq, rec_cast] at h
        subst h
        cases i with
        | zero =>
          rw [Nat.add_zero, hb]; congr 1
          apply BitVec.eq_of_getLsbD_eq; intro j hj
          simp [hj, BitVec.getLsbD_append]
        | succ i =>
          have := memRead_get m (n + 1) (a + 1) bytes hr i (by omega)
          rw [show a + (i + 1) = a + 1 + i by omega, this]; congr 1
          apply BitVec.eq_of_getLsbD_eq; intro j hj
          rw [BitVec.getLsbD_extractLsb', BitVec.getLsbD_extractLsb']
          simp only [BitVec.append_eq, BitVec.getLsbD_append, hj, decide_true, Bool.true_and]
          rw [if_neg (by omega)]
          congr 1; omega

set_option maxHeartbeats 2000000 in
/-- A read from its bytes, one by one. -/
theorem memRead_of_get (m : Mem) : ∀ (n a : Nat) (v : BitVec (8 * n)),
    (∀ i, i < n → m.get? (a + i) = some (v.extractLsb' (8 * i) 8)) → memRead m n a = some v
  | 0, _, v, _ => by
    simp only [memRead]; congr 1; apply BitVec.eq_of_getLsbD_eq; intro j hj; omega
  | 1, a, v, h => by
    have h0 := h 0 (by omega)
    simp only [memRead]; rw [Nat.add_zero] at h0; rw [h0]; congr 1
    apply BitVec.eq_of_getLsbD_eq; intro j hj; simp [hj]
  | n + 2, a, v, h => by
    rw [memRead_succ]
    have h0 := h 0 (by omega)
    rw [Nat.add_zero] at h0
    have hr := memRead_of_get m (n + 1) (a + 1) (v.extractLsb' 8 (8 * (n + 1))) (by
      intro i hi
      rw [show a + 1 + i = a + (i + 1) by omega, h (i + 1) (by omega)]; congr 1
      apply BitVec.eq_of_getLsbD_eq; intro j hj
      simp only [BitVec.getLsbD_extractLsb', hj, decide_true, Bool.true_and]
      have hl : 8 * i + j < 8 * (n + 1) := by omega
      simp only [hl, decide_true, Bool.true_and]; congr 1; omega)
    rw [h0, hr]
    simp only [Option.bind_eq_bind, Option.bind_some, Option.pure_def, Option.some.injEq]
    apply BitVec.eq_of_getLsbD_eq; intro j hj
    simp only [BitVec.append_eq, BitVec.getLsbD_append, BitVec.getLsbD_extractLsb']
    by_cases hj8 : j < 8
    · simp [hj8]
    · simp only [hj8, if_false]
      have hl : j - 8 < 8 * (n + 1) := by omega
      simp only [hl, decide_true, Bool.true_and]; congr 1; omega

/-- Part of a read: q bytes from p on. -/
theorem memRead_piece {m : Mem} {n a : Nat} {v : BitVec (8 * n)} (hv : memRead m n a = some v)
    (p q : Nat) (h : p + q ≤ n) : memRead m q (a + p) = some (v.extractLsb' (8 * p) (8 * q)) := by
  apply memRead_of_get
  intro i hi
  rw [show a + p + i = a + (p + i) by omega, memRead_get m n a v hv (p + i) (by omega)]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro j hj
  simp only [BitVec.getLsbD_extractLsb', hj, decide_true, Bool.true_and]
  have hl : 8 * i + j < 8 * q := by omega
  simp only [hl, decide_true, Bool.true_and]; congr 1; omega

end Equiv
