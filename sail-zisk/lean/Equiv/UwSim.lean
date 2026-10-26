/-
  The simulation on Zba's .uw forms, any registers: add.uw, sh1add.uw,
  sh2add.uw, sh3add.uw. One sail-riscv step (ZBA_RTYPEUW: rs1's low word,
  zero-extended, shifted by 0 to 3, plus rs2), two ZisK steps, as the
  transpiler makes them: rs1's low word (an and with 0xffffffff) to ZisK's
  own register 32, on to an internal address; there the add (add.uw) or the
  shift-and-add with r[32] into rd, on to pc + 4. Register 32 is ZisK's
  alone: the relation ties x1..x31 only.
-/
import Equiv.ZbbCountSim
import Equiv.RiscvDecodeZba

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (ShAdd)

/-! ## The instruction -/

structure UwInstr where
  op : Option ShAdd
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

/-- The shift, as sail-riscv's ZBA_RTYPEUW carries it. -/
def UwInstr.sh (i : UwInstr) : BitVec 2 := match i.op with | none => 0 | some o => o.sh

def UwInstr.f7 (i : UwInstr) : BitVec 7 := match i.op with | none => 0x04 | some _ => 0x10

def UwInstr.f3 (i : UwInstr) : BitVec 3 := match i.op with | none => 0 | some o => o.f3

def UwInstr.encode (i : UwInstr) : BitVec 32 :=
  i.f7 ++ i.rs2 ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x3B#7

