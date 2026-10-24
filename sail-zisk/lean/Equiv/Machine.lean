/-
  ZisK's RISC-V machine (ZiskRiscv/Machine.lean) over sail-riscv's Lean, for
  the proofs: the `RiscvModel` instance, as ZiskRiscvRun/Model.lean makes it
  over the executable Lean, and the machine's step where the machine leaves
  the instruction to sail-riscv — what the simulation's steps are about.

  The machine steps sail-riscv with `try_step 0 true`; the step lemmas are
  about `try_step n false`, the same step once the hart is active
  (Equiv/RiscvTryStep.lean).
-/
import Equiv.ProgSpec
import Equiv.RiscvTryStep
import ZiskRiscv.Machine

namespace Equiv.Machine

open Equiv.Riscv Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- Where the machine's model points mtvec: outside ZisK's memory, so that a
    trap is seen as one (ZiskRiscvRun/Model.lean). -/
def TRAP_VEC : BitVec 64 := 0x4

/-- The initial ra: the BIOS's return address (contract § 3). -/
def EXIT_ADDR : BitVec 64 := 0x101C

def runM (m : SailM α) (s : RVState) : Except ZiskRiscv.StepError (α × RVState) :=
  match (interpretSailEffects m).run s with
  | .ok a s => .ok (a, s)
  | .error e _ => .error (.ofSail e)

/-- sail-riscv's Lean as the machine's RISC-V model. -/
noncomputable def model : ZiskRiscv.RiscvModel where
  State := RVState
  mem s := s.mem
  setMem s m := { s with mem := m }
  getX s r := (getX s r).getD 0
  setX s r v := setX s r v
  getPC s := (s.regs.get? Register.PC).getD 0
  setPC s v := { s with regs := s.regs.insert Register.PC v }
  step s := do let (_, s) ← runM (try_step 0 true) s; pure s
  trapped s := (s.regs.get? Register.PC).getD 0 == TRAP_VEC
  exitAddr := EXIT_ADDR

/-! ## The machine's decode on plain RISC-V -/

theorem bytes_word (e : BitVec 32) :
    @Eq (BitVec 32) (Equiv.Sim.byte e 3 ++ Equiv.Sim.byte e 2 ++ Equiv.Sim.byte e 1 ++ Equiv.Sim.byte e 0) e := by
  simp only [Equiv.Sim.byte]; bv_decide

/-- The word at a, from its four bytes in memory. -/
theorem readWord_of {m : Equiv.Mem} {a : Nat} {e : BitVec 32}
    (h0 : m.get? a = some (Equiv.Sim.byte e 0)) (h1 : m.get? (a + 1) = some (Equiv.Sim.byte e 1))
    (h2 : m.get? (a + 2) = some (Equiv.Sim.byte e 2)) (h3 : m.get? (a + 3) = some (Equiv.Sim.byte e 3)) :
    ZiskRiscv.readWord m a = some e := by
  simp only [ZiskRiscv.readWord, h0, h1, h2, h3, Option.pure_def, Option.bind_eq_bind, Option.bind_some]
  rw [bytes_word]

