/-
  The simulation over whole runs, with the ZisK program the transpiler's
  spec makes of the RISC-V code: at each step, the ZisK program holds the
  spec's translation of the word at pc (`Transpiled`), whatever its form,
  x0 forms included, and nothing else is assumed of it.

  As prog_sim3 (Equiv/FusedSim.lean), with the steps of Equiv/X0Sim.lean for
  the instructions they generalize, and jal, jalr (both forms) and the fused
  auipc-jalr pair with their ZisK instructions from the spec
  (Equiv/FromSpec.lean).
-/
import Equiv.X0Sim
import Equiv.WSim
import Equiv.MSim
import Equiv.FenceSim
import Equiv.ZbaSim
import Equiv.ZbsSim
import Equiv.ZbbSim
import Equiv.ZbbRotSim
import Equiv.ZbkbSim
import Equiv.ZbbByteSim
import Equiv.ZbbCountSim
import Equiv.UwSim
import Equiv.AmoSim
import Equiv.LrScSim
import ZiskRiscv.Machine
import Equiv.RamOnly

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## jal, jalr and the fused pair, their ZisK instructions from the spec -/

structure JtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tal : (pc + i.imm.signExtend 64)[0] = false ∧ (pc + i.imm.signExtend 64)[1] = false
  tr : Transpiled zrom pc i.encode next

theorem JtAt.toJAt {s rs pc zrom i next} (h : JtAt s rs pc zrom i next) : JAt s rs pc zrom i :=
  ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, h.tal, i.rom_of_spec h.tr⟩

/-- jalr at pc, any rs1 (x0 reads 0), either offset. -/
structure JrtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JrInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem JrtAt.toJrAt {s rs pc zrom i next} (h : JrtAt s rs pc zrom i next) (he : i.imm[0] = false) :
    JrAt s rs pc zrom i :=
  ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, he, i.rom_of_spec he h.tr⟩

theorem JrtAt.toJoAt {s rs pc zrom i next} (h : JrtAt s rs pc zrom i next) (ho : i.imm[0] = true) :
    ∃ ia, JoAt s rs pc zrom i ia := by
  obtain ⟨ia, r1, r2⟩ := i.rom_of_spec_odd ho h.tr
  exact ⟨ia, ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, r1, r2⟩⟩

/-- The fused pair at pc, its ZisK instruction from the spec. -/
structure FtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : FInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  x3 : ExecAt rs (pc + 4)
  x4 : ExecAt rs (pc + 4 + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.auipc.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.auipc.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.auipc.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.auipc.encode 3)
  g0 : s.mem.get? (pc + 4).toNat = some (byte i.jalr.encode 0)
  g1 : s.mem.get? ((pc + 4).toNat + 1) = some (byte i.jalr.encode 1)
  g2 : s.mem.get? ((pc + 4).toNat + 2) = some (byte i.jalr.encode 2)
  g3 : s.mem.get? ((pc + 4).toNat + 3) = some (byte i.jalr.encode 3)
  rd : i.rd ≠ 0
  link : i.rd2 = i.rd ∨ i.rd2 = 0
  tal : (i.T pc)[0] = false ∧ (i.T pc)[1] = false
  tr : Transpiled zrom pc i.auipc.encode (some i.jalr.encode)

theorem FtAt.toFAt {s rs pc zrom i} (h : FtAt s rs pc zrom i) : FAt s rs pc zrom i :=
  ⟨h.al, h.x1, h.x2, h.x3, h.x4, h.h0, h.h1, h.h2, h.h3, h.g0, h.g1, h.g2, h.g3, h.rd, h.link, h.tal,
    i.rom_of_spec h.rd h.link h.tr⟩

/-! ## One step -/

/-- An instruction of the runs: every one proved, any registers (RV64I's
    word instructions and the M extension among them). -/
inductive InstrT where
  | r (i : RInstr)
  | i (i : IInstr)
  | sh (i : SInstr)
  | u (i : UInstr)
  | ld (i : LdInstr)
  | lw (i : LwInstr)
  | sd (i : SdInstr)
  | sw (i : SwInstr)
  | br (i : BInstr)
  | j (i : JInstr)
  | jr (i : JrInstr)
  | rw (i : RwInstr)
  | iw (i : IwInstr)
  | siw (i : SiwInstr)
  | m (i : MInstr)
  | fence (i : FenceInstr)
  | zba (i : ZbaInstr)
  | slliuw (i : SlliuwInstr)
  | zbs (i : ZbsInstr)
  | zbsi (i : ZbsiInstr)
  | zbb (i : ZbbInstr)
  | zbbw (i : ZbbWInstr)
  | rori (i : RoriInstr)
  | roriw (i : RoriwInstr)
  | pack (i : PackInstr)
  | packw (i : PackwInstr)
  | sext (i : SextInstr)
  | zexth (i : ZexthInstr)
  | byte (i : ByteInstr)
  | count (i : CountInstr)

