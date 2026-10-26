/-
  Runs of the machine on its whole state, the fcall state with the RISC-V
  one (`sNext`, `sIter`): a ZisK call is the machine's (`ZiskRiscv.call`),
  every other step `mNext` on the RISC-V state. The run's coverage
  (`RunOkM`) takes each step as `OneOkT` does, or as a ZisK call
  (`CallAt`); the simulation over it (`prog_romM`) relates ZisK's fcall
  registers to the machine's fcall state too, and lets a call's memory
  change off the RAM in the input's first 8 bytes, the free input's mirror.
-/
import Equiv.CallRun
import Equiv.SpecRom

namespace Equiv.Sim

open Equiv.Riscv Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The state the machine's step `n` leads to from S: on a ZisK call, the
    call's (S, should it fail); otherwise `mNext` on the RISC-V state, the
    fcall state as it is. -/
noncomputable def sNext (n : Nat) (S : ZiskRiscv.State model) : ZiskRiscv.State model :=
  match ZiskRiscv.decode (ZiskRiscv.readWord S.rv.mem) ((S.rv.regs.get? Register.PC).getD 0).toNat with
  | .call c =>
    match ZiskRiscv.call S c with
    | .ok S' => S'
    | .error _ => S
  | _ => { S with rv := mNext n S.rv }

/-- The state k steps of the machine reach from S, from step n. -/
noncomputable def sIter : Nat → Nat → ZiskRiscv.State model → ZiskRiscv.State model
  | _, 0, S => S
  | n, k + 1, S => sIter (n + 1) k (sNext n S)