theorem UwInstr.fields (i : UwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x3B#7 ∧ v.extractLsb 31 25 = i.f7 ∧ v.extractLsb 14 12 = i.f3 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, UwInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- The decoder's view: ZBA_RTYPEUW with the instruction's fields. -/
theorem UwInstr.decode (i : UwInstr) (t : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x3B#7)
    (hf7 : w.extractLsb 31 25 = i.f7) (hf3 : w.extractLsb 14 12 = i.f3)
    (hp : t.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : t.regs.get? Register.mseccfg = some sec)
    (mv : RegisterType Register.misa) (hmv : t.regs.get? Register.misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run t
      = .ok (.ZBA_RTYPEUW (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          i.sh)) t := by
  rcases hk : i.op with _ | o
  · simp only [UwInstr.f7, UwInstr.f3, UwInstr.sh, hk] at hf7 hf3 ⊢
    exact decode_add_uw t w hop hf7 hf3 hp sec hsec mv hmv hmB
  · simp only [UwInstr.f7, UwInstr.f3, UwInstr.sh, hk] at hf7 hf3 ⊢
    have h12 : w.extractLsb 12 12 = 0#1 := by
      cases o <;> simp only [Transpiler.ShAdd.f3, Transpiler.ShAdd.sh] at hf3 <;> bv_decide
    have hsh : w.extractLsb 14 13 = o.sh := by
      cases o <;> simp only [Transpiler.ShAdd.f3, Transpiler.ShAdd.sh] at hf3 ⊢ <;> bv_decide
    have hne : w.extractLsb 14 13 ≠ 0#2 := by
      rw [hsh]; cases o <;> decide
    rw [← hsh]
    exact decode_zba_uw t w hop hf7 h12 hne hp sec hsec mv hmv hmB

/-- The spec's translation: the and at pc, the add or shift-and-add at the
    internal address ia. -/
theorem UwInstr.rom_of_spec (i : UwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    ∃ ia, zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, 0xFFFFFFFF) (.ZOP_AND ())
        (Transpiler.storeReg 32) (ia - pc) (ia - pc) ∧
      zrom ia = (match i.op with
        | none => Transpiler.mk (Transpiler.srcA i.rs2.toNat) (Transpiler.srcB 32) (.ZOP_ADD ())
            (Transpiler.storeReg i.rd.toNat) (pc + 4 - ia) (pc + 4 - ia)
        | some o => Transpiler.mk (Transpiler.srcA 32) (Transpiler.srcB i.rs2.toNat) o.zop
            (Transpiler.storeReg i.rd.toNat) (pc + 4 - ia) (pc + 4 - ia)) := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, UwInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, UwInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = i.rs2.toNat := by
    simp only [Transpiler.rs2, UwInstr.encode]; congr 1; bv_decide
  have ho : Transpiler.opcode i.encode = 0x3B#7 := by
    simp only [Transpiler.opcode, UwInstr.encode]; bv_decide
  have hf7 : Transpiler.funct7 i.encode = i.f7 := by
    simp only [Transpiler.funct7, UwInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = i.f3 := by
    simp only [Transpiler.funct3, UwInstr.encode]; bv_decide
  obtain ⟨ia, l, hl, hz⟩ := h
  rcases hk : i.op with _ | o <;> simp only [UwInstr.f7, UwInstr.f3, hk] at hf7 hf3
  all_goals
    try cases o
    all_goals
      simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh,
        Transpiler.uw, hr1, hr2, hrd] at hl
      subst hl
      simp only [List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil,
        or_false] at hz
      exact ⟨ia, hz.1, hz.2⟩

structure UwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : UwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

/-! ## Register 32 -/

theorem upd32_get (xs : Vector (BitVec 64) 40) (t : BitVec 64) {r : Nat} (hr : r < 32) :
    (vectorUpdate xs 32 t)[r]! = xs[r]! := by
  simp [vectorUpdate, Vector.set!, show r < 40 by omega]
  rw [Array.getElem_setIfInBounds]; simp [show (32 : Nat) ≠ r by omega]; rfl

theorem regv_upd32 (xs : Vector (BitVec 64) 40) (t : BitVec 64) {r : Nat} (hr : r < 32) :
    regv (vectorUpdate xs 32 t) r = regv xs r := by
  unfold regv; split
  · rfl
  · rw [getElem!_int, getElem!_int, upd32_get xs t hr]

theorem agree_upd32 {s : RVState} {xs : Vector (BitVec 64) 40} (agree : RegsAgree s xs) (t : BitVec 64) :
    RegsAgree s (vectorUpdate xs 32 t) := by
  intro k hk1 hk2
  rw [upd32_get xs t hk2]; exact agree k hk1 hk2

/-! ## The values -/

theorem uw_val (i : UwInstr) (a b : BitVec 64) :
    (match i.op with
      | none => (_root_.Zisk.Functions.zisk_exec (.ZOP_ADD ()) b
          (_root_.Zisk.Functions.zisk_exec (.ZOP_AND ()) a 0xFFFFFFFF).1).1
      | some o => (_root_.Zisk.Functions.zisk_exec o.zop
          (_root_.Zisk.Functions.zisk_exec (.ZOP_AND ()) a 0xFFFFFFFF).1 b).1) = zbauwVal i.sh a b := by
  rcases hk : i.op with _ | o
  · simp [UwInstr.sh, hk, _root_.Zisk.Functions.zisk_exec, zbauwVal, shift_bits_left, zero_extend,
      Sail.BitVec.zeroExtend, Sail.BitVec.extractLsb]
    bv_decide
  · simp only [UwInstr.sh, hk, Transpiler.ShAdd.zval, zbaVal, zbauwVal]
    cases o <;> simp [Transpiler.ShAdd.sh, _root_.Zisk.Functions.zisk_exec, zero_extend, Sail.BitVec.zeroExtend,
      Sail.BitVec.extractLsb, shift_bits_left] <;> bv_decide

theorem upd32_self (xs : Vector (BitVec 64) 40) (t : BitVec 64) : regv (vectorUpdate xs 32 t) 32 = t := by
  unfold regv
  simp only [show (32 : Nat) ≠ 0 by omega, if_false, getElem!_int]
  simp [vectorUpdate, Vector.set!]

theorem srcB32 (xs : Vector (BitVec 64) 40) : BSrc (Transpiler.srcB 32) xs (regv xs 32) :=
  Or.inr ⟨32, by omega, by omega, by simp [Transpiler.srcB], by simp [regv]⟩

theorem srcA32 (xs : Vector (BitVec 64) 40) : ASrc (Transpiler.srcA 32) xs (regv xs 32) :=
  Or.inr ⟨32, by omega, by omega, by simp [Transpiler.srcA], by simp [regv]⟩

/-- x0..x31 after rd's update, register 32 changed first or not. -/
theorem regv_xsUpd32 (xs : Vector (BitVec 64) 40) (t : BitVec 64) (rd : Nat) (v : BitVec 64) (ld : rd < 32)
    {k : Nat} (hk : k < 32) :
    regv (Equiv.Zisk.xsUpd (vectorUpdate xs 32 t) rd v) k = regv (Equiv.Zisk.xsUpd xs rd v) k := by
  unfold Equiv.Zisk.xsUpd
  split
  · exact regv_upd32 xs t hk
  · unfold regv; split
    · rfl
    · rw [getElem!_int, getElem!_int]
      have s40 : k < xs.toArray.size := by simp; omega
      simp [vectorUpdate, Vector.set!, show k < 40 by omega]
      rw [Array.getElem_setIfInBounds, Array.getElem_setIfInBounds, Array.getElem_setIfInBounds]
      all_goals first | (by_cases e : rd = k <;> simp [e] <;> omega) | simpa using s40

set_option maxHeartbeats 4000000 in
/-- ZisK's two steps: the and to register 32 at pc, the add or the
    shift-and-add into rd at ia, on to pc + 4. -/
theorem zisk_uw (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (z : Equiv.Zisk.ZState) (i : UwInstr)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (hpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (hze : z.regs.get? _root_.Zisk.Register.Zerror = some false)
    (r1 : zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, 0xFFFFFFFF) (.ZOP_AND ())
        (Transpiler.storeReg 32) (ia - pc) (ia - pc))
    (r2 : zrom ia = (match i.op with
        | none => Transpiler.mk (Transpiler.srcA i.rs2.toNat) (Transpiler.srcB 32) (.ZOP_ADD ())
            (Transpiler.storeReg i.rd.toNat) (pc + 4 - ia) (pc + 4 - ia)
        | some o => Transpiler.mk (Transpiler.srcA 32) (Transpiler.srcB i.rs2.toNat) o.zop
            (Transpiler.storeReg i.rd.toNat) (pc + 4 - ia) (pc + 4 - ia))) :
    ∃ z', ZRun zrom 2 z z' ∧
      (∃ xs', z'.regs.get? _root_.Zisk.Register.Xs = some xs' ∧ ∀ k, k < 32 → regv xs' k =
        regv (Equiv.Zisk.xsUpd xs i.rd.toNat (zbauwVal i.sh (regv xs i.rs1.toNat) (regv xs i.rs2.toNat))) k) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? _root_.Zisk.Register.Step = some st ∧
        z'.regs.get? _root_.Zisk.Register.Zerror = some false) ∧ z'.mem = z.mem := by
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  -- step 1: the and, to register 32, on to ia
  have hz1 := Equiv.Zisk.execute_mkj z (Transpiler.srcA i.rs1.toNat) (.B_IMM, 0xFFFFFFFF) (.ZOP_AND ())
    Equiv.Zisk.and_pure 32 (by omega) (ia - pc) (ia - pc) xs pc st hx hpc hst
    _ 0xFFFFFFFF (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl)
  rw [← r1, ← zstep_run zrom z pc hpc] at hz1
  simp only [ite_self] at hz1
  have eia : pc + (ia - pc) = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have e4 : ia + (pc + 4 - ia) = pc + 4 := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  generalize ht : (_root_.Zisk.Functions.zisk_exec (.ZOP_AND ()) (regv xs i.rs1.toNat) 0xFFFFFFFF).1 = t at hz1
  generalize hz1s : ({ z with regs := _ } : Equiv.Zisk.ZState) = z1 at hz1
  have hx1 : z1.regs.get? _root_.Zisk.Register.Xs = some (vectorUpdate xs 32 t) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs]
  have hpc1 : z1.regs.get? _root_.Zisk.Register.PC = some ia := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
  have hst1 : z1.regs.get? _root_.Zisk.Register.Step = some (BitVec.addInt st 1) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert]
  have hze1 : z1.regs.get? _root_.Zisk.Register.Zerror = some false := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs, hze]
  have hm1 : z1.mem = z.mem := by subst hz1s; rfl
  have u1 : regv (vectorUpdate xs 32 t) i.rs1.toNat = regv xs i.rs1.toNat := regv_upd32 xs t l1
  have u2 : regv (vectorUpdate xs 32 t) i.rs2.toNat = regv xs i.rs2.toNat := regv_upd32 xs t l2
  -- step 2, at ia
  rcases hk : i.op with _ | o <;> simp only [hk] at r2
  · have hz2 := Equiv.Zisk.execute_mkj z1 (Transpiler.srcA i.rs2.toNat) (Transpiler.srcB 32) (.ZOP_ADD ())
      Equiv.Zisk.add_pure i.rd.toNat (by omega) (pc + 4 - ia) (pc + 4 - ia)
      (vectorUpdate xs 32 t) ia (BitVec.addInt st 1) hx1 hpc1 hst1 _ _
      (Equiv.Zisk.srcA_ASrc _ _ l2) (srcB32 _)
    rw [← r2, ← zstep_run zrom z1 ia hpc1] at hz2
    have hv := uw_val i (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)
    simp only [hk, ht] at hv
    rw [u2, upd32_self] at hz2
    dsimp only at hz2
    rw [hv] at hz2
    refine ⟨_, .step hz1 (.step hz2 (.done _)), ⟨Equiv.Zisk.xsUpd (vectorUpdate xs 32 t) i.rd.toNat _, ?_,
      fun k hk => regv_xsUpd32 xs t _ _ ld hk⟩, ?_, ⟨BitVec.addInt (BitVec.addInt st 1) 1, ?_, ?_⟩, ?_⟩
    · simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
      exact linkRegs_xs _ _ _ _ hx1
    · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
    · simp [Std.ExtDHashMap.get?_insert]
    · simp [Std.ExtDHashMap.get?_insert, hze1]
    · exact hm1
  · have hz2 := Equiv.Zisk.execute_mkj z1 (Transpiler.srcA 32) (Transpiler.srcB i.rs2.toNat) o.zop
      o.pure i.rd.toNat (by omega) (pc + 4 - ia) (pc + 4 - ia)
      (vectorUpdate xs 32 t) ia (BitVec.addInt st 1) hx1 hpc1 hst1 _ _
      (srcA32 _) (Equiv.Zisk.srcB_BSrc _ _ l2)
    rw [← r2, ← zstep_run zrom z1 ia hpc1] at hz2
    have hv := uw_val i (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)
    simp only [hk, ht] at hv
    rw [u2, upd32_self] at hz2
    dsimp only at hz2
    rw [hv] at hz2
    refine ⟨_, .step hz1 (.step hz2 (.done _)), ⟨Equiv.Zisk.xsUpd (vectorUpdate xs 32 t) i.rd.toNat _, ?_,
      fun k hk => regv_xsUpd32 xs t _ _ ld hk⟩, ?_, ⟨BitVec.addInt (BitVec.addInt st 1) 1, ?_, ?_⟩, ?_⟩
    · simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
      exact linkRegs_xs _ _ _ _ hx1
    · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
    · simp [Std.ExtDHashMap.get?_insert]
    · simp [Std.ExtDHashMap.get?_insert, hze1]
    · exact hm1

theorem execute_zba_rtypeuw_eq (a b c : regidx) (sh : BitVec 2) :
    execute (.ZBA_RTYPEUW (a, b, c, sh)) = execute_ZBA_RTYPEUW a b c sh := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One sail-riscv step on a .uw form, any registers, and the transpiler's
    two ZisK steps keep the states related. -/
theorem uwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : UwInstr) (next : Option (BitVec 32))
    (hi : UwAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ ZRun zrom 2 z z' ∧
      Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf7, hf3, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have v1 := getX_regv agree l1
  have v2 := getX_regv agree l2
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ := try_step_gen s rs hr pc' hpc _ _ _ _ hw
    (.ZBA_RTYPEUW (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.sh))
    (by
      intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
      have := i.decode t _ hop hf7 hf3 htp w hw' mb hmb hmbB
      rw [hrdx, hr1x, hr2x] at this; exact this)
    i.rd.toNat (zbauwVal i.sh (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4)
    (by
      intro t he _ _ _ _ hnext _
      rw [execute_zba_rtypeuw_eq, execute_zba_rtypeuw t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
        (getX_of_agree he l2 v2), insert_same t _ _ hnext]
      rfl) n
  obtain ⟨ia, r1, r2⟩ := i.rom_of_spec hi.tr
  obtain ⟨z', hz, ⟨xs', hx', hxs''⟩, hzpc', hst', hzm'⟩ := zisk_uw zrom z i xs pc' st ia hx hzpc hst hze r1 r2
  refine ⟨s', z', hrun, hz, ⟨hr.keep hwf ho, ⟨xs', hx', ?_⟩, ⟨_, hpc'', hzpc'⟩, hst',
    Equiv.normIn_congr hzm hzm' hmem⟩, hpc'', hmem⟩
  intro k hk1 hk2
  rw [hxs' k hk2]
  have a := regs_after0 s xs i.rd.toNat (zbauwVal i.sh (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) ld agree k hk1 hk2
  have e := hxs'' k hk2
  simp only [regv, show k ≠ 0 by omega, if_false, getElem!_int] at e
  rw [e]; exact a

end Equiv.Sim
