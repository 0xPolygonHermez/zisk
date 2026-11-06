/-
  A ZisK call on both sides: the machine runs sail-zisk's operation on a
  ZisK state it builds from the RISC-V memory and its fcall state
  (`ZiskRiscv.runOp`, `ziskState`); ZisK runs it on its own. Both are runs
  of the same operation from states that agree on memory and the fcall
  registers, so by `Frame.exec_mem` both are the reference run's — on the
  RISC-V memory without the input's first 8 bytes, whose success is the
  call's condition.
-/
import Equiv.ZiskCall
import ZiskRiscv.Machine
import Equiv.MemGrow

set_option linter.unusedSimpArgs false

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- An fcall register's value in the machine's fcall state. -/
def fval (f : ZiskRiscv.Fcall) : (r : Register) → Option (RegisterType r)
  | .FreeInput => some f.freeInput
  | .FcallNext => some f.next
  | .FcallBase => some f.base
  | .FcallSize => some f.size
  | .FcallGot => some f.got
  | _ => none

/-- ZisK's fcall registers hold the machine's fcall state. -/
def FcallRel (f : ZiskRiscv.Fcall) (z : ZState) : Prop := ∀ r, fregs r = true → z.regs.get? r = fval f r

theorem ziskState_fregs (m : Mem) (f : ZiskRiscv.Fcall) :
    ∀ r, fregs r = true → (ZiskRiscv.ziskState m f).regs.get? r = fval f r := by
  intro r hr
  cases r <;> simp [fregs] at hr <;> simp [ZiskRiscv.ziskState, fval, Std.ExtDHashMap.get?_insert]

/-- The reference state: the RISC-V memory without the input's first 8 bytes. -/
theorem FR.ref_machine (m : Mem) (f : ZiskRiscv.Fcall) :
    FR (ZiskRiscv.ziskState (Equiv.normIn m) f) (ZiskRiscv.ziskState m f) := by
  refine ⟨fun k v h => ?_, fun r _ => rfl⟩
  have h' : (Equiv.normIn m).get? k = some v := h
  rw [Equiv.normIn_get] at h'
  split at h'
  · cases h'
  · exact h'

theorem FR.ref_zisk {m : Mem} {f : ZiskRiscv.Fcall} {z : ZState} (hzm : Equiv.normIn z.mem = Equiv.normIn m)
    (hf : FcallRel f z) : FR (ZiskRiscv.ziskState (Equiv.normIn m) f) z := by
  refine ⟨fun k v h => ?_, fun r hr => by rw [ziskState_fregs _ _ r hr, hf r hr]⟩
  have h' : (Equiv.normIn m).get? k = some v := h
  rw [Equiv.normIn_get] at h'
  split at h'
  · cases h'
  · next hk => rw [(Equiv.normIn_eq_iff _ _).1 hzm k hk]; exact h'

/-- The fcall state the machine reads back from a ZisK state (`runOp`). -/
def fcallOf (s : ZState) : ZiskRiscv.Fcall :=
  let get (r : Register) (h : RegisterType r = BitVec 64) : BitVec 64 :=
    match s.regs.get? r with | some v => h ▸ v | none => 0
  { freeInput := get .FreeInput rfl, next := get .FcallNext rfl, base := get .FcallBase rfl,
    size := get .FcallSize rfl, got := get .FcallGot rfl }

theorem FcallRel.of_some {s : ZState} (h : ∀ r, fregs r = true → s.regs.get? r ≠ none) : FcallRel (fcallOf s) s := by
  intro r hr
  cases r <;> simp [fregs] at hr <;> simp only [fcallOf, fval] <;>
  · split
    · next v e => rw [e]
    · next e => exact absurd e (h _ rfl)

set_option maxHeartbeats 4000000 in
/-- A call's operation on both sides: given the reference run (on the
    RISC-V memory without the input's first 8 bytes), the machine's run
    (`runOp`) and ZisK's give its result, memories equal without those
    bytes, and ZisK's fcall registers the machine's fcall state; ZisK's
    other registers untouched. -/