/-- The word after pc, as the transpiler reads it to translate the one at
    pc (riscv2zisk_context.rs: the next instruction, for the auipc-jalr
    pair). -/
noncomputable def nextWord (s : RVState) (pc : BitVec 64) : Option (BitVec 32) :=
  ZiskRiscv.readWord s.mem (pc + 4).toNat

/-- What instruction `o` needs to run at pc in state s, in one step on each
    side: to be there, in memory and, as the spec translates it, in the ZisK
    program; and, for a memory access, its address good, for a jump, its
    target 4-aligned. -/
def StepOkT (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : InstrT → Prop
  | .r i => RtAt s rs pc zrom i (nextWord s pc)
  | .i i => ItAt s rs pc zrom i (nextWord s pc)
  | .sh i => StAt s rs pc zrom i (nextWord s pc)
  | .u i => UtAt s rs pc zrom i (nextWord s pc)
  | .ld i => LdtAt s rs pc zrom i (nextWord s pc) ∧
      ∃ v1 v, getX s i.rs1.toNat = some v1 ∧ LdOk s rs (v1 + i.imm.signExtend 64) v
  | .lw i => LwtAt s rs pc zrom i (nextWord s pc) ∧
      ∃ v1 v, getX s i.rs1.toNat = some v1 ∧ LwOk s rs (v1 + i.imm.signExtend 64) i.k.w v
  | .sd i => SdtAt s rs pc zrom i (nextWord s pc) ∧
      ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧ getX s i.rs2.toNat = some v2 ∧
        SdOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64)
  | .sw i => SwtAt s rs pc zrom i (nextWord s pc) ∧
      ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧ getX s i.rs2.toNat = some v2 ∧
        SwOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w
  | .br i => BtAt s rs pc zrom i (nextWord s pc)
  | .j i => JtAt s rs pc zrom i (nextWord s pc)
  | .jr i => JrtAt s rs pc zrom i (nextWord s pc) ∧ i.imm[0] = false ∧
      ∃ v1, getX s i.rs1.toNat = some v1 ∧ (jalrTarget v1 i.imm)[0] = false ∧ (jalrTarget v1 i.imm)[1] = false
  | .rw i => RwAt s rs pc zrom i (nextWord s pc)
  | .iw i => IwAt s rs pc zrom i (nextWord s pc)
  | .siw i => SiwAt s rs pc zrom i (nextWord s pc)
  | .m i => MAt s rs pc zrom i (nextWord s pc)
  | .fence i => FenceAt s rs pc zrom i (nextWord s pc)
  | .zba i => ZbaAt s rs pc zrom i (nextWord s pc)
  | .slliuw i => SlliuwAt s rs pc zrom i (nextWord s pc)
  | .zbs i => ZbsAt s rs pc zrom i (nextWord s pc)
  | .zbsi i => ZbsiAt s rs pc zrom i (nextWord s pc)
  | .zbb i => ZbbAt s rs pc zrom i (nextWord s pc)
  | .zbbw i => ZbbWAt s rs pc zrom i (nextWord s pc)
  | .rori i => RoriAt s rs pc zrom i (nextWord s pc)
  | .roriw i => RoriwAt s rs pc zrom i (nextWord s pc)
  | .pack i => WordsAt s rs pc zrom i.encode (nextWord s pc)
  | .packw i => WordsAt s rs pc zrom i.encode (nextWord s pc)
  | .sext i => WordsAt s rs pc zrom i.encode (nextWord s pc)
  | .zexth i => WordsAt s rs pc zrom i.encode (nextWord s pc)
  | .byte i => WordsAt s rs pc zrom i.encode (nextWord s pc)
  | .count i => WordsAt s rs pc zrom i.encode (nextWord s pc)

end Equiv.Sim

namespace Equiv.Machine

open Equiv.Riscv Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## The words of the instructions -/

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


/-! ## The instructions of the simulation are plain RISC-V -/

