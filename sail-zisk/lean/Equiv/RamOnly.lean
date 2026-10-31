/-
  A step that changes only ZisK's RAM: every byte outside
  [RAM_ADDR, RAM_ADDR + 512 MB) as it was. What the simulation's steps do —
  a store only where sail-zisk's `writable` allows, the RAM — and so the code,
  outside the RAM, stays what the transpiler translated.
-/
import Equiv.MemWrite

namespace Equiv

/-- Outside the RAM. -/
def OffRam (a : Nat) : Prop := a < 0xA000_0000 ∨ 0xC000_0000 ≤ a

/-- m' is m but in the RAM. -/
def RamOnly (m m' : Mem) : Prop := ∀ a, OffRam a → m'.get? a = m.get? a

theorem RamOnly.refl (m : Mem) : RamOnly m m := fun _ _ => rfl

theorem RamOnly.of_eq {m m' : Mem} (h : m' = m) : RamOnly m m' := h ▸ RamOnly.refl m

theorem RamOnly.trans {m1 m2 m3 : Mem} (h1 : RamOnly m1 m2) (h2 : RamOnly m2 m3) : RamOnly m1 m3 :=
  fun a ha => (h2 a ha).trans (h1 a ha)

/-- sail-zisk's `writable`: the w bytes at a in the RAM. -/
theorem writable_ram {a : BitVec 64} {w : Nat} (h : _root_.Zisk.Functions.writable a w = true) :
    0xA000_0000 ≤ a.toNat ∧ a.toNat + w ≤ 0xC000_0000 := by
  simp only [_root_.Zisk.Functions.writable, _root_.Zisk.Functions.mem_region_of] at h
  split at h <;> try simp at h
  rename_i hr
  split at hr <;> try simp at hr
  split at hr <;> try simp at hr
  simp [Sail.BitVec.toNatInt] at hr
  omega

theorem foldl_insert_off (l : List (Nat × BitVec 8)) (a : Nat) (hl : ∀ p ∈ l, p.1 ≠ a) :
    ∀ m : Mem, (l.foldl (fun m p => m.insert p.1 p.2) m).get? a = m.get? a := by
  induction l with
  | nil => intro m; rfl
  | cons p l ih =>
    intro m
    simp only [List.foldl_cons]
    rw [ih (fun q hq => hl q (List.mem_cons_of_mem _ hq))]
    simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert]
    have := hl p List.mem_cons_self
    simp [this]

/-- A store sail-zisk allows changes only the RAM. -/
theorem memWrite_ramOnly {w : Nat} (m : Mem) (a : BitVec 64) (v : BitVec (8 * w))
    (h : _root_.Zisk.Functions.writable a w = true) : RamOnly m (memWrite m a.toNat v) := by
  intro b hb
  have := writable_ram h
  apply foldl_insert_off
  intro p hp
  simp only [byteList, List.mem_ofFn] at hp
  obtain ⟨i, rfl⟩ := hp
  simp only
  unfold OffRam at hb
  omega

end Equiv