/-- An instruction outside SYSTEM's opcode is the model's to run. -/
theorem decode_riscv {fetch : Nat → Option (BitVec 32)} {a : Nat} {w : BitVec 32} (hf : fetch a = some w)
    (hop : w.extractLsb' 0 7 ≠ 0x73#7) : ZiskRiscv.decode fetch a = .riscv := by
  have : (ZiskRiscv.fields w).opcode ≠ 0x73 := by
    intro e; apply hop; apply BitVec.eq_of_toNat_eq; simpa [ZiskRiscv.fields] using e
  simp only [ZiskRiscv.decode, hf]
  generalize hk : ZiskRiscv.kind (ZiskRiscv.fields w) = k
  have : k ≠ .ecall ∧ k ≠ .csrrs ∧ k ≠ .csrrsi ∧ k ≠ .csrrwi := by
    subst hk
    simp only [ZiskRiscv.kind]
    split <;> simp_all <;> split <;> simp
  cases k <;> simp_all

/-! ## The machine's step on plain RISC-V -/

/-- The machine's step where the model runs the instruction: not at the exit
    address, plain RISC-V at pc, the model's step taken and not trapped. -/
theorem step_riscv {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (rv' : M.State)
    (hx : M.getPC S.rv ≠ M.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .riscv)
    (hs : M.step S.rv = .ok rv') (ht : M.trapped rv' = false) :
    ZiskRiscv.step S = .ok ({ S with rv := rv' }, .running) := by
  have e : ZiskRiscv.tryStep S = .ok ({ S with rv := rv' }, .running) := by
    simp only [ZiskRiscv.tryStep, hd, hs]
    simp [hx, bind, Except.bind, ht, pure, Except.pure]
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- The model's step, from a step of sail-riscv the simulation takes. -/
theorem model_step {s s' : RVState} {rs : List PMA_Region} (hr : StepReady s rs) {n : Nat}
    (h : (interpretSailEffects (try_step n false)).run s = .ok false s') : model.step s = .ok s' := by
  have e := (try_step_any s rs hr 0 true).trans ((try_step_any s rs hr n false).symm.trans h)
  simp [model, runM, e]; rfl

/-! ## The instructions of the simulation are plain RISC-V -/

theorem notSys_r (i : RInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [RInstr.encode]; bv_decide
theorem notSys_i (i : IInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [IInstr.encode]; bv_decide
theorem notSys_sh (i : SInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [SInstr.encode]; bv_decide
theorem notSys_u (i : UInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by
  cases h : i.op <;> simp only [UInstr.encode, h, uopEnc] <;> bv_decide
theorem notSys_ld (i : LdInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [LdInstr.encode]; bv_decide
theorem notSys_lw (i : LwInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [LwInstr.encode]; bv_decide
theorem notSys_sd (i : SdInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [SdInstr.encode]; bv_decide
theorem notSys_sw (i : SwInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [SwInstr.encode]; bv_decide
theorem notSys_br (i : BInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [BInstr.encode]; bv_decide
theorem notSys_j (i : JInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [JInstr.encode]; bv_decide
theorem notSys_jr (i : JrInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [JrInstr.encode]; bv_decide
theorem notSys_rw (i : RwInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [RwInstr.encode]; bv_decide
theorem notSys_iw (i : IwInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by simp only [IwInstr.encode]; bv_decide
theorem notSys_siw (i : SiwInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by
  simp only [SiwInstr.encode]; bv_decide
theorem notSys_m (i : MInstr) : i.encode.extractLsb' 0 7 ≠ 0x73#7 := by
  cases h : i.op <;> simp only [MInstr.encode, h, Transpiler.MOp.opc] <;> bv_decide

/-- What the machine needs of the instruction at pc to leave it to
    sail-riscv: executable there, a word not of SYSTEM's opcode. -/
def WordOk (m : Equiv.Mem) (rs : List PMA_Region) (pc : BitVec 64) : Prop :=
  ExecAt rs pc ∧ ∃ e : BitVec 32, ZiskRiscv.readWord m pc.toNat = some e ∧ e.extractLsb' 0 7 ≠ 0x73#7

theorem StepOkT.word {s : RVState} {rs : List PMA_Region} {pc : BitVec 64}
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} : ∀ o, StepOkT s rs pc zrom o → WordOk s.mem rs pc
  | .r i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_r i⟩
  | .i i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_i i⟩
  | .sh i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_sh i⟩
  | .u i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_u i⟩
  | .ld i, ⟨hi, _⟩ => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_ld i⟩
  | .lw i, ⟨hi, _⟩ => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_lw i⟩
  | .sd i, ⟨hi, _⟩ => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_sd i⟩
  | .sw i, ⟨hi, _⟩ => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_sw i⟩
  | .br i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_br i⟩
  | .j i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_j i⟩
  | .jr i, ⟨hi, _⟩ => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_jr i⟩
  | .rw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_rw i⟩
  | .iw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_iw i⟩
  | .siw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_siw i⟩
  | .m i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_m i⟩

theorem OneOkT.word {rs : List PMA_Region} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {s : RVState}
    (h : OneOkT rs zrom s) : ∃ pc, s.regs.get? Register.PC = some pc ∧ WordOk s.mem rs pc := by
  obtain ⟨pc, hpc, ⟨o, ho⟩ | ⟨i, hi, -⟩⟩ := h
  · exact ⟨pc, hpc, StepOkT.word o ho⟩
  · exact ⟨pc, hpc, hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_jr i⟩

/-- A state the runs go on from is at an executable pc. -/
theorem RunOkT.exec {rs : List PMA_Region} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {s : RVState} {n : Nat} :
    ∀ {k}, RunOkT rs zrom (k + 1) s n → ∃ pc, s.regs.get? Register.PC = some pc ∧ ExecAt rs pc
  | 0, h => by obtain ⟨pc, hpc, hw, -⟩ := OneOkT.word h; exact ⟨pc, hpc, hw⟩
  | _ + 1, .inl ⟨h, _⟩ => by obtain ⟨pc, hpc, hw, -⟩ := OneOkT.word h; exact ⟨pc, hpc, hw⟩
  | _ + 1, .inr ⟨⟨pc, _, hpc, hi⟩, _⟩ => ⟨pc, hpc, hi.x1⟩

/-! ## The machine's runs -/

/-- `n` steps of the machine, each leaving the run going. -/
inductive MRun : Nat → ZiskRiscv.State model → ZiskRiscv.State model → Prop
  | done (S : ZiskRiscv.State model) : MRun 0 S S
  | step {n : Nat} {S S' S'' : ZiskRiscv.State model} :
      ZiskRiscv.step S = .ok (S', .running) → MRun n S' S'' → MRun (n + 1) S S''

theorem MRun.append {a b : Nat} {S T U : ZiskRiscv.State model} (h1 : MRun a S T) (h2 : MRun b T U) :
    MRun (a + b) S U := by
  induction h1 with
  | done => simpa using h2
  | @step m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .step h (ih h2)

/-- Where a step leaves sail-riscv, not trapped: its pc not the trap vector. -/
def PcOk (s : RVState) : Prop := ∃ p, s.regs.get? Register.PC = some p ∧ p ≠ TRAP_VEC

theorem PcOk.of_exec {rs : List PMA_Region} (ht : ¬ ExecAt rs TRAP_VEC) {s : RVState} {p : BitVec 64}
    (hp : s.regs.get? Register.PC = some p) (hx : ExecAt rs p) : PcOk s :=
  ⟨p, hp, fun e => ht (e ▸ hx)⟩

/-- The machine's step on a step of sail-riscv the simulation takes: the
    instruction executable and plain RISC-V, the exit address not
    executable, and the step not trapped. -/
theorem mstep (S : ZiskRiscv.State model) (rs : List PMA_Region) (hr : StepReady S.rv rs) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (hw : WordOk S.rv.mem rs pc) (hx : ¬ ExecAt rs EXIT_ADDR)
    {n : Nat} {s' : RVState} (hrun : (interpretSailEffects (try_step n false)).run S.rv = .ok false s')
    (ht : PcOk s') : ZiskRiscv.step S = .ok ({ S with rv := s' }, .running) := by
  obtain ⟨hex, e, he, hop⟩ := hw
  obtain ⟨p, hp, hpt⟩ := ht
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  refine step_riscv S s' ?_ ?_ (model_step hr hrun) ?_
  · rw [hg]; intro h; have h' : pc = EXIT_ADDR := h; exact hx (h' ▸ hex)
  · rw [hg]; exact decode_riscv he hop
  · simp [model, hp, hpt]

/-! ## Whole runs on the machine -/

set_option maxHeartbeats 1000000 in
/-- The simulation over whole runs (`prog_spec`), on the machine: from
    related states, the steps `RunOkT` covers are also steps of ZisK's RISC-V
    machine, each leaving the run going — given that neither the exit
    address nor the trap vector is executable, and that the last step does
    not land on the trap vector. -/
theorem prog_machine (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region)
    (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) :
    ∀ (k : Nat) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel S.rv z rs → RunOkT rs zrom k S.rv n →
      ∃ s' z' k', RVRun n k S.rv s' ∧ ZRun zrom k' z z' ∧ Rel s' z' rs ∧
        (PcOk s' → MRun k S { S with rv := s' })
  | 0, S, z, n, hrel, _ => ⟨S.rv, z, 0, .done n _, .done z, hrel, fun _ => .done _⟩
  | 1, S, z, n, hrel, ho => by
    obtain ⟨s1, z1, k1, hrun, hzrun, hrel1, -⟩ := one_stepT S.rv z rs hrel zrom ho n
    obtain ⟨pc, hpc, hw⟩ := OneOkT.word ho
    exact ⟨s1, z1, k1, .step hrun (.done _ _), hzrun, hrel1,
      fun hp => .step (mstep S rs hrel.ready pc hpc hw hx hrun hp) (.done _)⟩
  | k + 2, S, z, n, hrel, .inl ⟨ho, hrest⟩ => by
    obtain ⟨s1, z1, k1, hrun, hzrun, hrel1, -⟩ := one_stepT S.rv z rs hrel zrom ho n
    obtain ⟨pc, hpc, hw⟩ := OneOkT.word ho
    rw [rvNext_of hrun] at hrest
    obtain ⟨p1, hp1, hx1⟩ := RunOkT.exec hrest
    have hm1 := mstep S rs hrel.ready pc hpc hw hx hrun (PcOk.of_exec ht hp1 hx1)
    obtain ⟨s2, z2, k2, hrun2, hzrun2, hrel2, hm2⟩ :=
      prog_machine zrom rs hx ht (k + 1) { S with rv := s1 } z1 (n + 1) hrel1 hrest
    exact ⟨s2, z2, k1 + k2, .step hrun hrun2, ZRun.append hzrun hzrun2, hrel2,
      fun hp => .step hm1 (hm2 hp)⟩
  | k + 2, S, z, n, hrel, .inr ⟨⟨pc, i, hpc, hi⟩, hrest⟩ => by
    obtain ⟨s2, z1, hrun, hzrun, hrel1, -, hr1, hpc1, hmem1, -⟩ :=
      fused_rel S.rv z rs hrel pc hpc zrom i hi.toFAt n
    obtain ⟨t1, h1, h2⟩ : ∃ t1, (interpretSailEffects (try_step n false)).run S.rv = .ok false t1 ∧
        (interpretSailEffects (try_step (n + 1) false)).run t1 = .ok false s2 := by
      have hrun' := hrun
      cases hrun' with
      | step h1 r => cases r with
        | step h2 r2 => cases r2 with
          | done => exact ⟨_, h1, h2⟩
    have e1 : rvNext n S.rv = t1 := rvNext_of h1
    rw [e1] at hr1 hpc1 hmem1
    rw [e1, rvNext_of h2] at hrest
    obtain ⟨s3, z3, k3, hrun3, hzrun3, hrel3, hm3⟩ :=
      prog_machine zrom rs hx ht k { S with rv := s2 } z1 (n + 2) hrel1 hrest
    have hw1 : WordOk S.rv.mem rs pc := ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_u _⟩
    have hw2 : WordOk t1.mem rs (pc + 4) :=
      ⟨hi.x3, _, readWord_of (by rw [hmem1]; exact hi.g0) (by rw [hmem1]; exact hi.g1)
        (by rw [hmem1]; exact hi.g2) (by rw [hmem1]; exact hi.g3), notSys_jr _⟩
    refine ⟨s3, z3, 1 + k3, by simpa [Nat.add_comm] using RVRun.append hrun hrun3,
      ZRun.append hzrun hzrun3, hrel3, fun hp => ?_⟩
    have hp2 : PcOk s2 := by
      cases k with
      | zero => cases hrun3; exact hp
      | succ k => obtain ⟨p, hp', hx'⟩ := RunOkT.exec hrest; exact PcOk.of_exec ht hp' hx'
    have m1 := mstep S rs hrel.ready pc hpc hw1 hx h1 (PcOk.of_exec ht hpc1 hi.x3)
    have m2 := mstep { S with rv := t1 } rs hr1 (pc + 4) hpc1 hw2 hx h2 hp2
    rw [show k + 2 = 0 + 1 + 1 + k by omega]
    exact MRun.append (.step m1 (.step m2 (.done _))) (hm3 hp)

/-! ## The machine's end -/

/-- The return to the initial ra: the run ends, with code 0. -/
theorem step_return {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (hx : M.getPC S.rv = M.exitAddr) :
    ZiskRiscv.step S = .ok (S, .exit 0) := by
  have e : ZiskRiscv.tryStep S = .ok (S, .exit 0) := by simp [ZiskRiscv.tryStep, hx]; rfl
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- `ecall`'s word. -/
def ECALL : BitVec 32 := 0x00000073

/-- The exit system call: the run ends, with code a0. -/
theorem step_exit {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (hx : M.getPC S.rv ≠ M.exitAddr)
    (hw : ZiskRiscv.readWord (M.mem S.rv) (M.getPC S.rv).toNat = some ECALL) (h17 : M.getX S.rv 17 = 93) :
    ZiskRiscv.step S = .ok (S, .exit (M.getX S.rv 10)) := by
  have hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .ecall := by
    simp only [ZiskRiscv.decode, hw]; rfl
  have e : ZiskRiscv.tryStep S = .ok (S, .exit (M.getX S.rv 10)) := by
    simp [ZiskRiscv.tryStep, hx, hd, h17]; rfl
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

theorem run_go_of {k : Nat} {S T T' : ZiskRiscv.State model} {st : ZiskRiscv.Status} (h : MRun k S T)
    (he : ZiskRiscv.step T = .ok (T', st)) (hst : st ≠ .running) (fuel n : Nat) :
    ZiskRiscv.run.go (k + 1 + fuel) n S = .ok (T', st, n + k) := by
  induction h generalizing n with
  | done =>
    rw [show 0 + 1 + fuel = fuel + 1 by omega]
    simp only [ZiskRiscv.run.go, he, bind, Except.bind]
    cases st <;> simp_all <;> rfl
  | @step m _ _ _ h1 _ ih =>
    rw [show m + 1 + 1 + fuel = (m + 1 + fuel) + 1 by omega]
    simp only [ZiskRiscv.run.go, h1, bind, Except.bind]
    rw [ih he (n + 1)]; congr 3; omega

/-- A run: steps that leave it going, then one that ends it. -/
theorem run_of {k : Nat} {S T T' : ZiskRiscv.State model} {st : ZiskRiscv.Status} (h : MRun k S T)
    (he : ZiskRiscv.step T = .ok (T', st)) (hst : st ≠ .running) (fuel : Nat) :
    ZiskRiscv.run (k + 1 + fuel) S = .ok (T', st, k) := by
  have := run_go_of h he hst fuel 0
  rw [Nat.zero_add] at this
  exact this

end Equiv.Machine
