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
import Equiv.MemGrow

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

/-- The run's first k steps start at code addresses. -/
def RunIn (C : BitVec 64 → Prop) (k : Nat) (s : RVState) (n : Nat) : Prop :=
  ∀ i, i < k → ∀ pc, (rvIter n i s).regs.get? Register.PC = some pc → C pc

theorem RunIn.head {C : BitVec 64 → Prop} {k : Nat} {s : RVState} {n : Nat} (h : RunIn C (k + 1) s n)
    {pc : BitVec 64} (hpc : s.regs.get? Register.PC = some pc) : C pc := h 0 (by omega) pc hpc

theorem RunIn.tail {C : BitVec 64 → Prop} {k : Nat} {s : RVState} {n : Nat} (h : RunIn C (k + 1) s n) :
    RunIn C k (mNext n s) (n + 1) := fun i hi pc hpc => h (i + 1) (by omega) pc hpc

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
  | .pack _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .packw _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .sext _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .zexth _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .byte _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }
  | .count _, hi => { hi with tr := retr hs hc hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }

theorem OneOkT.retarget {zr zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {rs : List PMA_Region} {s : RVState} (hs : SpecRom zrom C s.mem)
    (hc : ∀ pc, s.regs.get? Register.PC = some pc → C pc) (h : OneOkT rs zr s) : OneOkT rs zrom s := by
  obtain ⟨pc, hpc, ⟨o, ho⟩ | ⟨i, hi, rest⟩ | ⟨i, hi⟩ | ⟨i, hi, rest⟩ | ⟨i, hi, rest⟩ | ⟨i, hi, rest⟩⟩ := h
  · exact ⟨pc, hpc, .inl ⟨o, StepOkT.retarget hs (hc pc hpc) o ho⟩⟩
  · exact ⟨pc, hpc, .inr (.inl ⟨i, { hi with tr := retr hs (hc pc hpc) hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩)⟩
  · exact ⟨pc, hpc, .inr (.inr (.inl ⟨i, { hi with tr := retr hs (hc pc hpc) hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }⟩))⟩
  · exact ⟨pc, hpc, .inr (.inr (.inr (.inl
      ⟨i, { hi with tr := retr hs (hc pc hpc) hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩)))⟩
  · exact ⟨pc, hpc, .inr (.inr (.inr (.inr (.inl
      ⟨i, { hi with tr := retr hs (hc pc hpc) hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩))))⟩
  · exact ⟨pc, hpc, .inr (.inr (.inr (.inr (.inr
      ⟨i, { hi with tr := retr hs (hc pc hpc) hi.h0 hi.h1 hi.h2 hi.h3 hi.tr }, rest⟩))))⟩

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
/-- The simulation over whole runs, with the ZisK program given by
    `SpecRom` alone: the run's coverage stated with any ZisK program `zr`
    (what it says of the RISC-V side), the simulation runs with `zrom`, the
    program the spec makes. From related states, k steps of the machine and
    some number of ZisK steps end related; the run changes only the RAM, and
    memory only grows. Its steps are steps of ZisK's RISC-V machine, each
    leaving the run going — given that neither the exit address nor the
    trap vector is executable, and that the last step does not land on the
    trap vector. -/
theorem prog_rom (zr zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) (C : BitVec 64 → Prop)
    (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (hnx : NoCodeRam C) :
    ∀ (k : Nat) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel S.rv z rs → RunOkT rs zr k S.rv n → RunIn C k S.rv n → SpecRom zrom C S.rv.mem →
      ∃ z' k', ZRun zrom k' z z' ∧ Rel (rvIter n k S.rv) z' rs ∧ RamOnly S.rv.mem (rvIter n k S.rv).mem ∧
        Grows S.rv.mem (rvIter n k S.rv).mem ∧
        (PcOk (rvIter n k S.rv) → MRunR C k S { S with rv := rvIter n k S.rv })
  | 0, S, z, n, hrel, _, _, _ =>
    ⟨z, 0, .done z, hrel, RamOnly.refl _, Grows.refl _, fun _ => .done _⟩
  | 1, S, z, n, hrel, ho, hin, hs => by
    obtain ⟨s1, z1, k1, hm, hzrun, hrel1, hm1⟩ :=
      one_stepT S.rv z rs hrel zrom (OneOkT.retarget hs (fun _ h => hin.head h) ho) n
    obtain ⟨pc, hpc, hex⟩ := OneOkT.exec ho
    have e : rvIter n 1 S.rv = s1 := by simp only [rvIter, hm.next]
    rw [e]
    exact ⟨z1, k1, hzrun, hrel1, hm1, hm.grows,
      fun hp => hm.toRun hrel.ready hpc hex (hin.head hpc) hx hp (.done _)⟩
  | k + 2, S, z, n, hrel, .inl ⟨ho, hrest⟩, hin, hs => by
    obtain ⟨s1, z1, k1, hm, hzrun, hrel1, hm1⟩ :=
      one_stepT S.rv z rs hrel zrom (OneOkT.retarget hs (fun _ h => hin.head h) ho) n
    obtain ⟨pc, hpc, hex⟩ := OneOkT.exec ho
    have hin1 := hin.tail
    rw [hm.next] at hrest hin1
    obtain ⟨p1, hp1, hx1⟩ := RunOkT.exec hrest
    obtain ⟨z2, k2, hzrun2, hrel2, hm2, hg2, hmr2⟩ :=
      prog_rom zr zrom rs C hx ht hnx (k + 1) { S with rv := s1 } z1 (n + 1) hrel1 hrest hin1 (hs.keep hm1 hnx)
    have e : rvIter n (k + 2) S.rv = rvIter (n + 1) (k + 1) s1 := by rw [rvIter, hm.next]
    rw [e]
    exact ⟨z2, k1 + k2, ZRun.append hzrun hzrun2, hrel2, hm1.trans hm2, hm.grows.trans hg2,
      fun hp => hm.toRun hrel.ready hpc hex (hin.head hpc) hx (PcOk.of_exec ht hp1 hx1) (hmr2 hp)⟩
  | k + 2, S, z, n, hrel, .inr ⟨⟨pc, i, hpc, hi⟩, hrest⟩, hin, hs => by
    have hi' := FtAt.retarget hs (hin.head hpc) hi
    obtain ⟨s2, z1, hrun, hzrun, hrel1, -, hr1, hpc1, hmem1, hmem2⟩ :=
      fused_rel S.rv z rs hrel pc hpc zrom i hi'.toFAt n
    obtain ⟨t1, h1, -, m1, m2, e1, e2⟩ := fused_mnext hpc hi hrun hpc1 hmem1
    have hc1 : C (pc + 4) := by
      have := hin.tail.head (pc := pc + 4); rw [e1, ← rvNext_of h1] at this; exact this hpc1
    have hin2 := hin.tail.tail
    rw [rvNext_of h1] at hr1 hpc1
    rw [e1, e2] at hrest hin2
    have hm : RamOnly S.rv.mem s2.mem := RamOnly.of_eq hmem2
    obtain ⟨z3, k3, hzrun3, hrel3, hm3, hg3, hmr3⟩ :=
      prog_rom zr zrom rs C hx ht hnx k { S with rv := s2 } z1 (n + 2) hrel1 hrest hin2 (hs.keep hm hnx)
    have e : rvIter n (k + 2) S.rv = rvIter (n + 2) k s2 := by rw [rvIter, rvIter, e1, e2]
    rw [e]
    refine ⟨z3, 1 + k3, ZRun.append hzrun hzrun3, hrel3, hm.trans hm3, (m1.grows.trans m2.grows).trans hg3,
      fun hp => ?_⟩
    have hp2 : PcOk s2 := by
      cases k with
      | zero => exact hp
      | succ k => obtain ⟨p, hp', hx'⟩ := RunOkT.exec hrest; exact PcOk.of_exec ht hp' hx'
    exact m1.toRun hrel.ready hpc hi.x1 (hin.head hpc) hx (PcOk.of_exec ht hpc1 hi.x3)
      (m2.toRun (S := { S with rv := t1 }) hr1 hpc1 hi.x3 hc1 hx hp2 (hmr3 hp))

end Equiv.Sim
