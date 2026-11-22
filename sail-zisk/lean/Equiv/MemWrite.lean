/-
  Writing n bytes to a byte map, little-endian: what both interpreters'
  `writeBytes` do — lean-sail's sequential one under sail-riscv and its
  concurrency-interface one under sail-zisk. Stated once, so that a store
  leaves the same memory on both sides.
-/
import Equiv.MemRead

namespace Equiv

/-- The bytes of v at a, a+1, …, as both interpreters list them. -/
def byteList (a : Nat) (v : BitVec (8 * n)) : List (Nat × BitVec 8) :=
  List.ofFn (fun i : Fin n => (a + i.val, v.extractLsb' (8 * i.val) 8))

/-- The map after writing v's n bytes at a. -/
def memWrite (m : Mem) (a : Nat) (v : BitVec (8 * n)) : Mem :=
  (byteList a v).foldl (fun m p => m.insert p.1 p.2) m

theorem riscv_writeBytes_list (l : List (Nat × BitVec 8)) (s : Equiv.Riscv.RVState) :
    (List.forM l (fun p => (Sail.ArchSem.writeByte p.1 p.2 :
        Sail.ArchSem.ArchSemSeqM LeanZiskRV.exception _))).run s
      = .ok () { s with mem := l.foldl (fun m p => m.insert p.1 p.2) s.mem } := by
  induction l generalizing s with
  | nil => rfl
  | cons p l ih =>
    simp only [List.foldl_cons]
    show (Sail.ArchSem.writeByte p.1 p.2 >>= fun _ => List.forM l _).run s = _
    simp only [Equiv.Riscv.run_bind']
    have : (Sail.ArchSem.writeByte p.1 p.2 : Sail.ArchSem.ArchSemSeqM LeanZiskRV.exception _).run s
        = .ok () { s with mem := s.mem.insert p.1 p.2 } := rfl
    rw [this]; dsimp only; rw [ih]

theorem riscv_writeBytes (s : Equiv.Riscv.RVState) (a : Nat) (v : BitVec (8 * n)) :
    (Sail.ArchSem.writeBytes a v : Sail.ArchSem.ArchSemSeqM LeanZiskRV.exception _).run s
      = .ok () { s with mem := memWrite s.mem a v } := by
  simp only [Sail.ArchSem.writeBytes]
  exact riscv_writeBytes_list _ s

theorem zisk_writeBytes_list (l : List (Nat × BitVec 8)) (z : Equiv.Zisk.ZState) :
    (List.forM l (fun p => (Sail.ConcurrencyInterfaceV1.PreSail.writeByte p.1 p.2 : _root_.Zisk.SailM _))).run z
      = .ok () { z with mem := l.foldl (fun m p => m.insert p.1 p.2) z.mem } := by
  induction l generalizing z with
  | nil => rfl
  | cons p l ih =>
    simp only [List.foldl_cons]
    show (Sail.ConcurrencyInterfaceV1.PreSail.writeByte p.1 p.2 >>= fun _ => List.forM l _).run z = _
    simp only [Equiv.Zisk.run_bind']
    have : (Sail.ConcurrencyInterfaceV1.PreSail.writeByte p.1 p.2 : _root_.Zisk.SailM _).run z
        = .ok () { z with mem := z.mem.insert p.1 p.2 } := rfl
    rw [this]; dsimp only; rw [ih]

theorem zisk_writeBytes (z : Equiv.Zisk.ZState) (a : Nat) (v : BitVec (8 * n)) :
    (Sail.ConcurrencyInterfaceV1.PreSail.writeBytes a v : _root_.Zisk.SailM _).run z
      = .ok true { z with mem := memWrite z.mem a v } := by
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.writeBytes, Equiv.Zisk.run_bind']
  rw [zisk_writeBytes_list]; rfl

/-- The bytes of hi ++ lo: lo's, then hi's. -/
theorem byteList_append {p q : Nat} (a : Nat) (lo : BitVec (8 * p)) (hi : BitVec (8 * q)) :
    byteList (n := p + q) a ((show 8 * q + 8 * p = 8 * (p + q) by omega) ▸ (hi ++ lo)) =
      byteList a lo ++ byteList (a + p) hi := by
  simp only [byteList, List.ofFn_add, rec_cast]
  congr 1
  · congr 1; funext i
    simp only [Fin.val_castLE, Prod.mk.injEq, true_and]
    apply BitVec.eq_of_getLsbD_eq; intro j hj
    have := i.isLt
    simp [BitVec.getLsbD_append, hj]
    intro h; omega
  · congr 1; funext i
    simp only [Fin.val_natAdd, Prod.mk.injEq]
    refine ⟨by omega, ?_⟩
    apply BitVec.eq_of_getLsbD_eq; intro j hj
    simp [BitVec.getLsbD_append, hj]
    rw [if_neg (by omega)]; congr 1; omega

/-- Two writes side by side, lo's then hi's after it: one write of both. -/
theorem memWrite_append {p q : Nat} (m : Mem) (a : Nat) (lo : BitVec (8 * p)) (hi : BitVec (8 * q)) :
    memWrite (memWrite m a lo) (a + p) hi =
      memWrite (n := p + q) m a ((show 8 * q + 8 * p = 8 * (p + q) by omega) ▸ (hi ++ lo)) := by
  simp only [memWrite, byteList_append, List.foldl_append]

end Equiv
