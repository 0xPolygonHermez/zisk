/-
  Memory less the 8 bytes at INPUT_ADDR, which are not memory (contract
  § 2): an 8-byte load there is the free input, which the machine mirrors
  into them at each ZisK call and ZisK reads from its FreeInput register,
  leaving its own bytes as they were. The relation compares the two
  memories without them (`normIn`); a write keeps that, and a read off them
  sees the same bytes.
-/
import Equiv.WordMem

namespace Equiv

/-- INPUT_ADDR. -/
def IN_LO : Nat := 0x1000_0000

/-- The map without the 8 bytes at INPUT_ADDR. -/
def normIn (m : Mem) : Mem := (List.range 8).foldl (fun m i => m.erase (IN_LO + i)) m

theorem normIn_get (m : Mem) (k : Nat) :
    (normIn m).get? k = if IN_LO ≤ k ∧ k < IN_LO + 8 then none else m.get? k := by
  simp only [normIn, List.range, List.range.loop, List.foldl, Std.ExtHashMap.get?_eq_getElem?,
    Std.ExtHashMap.getElem?_erase]
  by_cases h : IN_LO ≤ k ∧ k < IN_LO + 8
  · simp only [h, and_self, if_true]
    have : k = IN_LO + 0 ∨ k = IN_LO + 1 ∨ k = IN_LO + 2 ∨ k = IN_LO + 3 ∨ k = IN_LO + 4 ∨ k = IN_LO + 5 ∨
        k = IN_LO + 6 ∨ k = IN_LO + 7 := by omega
    rcases this with e | e | e | e | e | e | e | e <;> subst e <;> simp
  · simp only [h, if_false]
    have : ∀ i, i < 8 → (IN_LO + i == k) = false := fun i hi => by simp; omega
    have h0 : (IN_LO == k) = false := by simpa using this 0 (by omega)
    simp [h0, this 1 (by omega), this 2 (by omega), this 3 (by omega), this 4 (by omega),
      this 5 (by omega), this 6 (by omega), this 7 (by omega)]

theorem normIn_eq_iff (m1 m2 : Mem) :
    normIn m1 = normIn m2 ↔ ∀ k, ¬ (IN_LO ≤ k ∧ k < IN_LO + 8) → m1.get? k = m2.get? k := by
  constructor
  · intro h k hk
    have := congrArg (fun m => m.get? k) h
    simp only [normIn_get, hk, if_false] at this
    exact this
  · intro h
    apply Std.ExtHashMap.ext_getElem?; intro k
    rw [← Std.ExtHashMap.get?_eq_getElem?, ← Std.ExtHashMap.get?_eq_getElem?, normIn_get, normIn_get]
    split
    · rfl
    · next hk => exact h k hk

/-- Both memories unchanged. -/
theorem normIn_congr {m1 m2 m1' m2' : Mem} (h : normIn m1 = normIn m2) (h1 : m1' = m1) (h2 : m2' = m2) :
    normIn m1' = normIn m2' := by rw [h1, h2]; exact h

/-- A write keeps two memories equal off the input's 8 bytes. -/
theorem normIn_memWrite {n : Nat} {m1 m2 : Mem} (h : normIn m1 = normIn m2) (a : Nat) (v : BitVec (8 * n)) :
    normIn (memWrite m1 a v) = normIn (memWrite m2 a v) := by
  rw [normIn_eq_iff] at h ⊢
  intro k hk; exact memWrite_pointwise _ _ a v k (h k hk)

theorem normIn_writeWord {m1 m2 : Mem} (h : normIn m1 = normIn m2) (p : Nat) (w : BitVec 32) :
    normIn (ZiskRiscv.writeWord m1 p w) = normIn (ZiskRiscv.writeWord m2 p w) := by
  rw [normIn_eq_iff] at h ⊢
  intro k hk; rw [writeWord_get, writeWord_get]; split <;> (try rfl) <;> split <;> (try rfl) <;>
    split <;> (try rfl) <;> split <;> (try rfl) <;> exact h k hk

/-- An access of w bytes at a off the input's 8 bytes. -/
def InOff (a : BitVec 64) (w : Nat) : Prop := a.toNat + w ≤ IN_LO ∨ IN_LO + 8 ≤ a.toNat

/-- A read off the input's 8 bytes sees the same bytes in two memories equal
    without them. -/
theorem memRead_normIn {m1 m2 : Mem} (h : normIn m1 = normIn m2) {n : Nat} {a : Nat}
    (hoff : a + n ≤ IN_LO ∨ IN_LO + 8 ≤ a) : memRead m1 n a = memRead m2 n a := by
  rw [normIn_eq_iff] at h
  exact memRead_frame m2 m1 n a (fun k h1 h2 => h k (by omega))

end Equiv
