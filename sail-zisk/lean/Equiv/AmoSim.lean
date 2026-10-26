/-
  The simulation on A's AMOs, any registers: amoswap, amoadd, amoxor,
  amoand, amoor, amomin, amomax, amominu, amomaxu, .w and .d. One sail-riscv
  step (execute_amo8, execute_amo4), and the transpiler's ZisK steps (the
  load, the op on c, the store of c; through register 32 when rd is rs1 or
  rs2; ZiskAmoRun.lean): the same memory written, rd the same.
-/
import Equiv.ZiskAmoRun
import Equiv.RiscvAmo
import Equiv.RiscvDecodeAmo
import Equiv.RamOnly

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (AmoOp)

/-! ## The ops, both ways -/

theorem rI_s {n} (x y : BitVec n) : zopz0zI_s x y = x.slt y := by unfold zopz0zI_s; rw [BitVec.slt]
theorem rK_s {n} (x y : BitVec n) : zopz0zK_s x y = y.slt x := by unfold zopz0zK_s; rw [BitVec.slt]
theorem rI_u {n} (x y : BitVec n) : zopz0zI_u x y = x.ult y := by simp [zopz0zI_u, BitVec.toNatInt, BitVec.ult]
theorem rK_u {n} (x y : BitVec n) : zopz0zK_u x y = y.ult x := by simp [zopz0zK_u, BitVec.toNatInt, BitVec.ult]
theorem zI_s {n} (x y : BitVec n) : _root_.Zisk.Functions.zopz0zI_s x y = x.slt y := by
  unfold _root_.Zisk.Functions.zopz0zI_s; rw [BitVec.slt]
theorem zI_u {n} (x y : BitVec n) : _root_.Zisk.Functions.zopz0zI_u x y = x.ult y := by
  simp [_root_.Zisk.Functions.zopz0zI_u, BitVec.toNatInt, BitVec.ult]

def _root_.Transpiler.AmoOp.r : AmoOp → amoop
  | .swap => .AMOSWAP | .add => .AMOADD | .xor => .AMOXOR | .and => .AMOAND | .or => .AMOOR
  | .min => .AMOMIN | .max => .AMOMAX | .minu => .AMOMINU | .maxu => .AMOMAXU

theorem amo_val_d (op : AmoOp) (l v2 : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (op.zop true) l v2).1 = amoVal op.r l v2 := by
  cases op <;> simp [Transpiler.AmoOp.zop, Transpiler.AmoOp.r, _root_.Zisk.Functions.zisk_exec, amoVal, rI_s, rK_s,
    rI_u, rK_u, zI_s, zI_u] <;> bv_decide

theorem amo_val_w (op : AmoOp) (l : BitVec 32) (v2 : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (op.zop false) (l.signExtend 64) v2).1.setWidth 32
      = amoVal op.r l (v2.setWidth 32) := by
  cases op <;> simp [Transpiler.AmoOp.zop, Transpiler.AmoOp.r, _root_.Zisk.Functions.zisk_exec, amoVal, rI_s, rK_s,
    rI_u, rK_u, zI_s, zI_u, _root_.Zisk.Functions.sx32, _root_.Zisk.Functions.sign_extend, Sail.BitVec.signExtend,
    Sail.BitVec.extractLsb] <;> bv_decide

theorem _root_.Transpiler.AmoOp.pure (op : AmoOp) (d : Bool) : Equiv.Zisk.PureOp (op.zop d) := by
  cases op <;> cases d <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;>
    first | rfl | simp [Transpiler.AmoOp.zop] | decide

theorem aLoad_pure (d : Bool) : Equiv.Zisk.PureOp (Transpiler.aLoad d) := by
  cases d <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.aLoad] | decide

/-- The load's value: the 8 bytes, or the 4 sign-extended. -/
theorem load_val_d (a : BitVec 64) (v : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (Transpiler.aLoad true) a (v.setWidth 64)).1 = v := by
  simp [Transpiler.aLoad, _root_.Zisk.Functions.zisk_exec]

