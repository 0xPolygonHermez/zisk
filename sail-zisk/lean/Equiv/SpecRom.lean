/-
  The ZisK program as the transpiler makes it, as a hypothesis on the
  program rather than on its run: wherever executable memory holds a word
  the spec describes, the ZisK program holds the spec's translation of it
  (`SpecRom`). With no code in the RAM (`NoExecRam`, as ZisK's memory map
  has it), a run, which changes only the RAM, keeps it; and it turns any
  run's coverage, whatever ZisK program it was stated with, into coverage
  with this one (`StepOkT.retarget`, `OneOkT.retarget`).
-/
import Equiv.ProgSpec
import Equiv.Machine

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The ZisK program holds the spec's translation of every word of m at a
    code address (`C`: the addresses the transpiler translates) the spec
    describes, the next word as the transpiler reads it. -/
def SpecRom (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (m : Mem) : Prop :=
  ∀ pc w, C pc → ZiskRiscv.readWord m pc.toNat = some w →
    (∃ ia l, Transpiler.convert pc w (ZiskRiscv.readWord m (pc + 4).toNat) ia = some l) →
    Transpiled zrom pc w (ZiskRiscv.readWord m (pc + 4).toNat)

/-- No code in the RAM: a code word, and the word after it, off it. -/
def NoCodeRam (C : BitVec 64 → Prop) : Prop :=
  ∀ pc, C pc → (∀ i, i < 4 → OffRam (pc.toNat + i)) ∧ (∀ i, i < 4 → OffRam ((pc + 4).toNat + i))

/-- The state k steps of sail-riscv's main loop reach from s, from step n. -/
noncomputable def rvIter : Nat → Nat → RVState → RVState
  | _, 0, s => s
  | n, k + 1, s => rvIter (n + 1) k (rvNext n s)

theorem RVRun.iter {n k : Nat} {s s' : RVState} (h : RVRun n k s s') : s' = rvIter n k s := by
  induction h with
  | done => rfl
  | step h1 _ ih => rw [ih, rvIter, rvNext_of h1]

/-- The run's first k steps start at code addresses. -/
def RunIn (C : BitVec 64 → Prop) (k : Nat) (s : RVState) (n : Nat) : Prop :=
  ∀ i, i < k → ∀ pc, (rvIter n i s).regs.get? Register.PC = some pc → C pc

theorem RunIn.head {C : BitVec 64 → Prop} {k : Nat} {s : RVState} {n : Nat} (h : RunIn C (k + 1) s n)
    {pc : BitVec 64} (hpc : s.regs.get? Register.PC = some pc) : C pc := h 0 (by omega) pc hpc

theorem RunIn.tail {C : BitVec 64 → Prop} {k : Nat} {s : RVState} {n : Nat} (h : RunIn C (k + 1) s n) :
    RunIn C k (rvNext n s) (n + 1) := fun i hi pc hpc => h (i + 1) (by omega) pc hpc

theorem readWord_congr {m m' : Mem} {a : Nat} (h : ∀ i, i < 4 → m'.get? (a + i) = m.get? (a + i)) :
    ZiskRiscv.readWord m' a = ZiskRiscv.readWord m a := by
  have h0 := h 0 (by omega); have h1 := h 1 (by omega); have h2 := h 2 (by omega); have h3 := h 3 (by omega)
  simp only [Nat.add_zero] at h0
  simp only [ZiskRiscv.readWord, h0, h1, h2, h3]

theorem SpecRom.keep {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} {m m' : Mem}
    (h : SpecRom zrom C m) (hr : RamOnly m m') (hx : NoCodeRam C) : SpecRom zrom C m' := by
  intro pc w he hw hc
  obtain ⟨o1, o2⟩ := hx pc he
  have e1 : ZiskRiscv.readWord m' pc.toNat = ZiskRiscv.readWord m pc.toNat :=
    readWord_congr fun i hi => hr _ (o1 i hi)
  have e2 : ZiskRiscv.readWord m' (pc + 4).toNat = ZiskRiscv.readWord m (pc + 4).toNat :=
    readWord_congr fun i hi => hr _ (o2 i hi)
  rw [e1] at hw; rw [e2] at hc ⊢
  exact h pc w he hw hc

/-- A translation the spec makes, with any ZisK program, is in this one. -/
theorem retr {zr zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} {s : RVState}
    {pc : BitVec 64} {e : BitVec 32} (hs : SpecRom zrom C s.mem) (x1 : C pc)
    (h0 : s.mem.get? pc.toNat = some (byte e 0)) (h1 : s.mem.get? (pc.toNat + 1) = some (byte e 1))
    (h2 : s.mem.get? (pc.toNat + 2) = some (byte e 2)) (h3 : s.mem.get? (pc.toNat + 3) = some (byte e 3))
    (tr : Transpiled zr pc e (nextWord s pc)) : Transpiled zrom pc e (nextWord s pc) := by
  obtain ⟨ia, l, hl, -⟩ := tr
  exact hs pc e x1 (Equiv.Machine.readWord_of h0 h1 h2 h3) ⟨ia, l, hl⟩

theorem StepOkT.retarget {zr zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {rs : List PMA_Region} {s : RVState} {pc : BitVec 64} (hs : SpecRom zrom C s.mem) (hc : C pc) :
    ∀ o, StepOkT s rs pc zr o → StepOkT s rs pc zrom o
  | .r _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .i _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .sh _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .u _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .ld _, ⟨hi, rest⟩ => ⟨{ hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩
  | .lw _, ⟨hi, rest⟩ => ⟨{ hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩
  | .sd _, ⟨hi, rest⟩ => ⟨{ hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩
  | .sw _, ⟨hi, rest⟩ => ⟨{ hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩
  | .br _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .j _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .jr _, ⟨hi, rest⟩ => ⟨{ hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩
  | .rw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .iw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .siw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .m _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .fence _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .zba _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .slliuw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .zbs _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .zbsi _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .zbb _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .zbbw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .rori _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .roriw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }

theorem OneOkT.retarget {zr zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {rs : List PMA_Region} {s : RVState} (hs : SpecRom zrom C s.mem)
    (hc : ∀ pc, s.regs.get? Register.PC = some pc → C pc) (h : OneOkT rs zr s) : OneOkT rs zrom s := by
  obtain ⟨pc, hpc, ⟨o, ho⟩ | ⟨i, hi, rest⟩⟩ := h
  · exact ⟨pc, hpc, .inl ⟨o, StepOkT.retarget hs (hc pc hpc) o ho⟩⟩
  · exact ⟨pc, hpc, .inr ⟨i, { hi with tr := retr hs (hc pc hpc) hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩⟩

/-- The fused pair: its translation, with the jalr as the next word. -/
theorem FtAt.retarget {zr zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {rs : List PMA_Region} {s : RVState} {pc : BitVec 64} {i : FInstr} (hs : SpecRom zrom C s.mem) (hc : C pc)
    (hi : FtAt s rs pc zr i) :
    FtAt s rs pc zrom i := by
  have hn : nextWord s pc = some i.jalr.encode := Equiv.Machine.readWord_of hi.g0 hi.g1 hi.g2 hi.g3
  have tr : Transpiled zr pc i.auipc.encode (nextWord s pc) := hn ▸ hi.tr
  have := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 tr
  exact { hi with tr := hn ▸ this }

open Equiv.Machine in
set_option maxHeartbeats 1000000 in
/-- `prog_machine` with the ZisK program given by `SpecRom` alone: the run's
    coverage stated with any ZisK program `zr` (what it says of the RISC-V
    side), the simulation runs with `zrom`, the program the spec makes. The
    run changes only the RAM. -/
theorem prog_rom (zr zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) (C : BitVec 64 → Prop)
    (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (hnx : NoCodeRam C) :
    ∀ (k : Nat) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel S.rv z rs → RunOkT rs zr k S.rv n → RunIn C k S.rv n → SpecRom zrom C S.rv.mem →
      ∃ s' z' k', RVRun n k S.rv s' ∧ ZRun zrom k' z z' ∧ Rel s' z' rs ∧ RamOnly S.rv.mem s'.mem ∧
        (PcOk s' → MRunR k S { S with rv := s' })
  | 0, S, z, n, hrel, _, _, _ =>
    ⟨S.rv, z, 0, .done n _, .done z, hrel, RamOnly.refl _, fun _ => .done _⟩
  | 1, S, z, n, hrel, ho, hin, hs => by
    obtain ⟨s1, z1, k1, hrun, hzrun, hrel1, hm1⟩ :=
      one_stepT S.rv z rs hrel zrom (OneOkT.retarget hs (fun _ h => hin.head h) ho) n
    obtain ⟨pc, hpc, hw⟩ := OneOkT.word ho
    exact ⟨s1, z1, k1, .step hrun (.done _ _), hzrun, hrel1, hm1,
      fun hp => .step (mstepR S rs hrel.ready pc hpc hw hx hrun hp) (.done _)⟩
  | k + 2, S, z, n, hrel, .inl ⟨ho, hrest⟩, hin, hs => by
    obtain ⟨s1, z1, k1, hrun, hzrun, hrel1, hm1⟩ :=
      one_stepT S.rv z rs hrel zrom (OneOkT.retarget hs (fun _ h => hin.head h) ho) n
    obtain ⟨pc, hpc, hw⟩ := OneOkT.word ho
    have hin1 := hin.tail
    rw [rvNext_of hrun] at hrest hin1
    obtain ⟨p1, hp1, hx1⟩ := RunOkT.exec hrest
    have hmr1 := mstepR S rs hrel.ready pc hpc hw hx hrun (PcOk.of_exec ht hp1 hx1)
    obtain ⟨s2, z2, k2, hrun2, hzrun2, hrel2, hm2, hmr2⟩ :=
      prog_rom zr zrom rs C hx ht hnx (k + 1) { S with rv := s1 } z1 (n + 1) hrel1 hrest hin1 (hs.keep hm1 hnx)
    exact ⟨s2, z2, k1 + k2, .step hrun hrun2, ZRun.append hzrun hzrun2, hrel2, hm1.trans hm2,
      fun hp => .step hmr1 (hmr2 hp)⟩
  | k + 2, S, z, n, hrel, .inr ⟨⟨pc, i, hpc, hi⟩, hrest⟩, hin, hs => by
    have hi' := FtAt.retarget hs (hin.head hpc) hi
    have hin2 := hin.tail.tail
    obtain ⟨s2, z1, hrun, hzrun, hrel1, -, hr1, hpc1, hmem1, hmem2⟩ :=
      fused_rel S.rv z rs hrel pc hpc zrom i hi'.toFAt n
    obtain ⟨t1, h1, h2⟩ : ∃ t1, (interpretSailEffects (try_step n false)).run S.rv = .ok false t1 ∧
        (interpretSailEffects (try_step (n + 1) false)).run t1 = .ok false s2 := by
      have hrun' := hrun
      cases hrun' with
      | step h1 r => cases r with
        | step h2 r2 => cases r2 with
          | done => exact ⟨_, h1, h2⟩
    have e1 : rvNext n S.rv = t1 := rvNext_of h1
    rw [e1] at hr1 hpc1 hmem1
    rw [e1, rvNext_of h2] at hrest hin2
    have hm : RamOnly S.rv.mem s2.mem := RamOnly.of_eq hmem2
    obtain ⟨s3, z3, k3, hrun3, hzrun3, hrel3, hm3, hmr3⟩ :=
      prog_rom zr zrom rs C hx ht hnx k { S with rv := s2 } z1 (n + 2) hrel1 hrest hin2 (hs.keep hm hnx)
    have hw1 : WordOk S.rv.mem rs pc := ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_u _⟩
    have hw2 : WordOk t1.mem rs (pc + 4) :=
      ⟨hi.x3, _, readWord_of (by rw [hmem1]; exact hi.g0) (by rw [hmem1]; exact hi.g1)
        (by rw [hmem1]; exact hi.g2) (by rw [hmem1]; exact hi.g3), notSys_jr _⟩
    refine ⟨s3, z3, 1 + k3, by simpa [Nat.add_comm] using RVRun.append hrun hrun3,
      ZRun.append hzrun hzrun3, hrel3, hm.trans hm3, fun hp => ?_⟩
    have hp2 : PcOk s2 := by
      cases k with
      | zero => cases hrun3; exact hp
      | succ k => obtain ⟨p, hp', hx'⟩ := RunOkT.exec hrest; exact PcOk.of_exec ht hp' hx'
    have m1 := mstepR S rs hrel.ready pc hpc hw1 hx h1 (PcOk.of_exec ht hpc1 hi.x3)
    have m2 := mstepR { S with rv := t1 } rs hr1 (pc + 4) hpc1 hw2 hx h2 hp2
    rw [show k + 2 = 0 + 1 + 1 + k by omega]
    exact MRunR.append (.step m1 (.step m2 (.done _))) (hmr3 hp)

end Equiv.Sim