theorem runOp_sim (op : zisk_op) (a b ext : BitVec 64) (m : Mem) (f : ZiskRiscv.Fcall) (z : ZState)
    (hzm : Equiv.normIn z.mem = Equiv.normIn m) (hf : FcallRel f z)
    (r : Option ((BitVec 64) × Bool)) (σ' : ZState)
    (href : (zisk_exec_mem op a b (if zisk_op_is_precompiled op then ext else 0)).run
      (ZiskRiscv.ziskState (Equiv.normIn m) f) = .ok r σ') :
    ∃ m' f' z1, ZiskRiscv.runOp op a b ext m f = .ok ((opResult op a b r).1, m', f') ∧
      (zisk_exec_mem op a b (if zisk_op_is_precompiled op then ext else 0)).run z = .ok r z1 ∧
      Equiv.normIn z1.mem = Equiv.normIn m' ∧ FcallRel f' z1 ∧
      (∀ reg, fregs reg = false → z1.regs.get? reg = z.regs.get? reg) ∧
      Equiv.RamOnly m m' ∧ Equiv.RamOnly z.mem z1.mem ∧ Equiv.Grows m m' := by
  obtain ⟨τ1, ht1, hfr1, -, hk1⟩ :=
    (Frame.exec_mem op a b _).1 _ _ (FR.ref_machine m f) r σ' href
  obtain ⟨z1, hz1, hfr2, hr2, hk2⟩ :=
    (Frame.exec_mem op a b _).1 _ _ (FR.ref_zisk hzm hf) r σ' href
  have hsome : ∀ reg, fregs reg = true → σ'.regs.get? reg ≠ none := fun reg hr =>
    hk1.rgrow reg (by rw [ziskState_fregs _ _ reg hr]; cases reg <;> simp [fregs] at hr <;> simp [fval])
  refine ⟨τ1.mem, fcallOf τ1, z1, ?_, hz1, ?_, fun reg hr => ?_, hr2, hk1.ram, hk2.ram, fun k hk => ?_⟩
  · simp only [ZiskRiscv.runOp, run_bind', ht1]
    cases r <;> simp [opResult, fcallOf] <;> first | rfl | exact ⟨rfl, rfl, rfl, rfl, rfl⟩
  · rw [Equiv.normIn_eq_iff]
    intro k hk
    cases hs : σ'.mem.get? k with
    | some v => rw [hfr2.mem k v hs, hfr1.mem k v hs]
    | none =>
      have h0 := hk1.none hs
      rw [hk2.same k h0 hs, hk1.same k h0 hs]
      have : (ZiskRiscv.ziskState m f).mem = m := rfl
      rw [this]; exact (Equiv.normIn_eq_iff _ _).1 hzm k hk
  · have e := FcallRel.of_some (fun reg h => by rw [← hfr1.regs reg h]; exact hsome reg h) reg hr
    rw [← hfr2.regs reg hr, hfr1.regs reg hr]; exact e
  · -- a byte of m stays: where the reference run has one, τ1 has it too; else it is as in m
    cases hs : σ'.mem.get? k with
    | some v => rw [hfr1.mem k v hs]; rfl
    | none =>
      have h0 := hk1.none hs
      rw [hk1.same k h0 hs]; exact hk

/-- The machine's `runOp`, from the operation's run on its ZisK state. -/
theorem runOp_ok {op : zisk_op} {a b ext : BitVec 64} {m : Mem} {f : ZiskRiscv.Fcall}
    {r : Option ((BitVec 64) × Bool)} {σ' : ZState}
    (h : (zisk_exec_mem op a b (if zisk_op_is_precompiled op then ext else 0)).run (ZiskRiscv.ziskState m f) =
      .ok r σ') :
    ZiskRiscv.runOp op a b ext m f = .ok ((opResult op a b r).1, σ'.mem, fcallOf σ') := by
  simp only [ZiskRiscv.runOp, run_bind', h]
  cases r <;> simp [opResult, fcallOf] <;> first | rfl | exact ⟨rfl, rfl, rfl, rfl, rfl⟩

theorem runOp_err {op : zisk_op} {a b ext : BitVec 64} {m : Mem} {f : ZiskRiscv.Fcall}
    {e : Sail.Error exception} {σ' : ZState}
    (h : (zisk_exec_mem op a b (if zisk_op_is_precompiled op then ext else 0)).run (ZiskRiscv.ziskState m f) =
      .error e σ') :
    ZiskRiscv.runOp op a b ext m f = .error (.ofSail e) := by
  simp only [ZiskRiscv.runOp, run_bind', h]

/-- Two ZisK states with the same fcall registers give the same fcall state. -/
theorem fcallOf_eq {σ τ : ZState} (h : ∀ r, fregs r = true → σ.regs.get? r = τ.regs.get? r) :
    fcallOf σ = fcallOf τ := by
  simp only [fcallOf, h .FreeInput rfl, h .FcallNext rfl, h .FcallBase rfl, h .FcallSize rfl, h .FcallGot rfl]

end Equiv.Zisk