theorem load_val_w (a : BitVec 64) (v : BitVec 32) :
    (_root_.Zisk.Functions.zisk_exec (Transpiler.aLoad false) a (v.setWidth 64)).1 = v.signExtend 64 := by
  simp [Transpiler.aLoad, _root_.Zisk.Functions.zisk_exec, _root_.Zisk.Functions.sign_extend, Sail.BitVec.signExtend,
    Sail.BitVec.extractLsb]
  bv_decide

/-! ## The instruction -/

structure AmoInstr where
  op : AmoOp
  d : Bool
  aq : BitVec 1
  rl : BitVec 1
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def _root_.Transpiler.AmoOp.f5 : AmoOp → BitVec 5
  | .swap => 1 | .add => 0 | .xor => 4 | .and => 12 | .or => 8 | .min => 16 | .max => 20 | .minu => 24
  | .maxu => 28

def AmoInstr.f3 (i : AmoInstr) : BitVec 3 := if i.d then 3 else 2

def AmoInstr.encode (i : AmoInstr) : BitVec 32 :=
  i.op.f5 ++ i.aq ++ i.rl ++ i.rs2 ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x2F#7

/-- sail-riscv's instruction. -/
def AmoInstr.inst (i : AmoInstr) : instruction :=
  .AMO (i.op.r, bool_bit_backwards i.aq, bool_bit_backwards i.rl, .Regidx i.rs2, .Regidx i.rs1,
    if i.d then 8 else 4, .Regidx i.rd)

