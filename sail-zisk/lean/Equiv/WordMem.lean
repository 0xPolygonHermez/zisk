/-
  The machine's word writes (ZiskRiscv.writeWord: the four bytes of an
  instruction word at an address), against reads and writes elsewhere: what
  `lr` and `sc`'s substitution needs (the plain load or store's word put at
  pc for a step, the original put back).
-/
import Equiv.RamOnly
import ZiskRiscv.Machine

namespace Equiv

theorem writeWord_get (m : Mem) (p : Nat) (w : BitVec 32) (k : Nat) :
    (ZiskRiscv.writeWord m p w).get? k =
      if k = p then some (w.extractLsb' 0 8) else if k = p + 1 then some (w.extractLsb' 8 8)
      else if k = p + 2 then some (w.extractLsb' 16 8) else if k = p + 3 then some (w.extractLsb' 24 8)
      else m.get? k := by
  simp only [ZiskRiscv.writeWord, List.range, List.range.loop, List.foldl, Std.ExtHashMap.get?_eq_getElem?,
    Std.ExtHashMap.getElem?_insert, beq_iff_eq]
  by_cases e0 : k = p
  · subst e0; simp
  by_cases e1 : k = p + 1
  · subst e1; simp
  by_cases e2 : k = p + 2
  · subst e2; simp
  by_cases e3 : k = p + 3
  · subst e3; simp
  simp [e0, e1, e2, e3, show ¬ p + 3 = k by omega, show ¬ p + 2 = k by omega, show ¬ p + 1 = k by omega,
    show ¬ p = k by omega]

/-- Writing the word that is there changes nothing. -/
theorem writeWord_same (m : Mem) (p : Nat) (w : BitVec 32) (h : ZiskRiscv.readWord m p = some w) :
    ZiskRiscv.writeWord m p w = m := by
  simp only [ZiskRiscv.readWord, bind, Option.bind, pure] at h
  split at h <;> try simp at h
  rename_i b0 h0
  split at h <;> try simp at h
  rename_i b1 h1
  split at h <;> try simp at h
  rename_i b2 h2
  split at h <;> try simp at h
  rename_i b3 h3
  subst h
  simp only [← Std.ExtHashMap.get?_eq_getElem?] at h0 h1 h2 h3
  apply Std.ExtHashMap.ext_getElem?; intro k
  rw [← Std.ExtHashMap.get?_eq_getElem?, ← Std.ExtHashMap.get?_eq_getElem?, writeWord_get]
  by_cases e0 : k = p
  · subst e0; rw [h0]; simp; bv_decide
  by_cases e1 : k = p + 1
  · subst e1; rw [h1]; simp [e0]; bv_decide
  by_cases e2 : k = p + 2
  · subst e2; rw [h2]; simp [e0, e1]; bv_decide
  by_cases e3 : k = p + 3
  · subst e3; rw [h3]; simp [e0, e1, e2]; bv_decide
  simp [e0, e1, e2, e3]

/-- A second word at the same address replaces the first. -/
theorem writeWord_twice (m : Mem) (p : Nat) (a b : BitVec 32) :
    ZiskRiscv.writeWord (ZiskRiscv.writeWord m p a) p b = ZiskRiscv.writeWord m p b := by
  apply Std.ExtHashMap.ext_getElem?; intro k
  rw [← Std.ExtHashMap.get?_eq_getElem?, ← Std.ExtHashMap.get?_eq_getElem?, writeWord_get, writeWord_get,
    writeWord_get]
  by_cases e0 : k = p <;> by_cases e1 : k = p + 1 <;> by_cases e2 : k = p + 2 <;> by_cases e3 : k = p + 3 <;>
    simp [e0, e1, e2, e3]

/-- Reads see only the bytes they read. -/
theorem memRead_frame (m m' : Mem) : ∀ (n a : Nat), (∀ k, a ≤ k → k < a + n → m'.get? k = m.get? k) →
    memRead m' n a = memRead m n a
  | 0, _, _ => rfl
  | 1, a, h => h a (Nat.le_refl _) (by omega)
  | n + 2, a, h => by
    simp only [memRead]
    rw [h a (Nat.le_refl _) (by omega), memRead_frame m m' (n + 1) (a + 1) (fun k h1 h2 => h k (by omega) (by omega))]

/-- A write of n bytes at a, at a key outside them: nothing changes. -/
theorem memWrite_get_off {n : Nat} (m : Mem) (a : Nat) (v : BitVec (8 * n)) (k : Nat) (hk : k < a ∨ a + n ≤ k) :
    (memWrite m a v).get? k = m.get? k := by
  apply foldl_insert_off
  intro p hp
  simp only [byteList, List.mem_ofFn] at hp
  obtain ⟨i, rfl⟩ := hp
  simp only; have := i.isLt; omega

/-- A write sees, at each key, only the map's value there. -/
theorem foldl_insert_pointwise (l : List (Nat × BitVec 8)) (k : Nat) :
    ∀ m1 m2 : Mem, m1.get? k = m2.get? k →
      (l.foldl (fun m p => m.insert p.1 p.2) m1).get? k = (l.foldl (fun m p => m.insert p.1 p.2) m2).get? k := by
  induction l with
  | nil => intro m1 m2 h; exact h
  | cons p l ih =>
    intro m1 m2 h
    simp only [List.foldl_cons]
    apply ih
    simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert] at h ⊢
    split <;> simp_all

theorem memWrite_pointwise {n : Nat} (m1 m2 : Mem) (a : Nat) (v : BitVec (8 * n)) (k : Nat)
    (h : m1.get? k = m2.get? k) : (memWrite m1 a v).get? k = (memWrite m2 a v).get? k :=
  foldl_insert_pointwise _ k m1 m2 h

/-- sc's memory: the store's word put at p, the store, the original word w
    put back: the store alone, when the store misses p's word. -/
theorem writeWord_memWrite {n : Nat} (m : Mem) (p : Nat) (w w' : BitVec 32) (hw : ZiskRiscv.readWord m p = some w)
    (a : Nat) (v : BitVec (8 * n)) (hd : a + n ≤ p ∨ p + 4 ≤ a) :
    ZiskRiscv.writeWord (memWrite (ZiskRiscv.writeWord m p w') a v) p w = memWrite m a v := by
  apply Std.ExtHashMap.ext_getElem?; intro k
  rw [← Std.ExtHashMap.get?_eq_getElem?, ← Std.ExtHashMap.get?_eq_getElem?, writeWord_get]
  have hm := congrArg (fun m => m.get? k) (writeWord_same m p w hw)
  simp only at hm
  rw [writeWord_get] at hm
  by_cases hin : p ≤ k ∧ k < p + 4
  · rw [memWrite_get_off m a v k (by omega)]
    rw [← hm]
    by_cases e0 : k = p <;> by_cases e1 : k = p + 1 <;> by_cases e2 : k = p + 2 <;> by_cases e3 : k = p + 3 <;>
      simp [e0, e1, e2, e3] <;> omega
  · have e : ¬ k = p ∧ ¬ k = p + 1 ∧ ¬ k = p + 2 ∧ ¬ k = p + 3 := by omega
    simp only [e.1, e.2.1, e.2.2.1, e.2.2.2, if_false]
    apply memWrite_pointwise
    rw [writeWord_get]; simp [e.1, e.2.1, e.2.2.1, e.2.2.2]

end Equiv