theorem MStep.snext_eq {rs : List PMA_Region} {n : Nat} {S : ZiskRiscv.State model} {s' : RVState}
    (h : MStep rs n S.rv s') : sNext n S = { S with rv := s' } := by
  have hn := h.next
  have hd : ∀ c, ZiskRiscv.decode (ZiskRiscv.readWord S.rv.mem) ((S.rv.regs.get? Register.PC).getD 0).toNat ≠
      .call c := by
    intro c
    cases h with
    | plain pc e hpc hw hop _ => rw [hpc, Option.getD_some, hop _ _ hw]; exact fun h => nomatch h
    | subst pc w w' zr s1 hpc hw hd _ _ _ => rw [hpc, Option.getD_some, hd _ hw]; exact fun h => nomatch h
  unfold sNext
  split
  · next c hc => exact absurd hc (hd c)
  · rw [hn]

theorem _root_.Equiv.Machine.CStep.snext_eq {C : BitVec 64 → Prop} {S S' : ZiskRiscv.State model}
    (h : CStep C S S') (n : Nat) : sNext n S = S' := by
  obtain ⟨-, -, c, hc, -, -, hd⟩ := h
  have hd' : ZiskRiscv.decode (ZiskRiscv.readWord S.rv.mem) ((S.rv.regs.get? Register.PC).getD 0).toNat =
      .call c := hd _ rfl (fun _ => rfl) fun _ => rfl
  simp only [sNext, hd', hc]

/-! ## Memory off the RAM -/

/-- m' is m but in the RAM and the input's first 8 bytes. -/
def RamOnlyX (m m' : Mem) : Prop :=
  ∀ a, OffRam a → ¬ (Equiv.IN_LO ≤ a ∧ a < Equiv.IN_LO + 8) → m'.get? a = m.get? a

theorem RamOnlyX.refl (m : Mem) : RamOnlyX m m := fun _ _ _ => rfl

theorem RamOnlyX.trans {m1 m2 m3 : Mem} (h1 : RamOnlyX m1 m2) (h2 : RamOnlyX m2 m3) : RamOnlyX m1 m3 :=
  fun a ha hi => (h2 a ha hi).trans (h1 a ha hi)

theorem RamOnlyX.of {m m' : Mem} (h : RamOnly m m') : RamOnlyX m m' := fun a ha _ => h a ha

/-- No code in the input's first 8 bytes: a code word, and the two words
    after it, off them. -/
def NoCodeIn (C : BitVec 64 → Prop) : Prop :=
  ∀ pc, C pc → (∀ i, i < 4 → ¬ (Equiv.IN_LO ≤ pc.toNat + i ∧ pc.toNat + i < Equiv.IN_LO + 8)) ∧
    (∀ i, i < 4 → ¬ (Equiv.IN_LO ≤ (pc + 4).toNat + i ∧ (pc + 4).toNat + i < Equiv.IN_LO + 8)) ∧
    (∀ i, i < 4 → ¬ (Equiv.IN_LO ≤ (pc + 8).toNat + i ∧ (pc + 8).toNat + i < Equiv.IN_LO + 8))

theorem SpecRom.keepX {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} {m m' : Mem}
    (h : SpecRom zrom C m) (hr : RamOnlyX m m') (hx : NoCodeRam C) (hi : NoCodeIn C) : SpecRom zrom C m' := by
  intro pc w he hw hc
  obtain ⟨o1, o2, o3⟩ := hx pc he
  obtain ⟨i1, i2, i3⟩ := hi pc he
  have e1 : ZiskRiscv.readWord m' pc.toNat = ZiskRiscv.readWord m pc.toNat :=
    readWord_congr fun i hi => hr _ (o1 i hi) (i1 i hi)
  have e2 : ZiskRiscv.readWord m' (pc + 4).toNat = ZiskRiscv.readWord m (pc + 4).toNat :=
    readWord_congr fun i hi => hr _ (o2 i hi) (i2 i hi)
  have e3 : ZiskRiscv.readWord m' (pc + 8).toNat = ZiskRiscv.readWord m (pc + 8).toNat :=
    readWord_congr fun i hi => hr _ (o3 i hi) (i3 i hi)
  rw [e1] at hw; rw [e2, e3] at hc ⊢
  exact h pc w he hw hc

/-! ## The run's coverage -/

/-- A ZisK call at pc, an executable address: the word there, and the two
    after it, what the spec decodes as a call, its translation in the ZisK
    program; the call's condition (`CallOk`); pc + 4 a code address too for
    a call of more than one word, pc + 8 for one of three. -/
def CallAt (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop)
    (S : ZiskRiscv.State model) : Prop :=
  ∃ pc w c, S.rv.regs.get? Register.PC = some pc ∧ ExecAt rs pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
    Transpiled zrom pc w (nextWord S.rv pc) (nextWord2 S.rv pc) ∧
    (pc + 4).toNat = pc.toNat + 4 ∧ (pc + 8).toNat = pc.toNat + 8 ∧
    ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc)) pc.toNat = .call c ∧
    (c.len ≠ 1 → C (pc + 4)) ∧ (c.len = 3 → C (pc + 8)) ∧ CallOk S c

/-- What one step of the machine needs: as `OneOkT`, or a ZisK call. -/
def OneOkM (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop)
    (S : ZiskRiscv.State model) : Prop :=
  OneOkT rs zrom S.rv ∨ CallAt rs zrom C S

/-- For the next k steps of the machine, from the state it reaches: each one
    covered as `OneOkM` says, or, two at a time, a fused auipc-jalr pair. -/
def RunOkM (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) :
    Nat → ZiskRiscv.State model → Nat → Prop
  | 0, _, _ => True
  | 1, S, _ => OneOkM rs zrom C S
  | k + 2, S, n =>
    (OneOkM rs zrom C S ∧ RunOkM rs zrom C (k + 1) (sNext n S) (n + 1)) ∨
      ((∃ pc i, S.rv.regs.get? Register.PC = some pc ∧ FtAt S.rv rs pc zrom i) ∧
        RunOkM rs zrom C k (sNext (n + 1) (sNext n S)) (n + 2))

/-- The run's first k steps start at code addresses. -/
def RunInM (C : BitVec 64 → Prop) (k : Nat) (S : ZiskRiscv.State model) (n : Nat) : Prop :=
  ∀ i, i < k → ∀ pc, (sIter n i S).rv.regs.get? Register.PC = some pc → C pc

theorem RunInM.head {C : BitVec 64 → Prop} {k : Nat} {S : ZiskRiscv.State model} {n : Nat}
    (h : RunInM C (k + 1) S n) {pc : BitVec 64} (hpc : S.rv.regs.get? Register.PC = some pc) : C pc :=
  h 0 (by omega) pc hpc

theorem RunInM.tail {C : BitVec 64 → Prop} {k : Nat} {S : ZiskRiscv.State model} {n : Nat}
    (h : RunInM C (k + 1) S n) : RunInM C k (sNext n S) (n + 1) := fun i hi pc hpc => h (i + 1) (by omega) pc hpc

theorem OneOkM.exec {rs : List PMA_Region} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} (h : OneOkM rs zrom C S) :
    ∃ pc, S.rv.regs.get? Register.PC = some pc ∧ ExecAt rs pc := by
  rcases h with h | ⟨pc, w, c, hpc, hx, -⟩
  · exact OneOkT.exec h
  · exact ⟨pc, hpc, hx⟩

theorem RunOkM.exec {rs : List PMA_Region} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {n : Nat} :
    ∀ {k}, RunOkM rs zrom C (k + 1) S n → ∃ pc, S.rv.regs.get? Register.PC = some pc ∧ ExecAt rs pc
  | 0, h => OneOkM.exec h
  | _ + 1, .inl ⟨h, _⟩ => OneOkM.exec h
  | _ + 1, .inr ⟨⟨pc, _, hpc, hi⟩, _⟩ => ⟨pc, hpc, hi.x1⟩

theorem OneOkM.retarget {zr zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {rs : List PMA_Region} {S : ZiskRiscv.State model} (hs : SpecRom zrom C S.rv.mem)
    (hc : ∀ pc, S.rv.regs.get? Register.PC = some pc → C pc) (h : OneOkM rs zr C S) : OneOkM rs zrom C S := by
  rcases h with h | ⟨pc, w, c, hpc, hx, hrw, ⟨ia, l, hl, -⟩, rest⟩
  · exact .inl (OneOkT.retarget hs hc h)
  · exact .inr ⟨pc, w, c, hpc, hx, hrw, hs pc w (hc pc hpc) hrw ⟨ia, l, hl⟩, rest⟩

/-! ## The simulation -/

/-- One step of the machine, covered: ZisK's steps from related states to
    related states, the fcall relation kept; the step recorded. -/
theorem one_stepM (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState)
    (rs : List PMA_Region) (hrel : Rel S.rv z rs) (hf : Equiv.Zisk.FcallRel S.fcall z)
    (hx : ¬ ExecAt rs EXIT_ADDR) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : OneOkM rs zrom C S)
    (hC : ∀ pc, S.rv.regs.get? Register.PC = some pc → C pc) (n : Nat) :
    ∃ z' k', ZRun zrom k' z z' ∧ Rel (sNext n S).rv z' rs ∧ Equiv.Zisk.FcallRel (sNext n S).fcall z' ∧
      RamOnlyX S.rv.mem (sNext n S).rv.mem ∧ Grows S.rv.mem (sNext n S).rv.mem ∧
      ∀ m S'', PcOk (sNext n S).rv → MRunR C m (sNext n S) S'' → MRunR C (m + 1) S S'' := by
  rcases h with ho | ⟨pc, w, c, hpc, hex, hrw, htr, h4, h8, hd, hC4, hC8, hok⟩
  · obtain ⟨s', z', k', hm, hzr, hrel', hro, hk⟩ := one_stepT S.rv z rs hrel zrom ho n
    obtain ⟨pc, hpc, hex⟩ := OneOkT.exec ho
    rw [hm.snext_eq]
    exact ⟨z', k', hzr, hrel', fun r hr => (hk r hr).trans (hf r hr), (RamOnlyX.of hro), hm.grows,
      fun m S'' hp hrest => hm.toRun hrel.ready hpc hex (hC pc hpc) hx hp hrest⟩
  · obtain ⟨S', z', k', hcs, hzr, hrel', hf', hmem, hg⟩ :=
      callstep_m C S z rs hrel hf hx pc hpc zrom w hex hrw htr h4 h8 c hd (hC pc hpc) hC4 hC8 hok
    rw [hcs.snext_eq n]
    exact ⟨z', k', hzr, hrel', hf', hmem, hg, fun m S'' _ hrest => .call hcs hrest⟩

set_option maxHeartbeats 1000000 in
/-- The simulation over whole runs of the machine, ZisK calls among their
    steps, with the ZisK program given by `SpecRom` alone (as `prog_rom`):
    from related states, ZisK's fcall registers the machine's fcall state,
    k steps of the machine and some number of ZisK steps end so related;
    the run changes only the RAM and the input's first 8 bytes, and memory
    only grows. Its steps are the machine's, each leaving the run going —
    given that neither the exit address nor the trap vector is executable,
    and that the last step does not land on the trap vector. -/
theorem prog_romM (zr zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) (C : BitVec 64 → Prop)
    (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (hnx : NoCodeRam C) (hni : NoCodeIn C) :
    ∀ (k : Nat) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel S.rv z rs → Equiv.Zisk.FcallRel S.fcall z → RunOkM rs zr C k S n → RunInM C k S n →
      SpecRom zrom C S.rv.mem →
      ∃ z' k', ZRun zrom k' z z' ∧ Rel (sIter n k S).rv z' rs ∧ Equiv.Zisk.FcallRel (sIter n k S).fcall z' ∧
        RamOnlyX S.rv.mem (sIter n k S).rv.mem ∧ Grows S.rv.mem (sIter n k S).rv.mem ∧
        (PcOk (sIter n k S).rv → MRunR C k S (sIter n k S))
  | 0, S, z, n, hrel, hf, _, _, _ =>
    ⟨z, 0, .done z, hrel, hf, RamOnlyX.refl _, Grows.refl _, fun _ => .done _⟩
  | 1, S, z, n, hrel, hf, ho, hin, hs => by
    obtain ⟨z1, k1, hzr, hrel1, hf1, hm1, hg1, hmr⟩ := one_stepM C S z rs hrel hf hx zrom
      (OneOkM.retarget hs (fun _ h => hin.head h) ho) (fun _ h => hin.head h) n
    exact ⟨z1, k1, hzr, hrel1, hf1, hm1, hg1, fun hp => hmr 0 _ hp (.done _)⟩
  | k + 2, S, z, n, hrel, hf, .inl ⟨ho, hrest⟩, hin, hs => by
    obtain ⟨z1, k1, hzr, hrel1, hf1, hm1, hg1, hmr⟩ := one_stepM C S z rs hrel hf hx zrom
      (OneOkM.retarget hs (fun _ h => hin.head h) ho) (fun _ h => hin.head h) n
    obtain ⟨p1, hp1, hx1⟩ := RunOkM.exec hrest
    obtain ⟨z2, k2, hzr2, hrel2, hf2, hm2, hg2, hmr2⟩ :=
      prog_romM zr zrom rs C hx ht hnx hni (k + 1) (sNext n S) z1 (n + 1) hrel1 hf1 hrest hin.tail
        (hs.keepX hm1 hnx hni)
    exact ⟨z2, k1 + k2, ZRun.append hzr hzr2, hrel2, hf2, hm1.trans hm2, hg1.trans hg2,
      fun hp => hmr _ _ (PcOk.of_exec ht hp1 hx1) (hmr2 hp)⟩
  | k + 2, S, z, n, hrel, hf, .inr ⟨⟨pc, i, hpc, hi⟩, hrest⟩, hin, hs => by
    have hi' := FtAt.retarget hs (hin.head hpc) hi
    obtain ⟨s2, z1, hrun, hzrun, hrel1, -, hr1, hpc1, hmem1, hmem2, hk⟩ :=
      fused_rel S.rv z rs hrel pc hpc zrom i hi'.toFAt n
    obtain ⟨t1, h1, -, m1, m2, -, -⟩ := fused_mnext hpc hi hrun hpc1 hmem1
    have q1 : sNext n S = { S with rv := t1 } := m1.snext_eq
    have q2 : sNext (n + 1) { S with rv := t1 } = { S with rv := s2 } := m2.snext_eq (S := { S with rv := t1 })
    rw [rvNext_of h1] at hr1 hpc1
    have hc1 : C (pc + 4) := by
      have := hin.tail.head (pc := pc + 4); rw [q1] at this; exact this hpc1
    have hin2 := hin.tail.tail
    rw [q1, q2] at hrest hin2
    have hm : RamOnly S.rv.mem s2.mem := RamOnly.of_eq hmem2
    obtain ⟨z3, k3, hzrun3, hrel3, hf3, hm3, hg3, hmr3⟩ :=
      prog_romM zr zrom rs C hx ht hnx hni k { S with rv := s2 } z1 (n + 2) hrel1
        (fun r hr => (hk r hr).trans (hf r hr)) hrest hin2 (hs.keepX (RamOnlyX.of hm) hnx hni)
    have e : sIter n (k + 2) S = sIter (n + 2) k { S with rv := s2 } := by
      simp only [sIter, q1, q2]
    rw [e]
    refine ⟨z3, 1 + k3, ZRun.append hzrun hzrun3, hrel3, hf3, (RamOnlyX.of hm).trans hm3,
      (m1.grows.trans m2.grows).trans hg3, fun hp => ?_⟩
    have hp2 : PcOk s2 := by
      cases k with
      | zero => exact hp
      | succ k => obtain ⟨p, hp', hx'⟩ := RunOkM.exec hrest; exact PcOk.of_exec ht hp' hx'
    exact m1.toRun hrel.ready hpc hi.x1 (hin.head hpc) hx (PcOk.of_exec ht hpc1 hi.x3)
      (m2.toRun (S := { S with rv := t1 }) hr1 hpc1 hi.x3 hc1 hx hp2 (hmr3 hp))

end Equiv.Sim
