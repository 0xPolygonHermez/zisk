/-
  The simulation, one step, on the narrow loads: lb, lbu, lh, lhu, lw and
  lwu, and the transpiler's load_op for each (signextend_b/h/w or copyb of
  the 1, 2 or 4 bytes at r[rs1] + imm), as Equiv/LoadSim.lean has it for ld.
-/
import Equiv.LoadSim
import Equiv.RiscvLoadW
import Equiv.RiscvDecodeLoad
import Equiv.ZiskLoadW

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The narrow loads. -/
inductive LKind | lb | lh | lw | lbu | lhu | lwu

/-- Its funct3. -/
def LKind.f3 : LKind → BitVec 3
  | .lb => 0 | .lh => 1 | .lw => 2 | .lbu => 4 | .lhu => 5 | .lwu => 6

/-- How many bytes it reads. -/
def LKind.w : LKind → Nat
  | .lb | .lbu => 1 | .lh | .lhu => 2 | .lw | .lwu => 4

/-- Whether it zero-extends them. -/
def LKind.u : LKind → Bool
  | .lb | .lh | .lw => false | .lbu | .lhu | .lwu => true

/-- The op the transpiler emits for it (riscv2zisk_context.rs). -/
def LKind.op : LKind → _root_.Zisk.zisk_op
  | .lb => .ZOP_SIGNEXTEND_B () | .lh => .ZOP_SIGNEXTEND_H () | .lw => .ZOP_SIGNEXTEND_W ()
  | .lbu | .lhu | .lwu => .ZOP_COPYB ()

theorem LKind.pure (k : LKind) : Equiv.Zisk.PureOp k.op := by
  cases k
  · exact Equiv.Zisk.sextb_pure
  · exact Equiv.Zisk.sexth_pure
  · exact Equiv.Zisk.sextw_pure
  all_goals exact Equiv.Zisk.copyb_pure

/-- ZisK's op on the bytes, zero-extended as source_b reads them, is
    sail-riscv's extension of them. -/
theorem LKind.val (k : LKind) (a : BitVec 64) (v : BitVec (8 * k.w)) :
    _root_.Zisk.Functions.zisk_exec k.op a (v.setWidth 64) = (extend_value k.u v, false) := by
  cases k <;> revert v
  · change ∀ v : BitVec 8, (BitVec.signExtend 64 (BitVec.extractLsb 7 0 (BitVec.setWidth 64 v)), false)
      = (BitVec.signExtend 64 v, false)
    intro v; congr 1; bv_decide
  · change ∀ v : BitVec 16, (BitVec.signExtend 64 (BitVec.extractLsb 15 0 (BitVec.setWidth 64 v)), false)
      = (BitVec.signExtend 64 v, false)
    intro v; congr 1; bv_decide
  · change ∀ v : BitVec 32, (BitVec.signExtend 64 (BitVec.extractLsb 31 0 (BitVec.setWidth 64 v)), false)
      = (BitVec.signExtend 64 v, false)
    intro v; congr 1; bv_decide
  all_goals intro v; rfl

/-- A narrow load, by its kind and fields. -/
structure LwInstr where
  k : LKind
  rd : BitVec 5
  rs1 : BitVec 5
  imm : BitVec 12

def LwInstr.encode (i : LwInstr) : BitVec 32 :=
  i.imm ++ i.rs1 ++ i.k.f3 ++ i.rd ++ 0x03#7

theorem LwInstr.fields (i : LwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x03#7 ∧ v.extractLsb 14 12 = i.k.f3 ∧ v.extractLsb 31 20 = i.imm ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, LwInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- The decoder's unsigned bit and width, from funct3. -/
theorem LKind.dec (k : LKind) (w : BitVec 32) (hf3 : w.extractLsb 14 12 = k.f3) :
    bool_bit_backwards (w.extractLsb 14 14) = k.u ∧
    width_enc_backwards (w.extractLsb 13 12) = (k.w : Int) := by
  have h14 : w.extractLsb 14 14 = (k.f3).extractLsb 2 2 := by rw [← hf3]; bv_decide
  have h13 : w.extractLsb 13 12 = (k.f3).extractLsb 1 0 := by rw [← hf3]; bv_decide
  rw [h14, h13]; cases k <;> decide

theorem LKind.valid (k : LKind) :
    valid_load_encdec (k.w : Int).toNat k.u = true := by
  cases k <;> decide

/-- A narrow load `i` at pc: its encoding in executable memory there, rd,
    rs1 ≠ x0, and the transpiler's instruction for it at pc. -/
structure LwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LwInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rd : i.rd ≠ 0
  rs1 : i.rs1 ≠ 0
  rom : zrom pc = Equiv.Zisk.loadInst i.k.op i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) i.k.w