theorem AmoInstr.fields (i : AmoInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x2F#7 ∧ v.extractLsb 31 27 = i.op.f5 ∧ v.extractLsb 14 12 = i.f3 ∧
    v.extractLsb 26 26 = i.aq ∧ v.extractLsb 25 25 = i.rl ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, AmoInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem AmoInstr.decode (i : AmoInstr) (t : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x2F#7)
    (hf5 : w.extractLsb 31 27 = i.op.f5) (hf3 : w.extractLsb 14 12 = i.f3)
    (haq : w.extractLsb 26 26 = i.aq) (hrl : w.extractLsb 25 25 = i.rl)
    (hrd : w.extractLsb 11 7 = i.rd) (hr1 : w.extractLsb 19 15 = i.rs1) (hr2 : w.extractLsb 24 20 = i.rs2)
    (hp : t.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : t.regs.get? Register.mseccfg = some sec)
    (mv : RegisterType Register.misa) (hmv : t.regs.get? Register.misa = some mv) (hmA : _get_Misa_A mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run t = .ok i.inst t := by
  obtain ⟨op, d, aq, rl, rd, rs1, rs2⟩ := i
  simp only [AmoInstr.inst, AmoInstr.f3] at *
  subst haq hrl hrd hr1 hr2
  cases op <;> cases d <;> simp only [Transpiler.AmoOp.f5, Transpiler.AmoOp.r, if_true, if_false,
    Bool.false_eq_true] at hf5 hf3 ⊢
  · exact decode_amo_swap_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_swap_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_add_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_add_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_xor_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_xor_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_and_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_and_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_or_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_or_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_min_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_min_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_max_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_max_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_minu_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_minu_d t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_maxu_w t w hop hf5 hf3 hp sec hsec mv hmv hmA
  · exact decode_amo_maxu_d t w hop hf5 hf3 hp sec hsec mv hmv hmA

/-- The spec's translation: the list `Transpiler.amo` builds, in the ROM. -/
theorem AmoInstr.rom_of_spec (i : AmoInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    ∃ ia, ∀ p ∈ Transpiler.amo i.encode i.op i.d pc ia, zrom p.1 = p.2 := by
  obtain ⟨ia, l, hl, hz⟩ := h
  have ho : Transpiler.opcode i.encode = 0x2F#7 := by simp only [Transpiler.opcode, AmoInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = i.f3 := by simp only [Transpiler.funct3, AmoInstr.encode]; bv_decide
  have hf5 : i.encode.extractLsb 31 27 = i.op.f5 := by simp only [AmoInstr.encode]; bv_decide
  refine ⟨ia, ?_⟩
  obtain ⟨op, d, aq, rl, rd, rs1, rs2⟩ := i
  simp only [AmoInstr.f3] at hf3 hf5 ⊢
  cases op <;> cases d <;> simp only [Transpiler.AmoOp.f5, if_true, if_false, Bool.false_eq_true] at hf3 hf5 <;>
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf5] at hl <;> subst hl <;> exact hz

/-- An AMO's width in bytes. -/
def aW (d : Bool) : Nat := if d then 8 else 4

theorem aWidth_eq (d : Bool) : Transpiler.aWidth d = ((aW d : Nat) : Int) := by cases d <;> rfl

theorem AmoInstr.regs (i : AmoInstr) :
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
      Transpiler.rs2 i.encode = i.rs2.toNat := by
  simp only [Transpiler.rd, Transpiler.rs1, Transpiler.rs2, AmoInstr.encode]
  generalize i.op.f5 = a; generalize i.f3 = b
  refine ⟨?_, ?_, ?_⟩ <;> congr 1 <;> bv_decide

/-- The value an AMO stores, on ZisK: the op of c and r[rs2], or, the swap, r[rs2]. -/
def amoStored (op : AmoOp) (d : Bool) (L b : BitVec 64) : BitVec 64 :=
  match op with
  | .swap => b
  | _ => (_root_.Zisk.Functions.zisk_exec (op.zop d) L b).1

set_option maxHeartbeats 16000000 in
/-- ZisK's run of an AMO as the spec translates it, whatever its shape: x1..x31
    as after rd gets the loaded value, the memory written with the op's
    result (the swap: r[rs2]), on to pc + 4. -/
theorem zisk_amo_any (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (z : Equiv.Zisk.ZState) (i : AmoInstr)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (hpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (hze : z.regs.get? _root_.Zisk.Register.Zerror = some false)
    (v : BitVec (8 * aW i.d))
    (hrb : _root_.Zisk.Functions.readable (regv xs i.rs1.toNat + 0) (aW i.d) = true)
    (hwr : _root_.Zisk.Functions.writable (regv xs i.rs1.toNat + 0) (aW i.d) = true)
    (hnr : _root_.Zisk.Functions.hits_registers (regv xs i.rs1.toNat + 0) (aW i.d) = false)
    (hin : (regv xs i.rs1.toNat + 0 == _root_.Zisk.Functions.INPUT_ADDR && aW i.d == 8) = false)
    (hm : Equiv.memRead z.mem (aW i.d) (regv xs i.rs1.toNat + 0).toNat = some v)
    (hrom : ∀ p ∈ Transpiler.amo i.encode i.op i.d pc ia, zrom p.1 = p.2) :
    let L := (_root_.Zisk.Functions.zisk_exec (Transpiler.aLoad i.d) (regv xs i.rs1.toNat) (v.setWidth 64)).1
    ∃ z' k, ZRun zrom k z z' ∧
      (∃ xs', z'.regs.get? _root_.Zisk.Register.Xs = some xs' ∧
        ∀ k, k < 32 → regv xs' k = regv (Equiv.Zisk.xsUpd xs i.rd.toNat L) k) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? _root_.Zisk.Register.Step = some st ∧
        z'.regs.get? _root_.Zisk.Register.Zerror = some false) ∧
      z'.mem = Equiv.memWrite (n := aW i.d) z.mem (regv xs i.rs1.toNat + 0).toNat
        ((amoStored i.op i.d L (regv xs i.rs2.toNat)).setWidth (8 * aW i.d)) := by
  intro L
  obtain ⟨hrd, hr1, hr2⟩ := i.regs
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  simp only [Transpiler.amo, hrd, hr1, hr2, aWidth_eq] at hrom
  by_cases hdir : i.rd.toNat ≠ i.rs1.toNat ∧ i.rd.toNat ≠ i.rs2.toNat
  · simp only [if_pos hdir] at hrom
    cases hop : i.op <;> simp only [hop, amoStored] at hrom ⊢
    · simp only [List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil, or_false] at hrom
      obtain ⟨q1, q2⟩ := hrom
      obtain ⟨z', hz, hxs, hp, hs, hmem⟩ := zisk_amo_swap zrom z _ (aLoad_pure i.d) (aW i.d) _ _ _ ld l1 l2
        hdir.1 hdir.2 xs pc st ia hx hpc hst hze v hrb hwr hnr hin hm q1 q2
      exact ⟨z', 2, hz, hxs, hp, hs, hmem⟩
    all_goals
      simp only [List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil, or_false] at hrom
      obtain ⟨q1, q2, q3⟩ := hrom
      obtain ⟨z', hz, hxs, hp, hs, hmem⟩ := zisk_amo_op zrom z _ _ (aLoad_pure i.d) (hop ▸ i.op.pure i.d) (aW i.d)
        _ _ _ ld l1 l2 hdir.1 hdir.2 xs pc st ia hx hpc hst hze v hrb hwr hnr hin hm q1 (hop ▸ q2) q3
      exact ⟨z', 3, hz, ⟨_, hxs, fun _ _ => rfl⟩, hp, hs, hmem⟩
  · simp only [if_neg hdir] at hrom
    cases hop : i.op <;> simp only [hop, amoStored] at hrom ⊢
    · simp only [List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil, or_false] at hrom
      obtain ⟨q1, q2, q3⟩ := hrom
      obtain ⟨z', hz, hxs, hp, hs, hmem⟩ := zisk_amo_swap32 zrom z _ (aLoad_pure i.d) (aW i.d) _ _ _ ld l1 l2
        xs pc st ia hx hpc hst hze v hrb hwr hnr hin hm q1 q2 q3
      exact ⟨z', 3, hz, hxs, hp, hs, hmem⟩
    all_goals
      simp only [List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil, or_false] at hrom
      obtain ⟨q1, q2, q3, q4⟩ := hrom
      obtain ⟨z', hz, hxs, hp, hs, hmem⟩ := zisk_amo_op32 zrom z _ _ (aLoad_pure i.d) (hop ▸ i.op.pure i.d) (aW i.d)
        _ _ _ ld l1 l2 xs pc st ia hx hpc hst hze v hrb hwr hnr hin hm q1 (hop ▸ q2) q3 q4
      exact ⟨z', 4, hz, hxs, hp, hs, hmem⟩

/-! ## The step -/

/-- What an AMO at address a needs: on sail-riscv, a region readable,
    writable and allowing the op, the access aligned or within the granule;
    on ZisK, the bytes in its RAM, off its registers; the bytes there. -/
structure AmoOk (s : RVState) (rs : List PMA_Region) (i : AmoInstr) (a : BitVec 64) : Prop where
  region : ∃ r, matching_pma_region rs (.Physaddr a) (aW i.d) = some r ∧ AmoPerm r.attributes i.op.r (aW i.d) ∧
    AlignOk r (amoAcc i.op.r (bool_bit_backwards i.aq) (bool_bit_backwards i.rl)) a (aW i.d)
  zread : _root_.Zisk.Functions.readable a (aW i.d) = true
  zwrite : _root_.Zisk.Functions.writable a (aW i.d) = true
  zregs : _root_.Zisk.Functions.hits_registers a (aW i.d) = false
  zin : (a == _root_.Zisk.Functions.INPUT_ADDR && aW i.d == 8) = false
  bytes : ∃ v, Equiv.memRead s.mem (aW i.d) a.toNat = some v

/-- Writing a register the value it has, and memory: the memory alone. -/
theorem mem_insert_same (t : RVState) (r : Register) (v : RegisterType r) (hv : t.regs.get? r = some v)
    (M : Equiv.Mem) : ({ t with regs := t.regs.insert r v, mem := M } : RVState) = { t with mem := M } := by
  have h := insert_same t r v hv
  cases t
  simp only [SequentialState.mk.injEq] at h ⊢
  exact ⟨h.1, trivial, trivial, trivial, trivial⟩

theorem AmoOp.r_ne_cas (op : AmoOp) : op.r ≠ .AMOCAS := by cases op <;> simp [Transpiler.AmoOp.r]

theorem amoStored_d (op : AmoOp) (v v2 : BitVec 64) : amoStored op true v v2 = amoVal op.r v v2 := by
  cases op <;> simp only [amoStored, amo_val_d] <;> rfl

theorem amoStored_w (op : AmoOp) (v : BitVec 32) (v2 : BitVec 64) :
    (amoStored op false (v.signExtend 64) v2).setWidth 32 = amoVal op.r v (v2.setWidth 32) := by
  cases op <;> simp only [amoStored, amo_val_w] <;> rfl

set_option maxHeartbeats 16000000 in
/-- One sail-riscv step and ZisK's steps on an AMO.d keep the states related. -/
theorem amostep_rel_d (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (op : AmoOp) (aq rl : BitVec 1) (rd rs1 rs2 : BitVec 5)
    (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom (AmoInstr.mk op true aq rl rd rs1 rs2).encode next) (v1 : BitVec 64) (hv1 : getX s (AmoInstr.mk op true aq rl rd rs1 rs2).rs1.toNat = some v1)
    (hok : AmoOk s rs (AmoInstr.mk op true aq rl rd rs1 rs2) v1) (n : Nat) :
    ∃ s' z' k, (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ ZRun zrom k z z' ∧
      Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ RamOnly s.mem s'.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf5, hf3, haq, hrl, hrdx, hr1x, hr2x, hc⟩ := (AmoInstr.mk op true aq rl rd rs1 rs2).fields
  have hw : WordAt s rs pc' (byte (AmoInstr.mk op true aq rl rd rs1 rs2).encode 0) (byte (AmoInstr.mk op true aq rl rd rs1 rs2).encode 1) (byte (AmoInstr.mk op true aq rl rd rs1 rs2).encode 2) (byte (AmoInstr.mk op true aq rl rd rs1 rs2).encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have ld : (AmoInstr.mk op true aq rl rd rs1 rs2).rd.toNat < 32 := (AmoInstr.mk op true aq rl rd rs1 rs2).rd.isLt
  have l1 : (AmoInstr.mk op true aq rl rd rs1 rs2).rs1.toNat < 32 := (AmoInstr.mk op true aq rl rd rs1 rs2).rs1.isLt
  have l2 : (AmoInstr.mk op true aq rl rd rs1 rs2).rs2.toNat < 32 := (AmoInstr.mk op true aq rl rd rs1 rs2).rs2.isLt
  have g1 := getX_regv agree l1
  have g2 := getX_regv agree l2
  have gd := getX_regv agree ld
  have hxv : regv xs (AmoInstr.mk op true aq rl rd rs1 rs2).rs1.toNat = v1 := by rw [hv1] at g1; exact (Option.some.inj g1).symm
  obtain ⟨r, hm, hperm, hal⟩ := hok.region
  obtain ⟨v, hbytes⟩ := hok.bytes
  have hzr := hok.zread
  have hzw := hok.zwrite
  have hzn := hok.zregs
  have hzi := hok.zin
  simp only [aW, if_true, if_false, Bool.false_eq_true] at hm hperm hal hzr hzw hzn hzi
  obtain ⟨sv, hsv, hpm⟩ := hr.pmm
  -- sail-riscv's step
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ := try_step_amo s rs hr pc' hpc _ _ _ _ hw (AmoInstr.mk op true aq rl rd rs1 rs2).inst
    (by
      intro t htp ⟨w, hw'⟩ _ _ ⟨ma, hma, hmaA⟩
      exact (AmoInstr.mk op true aq rl rd rs1 rs2).decode t _ hop hf5 hf3 haq hrl hrdx hr1x hr2x htp w hw' ma hma hmaA)
    (AmoInstr.mk op true aq rl rd rs1 rs2).rd.toNat v (pc' + 4) (Equiv.memWrite (n := 8) s.mem v1.toNat (amoVal (AmoInstr.mk op true aq rl rd rs1 rs2).op.r v (regv xs (AmoInstr.mk op true aq rl rd rs1 rs2).rs2.toNat)))
    (by
      intro t he htm hmr hsec hwf hnext _
      have hv := Equiv.riscv_readBytes t 8 _ v (htm ▸ hbytes)
      have ex : execute (AmoInstr.mk op true aq rl rd rs1 rs2).inst = execute_AMO (AmoInstr.mk op true aq rl rd rs1 rs2).op.r (bool_bit_backwards (AmoInstr.mk op true aq rl rd rs1 rs2).aq) (bool_bit_backwards (AmoInstr.mk op true aq rl rd rs1 rs2).rl)
          (.Regidx (AmoInstr.mk op true aq rl rd rs1 rs2).rs2) (.Regidx (AmoInstr.mk op true aq rl rd rs1 rs2).rs1) 8 (.Regidx (AmoInstr.mk op true aq rl rd rs1 rs2).rd) := by
        simp [AmoInstr.inst, execute]
      rw [ex, execute_amo8 t hwf rs hmr sv (hsec.trans hsv) hpm (AmoInstr.mk op true aq rl rd rs1 rs2).op.r (AmoOp.r_ne_cas _) _ _ (AmoInstr.mk op true aq rl rd rs1 rs2).rs1 (AmoInstr.mk op true aq rl rd rs1 rs2).rs2 (AmoInstr.mk op true aq rl rd rs1 rs2).rd v1
        (regv xs (AmoInstr.mk op true aq rl rd rs1 rs2).rs2.toNat) (regv xs (AmoInstr.mk op true aq rl rd rs1 rs2).rd.toNat) (getX_of_agree he l1 hv1) (getX_of_agree he l2 g2)
        (getX_of_agree he ld gd) r hm hperm hal v hv, htm]
      rw [mem_insert_same t _ _ hnext]
      rfl) n
  -- ZisK's steps
  obtain ⟨ia, hrom⟩ := (AmoInstr.mk op true aq rl rd rs1 rs2).rom_of_spec hi.tr
  obtain ⟨z', k, hz, ⟨xs', hx', hxs''⟩, hzpc', hst', hzm'⟩ := zisk_amo_any zrom z (AmoInstr.mk op true aq rl rd rs1 rs2) xs pc' st ia hx hzpc hst hze
    v (by rw [hxv]; simpa [aW] using hzr) (by rw [hxv]; simpa [aW] using hzw) (by rw [hxv]; simpa [aW] using hzn)
    (by rw [hxv]; simpa [aW] using hzi) (by rw [hxv, hzm]; simpa using hbytes) hrom
  refine ⟨s', z', k, hrun, hz, ⟨hr.keep hwf ho, ⟨xs', hx', ?_⟩, ⟨_, hpc'', hzpc'⟩, hst', ?_⟩, hpc'',
    hmem ▸ memWrite_ramOnly _ _ _ hzw⟩
  · intro k hk1 hk2
    rw [hxs' k hk2]
    have a := regs_after0 s xs (AmoInstr.mk op true aq rl rd rs1 rs2).rd.toNat v ld agree k hk1 hk2
    have e := hxs'' k hk2
    simp only [regv, show k ≠ 0 by omega, if_false, getElem!_int, hxv] at e
    rw [e]; erw [load_val_d]; exact a
  · rw [hzm', hmem, hzm]
    simp only [hxv, BitVec.add_zero]
    erw [load_val_d, amoStored_d]
    have sw : ∀ y : BitVec 64, y.setWidth (8 * aW true) = y := fun y => by simp [aW]
    rw [sw, show v1 + 0 = v1 from by bv_decide]
    rfl

set_option maxHeartbeats 16000000 in
/-- One sail-riscv step and ZisK's steps on an AMO.w keep the states related. -/
theorem amostep_rel_w (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (op : AmoOp) (aq rl : BitVec 1) (rd rs1 rs2 : BitVec 5)
    (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom (AmoInstr.mk op false aq rl rd rs1 rs2).encode next) (v1 : BitVec 64) (hv1 : getX s (AmoInstr.mk op false aq rl rd rs1 rs2).rs1.toNat = some v1)
    (hok : AmoOk s rs (AmoInstr.mk op false aq rl rd rs1 rs2) v1) (n : Nat) :
    ∃ s' z' k, (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ ZRun zrom k z z' ∧
      Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ RamOnly s.mem s'.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf5, hf3, haq, hrl, hrdx, hr1x, hr2x, hc⟩ := (AmoInstr.mk op false aq rl rd rs1 rs2).fields
  have hw : WordAt s rs pc' (byte (AmoInstr.mk op false aq rl rd rs1 rs2).encode 0) (byte (AmoInstr.mk op false aq rl rd rs1 rs2).encode 1) (byte (AmoInstr.mk op false aq rl rd rs1 rs2).encode 2) (byte (AmoInstr.mk op false aq rl rd rs1 rs2).encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have ld : (AmoInstr.mk op false aq rl rd rs1 rs2).rd.toNat < 32 := (AmoInstr.mk op false aq rl rd rs1 rs2).rd.isLt
  have l1 : (AmoInstr.mk op false aq rl rd rs1 rs2).rs1.toNat < 32 := (AmoInstr.mk op false aq rl rd rs1 rs2).rs1.isLt
  have l2 : (AmoInstr.mk op false aq rl rd rs1 rs2).rs2.toNat < 32 := (AmoInstr.mk op false aq rl rd rs1 rs2).rs2.isLt
  have g1 := getX_regv agree l1
  have g2 := getX_regv agree l2
  have gd := getX_regv agree ld
  have hxv : regv xs (AmoInstr.mk op false aq rl rd rs1 rs2).rs1.toNat = v1 := by rw [hv1] at g1; exact (Option.some.inj g1).symm
  obtain ⟨r, hm, hperm, hal⟩ := hok.region
  obtain ⟨v, hbytes⟩ := hok.bytes
  have hzr := hok.zread
  have hzw := hok.zwrite
  have hzn := hok.zregs
  have hzi := hok.zin
  simp only [aW, if_true, if_false, Bool.false_eq_true] at hm hperm hal hzr hzw hzn hzi
  obtain ⟨sv, hsv, hpm⟩ := hr.pmm
  -- sail-riscv's step
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ := try_step_amo s rs hr pc' hpc _ _ _ _ hw (AmoInstr.mk op false aq rl rd rs1 rs2).inst
    (by
      intro t htp ⟨w, hw'⟩ _ _ ⟨ma, hma, hmaA⟩
      exact (AmoInstr.mk op false aq rl rd rs1 rs2).decode t _ hop hf5 hf3 haq hrl hrdx hr1x hr2x htp w hw' ma hma hmaA)
    (AmoInstr.mk op false aq rl rd rs1 rs2).rd.toNat (v.signExtend 64) (pc' + 4) (Equiv.memWrite (n := 4) s.mem v1.toNat (amoVal (AmoInstr.mk op false aq rl rd rs1 rs2).op.r v ((regv xs (AmoInstr.mk op false aq rl rd rs1 rs2).rs2.toNat).setWidth 32)))
    (by
      intro t he htm hmr hsec hwf hnext _
      have hv := Equiv.riscv_readBytes t 4 _ v (htm ▸ hbytes)
      have ex : execute (AmoInstr.mk op false aq rl rd rs1 rs2).inst = execute_AMO (AmoInstr.mk op false aq rl rd rs1 rs2).op.r (bool_bit_backwards (AmoInstr.mk op false aq rl rd rs1 rs2).aq) (bool_bit_backwards (AmoInstr.mk op false aq rl rd rs1 rs2).rl)
          (.Regidx (AmoInstr.mk op false aq rl rd rs1 rs2).rs2) (.Regidx (AmoInstr.mk op false aq rl rd rs1 rs2).rs1) 4 (.Regidx (AmoInstr.mk op false aq rl rd rs1 rs2).rd) := by
        simp [AmoInstr.inst, execute]
      rw [ex, execute_amo4 t hwf rs hmr sv (hsec.trans hsv) hpm (AmoInstr.mk op false aq rl rd rs1 rs2).op.r (AmoOp.r_ne_cas _) _ _ (AmoInstr.mk op false aq rl rd rs1 rs2).rs1 (AmoInstr.mk op false aq rl rd rs1 rs2).rs2 (AmoInstr.mk op false aq rl rd rs1 rs2).rd v1
        (regv xs (AmoInstr.mk op false aq rl rd rs1 rs2).rs2.toNat) (regv xs (AmoInstr.mk op false aq rl rd rs1 rs2).rd.toNat) (getX_of_agree he l1 hv1) (getX_of_agree he l2 g2)
        (getX_of_agree he ld gd) r hm hperm hal v hv, htm]
      rw [mem_insert_same t _ _ hnext]
      rfl) n
  -- ZisK's steps
  obtain ⟨ia, hrom⟩ := (AmoInstr.mk op false aq rl rd rs1 rs2).rom_of_spec hi.tr
  obtain ⟨z', k, hz, ⟨xs', hx', hxs''⟩, hzpc', hst', hzm'⟩ := zisk_amo_any zrom z (AmoInstr.mk op false aq rl rd rs1 rs2) xs pc' st ia hx hzpc hst hze
    v (by rw [hxv]; simpa [aW] using hzr) (by rw [hxv]; simpa [aW] using hzw) (by rw [hxv]; simpa [aW] using hzn)
    (by rw [hxv]; simpa [aW] using hzi) (by rw [hxv, hzm]; simpa using hbytes) hrom
  refine ⟨s', z', k, hrun, hz, ⟨hr.keep hwf ho, ⟨xs', hx', ?_⟩, ⟨_, hpc'', hzpc'⟩, hst', ?_⟩, hpc'',
    hmem ▸ memWrite_ramOnly _ _ _ hzw⟩
  · intro k hk1 hk2
    rw [hxs' k hk2]
    have a := regs_after0 s xs (AmoInstr.mk op false aq rl rd rs1 rs2).rd.toNat (v.signExtend 64) ld agree k hk1 hk2
    have e := hxs'' k hk2
    simp only [regv, show k ≠ 0 by omega, if_false, getElem!_int, hxv] at e
    rw [e]; erw [load_val_w]; exact a
  · rw [hzm', hmem, hzm]
    simp only [hxv, BitVec.add_zero]
    erw [load_val_w, amoStored_w]
    rw [show v1 + 0 = v1 from by bv_decide]
    rfl

/-- One sail-riscv step on an AMO, and ZisK's steps on its translation,
    keep the states related, when the access is good (`AmoOk`). -/
theorem amostep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : AmoInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hok : AmoOk s rs i v1) (n : Nat) :
    ∃ s' z' k, (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ ZRun zrom k z z' ∧
      Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ RamOnly s.mem s'.mem := by
  obtain ⟨op, d, aq, rl, rd, rs1, rs2⟩ := i
  cases d
  · exact amostep_rel_w s z rs hrel pc hpc zrom op aq rl rd rs1 rs2 next hi v1 hv1 hok n
  · exact amostep_rel_d s z rs hrel pc hpc zrom op aq rl rd rs1 rs2 next hi v1 hv1 hok n

end Equiv.Sim