theorem notSys_r (i : RInstr) : NotMach i.encode := by
  unfold NotMach; simp only [RInstr.encode]; bv_decide
theorem notSys_i (i : IInstr) : NotMach i.encode := by
  unfold NotMach; simp only [IInstr.encode]; bv_decide
theorem notSys_sh (i : SInstr) : NotMach i.encode := by
  unfold NotMach; simp only [SInstr.encode]; bv_decide
theorem notSys_u (i : UInstr) : NotMach i.encode := by
  unfold NotMach
  cases h : i.op <;> simp only [UInstr.encode, h, uopEnc] <;> bv_decide
theorem notSys_ld (i : LdInstr) : NotMach i.encode := by
  unfold NotMach; simp only [LdInstr.encode]; bv_decide
theorem notSys_lw (i : LwInstr) : NotMach i.encode := by
  unfold NotMach; simp only [LwInstr.encode]; bv_decide
theorem notSys_sd (i : SdInstr) : NotMach i.encode := by
  unfold NotMach; simp only [SdInstr.encode]; bv_decide
theorem notSys_sw (i : SwInstr) : NotMach i.encode := by
  unfold NotMach; simp only [SwInstr.encode]; bv_decide
theorem notSys_br (i : BInstr) : NotMach i.encode := by
  unfold NotMach; simp only [BInstr.encode]; bv_decide
theorem notSys_j (i : JInstr) : NotMach i.encode := by
  unfold NotMach; simp only [JInstr.encode]; bv_decide
theorem notSys_jr (i : JrInstr) : NotMach i.encode := by
  unfold NotMach; simp only [JrInstr.encode]; bv_decide
theorem notSys_rw (i : RwInstr) : NotMach i.encode := by
  unfold NotMach; simp only [RwInstr.encode]; bv_decide
theorem notSys_iw (i : IwInstr) : NotMach i.encode := by
  unfold NotMach; simp only [IwInstr.encode]; bv_decide
