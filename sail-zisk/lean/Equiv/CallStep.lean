/-
  A ZisK call, one step of the machine (`ZiskRiscv.call`) and ZisK's steps
  on the spec's translation (`Transpiler.callInsts`), from related states
  to related states (`Equiv/CallSim.lean` for the operation).
-/
import Equiv.CallSim
import Equiv.Machine

set_option linter.unusedSimpArgs false

namespace Equiv.Machine

open Equiv.Riscv Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The machine's step on a ZisK call: not at the exit address, the call
    decoded at pc, its run. -/
theorem step_call {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (c : ZiskRiscv.Call) (S' : ZiskRiscv.State M)
    (hx : M.getPC S.rv ≠ M.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .call c)
    (hc : ZiskRiscv.call S c = .ok S') :
    ZiskRiscv.step S = .ok (S', .running) := by
  have e : ZiskRiscv.tryStep S = .ok (S', .running) := by
    simp only [ZiskRiscv.tryStep, hd]
    simp [hx, bind, Except.bind, hc, pure, Except.pure]
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- The memory a call's operation starts from: its count first stored at
    EXTRA_PARAMS_ADDR, if it has one. -/
noncomputable def callMem (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) : Equiv.Mem :=
  match c.count with
  | some r => ZiskRiscv.writeDword (model.mem S.rv) ZiskRiscv.EXTRA_PARAMS_ADDR (model.getX S.rv r)
  | none => model.mem S.rv

/-- The machine's call, its operation's run given. -/
theorem call_run (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) (cval : BitVec 64) (m' : Equiv.Mem)
    (f' : ZiskRiscv.Fcall)
    (hrun : ZiskRiscv.runOp c.op (S.argValue c.a) (S.argValue c.b) c.ext (callMem S c) S.fcall = .ok (cval, m', f')) :
    ZiskRiscv.call S c = .ok
      { rv := model.setPC (if c.rd != 0 then model.setX (model.setMem S.rv
                (ZiskRiscv.writeDword m' ZiskRiscv.INPUT_ADDR f'.freeInput)) c.rd cval
              else model.setMem S.rv (ZiskRiscv.writeDword m' ZiskRiscv.INPUT_ADDR f'.freeInput))
            (model.getPC S.rv + BitVec.ofNat 64 (4 * c.len)),
        fcall := f' } := by
  unfold ZiskRiscv.call
  simp only [bind, Except.bind]
  cases hc : c.count <;> simp only [callMem, hc] at hrun <;> simp only [hc, hrun] <;> rfl

/-- The machine's 8-byte write is the interpreters' (`memWrite`). -/
theorem writeDword_eq (m : Equiv.Mem) (a : Nat) (v : BitVec 64) :
    ZiskRiscv.writeDword m a v = Equiv.memWrite (n := 8) m a v := by
  simp [ZiskRiscv.writeDword, Equiv.memWrite, Equiv.byteList, List.range, List.range.loop, List.ofFn_succ]

/-- The free input's mirror is in the bytes the relation leaves aside. -/
theorem normIn_writeDword_input (m : Equiv.Mem) (v : BitVec 64) :
    Equiv.normIn (ZiskRiscv.writeDword m ZiskRiscv.INPUT_ADDR v) = Equiv.normIn m := by
  rw [Equiv.normIn_eq_iff, writeDword_eq]
  intro k hk
  exact Equiv.memWrite_get_off _ _ _ _ (by simp only [ZiskRiscv.INPUT_ADDR, Equiv.IN_LO] at hk ⊢; omega)

/-- A call as the machine's decoder makes it: registers x0..x31, a not the
    free input, an operation that does not halt and leaves its flag clear,
    a precompile no fcall, a count's call no extended argument. -/
structure CallWF (c : ZiskRiscv.Call) : Prop where
  rd : c.rd < 32
  a : ∀ r, c.a = .reg r → r < 32
  afree : c.a ≠ .freeInput
  b : ∀ r, c.b = .reg r → r < 32
  halt : ∀ u, c.op ≠ .ZOP_HALT u
  pflag : ∀ a b, (_root_.Zisk.Functions.zisk_exec c.op a b).2 = false
  pre : _root_.Zisk.Functions.zisk_op_is_precompiled c.op = true → Transpiler.fcallOp c.op = false
  count : ∀ r, c.count = some r → r < 32 ∧ c.ext = 0

/-- A call's condition: its operation runs on the memory it starts from,
    without the input's first 8 bytes (precondition I). -/
def CallOk (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) : Prop :=
  ∃ r σ', (_root_.Zisk.Functions.zisk_exec_mem c.op (S.argValue c.a) (S.argValue c.b)
    (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0)).run
    (ZiskRiscv.ziskState (Equiv.normIn (callMem S c)) S.fcall) = .ok r σ'

/-- The RISC-V state after a call: memory m (the free input mirrored), rd
    its c, pc past the sequence. -/
def callPost (s : RVState) (m : Equiv.Mem) (rd : Nat) (c : BitVec 64) (pc : BitVec 64) : RVState :=
  { setX { s with mem := m } rd c with regs := (setX { s with mem := m } rd c).regs.insert Register.PC pc }

theorem callPost_mem (s : RVState) (m : Equiv.Mem) (rd : Nat) (c pc : BitVec 64) :
    (callPost s m rd c pc).mem = m := by
  simp only [callPost, setX]; split <;> rfl

theorem call_post_eq (s : RVState) (m : Equiv.Mem) (rd : Nat) (c pc : BitVec 64) :
    model.setPC (if rd != 0 then model.setX (model.setMem s m) rd c else model.setMem s m) pc = callPost s m rd c pc := by
  simp only [model, callPost]
  by_cases h : rd = 0
  · subst h; simp [setX]
  · simp [h]

theorem getX_setPC (s : RVState) (pc : BitVec 64) (k : Nat) :
    getX { s with regs := s.regs.insert Register.PC pc } k = getX s k := by
  unfold getX; split
  · rfl
  · have hne : (Register.PC == xreg k) = false := by
      cases e : (Register.PC == xreg k)
      · rfl
      · simp at e; have := xreg_isX k; rw [← e] at this; cases this
    simp [Std.ExtDHashMap.get?_insert, hne]

/-- The argument values on both sides. -/
theorem argA_src {s : RVState} {xs : Vector (BitVec 64) 40} (agree : RegsAgree s xs) (f : ZiskRiscv.Fcall)
    (a : ZiskRiscv.Arg) (ha : ∀ r, a = .reg r → r < 32) (hf : a ≠ .freeInput) :
    Equiv.Zisk.ASrc (Transpiler.argA a) xs (({ rv := s, fcall := f } : ZiskRiscv.State model).argValue a) := by
  cases a with
  | reg r =>
    have hr := ha r rfl
    have e : (({ rv := s, fcall := f } : ZiskRiscv.State model).argValue (.reg r)) = Equiv.Zisk.regv xs r := by
      simp [ZiskRiscv.State.argValue, model, getX_regv agree hr]
    rw [e]; exact Equiv.Zisk.srcA_ASrc xs r hr
  | imm v => exact Or.inl rfl
  | freeInput => exact absurd rfl hf

theorem argB_src (s : RVState) (z : Equiv.Zisk.ZState) {xs : Vector (BitVec 64) 40} (agree : RegsAgree s xs)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (f : ZiskRiscv.Fcall) (hf : Equiv.Zisk.FcallRel f z)
    (b : ZiskRiscv.Arg) (hb : ∀ r, b = .reg r → r < 32) (ra : _root_.Zisk.a_src × BitVec 64) (op : _root_.Zisk.zisk_op)
    (st : _root_.Zisk.c_store × BitVec 64 × Bool) (j1 j2 va : BitVec 64) :
    (_root_.Zisk.Functions.source_b (Transpiler.mk ra (Transpiler.argB b) op st j1 j2) va).run z =
      .ok (({ rv := s, fcall := f } : ZiskRiscv.State model).argValue b) z := by
  cases b with
  | reg r =>
    have hr := hb r rfl
    have e : (({ rv := s, fcall := f } : ZiskRiscv.State model).argValue (.reg r)) = Equiv.Zisk.regv xs r := by
      simp [ZiskRiscv.State.argValue, model, getX_regv agree hr]
    rw [e]; exact Equiv.Zisk.source_b_run z ra _ op st j1 j2 xs hx va _ (Equiv.Zisk.srcB_BSrc xs r hr)
  | imm v => exact Equiv.Zisk.source_b_run z ra _ op st j1 j2 xs hx va v (Or.inl rfl)
  | freeInput =>
    exact Equiv.Zisk.source_b_free z ra op st j1 j2 va _ (by rw [hf .FreeInput rfl]; rfl)

set_option maxHeartbeats 16000000 in
/-- A ZisK call's operation: the machine's call, and ZisK's step on the
    spec's instruction for it at p (the call's own pc, or, after a count's
    store, an internal address), on past the sequence; from states related
    but for ZisK's pc and memory, its memory the call's (`callMem`). -/
theorem zcall_step (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) (hw : CallWF c) (rs : List PMA_Region)
    (hr : StepReady S.rv rs)
    (xs : Vector (BitVec 64) 40) (agree : RegsAgree S.rv xs) (pc' : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc')
    (z : Equiv.Zisk.ZState) (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (p : BitVec 64)
    (hzpc : z.regs.get? _root_.Zisk.Register.PC = some p) (st : BitVec 64)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (hze : z.regs.get? _root_.Zisk.Register.Zerror = some false)
    (hzm : Equiv.normIn z.mem = Equiv.normIn (callMem S c)) (hf : Equiv.Zisk.FcallRel S.fcall z)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (j1 j2 : BitVec 64)
    (hrom : zrom p = Transpiler.mk (Transpiler.argA c.a) (Transpiler.argB c.b) c.op (Transpiler.storeReg c.rd) j1 j2)
    (hj1 : (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then j1 else 0) =
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0))
    (hj2 : p + j2 = pc' + BitVec.ofNat 64 (4 * c.len))
    (hok : CallOk S c) :
    ∃ S' z', ZiskRiscv.call S c = .ok S' ∧ (zstep zrom).run z = .ok false z' ∧ Rel S'.rv z' rs ∧
      Equiv.Zisk.FcallRel S'.fcall z' ∧
      (∀ a, Equiv.OffRam a → ¬ (Equiv.IN_LO ≤ a ∧ a < Equiv.IN_LO + 8) → S'.rv.mem.get? a = (callMem S c).get? a) := by
  obtain ⟨r, σ', href⟩ := hok
  obtain ⟨m', f', z1, hrunOp, hz1, hnorm, hfr, hregs, hram1, hram2⟩ :=
    Equiv.Zisk.runOp_sim c.op (S.argValue c.a) (S.argValue c.b) c.ext (callMem S c) S.fcall z hzm hf r σ' href
  -- the machine
  have hcall := call_run S c _ m' f' hrunOp
  rw [call_post_eq] at hcall
  -- ZisK
  have hx1 : z1.regs.get? _root_.Zisk.Register.Xs = some xs := by rw [hregs _ rfl]; exact hx
  have hpc1 : z1.regs.get? _root_.Zisk.Register.PC = some p := by rw [hregs _ rfl]; exact hzpc
  have hst1 : z1.regs.get? _root_.Zisk.Register.Step = some st := by rw [hregs _ rfl]; exact hst
  have hze1 : z1.regs.get? _root_.Zisk.Register.Zerror = some false := by rw [hregs _ rfl]; exact hze
  have hflag : (Equiv.Zisk.opResult c.op (S.argValue c.a) (S.argValue c.b) r).2 = false := by
    cases r with
    | none => exact hw.pflag _ _
    | some p => exact Equiv.Zisk.exec_mem_flag c.op _ _ _ z p z1 hz1 p.1 p.2 rfl
  generalize hcf : Equiv.Zisk.opResult c.op (S.argValue c.a) (S.argValue c.b) r = cf at hflag hcall
  obtain ⟨cv, fl⟩ := cf
  simp only at hflag; subst hflag
  have hex := Equiv.Zisk.execute_zcall z (Transpiler.argA c.a) (Transpiler.argB c.b) c.op c.rd (by have := hw.rd; omega)
    j1 j2 xs p st
    (S.argValue c.a) (S.argValue c.b)
    (Equiv.Zisk.source_a_run z _ _ _ _ _ _ xs hx _ (argA_src agree S.fcall c.a hw.a hw.afree))
    (argB_src S.rv z agree hx S.fcall hf c.b hw.b _ _ _ _ _ _) hw.halt r z1 (by rw [hj1]; exact hz1)
    cv false hcf hx1 hpc1 hst1
  simp only [Bool.false_eq_true, if_false] at hex
  refine ⟨_, _, hcall, by rw [zstep_run zrom z p hzpc, hrom]; exact hex, ?_, ?_, ?_⟩
  · have hlk : ∀ q, q ≠ _root_.Zisk.Register.Xs →
        (Equiv.Zisk.linkRegs z1.regs xs c.rd cv).get? q = z1.regs.get? q := by
      intro q hq; unfold Equiv.Zisk.linkRegs; split
      · rfl
      · simp [Std.ExtDHashMap.get?_insert, Ne.symm hq]
    have hg : model.getPC S.rv = pc' := by simp [model, hpc]
    have hready1 := hr.with_mem (ZiskRiscv.writeDword m' ZiskRiscv.INPUT_ADDR f'.freeInput)
    refine ⟨?_, ⟨Equiv.Zisk.xsUpd xs c.rd cv, ?_, fun k hk1 hk2 => ?_⟩, ⟨pc' + BitVec.ofNat 64 (4 * c.len), ?_, ?_⟩,
      ⟨BitVec.addInt st 1, ?_, ?_⟩, ?_⟩
    · refine hready1.keep ⟨fun r => ?_⟩ (fun r h1 h2 _ _ _ => ?_)
      · simp only [callPost, Std.ExtDHashMap.get?_insert]
        split
        · simp
        · exact (hready1.wf.setX c.rd _).all r
      · simp only [callPost, Std.ExtDHashMap.get?_insert, show (Register.PC == r) = false from by simp [Ne.symm h2]]
        exact setX_get_other _ _ _ r h1
    · simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false, ite_false]
      exact linkRegs_xs _ xs _ _ hx1
    · simp only [callPost]
      rw [getX_setPC]
      exact regs_after0 { S.rv with mem := ZiskRiscv.writeDword m' ZiskRiscv.INPUT_ADDR f'.freeInput } xs c.rd _
        (by have := hw.rd; omega) (fun k h1 h2 => agree k h1 h2) k hk1 hk2
    · simp [callPost, hg]
    · simp [Std.ExtDHashMap.get?_insert, hj2]
    · simp [Std.ExtDHashMap.get?_insert]
    · simp [Std.ExtDHashMap.get?_insert, hlk .Zerror (by decide), hze1]
    · dsimp only
      rw [callPost_mem, normIn_writeDword_input]; exact hnorm
  · intro q hq
    have hne : ∀ q', Equiv.Zisk.fregs q' = false → (q' == q) = false := fun q' h => by
      cases e : (q' == q)
      · rfl
      · simp at e; subst e; rw [hq] at h; cases h
    simp only [Std.ExtDHashMap.get?_insert, hne .PC rfl, hne .Step rfl, hne .Zflag rfl, hne .Zc rfl,
      Bool.false_eq_true, dite_false]
    unfold Equiv.Zisk.linkRegs; split
    · exact hfr q hq
    · simp only [Std.ExtDHashMap.get?_insert, hne .Xs rfl, Bool.false_eq_true, dite_false]; exact hfr q hq
  · intro a ha hin
    dsimp only
    rw [callPost_mem, writeDword_eq, Equiv.memWrite_get_off _ _ _ _ (by simp only [ZiskRiscv.INPUT_ADDR, Equiv.IN_LO] at hin ⊢; omega)]
    exact hram1 a ha


set_option maxHeartbeats 16000000 in
/-- A ZisK call without a count: the machine's call and ZisK's one step on
    the spec's instruction keep the states related, ZisK's fcall registers
    the machine's fcall state. -/
theorem callstep1 (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState) (rs : List PMA_Region)
    (hrel : Rel S.rv z rs) (hf : Equiv.Zisk.FcallRel S.fcall z)
    (c : ZiskRiscv.Call) (hw : CallWF c) (hcn : c.count = none) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (zrom : BitVec 64 → _root_.Zisk.zisk_inst)
    (hrom : zrom pc = Transpiler.mk (Transpiler.argA c.a) (Transpiler.argB c.b) c.op (Transpiler.storeReg c.rd)
      (if Transpiler.fcallOp c.op then BitVec.ofNat 64 (4 * c.len) else c.ext) (BitVec.ofNat 64 (4 * c.len)))
    (hok : CallOk S c) :
    ∃ S' z', ZiskRiscv.call S c = .ok S' ∧ (zstep zrom).run z = .ok false z' ∧ Rel S'.rv z' rs ∧
      Equiv.Zisk.FcallRel S'.fcall z' ∧
      (∀ a, Equiv.OffRam a → ¬ (Equiv.IN_LO ≤ a ∧ a < Equiv.IN_LO + 8) → S'.rv.mem.get? a = S.rv.mem.get? a) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hcm : callMem S c = S.rv.mem := by simp [callMem, hcn, model]
  have hj1 : (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then
      (if Transpiler.fcallOp c.op then BitVec.ofNat 64 (4 * c.len) else c.ext) else 0) =
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0) := by
    cases hp : _root_.Zisk.Functions.zisk_op_is_precompiled c.op
    · rfl
    · simp [hw.pre hp]
  obtain ⟨S', z', h1, h2, h3, h4, h5⟩ := zcall_step S c hw rs hr xs agree pc' hpc z hx pc' hzpc st hst hze
    (by rw [hcm]; exact hzm) hf zrom _ _ hrom hj1 rfl hok
  exact ⟨S', z', h1, h2, h3, h4, by rw [← hcm]; exact h5⟩

set_option maxHeartbeats 16000000 in
/-- A ZisK call with a count: the machine's call, and ZisK's two steps on
    the spec's translation (the count's store to EXTRA_PARAMS_ADDR, the
    operation at ia), keep the states related. -/
theorem callstep2 (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState) (rs : List PMA_Region)
    (hrel : Rel S.rv z rs) (hf : Equiv.Zisk.FcallRel S.fcall z)
    (c : ZiskRiscv.Call) (hw : CallWF c) (rc : Nat) (hcn : c.count = some rc) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (ia : BitVec 64)
    (h1 : zrom pc = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB rc) (.ZOP_COPYB ())
      (.ST_MEM, Transpiler.EXTRA_PARAMS_ADDR, false) (ia - pc) (ia - pc))
    (h2 : zrom ia = Transpiler.mk (Transpiler.argA c.a) (Transpiler.argB c.b) c.op (Transpiler.storeReg c.rd) 0
      (pc + BitVec.ofNat 64 (4 * c.len) - ia))
    (hok : CallOk S c) :
    ∃ S' z', ZiskRiscv.call S c = .ok S' ∧ ZRun zrom 2 z z' ∧ Rel S'.rv z' rs ∧
      Equiv.Zisk.FcallRel S'.fcall z' ∧
      (∀ a, Equiv.OffRam a → ¬ (Equiv.IN_LO ≤ a ∧ a < Equiv.IN_LO + 8) → S'.rv.mem.get? a = S.rv.mem.get? a) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨lr, hce⟩ := hw.count rc hcn
  -- ZisK: the count's store
  have hz1 := Equiv.Zisk.execute_store_mem z (Transpiler.srcB rc) Transpiler.EXTRA_PARAMS_ADDR (ia - pc') xs pc' st
    hx hzpc hst _ (Equiv.Zisk.srcB_BSrc xs rc lr) (by decide) (by decide)
  rw [← h1, ← zstep_run zrom z pc' hzpc] at hz1
  generalize hz1s : ({ z with regs := _, mem := _ } : Equiv.Zisk.ZState) = z1 at hz1
  have eia : pc' + (ia - pc') = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have hx1 : z1.regs.get? _root_.Zisk.Register.Xs = some xs := by subst hz1s; simp [Std.ExtDHashMap.get?_insert, hx]
  have hpc1 : z1.regs.get? _root_.Zisk.Register.PC = some ia := by subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
  have hst1 : z1.regs.get? _root_.Zisk.Register.Step = some (BitVec.addInt st 1) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert]
  have hze1 : z1.regs.get? _root_.Zisk.Register.Zerror = some false := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, hze]
  have hf1 : Equiv.Zisk.FcallRel S.fcall z1 := by
    intro q hq; subst hz1s
    have hne : ∀ q', Equiv.Zisk.fregs q' = false → (q' == q) = false := fun q' h => by
      cases e : (q' == q)
      · rfl
      · simp at e; subst e; rw [hq] at h; cases h
    simp only [Std.ExtDHashMap.get?_insert, hne .PC rfl, hne .Step rfl, hne .Zflag rfl, hne .Zc rfl,
      Bool.false_eq_true, dite_false]
    exact hf q hq
  have hgx : model.getX S.rv rc = Equiv.Zisk.regv xs rc := by simp [model, getX_regv agree lr]
  have hcm : callMem S c = Equiv.memWrite (n := 8) S.rv.mem Transpiler.EXTRA_PARAMS_ADDR.toNat
      (Equiv.Zisk.regv xs rc) := by
    simp only [callMem, hcn, hgx, writeDword_eq]; rfl
  have hzm1 : Equiv.normIn z1.mem = Equiv.normIn (callMem S c) := by
    subst hz1s; rw [hcm]; exact Equiv.normIn_memWrite hzm _ _
  have hj1 : (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then (0 : BitVec 64) else 0) =
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0) := by rw [hce]
  have hj2 : ia + (pc' + BitVec.ofNat 64 (4 * c.len) - ia) = pc' + BitVec.ofNat 64 (4 * c.len) := by
    rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  obtain ⟨S', z', hc, hz2, hrel', hf', hmem⟩ := zcall_step S c hw rs hr xs agree pc' hpc z1 hx1 ia hpc1 _ hst1 hze1
    hzm1 hf1 zrom _ _ h2 hj1 hj2 hok
  refine ⟨S', z', hc, .step hz1 (.step hz2 (.done _)), hrel', hf', fun a ha hin => ?_⟩
  rw [hmem a ha hin, hcm, Equiv.memWrite_get_off]
  have := ha; unfold Equiv.OffRam at this
  have e : Transpiler.EXTRA_PARAMS_ADDR.toNat = 0xA0400F00 := rfl
  rw [e]; omega

end Equiv.Machine
