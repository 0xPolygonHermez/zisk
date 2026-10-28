/-
  `ebreak` (the one word 0x00100073), a breakpoint, and a write to a
  read-only CSR (`RoInstr`: a CSR numbered 0xC00 and above, any of the six
  CSR instructions that writes it — `unimp`, `csrrw x0, cycle, x0`, among
  them): sail-riscv's step traps (Equiv/RiscvTrap.lean: the breakpoint's
  trap, the illegal instruction's, Equiv/RiscvCsrRo.lean), so the machine's
  run fails; the transpiler's translation of both is the halt
  (`halt_with_error`), which ends sail-zisk's run in error.
-/
import Equiv.FaultSim
import Equiv.ZiskBios
import Equiv.RiscvDecodeCsr
import Equiv.RiscvCsrRo
import Equiv.RiscvDecodeIllegal
import Equiv.RiscvDecodeRej
import Equiv.RiscvDecodeVec
import Equiv.MNext
import Equiv.ZbkbSim

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

set_option maxHeartbeats 4000000 in
/-- The spec's halt (`halt_with_error`): the run's end, in error. -/
theorem execute_halt0 (s : ZState) (st : BitVec 64) (hst : s.regs.get? Step = some st) :
    ∃ s', (zisk_execute Transpiler.halt).run s = .ok true s' ∧ s'.regs.get? Zerror = some true := by
  have hm' := halt_mem
  have hp := halt_notPre
  simp [zisk_execute, Transpiler.halt, Transpiler.mk, source_a, source_b, store_c, value_to_store,
    hm', hp, writeReg_run]
  repeat (first
    | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst]))
  all_goals simp [Std.ExtDHashMap.get?_insert]

end Equiv.Zisk

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- `ebreak`'s word. -/
def EBREAK : BitVec 32 := 0x00100073

set_option maxHeartbeats 4000000 in
/-- `ebreak` in machine mode: a trap, the state as it was. -/
theorem execute_ebreak (t : RVState) (h : WF t) (hp : t.regs.get? Register.cur_privilege = some .Machine) :
    ∃ cp : ExceptionContext × BitVec 64,
      (interpretSailEffects (execute (.EBREAK ()))).run t = .ok (.Trap (.Machine, cp.1, cp.2)) t := by
  simp only [execute, execute_EBREAK, trap, exception_context, inVirtualPrivilege]
  repeat rv_step h
  exact ⟨(_, _), rfl⟩

/-- The machine leaves `ebreak` to sail-riscv. -/
theorem ebreak_plain : Equiv.Machine.Plain EBREAK := by
  intro fetch a hf
  simp only [ZiskRiscv.decode, hf]
  rfl

set_option maxHeartbeats 4000000 in
/-- `ebreak` at pc, one step of each: sail-riscv's to the trap vector;
    sail-zisk's on the halt, the run's end with Zerror set. -/