theorem notSys_siw (i : SiwInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [SiwInstr.encode]; bv_decide
theorem notSys_fence (i : FenceInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [FenceInstr.encode]; bv_decide
theorem notSys_zba (i : ZbaInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ZbaInstr.encode]; bv_decide
theorem notSys_slliuw (i : SlliuwInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [SlliuwInstr.encode]; bv_decide
theorem notSys_zbs (i : ZbsInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ZbsInstr.encode]; bv_decide
theorem notSys_zbsi (i : ZbsiInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ZbsiInstr.encode]; bv_decide
theorem notSys_zbb (i : ZbbInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ZbbInstr.encode]; bv_decide
theorem notSys_zbbw (i : ZbbWInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ZbbWInstr.encode]; bv_decide
theorem notSys_rori (i : RoriInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [RoriInstr.encode]; bv_decide
theorem notSys_roriw (i : RoriwInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [RoriwInstr.encode]; bv_decide
theorem notSys_pack (i : PackInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [PackInstr.encode]; bv_decide
theorem notSys_packw (i : PackwInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [PackwInstr.encode]; bv_decide
theorem notSys_sext (i : SextInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [SextInstr.encode]; bv_decide
theorem notSys_zexth (i : ZexthInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ZexthInstr.encode]; bv_decide
theorem notSys_byte (i : ByteInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [ByteInstr.encode]; bv_decide
theorem notSys_amo (i : AmoInstr) : NotMach i.encode := by
  unfold NotMach; simp only [AmoInstr.encode]
  generalize hf : i.f3 = f
  cases i.op <;> simp only [Transpiler.AmoOp.f5] <;> bv_decide
theorem notSys_uw (i : UwInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [UwInstr.encode]; bv_decide
theorem notSys_count (i : CountInstr) : NotMach i.encode := by
  unfold NotMach
  simp only [CountInstr.encode]; cases i.op <;> simp only [Transpiler.CountOp.opc] <;> bv_decide
theorem notSys_m (i : MInstr) : NotMach i.encode := by
  unfold NotMach
  cases h : i.op <;> simp only [MInstr.encode, h, Transpiler.MOp.opc] <;> bv_decide

/-- What the machine needs of the instruction at pc to leave it to
    sail-riscv: executable there, a word it leaves to the model. -/
def WordOk (m : Equiv.Mem) (rs : List PMA_Region) (pc : BitVec 64) : Prop :=
  ExecAt rs pc ∧ ∃ e : BitVec 32, ZiskRiscv.readWord m pc.toNat = some e ∧ NotMach e

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
  | .fence i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_fence i⟩
  | .zba i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_zba i⟩
  | .slliuw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_slliuw i⟩
  | .zbs i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_zbs i⟩
  | .zbsi i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_zbsi i⟩
  | .zbb i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_zbb i⟩
  | .zbbw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_zbbw i⟩
  | .rori i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_rori i⟩
  | .roriw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_roriw i⟩
  | .pack i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_pack i⟩
  | .packw i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_packw i⟩
  | .sext i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_sext i⟩
  | .zexth i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_zexth i⟩
  | .byte i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_byte i⟩
  | .count i, hi => ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, notSys_count i⟩

end Equiv.Machine

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Machine

theorem getX_some {s : RVState} {z : Equiv.Zisk.ZState} {rs : List PMA_Region} (hrel : Rel s z rs)
    {r : Nat} (hr : r < 32) : ∃ v, getX s r = some v := by
  obtain ⟨-, ⟨xs, -, agree⟩, -⟩ := hrel
  exact ⟨_, getX_regv agree hr⟩

theorem step_relT (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (o : InstrT) (ho : StepOkT s rs pc zrom o) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem := by
  match o, ho with
  | .r i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := rtstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .i i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := itstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .sh i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := ststep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .u i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := utstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .ld i, ⟨hi, v1, v, hv1, hok⟩ =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := ldtstep_rel s z rs hrel pc hpc zrom i _ hi v1 hv1 v hok n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .lw i, ⟨hi, v1, v, hv1, hok⟩ =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := lwtstep_rel s z rs hrel pc hpc zrom i _ hi v1 hv1 v hok n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .sd i, ⟨hi, v1, v2, hv1, hv2, hok⟩ =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := sdtstep_rel s z rs hrel pc hpc zrom i _ hi v1 v2 hv1 hv2 hok n
    exact ⟨s', z', h1, h2, h3, by rw [hm]; exact memWrite_ramOnly _ _ _ hok.zwrite⟩
  | .sw i, ⟨hi, v1, v2, hv1, hv2, hok⟩ =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := swtstep_rel s z rs hrel pc hpc zrom i _ hi v1 v2 hv1 hv2 hok n
    exact ⟨s', z', h1, h2, h3, by rw [hm]; exact memWrite_ramOnly _ _ _ hok.zwrite⟩
  | .br i, hi =>
    obtain ⟨v1, hv1⟩ := getX_some hrel i.rs1.isLt
    obtain ⟨v2, hv2⟩ := getX_some hrel i.rs2.isLt
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := btstep_rel s z rs hrel pc hpc zrom i _ hi v1 v2 hv1 hv2 n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .j i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := jstep_rel s z rs hrel pc hpc zrom i hi.toJAt n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .jr i, ⟨hi, he, v1, hv1, t0, t1⟩ =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := jrstep_rel s z rs hrel pc hpc zrom i (hi.toJrAt he) v1 hv1 ⟨t0, t1⟩ n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .rw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := rwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .iw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := iwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .siw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := siwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .m i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := mstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .fence i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := fstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .zba i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := zbastep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .slliuw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := slliuwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .zbs i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := zbsstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .zbsi i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := zbsistep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .zbb i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := zbbstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .zbbw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := zbbwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .rori i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := roristep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .roriw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := roriwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .pack i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := packstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .packw i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := packwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .sext i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := sextstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .zexth i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := zexthstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .byte i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := bytestep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩
  | .count i, hi =>
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := countstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', h1, h2, h3, RamOnly.of_eq hm⟩

/-- An lr or an sc at pc, its access good: run by the machine in its own
    way (`mNext`), in one or two ZisK steps. -/
def LrScOk (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) (pc : BitVec 64) :
    Prop :=
  (∃ i : LrInstr, WordsAt s rs pc zrom i.encode (nextWord s pc) ∧
    ∃ v1, getX s i.rs1.toNat = some v1 ∧ LrOk s rs i pc v1) ∨
  (∃ i : ScInstr, WordsAt s rs pc zrom i.encode (nextWord s pc) ∧
    ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧ getX s i.rs2.toNat = some v2 ∧ ScOk rs i pc v1)

/-- What one step of the machine needs: an instruction, run in one ZisK
    step, or a jalr with an odd offset or a Zba .uw form, in two, or an AMO,
    in two to four; or an lr or an sc. -/
def OneOkT (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) : Prop :=
  ∃ pc, s.regs.get? Register.PC = some pc ∧
    ((∃ o, StepOkT s rs pc zrom o) ∨
     (∃ i, JrtAt s rs pc zrom i (nextWord s pc) ∧ i.imm[0] = true ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        (jalrTarget v1 i.imm)[0] = false ∧ (jalrTarget v1 i.imm)[1] = false) ∨
     (∃ i, UwAt s rs pc zrom i (nextWord s pc)) ∨
     (∃ i, WordsAt s rs pc zrom i.encode (nextWord s pc) ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧ AmoOk s rs i v1) ∨
     LrScOk rs zrom s pc)

theorem one_stepT (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : OneOkT rs zrom s) (n : Nat) :
    ∃ s' z' k', MStep rs n s s' ∧ ZRun zrom k' z z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem := by
  obtain ⟨pc, hpc, ⟨o, ho⟩ | ⟨i, hi, hodd, v1, hv1, t0, t1⟩ | ⟨i, hi⟩ | ⟨i, hi, v1, hv1, hok⟩ |
    ⟨i, hi, v1, hv1, hok⟩ | ⟨i, hi, v1, v2, hv1, hv2, hok⟩⟩ := h
  · obtain ⟨s', z', h1, h2, h3, h4⟩ := step_relT s z rs hrel pc hpc zrom o ho n
    obtain ⟨-, e, he, hop⟩ := StepOkT.word o ho
    exact ⟨s', z', 1, .plain pc e hpc he hop h1, .step h2 (.done _), h3, h4⟩
  · obtain ⟨ia, hjo⟩ := hi.toJoAt hodd
    obtain ⟨s', z', h1, h2, h3, -, hm⟩ := jo_rel s z rs hrel pc hpc zrom i ia hjo v1 hv1 ⟨t0, t1⟩ n
    exact ⟨s', z', 2, .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_jr i) h1, h2, h3,
      RamOnly.of_eq hm⟩
  · obtain ⟨s', z', h1, h2, h3, -, hm⟩ := uwstep_rel s z rs hrel pc hpc zrom i _ hi n
    exact ⟨s', z', 2, .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_uw i) h1, h2, h3,
      RamOnly.of_eq hm⟩
  · obtain ⟨s', z', k, h1, h2, h3, -, hm⟩ := amostep_rel s z rs hrel pc hpc zrom i _ hi v1 hv1 hok n
    exact ⟨s', z', k, .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_amo i) h1, h2, h3, hm⟩
  · obtain ⟨s', z', h1, h2, h3, h4⟩ := lrstep_m s z rs hrel pc hpc zrom i _ hi v1 hv1 hok n
    exact ⟨s', z', 1, h1, h2, h3, h4⟩
  · exact scstep_m s z rs hrel pc hpc zrom i _ hi v1 v2 hv1 hv2 hok n

/-- A state the runs go on from is at an executable pc. -/
theorem OneOkT.exec {rs : List PMA_Region} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {s : RVState}
    (h : OneOkT rs zrom s) : ∃ pc, s.regs.get? Register.PC = some pc ∧ ExecAt rs pc := by
  obtain ⟨pc, hpc, ⟨o, ho⟩ | ⟨i, hi, -⟩ | ⟨i, hi⟩ | ⟨i, hi, -⟩ | ⟨i, hi, -⟩ | ⟨i, hi, -⟩⟩ := h
  · exact ⟨pc, hpc, (StepOkT.word o ho).1⟩
  all_goals exact ⟨pc, hpc, hi.x1⟩

/-- For the next k steps of the machine, from the state it reaches: each one
    covered as `OneOkT` says, or, two at a time, a fused auipc-jalr pair. -/
def RunOkT (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) :
    Nat → RVState → Nat → Prop
  | 0, _, _ => True
  | 1, s, _ => OneOkT rs zrom s
  | k + 2, s, n =>
    (OneOkT rs zrom s ∧ RunOkT rs zrom (k + 1) (mNext n s) (n + 1)) ∨
      ((∃ pc i, s.regs.get? Register.PC = some pc ∧ FtAt s rs pc zrom i) ∧
        RunOkT rs zrom k (mNext (n + 1) (mNext n s)) (n + 2))

theorem RunOkT.exec {rs : List PMA_Region} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {s : RVState} {n : Nat} :
    ∀ {k}, RunOkT rs zrom (k + 1) s n → ∃ pc, s.regs.get? Register.PC = some pc ∧ ExecAt rs pc
  | 0, h => OneOkT.exec h
  | _ + 1, .inl ⟨h, _⟩ => OneOkT.exec h
  | _ + 1, .inr ⟨⟨pc, _, hpc, hi⟩, _⟩ => ⟨pc, hpc, hi.x1⟩

/-- The fused pair's two steps are the machine's: plain RISC-V words. -/
theorem fused_mnext {s s2 : RVState} {rs : List PMA_Region} {pc : BitVec 64}
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {i : FInstr} {n : Nat}
    (hpc : s.regs.get? Register.PC = some pc) (hi : FtAt s rs pc zrom i) (hrun : RVRun n 2 s s2)
    (hpc1 : (rvNext n s).regs.get? Register.PC = some (pc + 4)) (hmem1 : (rvNext n s).mem = s.mem) :
    ∃ t1, (interpretSailEffects (try_step n false)).run s = .ok false t1 ∧
      (interpretSailEffects (try_step (n + 1) false)).run t1 = .ok false s2 ∧
      MStep rs n s t1 ∧ MStep rs (n + 1) t1 s2 ∧ mNext n s = t1 ∧ mNext (n + 1) t1 = s2 := by
  obtain ⟨t1, h1, h2⟩ : ∃ t1, (interpretSailEffects (try_step n false)).run s = .ok false t1 ∧
      (interpretSailEffects (try_step (n + 1) false)).run t1 = .ok false s2 := by
    cases hrun with
    | step h1 r => cases r with
      | step h2 r2 => cases r2 with
        | done => exact ⟨_, h1, h2⟩
  rw [rvNext_of h1] at hpc1 hmem1
  have m1 : MStep rs n s t1 := .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_u _) h1
  have m2 : MStep rs (n + 1) t1 s2 := .plain (pc + 4) _ hpc1
    (readWord_of (by rw [hmem1]; exact hi.g0) (by rw [hmem1]; exact hi.g1) (by rw [hmem1]; exact hi.g2)
      (by rw [hmem1]; exact hi.g3)) (notSys_jr _) h2
  exact ⟨t1, h1, h2, m1, m2, m1.next, m2.next⟩

/-- The simulation over whole runs, the ZisK program the spec's translation
    of the RISC-V code: k steps of the machine from related states, and
    some number of ZisK steps, end related. -/
theorem prog_spec (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) :
    ∀ (k : Nat) (s : RVState) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel s z rs → RunOkT rs zrom k s n →
      ∃ z' k', ZRun zrom k' z z' ∧ Rel (rvIter n k s) z' rs
  | 0, s, z, n, hrel, _ => ⟨z, 0, .done z, hrel⟩
  | 1, s, z, n, hrel, ho => by
    obtain ⟨s1, z1, k1, hm, hzrun, hrel1, -⟩ := one_stepT s z rs hrel zrom ho n
    exact ⟨z1, k1, hzrun, by simp only [rvIter, hm.next]; exact hrel1⟩
  | k + 2, s, z, n, hrel, .inl ⟨ho, hrest⟩ => by
    obtain ⟨s1, z1, k1, hm, hzrun, hrel1, -⟩ := one_stepT s z rs hrel zrom ho n
    rw [hm.next] at hrest
    obtain ⟨z2, k2, hzrun2, hrel2⟩ := prog_spec zrom rs (k + 1) s1 z1 (n + 1) hrel1 hrest
    exact ⟨z2, k1 + k2, ZRun.append hzrun hzrun2, by rw [rvIter, hm.next]; exact hrel2⟩
  | k + 2, s, z, n, hrel, .inr ⟨⟨pc, i, hpc, hi⟩, hrest⟩ => by
    obtain ⟨s2, z1, hrun, hzrun, hrel1, -, -, hpc1, hmem1, -⟩ := fused_rel s z rs hrel pc hpc zrom i hi.toFAt n
    obtain ⟨t1, -, -, -, -, e1, e2⟩ := fused_mnext hpc hi hrun hpc1 hmem1
    rw [e1, e2] at hrest
    obtain ⟨z3, k3, hzrun3, hrel3⟩ := prog_spec zrom rs k s2 z1 (n + 2) hrel1 hrest
    exact ⟨z3, 1 + k3, ZRun.append hzrun hzrun3, by rw [rvIter, rvIter, e1, e2]; exact hrel3⟩

end Equiv.Sim
