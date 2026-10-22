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

end Equiv