/-- What a load of w bytes needs when it runs, at address a: in a
    readable region on both sides, off ZisK's register file, and the bytes
    there. -/
structure LwOk (s : RVState) (rs : List PMA_Region) (a : BitVec 64) (w : Nat) (v : BitVec (8 * w)) : Prop where
  region : ∃ r, matching_pma_region rs (.Physaddr a) w = some r ∧ r.attributes.readable = true ∧
    AlignOk r (.Load .Data) a w
  zread : _root_.Zisk.Functions.readable a w = true
  zregs : _root_.Zisk.Functions.hits_registers a w = false
  /-- Off the input's first 8 bytes, which are not memory. -/
  inOff : Equiv.InOff a w
  bytes : Equiv.memRead s.mem w a.toNat = some v

theorem execute_load_w (imm : BitVec 12) (a b : regidx) (u : Bool) (w : Int) :
    execute (.LOAD (imm, a, b, u, w)) = execute_LOAD imm a b u w.toNat := by
  simp [execute]

/-- sail-riscv's execution of each narrow load. -/
theorem LKind.exec (k : LKind) (t : RVState) (hwf : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType Register.mseccfg) (hsec : t.regs.get? Register.mseccfg = some sec)
    (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) k.w = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) k.w)
    (v : BitVec (8 * k.w))
    (hv : (Sail.ArchSem.readBytes k.w (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) k.u k.w)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value k.u v)) := by
  cases k
  · exact execute_lb t hwf rs hs sec hsec hpm rs1 rd imm v1 h1 r hm hrd hal v hv
  · exact execute_lh t hwf rs hs sec hsec hpm rs1 rd imm v1 h1 r hm hrd hal v hv
  · exact execute_lw t hwf rs hs sec hsec hpm rs1 rd imm v1 h1 r hm hrd hal v hv
  · exact execute_lbu t hwf rs hs sec hsec hpm rs1 rd imm v1 h1 r hm hrd hal v hv
  · exact execute_lhu t hwf rs hs sec hsec hpm rs1 rd imm v1 h1 r hm hrd hal v hv
  · exact execute_lwu t hwf rs hs sec hsec hpm rs1 rd imm v1 h1 r hm hrd hal v hv

theorem LKind.w_ne8 (k : LKind) (a : BitVec 64) : (a == _root_.Zisk.Functions.INPUT_ADDR && k.w == 8) = false := by
  cases k <;> simp [LKind.w]

set_option maxHeartbeats 4000000 in
/-- One step of each on a narrow load keeps the states related, when the
    load's address `a` (rs1 + imm) is good to read and holds `v`. -/
theorem lwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LwInstr) (hi : LwAt s rs pc zrom i)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (v : BitVec (8 * i.k.w))
    (hok : LwOk s rs (v1 + i.imm.signExtend 64) i.k.w v) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  obtain ⟨hu, hwd⟩ := i.k.dec _ hf3
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have e1 := ne_zero_toNat hi.rs1
  have ed := ne_zero_toNat hi.rd
  have a1 := agree i.rs1.toNat (by omega) l1
  have hxv : xs[i.rs1.toNat]! = v1 := by rw [hv1] at a1; exact (Option.some.inj a1).symm
  have hrd5 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 11 7 ≠ 0#5 := by
    rw [hrdx]; exact hi.rd
  have hvalid := i.k.valid
  rw [← hu, ← hwd] at hvalid
  obtain ⟨⟨r, hm, hrb, hal⟩, hzr, hzn, hoff, hbytes⟩ := hok
  refine alu_step_rel s z rs hr xs hx agree pc' hpc hzpc _ _ _ _ hw
    (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, i.k.u, (i.k.w : Int))) ?_ i.rd.toNat ed ld
    (extend_value i.k.u v) ?_ zrom hzm ?_ n
  · intro t htp ⟨w, hw'⟩ hmt _
    have := decode_load t _ hop hrd5 hvalid htp w hw'
    rw [himm, hr1x, hrdx, hu, hwd] at this; exact this
  · intro t he htm hmr hsec hwf
    rw [execute_load_w, Int.toNat_natCast]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    have hv := Equiv.riscv_readBytes t i.k.w _ v (htm ▸ hbytes)
    exact i.k.exec t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
      r hm hrb hal v hv
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    have hxv' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv
    have hex := Equiv.Zisk.execute_load z i.k.op i.k.pure i.rd.toNat i.rs1.toNat _ i.k.w (by omega) (by omega)
      xs pc' st hx hzpc hst v (by rw [hxv']; exact hzr) (by rw [hxv']; exact hzn) (i.k.w_ne8 _)
      (by rw [hxv', Equiv.memRead_normIn hzm hoff]; exact hbytes)
    simp only [hxv', i.k.val] at hex
    refine ⟨_, hex, by simp [Std.ExtDHashMap.get?_insert], by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩

end Equiv.Sim
