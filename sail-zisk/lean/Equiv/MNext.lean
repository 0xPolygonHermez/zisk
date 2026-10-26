/-
  The step of ZisK's RISC-V machine (ZiskRiscv/Machine.lean) on the RISC-V
  state, as the runs follow it (`mNext`): sail-riscv's step, but on `lr` and
  `sc`, which the machine runs its own way (contract § 1), the plain load or
  store's word put at pc for the step, the original word put back after,
  and, for sc, rd then 0 (`mPost`). `MStep` is that step, taken: plain, or
  substituted.
-/
import Equiv.Run
import Equiv.WordMem
import Equiv.MemGrow
import ZiskRiscv.Machine

namespace Equiv.Machine

open Equiv.Riscv

/-- A word the machine leaves to the model: outside SYSTEM's opcode, and
    neither `lr` nor `sc` (opcode AMO, funct5 2 or 3), which the machine runs
    its own way. -/
def NotMach (w : BitVec 32) : Prop :=
  w.extractLsb' 0 7 ≠ 0x73#7 ∧
    ¬ (w.extractLsb' 0 7 = 0x2F#7 ∧ (w.extractLsb' 27 5 = 2#5 ∨ w.extractLsb' 27 5 = 3#5))

theorem funct5_eq (w : BitVec 32) : (ZiskRiscv.fields w).funct7 >>> 2 = (w.extractLsb' 27 5).toNat := by
  simp only [ZiskRiscv.fields, BitVec.extractLsb'_toNat, Nat.shiftRight_eq_div_pow]; omega

/-- An instruction the machine leaves to the model is the model's to run. -/
theorem decode_riscv {fetch : Nat → Option (BitVec 32)} {a : Nat} {w : BitVec 32} (hf : fetch a = some w)
    (hop : NotMach w) : ZiskRiscv.decode fetch a = .riscv := by
  obtain ⟨hop, hls⟩ := hop
  have : (ZiskRiscv.fields w).opcode ≠ 0x73 := by
    intro e; apply hop; apply BitVec.eq_of_toNat_eq; simpa [ZiskRiscv.fields] using e
  have hls' : ¬ ((ZiskRiscv.fields w).opcode = 0x2F ∧
      ((ZiskRiscv.fields w).funct7 >>> 2 = 2 ∨ (ZiskRiscv.fields w).funct7 >>> 2 = 3)) := by
    rintro ⟨e1, e2⟩; apply hls
    refine ⟨BitVec.eq_of_toNat_eq (by simpa [ZiskRiscv.fields] using e1), ?_⟩
    have := funct5_eq w
    rcases e2 with e2 | e2
    · exact .inl (BitVec.eq_of_toNat_eq (by rw [← this, e2]; rfl))
    · exact .inr (BitVec.eq_of_toNat_eq (by rw [← this, e2]; rfl))
  simp only [ZiskRiscv.decode, hf]
  generalize hk : ZiskRiscv.kind (ZiskRiscv.fields w) = k
  have : k ≠ .ecall ∧ k ≠ .csrrs ∧ k ≠ .csrrsi ∧ k ≠ .csrrwi ∧ k ≠ .lr ∧ k ≠ .sc := by
    subst hk
    simp only [ZiskRiscv.kind]
    split <;> simp_all <;> split <;> simp_all <;> split <;> simp_all
  cases k <;> simp_all

/-- A word the machine leaves to the model wherever it reads it: any fetch
    giving it at an address decodes there to `.riscv`. -/
def Plain (e : BitVec 32) : Prop :=
  ∀ (fetch : Nat → Option (BitVec 32)) (a : Nat), fetch a = some e → ZiskRiscv.decode fetch a = .riscv

theorem NotMach.plain {e : BitVec 32} (h : NotMach e) : Plain e := fun _ _ hf => decode_riscv hf h

end Equiv.Machine

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- After the substituted step: the word w put back at p, and register zr
    (sc's rd; 0, no register, for lr) set to 0. -/
def mPost (s1 : RVState) (p : Nat) (w : BitVec 32) (zr : Nat) : RVState :=
  setX { s1 with mem := ZiskRiscv.writeWord s1.mem p w } zr 0

/-- The state the machine's step `n` leads to: sail-riscv's (`rvNext`), or,
    on lr and sc, sail-riscv's on the plain word, then `mPost`. -/
noncomputable def mNext (n : Nat) (s : RVState) : RVState :=
  let p := ((s.regs.get? Register.PC).getD 0).toNat
  match ZiskRiscv.decode (ZiskRiscv.readWord s.mem) p, ZiskRiscv.readWord s.mem p with
  | .subst w' zr, some w => mPost (rvNext n { s with mem := ZiskRiscv.writeWord s.mem p w' }) p w zr
  | _, _ => rvNext n s

/-- The state k steps of the machine reach from s, from step n. -/
noncomputable def rvIter : Nat → Nat → RVState → RVState
  | _, 0, s => s
  | n, k + 1, s => rvIter (n + 1) k (mNext n s)

theorem mNext_plain {n : Nat} {s : RVState} {pc : BitVec 64} {e : BitVec 32}
    (hpc : s.regs.get? Register.PC = some pc) (hw : ZiskRiscv.readWord s.mem pc.toNat = some e)
    (hop : Equiv.Machine.Plain e) : mNext n s = rvNext n s := by
  simp only [mNext, hpc, Option.getD_some, hop _ _ hw]

theorem mNext_subst {n : Nat} {s : RVState} {pc : BitVec 64} {w w' : BitVec 32} {zr : Nat}
    (hpc : s.regs.get? Register.PC = some pc) (hw : ZiskRiscv.readWord s.mem pc.toNat = some w)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord s.mem) pc.toNat = .subst w' zr) :
    mNext n s = mPost (rvNext n { s with mem := ZiskRiscv.writeWord s.mem pc.toNat w' }) pc.toNat w zr := by
  simp only [mNext, hpc, Option.getD_some, hd, hw]

/-- One step of the machine on the RISC-V state, taken: sail-riscv's on a
    word the machine leaves to it; or, on lr and sc, sail-riscv's on the
    plain word w', from a state ready for it, then `mPost`. -/
inductive MStep (rs : List PMA_Region) (n : Nat) (s s' : RVState) : Prop
  | plain (pc : BitVec 64) (e : BitVec 32) :
      s.regs.get? Register.PC = some pc → ZiskRiscv.readWord s.mem pc.toNat = some e → Equiv.Machine.Plain e →
      (interpretSailEffects (try_step n false)).run s = .ok false s' → MStep rs n s s'
  | subst (pc : BitVec 64) (w w' : BitVec 32) (zr : Nat) (s1 : RVState) :
      s.regs.get? Register.PC = some pc → ZiskRiscv.readWord s.mem pc.toNat = some w →
      (∀ fetch : Nat → Option (BitVec 32), fetch pc.toNat = some w →
        ZiskRiscv.decode fetch pc.toNat = .subst w' zr) →
      StepReady { s with mem := ZiskRiscv.writeWord s.mem pc.toNat w' } rs →
      (interpretSailEffects (try_step n false)).run { s with mem := ZiskRiscv.writeWord s.mem pc.toNat w' } =
        .ok false s1 →
      s' = mPost s1 pc.toNat w zr → MStep rs n s s'

theorem MStep.next {rs : List PMA_Region} {n : Nat} {s s' : RVState} (h : MStep rs n s s') : mNext n s = s' := by
  cases h with
  | plain pc e hpc hw hop hr => rw [mNext_plain hpc hw hop, rvNext_of hr]
  | subst pc w w' zr s1 hpc hw hd _ hr he => rw [mNext_subst hpc hw (hd _ hw), rvNext_of hr, he]

theorem writeWord_grows (m : Mem) (p : Nat) (w : BitVec 32) : Grows m (ZiskRiscv.writeWord m p w) := by
  intro a ha
  rw [Equiv.writeWord_get]
  split <;> (try rfl) <;> split <;> (try rfl) <;> split <;> (try rfl) <;> split <;> (try rfl) <;> exact ha

theorem MStep.grows {rs : List PMA_Region} {n : Nat} {s s' : RVState} (h : MStep rs n s s') : Grows s.mem s'.mem := by
  cases h with
  | plain _ _ _ _ _ hr => exact interp_grows _ _ _ _ hr
  | subst pc w w' zr s1 _ _ _ _ hr he =>
    subst he
    have e : (mPost s1 pc.toNat w zr).mem = ZiskRiscv.writeWord s1.mem pc.toNat w := by
      simp only [mPost, setX]; split <;> rfl
    rw [e]
    exact ((writeWord_grows _ _ _).trans (interp_grows _ _ _ _ hr)).trans (writeWord_grows _ _ _)

end Equiv.Sim