theorem ebreak_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom EBREAK next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  obtain ⟨hr, -, ⟨pc', hpc', hzpc⟩, ⟨st, hst, -⟩, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw : WordAt s rs pc' (byte EBREAK 0) (byte EBREAK 1) (byte EBREAK 2) (byte EBREAK 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, by decide⟩
  refine ⟨try_step_trap s rs hr pc' hpc _ _ _ _ hw (.EBREAK ()) ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    exact decode_ebreak t _ (by decide) htp w hw'
  · intro t _ _ hmr _ hwf
    exact execute_ebreak t hwf hmr.priv
  · have hrom : zrom pc' = Transpiler.halt := hi.tr.one fun _ => rfl
    rw [zstep_run zrom z pc' hzpc, hrom]
    exact Equiv.Zisk.execute_halt0 z st hst

/-- A CSR instruction writing a read-only CSR (numbered 0xC00 and above):
    `csrrw` and `csrrwi` always write; `csrrs` and `csrrc` when rs1 is not
    x0; `csrrsi` and `csrrci` when their immediate (rs1's field) is not 0. -/
structure RoInstr where
  csr : BitVec 12
  rs1 : BitVec 5
  f3 : BitVec 3
  rd : BitVec 5
  ro : csr.extractLsb 11 10 = 3#2
  writes : f3 = 1 ∨ f3 = 5 ∨ ((f3 = 2 ∨ f3 = 3 ∨ f3 = 6 ∨ f3 = 7) ∧ rs1 ≠ 0)

def RoInstr.encode (i : RoInstr) : BitVec 32 := i.csr ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x73#7

theorem RoInstr.fields_m (i : RoInstr) :
    (ZiskRiscv.fields i.encode).opcode = 0x73 ∧ (ZiskRiscv.fields i.encode).funct3 = i.f3.toNat ∧
    (ZiskRiscv.fields i.encode).csr = i.csr.toNat ∧ (ZiskRiscv.fields i.encode).rs1 = i.rs1.toNat ∧
    (ZiskRiscv.fields i.encode).rd = i.rd.toNat := by
  simp only [ZiskRiscv.fields]
  rw [show i.encode.extractLsb' 0 7 = 0x73#7 by simp only [RoInstr.encode]; bv_decide,
    show i.encode.extractLsb' 12 3 = i.f3 by simp only [RoInstr.encode]; bv_decide,
    show i.encode.extractLsb' 20 12 = i.csr by simp only [RoInstr.encode]; bv_decide,
    show i.encode.extractLsb' 15 5 = i.rs1 by simp only [RoInstr.encode]; bv_decide,
    show i.encode.extractLsb' 7 5 = i.rd by simp only [RoInstr.encode]; bv_decide]
  exact ⟨rfl, rfl, rfl, rfl, rfl⟩

theorem RoInstr.csr_ge (i : RoInstr) : 3072 ≤ i.csr.toNat := by
  have h := i.ro
  have : i.csr ≥ 3072#12 := by revert h; bv_decide
  simpa [BitVec.le_def] using this

theorem onePointer_hi (k : Nat) (h : 3072 ≤ k) : ZiskRiscv.onePointer k = none := by
  unfold ZiskRiscv.onePointer; split <;> first | rfl | omega

theorem twoPointers_hi (k : Nat) (h : 3072 ≤ k) : ZiskRiscv.twoPointers k = none := by
  unfold ZiskRiscv.twoPointers; split <;> first | rfl | omega

/-- The machine leaves it to sail-riscv: no ZisK CSR is read-only. -/
theorem RoInstr.plain (i : RoInstr) : Equiv.Machine.Plain i.encode := by
  intro fetch a hf
  obtain ⟨h1, h2, h3, h4, h5⟩ := i.fields_m
  have hc := i.csr_ge
  have hc' := i.csr.isLt
  simp only [ZiskRiscv.decode, hf, ZiskRiscv.kind, h1, h2]
  have hr' : i.rs1 ≠ 0 → i.rs1.toNat ≠ 0 := fun h e => h (BitVec.eq_of_toNat_eq (by simpa using e))
  rcases i.writes with e | e | ⟨e | e | e | e, hr⟩ <;>
  ( have hf : i.f3.toNat = _ := congrArg BitVec.toNat e
    simp only [hf]
    simp (config := { decide := true }) [ZiskRiscv.decodeCsrrs, ZiskRiscv.decodeCsrrsi,
      ZiskRiscv.decodeCsrrwi, h3, h4, h5])
  all_goals (try (intros; omega))
  all_goals
    have hr1 := hr' hr
    generalize i.csr.toNat = k at *
    repeat' (first
      | (exfalso; omega)
      | (simp only [onePointer_hi k hc, twoPointers_hi k hc])
      | split
      | rfl
      | (simp only [hr1, Bool.false_and, if_false, beq_iff_eq, Bool.and_eq_true, decide_eq_true_eq] at *; done))

/-- The spec's translation: the halt. -/
theorem RoInstr.spec (i : RoInstr) (pc ia : BitVec 64) (next next2 : Option (BitVec 32)) :
    Transpiler.convert pc i.encode next ia next2 = some [(pc, Transpiler.halt)] := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := i.fields_m
  have hc := i.csr_ge
  have hc' := i.csr.isLt
  have hd : ZiskRiscv.decode (Transpiler.fetch2 pc i.encode next) pc.toNat = .riscv :=
    i.plain _ _ (by simp [Transpiler.fetch2])
  have ho : (Transpiler.opcode i.encode).toNat = 115 := by
    have : Transpiler.opcode i.encode = 0x73#7 := by simp only [Transpiler.opcode, RoInstr.encode]; bv_decide
    rw [this]; rfl
  have hne : (i.encode == 0x00000073#32) = false := by
    simp only [beq_eq_false_iff_ne]; intro e
    have : i.csr = 0 := by revert e; simp only [RoInstr.encode]; bv_decide
    rw [this] at hc; simp at hc
  have hne' : (i.encode == 0x00100073#32) = false := by
    simp only [beq_eq_false_iff_ne]; intro e
    have : i.csr = 1 := by revert e; simp only [RoInstr.encode]; bv_decide
    rw [this] at hc; simp at hc
  have hf3 : ((Transpiler.funct3 i.encode).toNat == 0) = false := by
    have : Transpiler.funct3 i.encode = i.f3 := by simp only [Transpiler.funct3, RoInstr.encode]; bv_decide
    rw [this]; rcases i.writes with e | e | ⟨e | e | e | e, -⟩ <;> rw [e] <;> rfl
  have hk : ZiskRiscv.kind (ZiskRiscv.fields i.encode) ≠ .csrrsi ∨ (ZiskRiscv.fields i.encode).csr ≠ 0x816 :=
    .inr (by omega)
  have hw : Transpiler.writesReadOnly (ZiskRiscv.fields i.encode) = true := by
    have hr' : i.rs1 ≠ 0 → i.rs1.toNat ≠ 0 := fun h e => h (BitVec.eq_of_toNat_eq (by simpa using e))
    have hq : i.csr.toNat / 1024 % 4 = 3 := by omega
    simp only [Transpiler.writesReadOnly, ZiskRiscv.kind, h1, h2, h3, h4, hq]
    rcases i.writes with e | e | ⟨e | e | e | e, hr⟩ <;>
    ( have hf : i.f3.toNat = _ := congrArg BitVec.toNat e
      simp only [hf]
      first
        | simp (config := { decide := true }) [hr' ‹i.rs1 ≠ 0›]
        | simp (config := { decide := true }) )
  have hrs : (ZiskRiscv.kind (ZiskRiscv.fields i.encode) == .csrrs) = true → (ZiskRiscv.fields i.encode).rs1 ≠ 0 := by
    intro hk1; revert hw
    cases hk' : ZiskRiscv.kind (ZiskRiscv.fields i.encode) <;> rw [hk'] at hk1 <;> (try exact absurd hk1 (by decide))
    simp only [Transpiler.writesReadOnly, hk']; simp; exact fun h _ => h
  cases next2 <;>
    simp [Transpiler.convert, Transpiler.decode, ho, hne, hne', hf3, Transpiler.csr3, Transpiler.csr, hd, hw, h3,
      show i.csr.toNat ≠ 2070 by omega]
  all_goals (intro h1' _ h3' _; exact absurd h3' (hrs h1'))

theorem RoInstr.fields (i : RoInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x73#7 ∧ v.extractLsb 14 12 = i.f3 ∧ v.extractLsb 31 20 = i.csr ∧
    v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 11 7 = i.rd ∧ (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, RoInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem RoInstr.acc (i : RoInstr) : csrAccess i.csr = 3#2 := by
  have h := i.ro; simp only [csrAccess]; exact h

theorem regidx_ne (x : BitVec 5) (h : x ≠ 0) : (regidx.Regidx x == zreg) = false := by
  change (x == zero_extend (m := 5) 0#2) = false
  simpa [zero_extend, Sail.BitVec.zeroExtend] using h

set_option maxHeartbeats 4000000 in
/-- A word the spec halts on, at pc, given sail-riscv's decoding of it to
    an instruction its execution finds illegal: one step of each, sail-riscv's
    to the trap vector, sail-zisk's on the halt. -/
theorem illegal_step_of (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (e : BitVec 32) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom e next)
    (hspec : ∀ pc ia next next2, Transpiler.convert pc e next ia next2 = some [(pc, Transpiler.halt)])
    (hc : (byte e 0).extractLsb 1 0 = 3#2) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? Register.cur_privilege = some .Machine →
      (∃ v, t.regs.get? Register.mseccfg = some v) →
      (∃ v, t.regs.get? Register.misa = some v ∧ _get_Misa_B v = 1#1) →
      (∃ v, t.regs.get? Register.misa = some v ∧ _get_Misa_A v = 1#1) →
      (∃ v, t.regs.get? Register.mstatus = some v) →
      (∃ v, t.regs.get? Register.vtype = some v ∧ VtypeOk v) →
      (∃ v, t.regs.get? Register.vl = some v) → (∃ v, t.regs.get? Register.vstart = some v) →
      (interpretSailEffects (ext_decode (byte e 3 ++ byte e 2 ++ (byte e 1 ++ byte e 0)))).run t
        = .ok ins t)
    (hexec : ∀ t : RVState, WF t → t.regs.get? Register.cur_privilege = some .Machine →
      (interpretSailEffects (execute ins)).run t = .ok (.Illegal_Instruction ()) t) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  obtain ⟨hr, -, ⟨pc', hpc', hzpc⟩, ⟨st, hst, -⟩, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw : WordAt s rs pc' (byte e 0) (byte e 1) (byte e 2) (byte e 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  refine ⟨try_step_illegal s rs hr hr.mtvec pc' hpc _ _ _ _ hw ins ?_ ?_ n, ?_⟩
  · intro t htp hsec _ hb ha hms hvt hvl hvs
    exact hdec t htp hsec hb ha hms hvt hvl hvs
  · intro t _ _ hmr _ hwf
    exact hexec t hwf hmr.priv
  · have hrom : zrom pc' = Transpiler.halt := hi.tr.one fun ia => hspec pc' ia _ _
    rw [zstep_run zrom z pc' hzpc, hrom]
    exact Equiv.Zisk.execute_halt0 z st hst

set_option maxHeartbeats 4000000 in
/-- A write to a read-only CSR at pc, one step of each: sail-riscv's to the
    trap vector (an illegal instruction); sail-zisk's on the halt, the run's
    end with Zerror set. -/
theorem ro_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RoInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  obtain ⟨hop, hf3, hcsr, hr1, hrd, -⟩ := i.fields
  have ha := i.acc
  rcases i.writes with e | e | ⟨e | e | e | e, hr⟩
  · refine illegal_step_of s z rs hrel pc hpc zrom _ next hi (i.spec) i.fields.2.2.2.2.2 (.CSRReg (i.csr, .Regidx i.rs1, .Regidx i.rd, .CSRRW))
      (fun t htp ⟨w, hw'⟩ _ _ _ _ _ _ => ?_) (fun t hwf hp => execute_csrreg_ro t hwf hp _ _ _ _ ?_) n
    · have := decode_csrrw t _ hop (by rw [hf3, e]; rfl) htp w hw'; rwa [hcsr, hr1, hrd] at this
    · cases (regidx.Regidx i.rd == zreg) <;> simp (config := { decide := true }) [check_CSR_access, csr_access_type, LeanZiskRV.not, ha]
  · refine illegal_step_of s z rs hrel pc hpc zrom _ next hi (i.spec) i.fields.2.2.2.2.2 (.CSRImm (i.csr, i.rs1, .Regidx i.rd, .CSRRW))
      (fun t htp ⟨w, hw'⟩ _ _ _ _ _ _ => ?_) (fun t hwf hp => execute_csrimm_ro t hwf hp _ _ _ _ ?_) n
    · have := decode_csrrwi t _ hop (by rw [hf3, e]; rfl) htp w hw'; rwa [hcsr, hr1, hrd] at this
    · cases (regidx.Regidx i.rd == zreg) <;> simp (config := { decide := true }) [check_CSR_access, csr_access_type, LeanZiskRV.not, ha]
  · refine illegal_step_of s z rs hrel pc hpc zrom _ next hi (i.spec) i.fields.2.2.2.2.2 (.CSRReg (i.csr, .Regidx i.rs1, .Regidx i.rd, .CSRRS))
      (fun t htp ⟨w, hw'⟩ _ _ _ _ _ _ => ?_) (fun t hwf hp => execute_csrreg_ro t hwf hp _ _ _ _ ?_) n
    · have := decode_csrrs t _ hop (by rw [hf3, e]; rfl) htp w hw'; rwa [hcsr, hr1, hrd] at this
    · simp (config := { decide := true }) [check_CSR_access, csr_access_type, LeanZiskRV.not, ha, regidx_ne _ hr]
  · refine illegal_step_of s z rs hrel pc hpc zrom _ next hi (i.spec) i.fields.2.2.2.2.2 (.CSRReg (i.csr, .Regidx i.rs1, .Regidx i.rd, .CSRRC))
      (fun t htp ⟨w, hw'⟩ _ _ _ _ _ _ => ?_) (fun t hwf hp => execute_csrreg_ro t hwf hp _ _ _ _ ?_) n
    · have := decode_csrrc t _ hop (by rw [hf3, e]; rfl) htp w hw'; rwa [hcsr, hr1, hrd] at this
    · simp (config := { decide := true }) [check_CSR_access, csr_access_type, LeanZiskRV.not, ha, regidx_ne _ hr]
  · refine illegal_step_of s z rs hrel pc hpc zrom _ next hi (i.spec) i.fields.2.2.2.2.2 (.CSRImm (i.csr, i.rs1, .Regidx i.rd, .CSRRS))
      (fun t htp ⟨w, hw'⟩ _ _ _ _ _ _ => ?_) (fun t hwf hp => execute_csrimm_ro t hwf hp _ _ _ _ ?_) n
    · have := decode_csrrsi t _ hop (by rw [hf3, e]; rfl) htp w hw'; rwa [hcsr, hr1, hrd] at this
    · rw [show (i.rs1 == zeros (n := 5)) = false by simpa [zeros] using hr]
      cases (regidx.Regidx i.rd == zreg) <;>
        simp (config := { decide := true }) [check_CSR_access, csr_access_type, LeanZiskRV.not, ha]
  · refine illegal_step_of s z rs hrel pc hpc zrom _ next hi (i.spec) i.fields.2.2.2.2.2 (.CSRImm (i.csr, i.rs1, .Regidx i.rd, .CSRRC))
      (fun t htp ⟨w, hw'⟩ _ _ _ _ _ _ => ?_) (fun t hwf hp => execute_csrimm_ro t hwf hp _ _ _ _ ?_) n
    · have := decode_csrrci t _ hop (by rw [hf3, e]; rfl) htp w hw'; rwa [hcsr, hr1, hrd] at this
    · rw [show (i.rs1 == zeros (n := 5)) = false by simpa [zeros] using hr]
      cases (regidx.Regidx i.rd == zreg) <;>
        simp (config := { decide := true }) [check_CSR_access, csr_access_type, LeanZiskRV.not, ha]

/-- `unimp`'s word: `csrrw x0, cycle, x0`. -/
def UNIMP : BitVec 32 := 0xC0001073

/-- `unimp`, a write to a read-only CSR. -/
def RoInstr.unimp : RoInstr := ⟨0xC00, 0, 1, 0, rfl, .inl rfl⟩

theorem RoInstr.unimp_encode : RoInstr.unimp.encode = UNIMP := rfl

/-- A word of a major opcode no clause of RISC-V's decoder has: the custom
    opcodes, the reserved one, the longer encodings'. RISC-V rejects it; so
    does the transpiler's decoder. -/
def RejWord (e : BitVec 32) : Prop :=
  e.extractLsb 6 0 = 0x0B#7 ∨ e.extractLsb 6 0 = 0x1F#7 ∨ e.extractLsb 6 0 = 0x2B#7 ∨
  e.extractLsb 6 0 = 0x3F#7 ∨ e.extractLsb 6 0 = 0x5B#7 ∨ e.extractLsb 6 0 = 0x5F#7 ∨
  e.extractLsb 6 0 = 0x6B#7 ∨ e.extractLsb 6 0 = 0x7B#7 ∨ e.extractLsb 6 0 = 0x7F#7

theorem RejWord.low {e : BitVec 32} (h : RejWord e) : (byte e 0).extractLsb 1 0 = 3#2 := by
  simp only [RejWord] at h; simp only [byte]; revert h; bv_decide

/-- The machine leaves it to sail-riscv: no call, no lr or sc. -/
theorem RejWord.plain {e : BitVec 32} (h : RejWord e) : Equiv.Machine.Plain e := by
  intro fetch a hf
  have ho : (ZiskRiscv.fields e).opcode = (e.extractLsb 6 0).toNat := by
    simp only [ZiskRiscv.fields]; congr 1
  simp only [RejWord] at h
  simp only [ZiskRiscv.decode, hf]
  rcases h with h | h | h | h | h | h | h | h | h <;>
    (simp only [ZiskRiscv.kind, ho, h]; rfl)

/-- The spec's translation: the halt. -/
theorem RejWord.spec {e : BitVec 32} (h : RejWord e) (pc ia : BitVec 64) (next next2 : Option (BitVec 32)) :
    Transpiler.convert pc e next ia next2 = some [(pc, Transpiler.halt)] := by
  have hd : Transpiler.decode e = none := by
    simp only [RejWord] at h
    have ho : Transpiler.opcode e = e.extractLsb 6 0 := rfl
    simp only [Transpiler.decode, ho]
    rcases h with h | h | h | h | h | h | h | h | h <;> (rw [h]; rfl)
  simp only [Transpiler.convert, hd]

theorem RejWord.word (e : BitVec 32) : (byte e 3 ++ byte e 2 ++ (byte e 1 ++ byte e 0) : BitVec 32) = e := by
  unfold byte; bv_decide

set_option maxHeartbeats 4000000 in
/-- A word RISC-V rejects at pc, one step of each: sail-riscv's to the trap
    vector (an illegal instruction); sail-zisk's on the halt, the run's end
    with Zerror set. -/
theorem rej_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (e : BitVec 32) (he : RejWord e) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom e next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  refine illegal_step_of s z rs hrel pc hpc zrom e next hi he.spec he.low (.ILLEGAL e)
    (fun t htp ⟨v, hv⟩ _ _ _ _ _ _ => ?_) (fun t _ _ => rfl) n
  rw [RejWord.word e]
  exact decode_rej_opcode t e (by simpa only [RejWord] using he) htp v hv

/-- A word RISC-V rejects within a major opcode the profile uses (but
    SYSTEM): one the spec decodes to nothing. The float opcodes are not
    among them. -/
def RejField (e : BitVec 32) : Prop := Transpiler.decode e = none ∧ RejOpcode e

theorem RejField.low {e : BitVec 32} (h : RejField e) : (byte e 0).extractLsb 1 0 = 3#2 := by
  obtain ⟨-, h⟩ := h
  simp only [RejOpcode, Transpiler.opcode, BitVec.toNat_eq_nat, Nat.reducePow, Nat.reduceLT, true_and] at h
  simp only [byte]; revert h; bv_decide

/-- The spec's translation: the halt. -/
theorem RejField.spec {e : BitVec 32} (h : RejField e) (pc ia : BitVec 64) (next next2 : Option (BitVec 32)) :
    Transpiler.convert pc e next ia next2 = some [(pc, Transpiler.halt)] := by
  simp only [Transpiler.convert, h.1]

/-- No instruction the machine tells apart: not SYSTEM, and no lr or sc
    (those the spec decodes). -/
theorem RejField.kind {e : BitVec 32} (h : RejField e) :
    ZiskRiscv.kind (ZiskRiscv.fields e) = .other ∨ ZiskRiscv.kind (ZiskRiscv.fields e) = .add ∨
      ZiskRiscv.kind (ZiskRiscv.fields e) = .addi := by
  obtain ⟨hd, ho⟩ := h
  have h47 := Transpiler.rej_47 hd
  simp only [RejOpcode, Transpiler.opcode, BitVec.extractLsb, Nat.reduceAdd, Nat.reduceSub] at ho h47
  unfold ZiskRiscv.kind
  simp only [ZiskRiscv.fields]
  split
  all_goals (try split)
  all_goals (try split)
  all_goals (first | (simp; done) | (exfalso; omega) | skip)
  all_goals (exfalso; specialize h47 (by omega))
  all_goals (simp only [← BitVec.toNat_ushiftRight, BitVec.toNat_eq_nat, Bool.and_eq_true, beq_iff_eq, Nat.reducePow,
    Nat.reduceLT, true_and] at *; bv_decide)

/-- The machine leaves it to sail-riscv: no call, no lr or sc. -/
theorem RejField.plain {e : BitVec 32} (h : RejField e) : Equiv.Machine.Plain e := by
  intro fetch a hf
  simp only [ZiskRiscv.decode, hf]
  rcases h.kind with hk | hk | hk <;> simp only [hk]

set_option maxHeartbeats 4000000 in
/-- A word RISC-V rejects within its opcode at pc, one step of each:
    sail-riscv's to the trap vector (an illegal instruction); sail-zisk's on
    the halt, the run's end with Zerror set. -/
theorem rejf_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (e : BitVec 32) (he : RejField e) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom e next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  refine illegal_step_of s z rs hrel pc hpc zrom e next hi he.spec he.low (.ILLEGAL e)
    (fun t htp ⟨v, hv⟩ ⟨b, hb, hbB⟩ ⟨a, ha, haA⟩ _ _ _ _ => ?_) (fun t _ _ => rfl) n
  rw [hb] at ha
  cases ha
  rw [RejWord.word e]
  exact decode_rej_field t e he.1 he.2 htp v hv b hb haA hbB

/-- A word of a vector opcode (OP-V 0x57, OP-VE 0x77): the profile has no
    vector extension. -/
def VecWord (e : BitVec 32) : Prop := e.extractLsb 6 0 = 0x57#7 ∨ e.extractLsb 6 0 = 0x77#7

theorem VecWord.low {e : BitVec 32} (h : VecWord e) : (byte e 0).extractLsb 1 0 = 3#2 := by
  simp only [VecWord] at h; simp only [byte]; revert h; bv_decide

/-- The machine leaves it to sail-riscv: no call, no lr or sc. -/
theorem VecWord.plain {e : BitVec 32} (h : VecWord e) : Equiv.Machine.Plain e := by
  intro fetch a hf
  have ho : (ZiskRiscv.fields e).opcode = (e.extractLsb 6 0).toNat := by
    simp only [ZiskRiscv.fields]; congr 1
  simp only [VecWord] at h
  simp only [ZiskRiscv.decode, hf]
  rcases h with h | h <;> (simp only [ZiskRiscv.kind, ho, h]; rfl)

/-- The spec's translation: the halt. -/
theorem VecWord.spec {e : BitVec 32} (h : VecWord e) (pc ia : BitVec 64) (next next2 : Option (BitVec 32)) :
    Transpiler.convert pc e next ia next2 = some [(pc, Transpiler.halt)] := by
  have hd : Transpiler.decode e = none := by
    simp only [VecWord] at h
    have ho : Transpiler.opcode e = e.extractLsb 6 0 := rfl
    simp only [Transpiler.decode, ho]
    rcases h with h | h <;> (rw [h]; rfl)
  simp only [Transpiler.convert, hd]

set_option maxHeartbeats 4000000 in
/-- A vector word at pc, one step of each: sail-riscv's to the trap vector
    (an illegal instruction); sail-zisk's on the halt, the run's end with
    Zerror set. -/
theorem vec_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (e : BitVec 32) (he : VecWord e) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom e next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  refine illegal_step_of s z rs hrel pc hpc zrom e next hi he.spec he.low (.ILLEGAL e)
    (fun t htp ⟨v, hv⟩ ⟨b, hb, hbB⟩ ⟨a, ha, haA⟩ ⟨ms, hms⟩ ⟨vt, hvt, hok⟩ ⟨l, hl⟩ ⟨st, hst⟩ => ?_)
    (fun t _ _ => rfl) n
  rw [hb] at ha
  cases ha
  rw [RejWord.word e]
  exact decode_vec t e (by simpa only [VecWord] using he) htp v hv b hb haA hbB ms hms vt hvt hok l hl st hst

/-- A word the transpiler halts on, and the simulation covers: `ebreak`, a
    write to a read-only CSR, or a word RISC-V rejects for its opcode or
    within it, a vector word among them. -/
def HaltWord (e : BitVec 32) : Prop :=
  e = EBREAK ∨ (∃ i : RoInstr, i.encode = e) ∨ RejWord e ∨ RejField e ∨ VecWord e

theorem HaltWord.plain {e : BitVec 32} (h : HaltWord e) : Equiv.Machine.Plain e := by
  rcases h with rfl | ⟨i, rfl⟩ | h | h | h
  · exact ebreak_plain
  · exact i.plain
  · exact h.plain
  · exact h.plain
  · exact h.plain

theorem halt_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (next : Option (BitVec 32)) (e : BitVec 32) (he : HaltWord e)
    (hi : WordsAt s rs pc zrom e next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  rcases he with rfl | ⟨i, rfl⟩ | he | he | he
  · exact ebreak_step s z rs hrel pc hpc zrom next hi n
  · exact ro_step s z rs hrel pc hpc zrom i next hi n
  · exact rej_step s z rs hrel pc hpc zrom e he next hi n
  · exact rejf_step s z rs hrel pc hpc zrom e he next hi n
  · exact vec_step s z rs hrel pc hpc zrom e he next hi n

end Equiv.Sim

