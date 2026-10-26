/-
  The simulation on `lr` and `sc` (contract § 1: a plain load, an
  unconditional store and 0 to rd), as ZisK's RISC-V machine runs them: the
  plain load or store's word put at pc (`lw`/`ld rd, 0(rs1)`, `sw`/`sd rs2,
  0(rs1)`), sail-riscv's step on that, the original word put back; for sc,
  rd then 0. ZisK runs the transpiler's translation on the memory as it
  is; the access misses pc's word (precondition L for lr; sc writes RAM),
  so both read and write the same bytes.
-/
import Equiv.AmoSim
import Equiv.MNext

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)

theorem _root_.Equiv.Riscv.StepReady.with_mem {s : RVState} {rs : List PMA_Region} (hr : StepReady s rs) (m : Mem) :
    StepReady { s with mem := m } rs :=
  ⟨⟨hr.wf.all⟩, hr.mip, hr.mie, hr.hart, hr.elp,
    ⟨hr.mem.priv, hr.mem.mstatus, hr.mem.misa, hr.mem.senvcfg, hr.mem.menvcfg, hr.mem.htif, hr.mem.pma⟩,
    hr.pmm, hr.misaM, hr.misaB, hr.misaA, hr.march, hr.mtvec⟩

/-- The relation with both memories replaced by the same one. -/
theorem Rel.with_mem {s : RVState} {z : Equiv.Zisk.ZState} {rs : List PMA_Region} (h : Rel s z rs) (m : Mem) :
    Rel { s with mem := m } { z with mem := m } rs := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc, hp, hzp⟩, hst, -⟩ := h
  exact ⟨hr.with_mem m, ⟨xs, hx, fun k h1 h2 => agree k h1 h2⟩, ⟨pc, hp, hzp⟩, hst, rfl⟩

/-- The bytes of the word written at p, read back. -/
theorem writeWord_bytes (m : Mem) (p : Nat) (w : BitVec 32) :
    (ZiskRiscv.writeWord m p w).get? p = some (byte w 0) ∧ (ZiskRiscv.writeWord m p w).get? (p + 1) = some (byte w 1) ∧
    (ZiskRiscv.writeWord m p w).get? (p + 2) = some (byte w 2) ∧
    (ZiskRiscv.writeWord m p w).get? (p + 3) = some (byte w 3) := by
  simp only [writeWord_get]
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [byte] <;> bv_decide

theorem bytes_word' (e : BitVec 32) :
    @Eq (BitVec 32) (byte e 3 ++ byte e 2 ++ byte e 1 ++ byte e 0) e := by
  simp only [byte]; bv_decide

theorem readWord_of' {m : Mem} {a : Nat} {e : BitVec 32}
    (h0 : m.get? a = some (byte e 0)) (h1 : m.get? (a + 1) = some (byte e 1))
    (h2 : m.get? (a + 2) = some (byte e 2)) (h3 : m.get? (a + 3) = some (byte e 3)) :
    ZiskRiscv.readWord m a = some e := by
  simp only [ZiskRiscv.readWord, h0, h1, h2, h3, Option.pure_def, Option.bind_eq_bind, Option.bind_some]
  rw [bytes_word']

/-- A read of n bytes at a, off p's word: the same after a word written at p. -/
theorem memRead_writeWord (m : Mem) (p : Nat) (w : BitVec 32) (n a : Nat) (hoff : a + n ≤ p ∨ p + 4 ≤ a) :
    memRead (ZiskRiscv.writeWord m p w) n a = memRead m n a :=
  memRead_frame m _ n a (fun k h1 h2 => by
    rw [writeWord_get]
    simp [show ¬ k = p by omega, show ¬ k = p + 1 by omega, show ¬ k = p + 2 by omega, show ¬ k = p + 3 by omega])

/-! ## lr -/

structure LrInstr where
  d : Bool
  aq : BitVec 1
  rl : BitVec 1
  rd : BitVec 5
  rs1 : BitVec 5

def LrInstr.f3 (i : LrInstr) : BitVec 3 := if i.d then 3 else 2

def LrInstr.encode (i : LrInstr) : BitVec 32 :=
  2#5 ++ i.aq ++ i.rl ++ 0#5 ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x2F#7

/-- The plain load the machine runs in its place. -/
def LrInstr.sub (i : LrInstr) : BitVec 32 := 0#12 ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x03#7

/-- An lr's access: the plain load's, and, precondition L, its bytes off
    the lr's own word. -/
structure LrOk (s : RVState) (rs : List PMA_Region) (i : LrInstr) (pc a : BitVec 64) : Prop where
  load : ∃ v, LwOk s rs a (if i.d then 8 else 4) v
  zin : (a == _root_.Zisk.Functions.INPUT_ADDR && (if i.d then 8 else 4) == 8) = false
  off : a.toNat + (if i.d then 8 else 4) ≤ pc.toNat ∨ pc.toNat + 4 ≤ a.toNat

theorem LrInstr.spec (i : LrInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IND, 0) (Transpiler.aLoad i.d)
      (Transpiler.storeReg i.rd.toNat) 4 4 (w := Transpiler.aWidth i.d) := by
  have ho : Transpiler.opcode i.encode = 0x2F#7 := by simp only [Transpiler.opcode, LrInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = i.f3 := by simp only [Transpiler.funct3, LrInstr.encode]; bv_decide
  have hf5 : i.encode.extractLsb 31 27 = 2#5 := by simp only [LrInstr.encode]; bv_decide
  have hr2 : Transpiler.rs2 i.encode = 0 := by
    simp only [Transpiler.rs2, LrInstr.encode]
    rw [show (2#5 ++ i.aq ++ i.rl ++ 0#5 ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x2F#7).extractLsb 24 20 = 0#5 by bv_decide]; rfl
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, LrInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, LrInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  obtain ⟨d, aq, rl, rd, rs1⟩ := i
  simp only [LrInstr.f3] at hf3 ⊢
  cases d <;> simp only [if_true, if_false, Bool.false_eq_true] at hf3 <;>
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf5, hr2, hrd, hr1]

set_option maxHeartbeats 16000000 in
/-- lr.d: sail-riscv's `ld rd, 0(rs1)` at pc in its place, the word put
    back; ZisK's load. -/
theorem lrdstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (aq rl : BitVec 1) (rd rs1 : BitVec 5) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom (LrInstr.mk true aq rl rd rs1).encode next) (v1 : BitVec 64) (hv1 : getX s rs1.toNat = some v1)
    (hok : LrOk s rs (LrInstr.mk true aq rl rd rs1) pc v1) (n : Nat) :
    ∃ s1' z', (interpretSailEffects (try_step n false)).run
        { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (LrInstr.mk true aq rl rd rs1).sub } = .ok false s1' ∧
      (zstep zrom).run z = .ok false z' ∧
      Rel { s1' with mem := ZiskRiscv.writeWord s1'.mem pc.toNat (LrInstr.mk true aq rl rd rs1).encode } z' rs ∧
      s1'.regs.get? Register.PC = some (pc + 4) ∧
      s1'.mem = ZiskRiscv.writeWord s.mem pc.toNat (LrInstr.mk true aq rl rd rs1).sub := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw0 : ZiskRiscv.readWord s.mem pc'.toNat = some (LrInstr.mk true aq rl rd rs1).encode := readWord_of' hi.h0 hi.h1 hi.h2 hi.h3
  let L : LdInstr := ⟨rd, rs1, 0⟩
  have hLs : L.encode = (LrInstr.mk true aq rl rd rs1).sub := by simp [L, LdInstr.encode, LrInstr.sub, LrInstr.f3]
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := L.fields
  obtain ⟨h14, h13⟩ := ld_f3 _ hf3
  obtain ⟨b0, b1, b2, b3⟩ := writeWord_bytes s.mem pc'.toNat L.encode
  rw [hLs] at b0 b1 b2 b3
  have hw : WordAt { s with mem := ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk true aq rl rd rs1).sub } rs pc'
      (byte L.encode 0) (byte L.encode 1) (byte L.encode 2) (byte L.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, by rw [hLs]; exact b0, by rw [hLs]; exact b1, by rw [hLs]; exact b2, by rw [hLs]; exact b3, hc⟩
  have l1 : rs1.toNat < 32 := rs1.isLt
  have ld : rd.toNat < 32 := rd.isLt
  have hr1 : regv xs rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  obtain ⟨⟨v, ⟨r, hm, hrb, hal⟩, hzr, hzn, hin, hbytes⟩, hzi, hoff⟩ := hok
  simp only [if_true] at hm hal hzr hzn hin hbytes hzi hoff
  have hbytes1 : Equiv.memRead (ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk true aq rl rd rs1).sub) 8 v1.toNat = some v := by
    rw [memRead_writeWord _ _ _ _ _ (by omega)]; exact hbytes
  have e0 : v1 + sign_extend (m := 64) (0 : BitVec 12) = v1 := by simp [sign_extend, Sail.BitVec.signExtend]
  obtain ⟨s1', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_gen { s with mem := ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk true aq rl rd rs1).sub } rs (hr.with_mem _) pc' hpc _ _ _ _ hw
    (.LOAD (0, .Regidx rs1, .Regidx rd, false, 8))
    (by
      intro t htp ⟨w, hw'⟩ hmt _
      have := decode_load0 t _ hop (by rw [h13, h14]; decide) htp w hw'
      rw [himm, hr1x, hrdx, h13, h14] at this; exact this)
    rd.toNat v (pc' + 4)
    (by
      intro t he htm hmr hsec hwf hnext _
      rw [execute_load_eq, insert_same t _ _ hnext]
      obtain ⟨sv, hsv, hpm⟩ := hr.pmm
      have hv := Equiv.riscv_readBytes t 8 (v1 + sign_extend (m := 64) (0 : BitVec 12)).toNat v
        (by rw [htm, e0]; exact hbytes1)
      have := execute_ld t hwf rs hmr sv (hsec.trans hsv) hpm rs1 rd 0 v1 (getX_of_agree he l1 hv1)
        r (by rw [e0]; exact hm) (by exact hrb) (by rw [e0]; exact hal) v (by rw [e0] at hv ⊢; exact hv)
      exact this) n
  -- ZisK: the load
  have hrom := (LrInstr.mk true aq rl rd rs1).spec hi.tr
  simp only [Transpiler.aLoad, Transpiler.aWidth, if_true] at hrom
  have ha := Equiv.Zisk.srcA_ASrc xs _ l1
  rw [hr1] at ha
  have hex := Equiv.Zisk.execute_load_mk z _ (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure rd.toNat ld 0 8
    xs pc' st hx hzpc hst v1 ha v (by simpa using hzr) (by simpa using hzn) (by simpa using hzi)
    (by rw [show v1 + 0 = v1 from by bv_decide, Equiv.memRead_normIn hzm hin]; exact hbytes)
  have hmw : ZiskRiscv.writeWord s1'.mem pc'.toNat (LrInstr.mk true aq rl rd rs1).encode = s.mem := by
    rw [hmem, writeWord_twice, writeWord_same _ _ _ hw0]
  refine ⟨s1', _, hrun, by rw [zstep_run zrom z pc' hzpc, hrom]; exact hex, ?_, hpc'', hmem⟩
  rw [hmw]
  refine ⟨(hr.keep hwf ho).with_mem _, ⟨Equiv.Zisk.xsUpd xs rd.toNat v, ?_, fun k hk1 hk2 => ?_⟩, ⟨_, hpc'', by simp⟩,
    ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, ?_⟩
  · simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Equiv.Zisk.copyb_val]
  · show getX s1' k = _
    rw [hxs' k hk2]
    exact regs_after0 { s with mem := ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk true aq rl rd rs1).sub } xs rd.toNat v ld
      (fun k h1 h2 => agree k h1 h2) k hk1 hk2
  · exact hzm

set_option maxHeartbeats 16000000 in
/-- lr.w: sail-riscv's `lw rd, 0(rs1)` at pc in its place, the word put
    back; ZisK's load. -/
theorem lrwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (aq rl : BitVec 1) (rd rs1 : BitVec 5) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom (LrInstr.mk false aq rl rd rs1).encode next) (v1 : BitVec 64) (hv1 : getX s rs1.toNat = some v1)
    (hok : LrOk s rs (LrInstr.mk false aq rl rd rs1) pc v1) (n : Nat) :
    ∃ s1' z', (interpretSailEffects (try_step n false)).run
        { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (LrInstr.mk false aq rl rd rs1).sub } = .ok false s1' ∧
      (zstep zrom).run z = .ok false z' ∧
      Rel { s1' with mem := ZiskRiscv.writeWord s1'.mem pc.toNat (LrInstr.mk false aq rl rd rs1).encode } z' rs ∧
      s1'.regs.get? Register.PC = some (pc + 4) ∧
      s1'.mem = ZiskRiscv.writeWord s.mem pc.toNat (LrInstr.mk false aq rl rd rs1).sub := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw0 : ZiskRiscv.readWord s.mem pc'.toNat = some (LrInstr.mk false aq rl rd rs1).encode := readWord_of' hi.h0 hi.h1 hi.h2 hi.h3
  let L : LwInstr := ⟨.lw, rd, rs1, 0⟩
  have hLs : L.encode = (LrInstr.mk false aq rl rd rs1).sub := by simp [L, LwInstr.encode, LrInstr.sub, LrInstr.f3, LKind.f3]
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := L.fields
  obtain ⟨hu, hwd⟩ := L.k.dec _ hf3
  have hvalid := L.k.valid
  rw [← hu, ← hwd] at hvalid
  obtain ⟨b0, b1, b2, b3⟩ := writeWord_bytes s.mem pc'.toNat L.encode
  rw [hLs] at b0 b1 b2 b3
  have hw : WordAt { s with mem := ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk false aq rl rd rs1).sub } rs pc'
      (byte L.encode 0) (byte L.encode 1) (byte L.encode 2) (byte L.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, by rw [hLs]; exact b0, by rw [hLs]; exact b1, by rw [hLs]; exact b2, by rw [hLs]; exact b3, hc⟩
  have l1 : rs1.toNat < 32 := rs1.isLt
  have ld : rd.toNat < 32 := rd.isLt
  have hr1 : regv xs rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  obtain ⟨⟨v, ⟨r, hm, hrb, hal⟩, hzr, hzn, hin, hbytes⟩, hzi, hoff⟩ := hok
  simp only [if_false, Bool.false_eq_true] at hm hal hzr hzn hin hbytes hzi hoff
  have hbytes1 : Equiv.memRead (ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk false aq rl rd rs1).sub) 4 v1.toNat = some v := by
    rw [memRead_writeWord _ _ _ _ _ (by omega)]; exact hbytes
  have e0 : v1 + sign_extend (m := 64) (0 : BitVec 12) = v1 := by simp [sign_extend, Sail.BitVec.signExtend]
  obtain ⟨s1', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_gen { s with mem := ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk false aq rl rd rs1).sub } rs (hr.with_mem _) pc' hpc _ _ _ _ hw
    (.LOAD (0, .Regidx rs1, .Regidx rd, LKind.lw.u, (LKind.lw.w : Int)))
    (by
      intro t htp ⟨w, hw'⟩ hmt _
      have := decode_load0 t _ hop hvalid htp w hw'
      rw [himm, hr1x, hrdx, hu, hwd] at this; exact this)
    rd.toNat (extend_value LKind.lw.u v) (pc' + 4)
    (by
      intro t he htm hmr hsec hwf hnext _
      rw [execute_load_w, Int.toNat_natCast, insert_same t _ _ hnext]
      obtain ⟨sv, hsv, hpm⟩ := hr.pmm
      have hv := Equiv.riscv_readBytes t LKind.lw.w (v1 + sign_extend (m := 64) (0 : BitVec 12)).toNat v
        (by rw [htm, e0]; exact hbytes1)
      exact LKind.lw.exec t hwf rs hmr sv (hsec.trans hsv) hpm rs1 rd 0 v1 (getX_of_agree he l1 hv1)
        r (by rw [e0]; exact hm) hrb (by rw [e0]; exact hal) v hv) n
  -- ZisK: the load
  have hrom := (LrInstr.mk false aq rl rd rs1).spec hi.tr
  simp only [Transpiler.aLoad, Transpiler.aWidth, if_false, Bool.false_eq_true] at hrom
  have ha := Equiv.Zisk.srcA_ASrc xs _ l1
  rw [hr1] at ha
  have hex := Equiv.Zisk.execute_load_mk z _ LKind.lw.op LKind.lw.pure rd.toNat ld 0 4
    xs pc' st hx hzpc hst v1 ha v (by simpa using hzr) (by simpa using hzn) (by simpa using hzi)
    (by rw [show v1 + 0 = v1 from by bv_decide, Equiv.memRead_normIn hzm hin]; exact hbytes)
  have hmw : ZiskRiscv.writeWord s1'.mem pc'.toNat (LrInstr.mk false aq rl rd rs1).encode = s.mem := by
    rw [hmem, writeWord_twice, writeWord_same _ _ _ hw0]
  refine ⟨s1', _, hrun, by rw [zstep_run zrom z pc' hzpc, hrom]; exact hex, ?_, hpc'', hmem⟩
  rw [hmw]
  refine ⟨(hr.keep hwf ho).with_mem _, ⟨Equiv.Zisk.xsUpd xs rd.toNat (extend_value LKind.lw.u v), ?_,
    fun k hk1 hk2 => ?_⟩, ⟨_, hpc'', by simp⟩,
    ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, ?_⟩
  · simp only [Std.ExtDHashMap.get?_insert]
    have hval := LKind.val .lw v1 v
    simp [linkRegs_xs _ xs _ _ hx]
    exact congrArg _ (congrArg Prod.fst hval)
  · show getX s1' k = _
    rw [hxs' k hk2]
    exact regs_after0 { s with mem := ZiskRiscv.writeWord s.mem pc'.toNat (LrInstr.mk false aq rl rd rs1).sub } xs rd.toNat
      (extend_value LKind.lw.u v) ld
      (fun k h1 h2 => agree k h1 h2) k hk1 hk2
  · exact hzm


/-- The machine's `kind` on a word of opcode AMO, funct3 2 or 3: lr, sc, or
    other, by funct5 (and, for lr, rs2). -/
theorem kind_lrsc (w : BitVec 32) (h1 : w.extractLsb' 0 7 = 0x2F#7)
    (hf3 : w.extractLsb' 12 3 = 2#3 ∨ w.extractLsb' 12 3 = 3#3) :
    ZiskRiscv.kind (ZiskRiscv.fields w) =
      if (w.extractLsb' 27 5).toNat == 2 && (w.extractLsb' 20 5).toNat == 0 then .lr
      else if (w.extractLsb' 27 5).toNat == 3 then .sc else .other := by
  have e := Equiv.Machine.funct5_eq w
  have ho : (ZiskRiscv.fields w).opcode = 47 := by simp [ZiskRiscv.fields, h1]
  have hr2 : (ZiskRiscv.fields w).rs2 = (w.extractLsb' 20 5).toNat := rfl
  rcases hf3 with h | h
  · have hf : (ZiskRiscv.fields w).funct3 = 2 := by simp [ZiskRiscv.fields, h]
    simp only [ZiskRiscv.kind, ho, hf, e, hr2]
  · have hf : (ZiskRiscv.fields w).funct3 = 3 := by simp [ZiskRiscv.fields, h]
    simp only [ZiskRiscv.kind, ho, hf, e, hr2]

/-- The machine's decode on an lr: the plain load in its place, no register
    to zero. -/
theorem LrInstr.decode_m (i : LrInstr) {fetch : Nat → Option (BitVec 32)} {a : Nat} (hf : fetch a = some i.encode) :
    ZiskRiscv.decode fetch a = .subst i.sub 0 := by
  obtain ⟨d, aq, rl, rd, rs1⟩ := i
  have hk := kind_lrsc (LrInstr.mk d aq rl rd rs1).encode (by simp only [LrInstr.encode]; bv_decide)
    (by cases d <;> simp only [LrInstr.encode, LrInstr.f3, if_true, if_false, Bool.false_eq_true] <;> bv_decide)
  rw [show (LrInstr.mk d aq rl rd rs1).encode.extractLsb' 27 5 = 2#5 by simp only [LrInstr.encode]; bv_decide,
    show (LrInstr.mk d aq rl rd rs1).encode.extractLsb' 20 5 = 0#5 by simp only [LrInstr.encode]; bv_decide] at hk
  have hk' : ZiskRiscv.kind (ZiskRiscv.fields (LrInstr.mk d aq rl rd rs1).encode) = .lr := by rw [hk]; rfl
  simp only [ZiskRiscv.decode, hf, hk']
  congr 1
  cases d <;> simp only [LrInstr.encode, LrInstr.sub, LrInstr.f3, if_true, if_false, Bool.false_eq_true] <;> bv_decide

/-- lr: sail-riscv's plain load in its place, the word put back: a step of
    the machine, ZisK's load. -/
theorem lrstep_m (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LrInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hok : LrOk s rs i pc v1) (n : Nat) :
    ∃ s' z', MStep rs n s s' ∧ ZRun zrom 1 z z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem ∧ FKeep z z' := by
  obtain ⟨pc0, hpc0, hzpc0⟩ := hrel.pc
  have hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc := by
    rw [hpc] at hpc0; cases hpc0; exact hzpc0
  have hrom := i.spec hi.tr
  have hw0 : ZiskRiscv.readWord s.mem pc.toNat = some i.encode := readWord_of' hi.h0 hi.h1 hi.h2 hi.h3
  have hd : ∀ fetch : Nat → Option (BitVec 32), fetch pc.toNat = some i.encode →
      ZiskRiscv.decode fetch pc.toNat = .subst i.sub 0 := fun _ hf => i.decode_m hf
  have hready := hrel.ready.with_mem (ZiskRiscv.writeWord s.mem pc.toNat i.sub)
  obtain ⟨d, aq, rl, rd, rs1⟩ := i
  have fin : ∀ s1' z', (interpretSailEffects (try_step n false)).run
        { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (LrInstr.mk d aq rl rd rs1).sub } = .ok false s1' →
      (zstep zrom).run z = .ok false z' →
      Rel { s1' with mem := ZiskRiscv.writeWord s1'.mem pc.toNat (LrInstr.mk d aq rl rd rs1).encode } z' rs →
      s1'.mem = ZiskRiscv.writeWord s.mem pc.toNat (LrInstr.mk d aq rl rd rs1).sub →
      ∃ s' z', MStep rs n s s' ∧ ZRun zrom 1 z z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem ∧ FKeep z z' := by
    intro s1' z' hrun hz hr hmem
    refine ⟨_, z', .subst pc _ _ 0 s1' hpc hw0 hd hready hrun (by simp [mPost, setX]), .step hz (.done _), hr, ?_,
      zstep_fkeep hzpc (by rw [hrom]; exact aLoad_keep d) hz⟩
    apply RamOnly.of_eq
    show ZiskRiscv.writeWord s1'.mem pc.toNat _ = s.mem
    rw [hmem, writeWord_twice, writeWord_same _ _ _ hw0]
  cases d
  · obtain ⟨s1', z', hrun, hz, hr, -, hmem⟩ := lrwstep_rel s z rs hrel pc hpc zrom aq rl rd rs1 next hi v1 hv1 hok n
    exact fin s1' z' hrun hz hr hmem
  · obtain ⟨s1', z', hrun, hz, hr, -, hmem⟩ := lrdstep_rel s z rs hrel pc hpc zrom aq rl rd rs1 next hi v1 hv1 hok n
    exact fin s1' z' hrun hz hr hmem

/-! ## sc -/

structure ScInstr where
  d : Bool
  aq : BitVec 1
  rl : BitVec 1
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def ScInstr.f3 (i : ScInstr) : BitVec 3 := if i.d then 3 else 2

def ScInstr.encode (i : ScInstr) : BitVec 32 :=
  3#5 ++ i.aq ++ i.rl ++ i.rs2 ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x2F#7

/-- The plain store the machine runs in its place. -/
def ScInstr.sub (i : ScInstr) : BitVec 32 := 0#7 ++ i.rs2 ++ i.rs1 ++ i.f3 ++ 0#5 ++ 0x23#7

/-- An sc's access: the plain store's, and its bytes off the sc's own word
    (it writes RAM, where no code is). -/
structure ScOk (rs : List PMA_Region) (i : ScInstr) (pc a : BitVec 64) : Prop where
  store : SwOk rs a (aW i.d)
  off : a.toNat + aW i.d ≤ pc.toNat ∨ pc.toNat + 4 ≤ a.toNat

/-- The machine's decode on an sc: the plain store in its place, rd to zero. -/
theorem ScInstr.decode_m (i : ScInstr) {fetch : Nat → Option (BitVec 32)} {a : Nat} (hf : fetch a = some i.encode) :
    ZiskRiscv.decode fetch a = .subst i.sub i.rd.toNat := by
  obtain ⟨d, aq, rl, rd, rs1, rs2⟩ := i
  have hrd : (ZiskRiscv.fields (ScInstr.mk d aq rl rd rs1 rs2).encode).rd = rd.toNat := by
    simp only [ZiskRiscv.fields]
    rw [show (ScInstr.mk d aq rl rd rs1 rs2).encode.extractLsb' 7 5 = rd by simp only [ScInstr.encode]; bv_decide]
  have hk := kind_lrsc (ScInstr.mk d aq rl rd rs1 rs2).encode (by simp only [ScInstr.encode]; bv_decide)
    (by cases d <;> simp only [ScInstr.encode, ScInstr.f3, if_true, if_false, Bool.false_eq_true] <;> bv_decide)
  rw [show (ScInstr.mk d aq rl rd rs1 rs2).encode.extractLsb' 27 5 = 3#5 by simp only [ScInstr.encode]; bv_decide] at hk
  have hk' : ZiskRiscv.kind (ZiskRiscv.fields (ScInstr.mk d aq rl rd rs1 rs2).encode) = .sc := by rw [hk]; rfl
  simp only [ZiskRiscv.decode, hf, hk', hrd]
  congr 1
  cases d <;> simp only [ScInstr.encode, ScInstr.sub, ScInstr.f3, if_true, if_false, Bool.false_eq_true] <;> bv_decide

theorem ScInstr.regs (i : ScInstr) :
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
      Transpiler.rs2 i.encode = i.rs2.toNat := by
  simp only [Transpiler.rd, Transpiler.rs1, Transpiler.rs2, ScInstr.encode]
  generalize i.f3 = b
  refine ⟨?_, ?_, ?_⟩ <;> congr 1 <;> bv_decide

/-- The spec's translation: the list `Transpiler.sc` builds, in the ROM. -/
theorem ScInstr.rom_of_spec (i : ScInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    ∃ ia, ∀ p ∈ Transpiler.sc i.encode i.d pc ia, zrom p.1 = p.2 := by
  obtain ⟨ia, l, hl, hz⟩ := h
  have ho : Transpiler.opcode i.encode = 0x2F#7 := by simp only [Transpiler.opcode, ScInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = i.f3 := by simp only [Transpiler.funct3, ScInstr.encode]; bv_decide
  have hf5 : i.encode.extractLsb 31 27 = 3#5 := by simp only [ScInstr.encode]; bv_decide
  refine ⟨ia, ?_⟩
  obtain ⟨d, aq, rl, rd, rs1, rs2⟩ := i
  simp only [ScInstr.f3] at hf3 ⊢
  cases d <;> simp only [if_true, if_false, Bool.false_eq_true] at hf3 <;>
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf5] at hl <;> subst hl <;> exact hz

set_option maxHeartbeats 16000000 in
/-- sail-riscv's step on the plain store in sc's place: `sd`/`sw rs2,
    0(rs1)`. -/
theorem sc_riscv (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs) (pc : BitVec 64)
    (hpc : s.regs.get? Register.PC = some pc) (hal : pc.toNat % 4 = 0) (x1 : ExecAt rs pc) (x2 : ExecAt rs (pc + 2))
    (i : ScInstr) (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2)
    (hok : SwOk rs v1 (aW i.d)) (n : Nat) :
    ∃ s1, (interpretSailEffects (try_step n false)).run
        { s with mem := ZiskRiscv.writeWord s.mem pc.toNat i.sub } = .ok false s1 ∧
      s1.mem = Equiv.memWrite (n := aW i.d) (ZiskRiscv.writeWord s.mem pc.toNat i.sub) v1.toNat
        (v2.setWidth (8 * aW i.d)) ∧
      WF s1 ∧ s1.regs.get? Register.PC = some (pc + 4) ∧ (∀ k, k < 32 → getX s1 k = getX s k) ∧
      (∀ r, isXReg r = false → r ≠ Register.PC → r ≠ Register.nextPC → r ≠ Register.minstret →
        r ≠ Register.minstret_increment → s1.regs.get? r = s.regs.get? r) := by
  obtain ⟨d, aq, rl, rd, rs1, rs2⟩ := i
  obtain ⟨⟨r, hm, hwr, hal'⟩, -, -⟩ := hok
  have e0 : v1 + sign_extend (m := 64) ((0 : BitVec 7) ++ (0 : BitVec 5)) = v1 := by
    simp [sign_extend, Sail.BitVec.signExtend]
  have l1 : rs1.toNat < 32 := rs1.isLt
  have l2 : rs2.toNat < 32 := rs2.isLt
  cases d
  · let L : SwInstr := ⟨.sw, rs1, rs2, 0, 0⟩
    have hLs : L.encode = (ScInstr.mk false aq rl rd rs1 rs2).sub := by
      simp [L, SwInstr.encode, ScInstr.sub, ScInstr.f3, SKind.f3]
    obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := L.fields
    obtain ⟨b0, b1, b2, b3⟩ := writeWord_bytes s.mem pc.toNat L.encode
    rw [hLs] at b0 b1 b2 b3
    have hw : WordAt { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk false aq rl rd rs1 rs2).sub } rs pc
        (byte L.encode 0) (byte L.encode 1) (byte L.encode 2) (byte L.encode 3) :=
      ⟨hal, x1, x2, by rw [hLs]; exact b0, by rw [hLs]; exact b1, by rw [hLs]; exact b2, by rw [hLs]; exact b3, hc⟩
    obtain ⟨s1, hrun, hmem, hwf, hpc', hxs, ho⟩ := try_step_store
      { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk false aq rl rd rs1 rs2).sub } rs
      (hr.with_mem _) pc hpc _ _ _ _ hw
      (.STORE ((0 : BitVec 7) ++ (0 : BitVec 5), .Regidx rs2, .Regidx rs1, (SKind.sw.w : Int)))
      (by intro t htp ⟨w, hw'⟩ hmt _
          have := SKind.sw.decode t _ hop hf3 htp w hw'
          rw [hhi, hlo, hr2x, hr1x] at this; exact this)
      (Equiv.memWrite (n := SKind.sw.w) (ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk false aq rl rd rs1 rs2).sub)
        (v1 + sign_extend (m := 64) ((0 : BitVec 7) ++ (0 : BitVec 5))).toNat (v2.setWidth (8 * SKind.sw.w)))
      (by intro t he htm hmr hsec hwf'
          rw [execute_store_w, Int.toNat_natCast]
          obtain ⟨sv, hsv, hpm⟩ := hr.pmm
          have htm' : t.mem = ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk false aq rl rd rs1 rs2).sub := htm
          rw [← htm']
          exact SKind.sw.exec t hwf' rs hmr sv (hsec.trans hsv) hpm rs1 rs2 _ v1 v2
            (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) r (by rw [e0]; exact hm) hwr
            (by rw [e0]; exact hal')) n
    exact ⟨s1, hrun, by rw [hmem, e0]; rfl, hwf, hpc', hxs, ho⟩
  · let L : SdInstr := ⟨rs1, rs2, 0, 0⟩
    have hLs : L.encode = (ScInstr.mk true aq rl rd rs1 rs2).sub := by
      simp [L, SdInstr.encode, ScInstr.sub, ScInstr.f3]
    obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := L.fields
    obtain ⟨b0, b1, b2, b3⟩ := writeWord_bytes s.mem pc.toNat L.encode
    rw [hLs] at b0 b1 b2 b3
    have hw : WordAt { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk true aq rl rd rs1 rs2).sub } rs pc
        (byte L.encode 0) (byte L.encode 1) (byte L.encode 2) (byte L.encode 3) :=
      ⟨hal, x1, x2, by rw [hLs]; exact b0, by rw [hLs]; exact b1, by rw [hLs]; exact b2, by rw [hLs]; exact b3, hc⟩
    obtain ⟨s1, hrun, hmem, hwf, hpc', hxs, ho⟩ := try_step_store
      { s with mem := ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk true aq rl rd rs1 rs2).sub } rs
      (hr.with_mem _) pc hpc _ _ _ _ hw
      (.STORE ((0 : BitVec 7) ++ (0 : BitVec 5), .Regidx rs2, .Regidx rs1, 8))
      (by intro t htp ⟨w, hw'⟩ hmt _
          have := decode_sd t _ hop hf3 htp w hw'
          rw [hhi, hlo, hr2x, hr1x] at this; exact this)
      (Equiv.memWrite (n := 8) (ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk true aq rl rd rs1 rs2).sub)
        (v1 + sign_extend (m := 64) ((0 : BitVec 7) ++ (0 : BitVec 5))).toNat v2)
      (by intro t he htm hmr hsec hwf'
          rw [execute_store_eq]
          obtain ⟨sv, hsv, hpm⟩ := hr.pmm
          have htm' : t.mem = ZiskRiscv.writeWord s.mem pc.toNat (ScInstr.mk true aq rl rd rs1 rs2).sub := htm
          rw [← htm']
          exact execute_sd t hwf' rs hmr sv (hsec.trans hsv) hpm rs1 rs2 _ v1 v2
            (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) r (by rw [e0]; exact hm) hwr
            (by rw [e0]; exact hal')) n
    refine ⟨s1, hrun, ?_, hwf, hpc', hxs, ho⟩
    rw [hmem, e0]
    show _ = Equiv.memWrite (n := 8) _ _ (v2.setWidth 64)
    rw [BitVec.setWidth_eq]

set_option maxHeartbeats 16000000 in
/-- ZisK's run of an sc as the spec translates it: the store of r[rs2] at
    r[rs1]; then, when rd is not x0, 0 to rd at ia; on to pc + 4. -/
theorem zisk_sc (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (z : Equiv.Zisk.ZState) (i : ScInstr)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (hpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (hze : z.regs.get? _root_.Zisk.Register.Zerror = some false)
    (hwr : _root_.Zisk.Functions.writable (regv xs i.rs1.toNat + 0) (aW i.d) = true)
    (hnr : _root_.Zisk.Functions.hits_registers (regv xs i.rs1.toNat + 0) (aW i.d) = false)
    (hrom : ∀ p ∈ Transpiler.sc i.encode i.d pc ia, zrom p.1 = p.2) :
    ∃ z' k, ZRun zrom k z z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some (Equiv.Zisk.xsUpd xs i.rd.toNat 0) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? _root_.Zisk.Register.Step = some st ∧
        z'.regs.get? _root_.Zisk.Register.Zerror = some false) ∧
      z'.mem = Equiv.memWrite (n := aW i.d) z.mem (regv xs i.rs1.toNat + 0).toNat
        ((regv xs i.rs2.toNat).setWidth (8 * aW i.d)) ∧ FKeep z z' := by
  obtain ⟨hrd, hr1, hr2⟩ := i.regs
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  simp only [Transpiler.sc, hrd, hr1, hr2, aWidth_eq] at hrom
  by_cases h0 : i.rd.toNat = 0
  · simp only [h0, if_true, List.mem_singleton, forall_eq] at hrom
    have hz := Equiv.Zisk.execute_store_mkj z (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) 0 (aW i.d) 4
      xs pc st hx hpc hst _ _ (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2) hwr hnr
    rw [← hrom, ← zstep_run zrom z pc hpc] at hz
    refine ⟨_, 1, .step hz (.done _), ?_, ?_, ⟨BitVec.addInt st 1, ?_, ?_⟩, rfl,
      zstep_fkeep hpc (by rw [hrom]; rfl) hz⟩
    · simp [Std.ExtDHashMap.get?_insert, hx, h0, Equiv.Zisk.xsUpd]
    · simp [Std.ExtDHashMap.get?_insert]
    · simp [Std.ExtDHashMap.get?_insert]
    · simp [Std.ExtDHashMap.get?_insert, hze]
  · simp only [h0, if_false, List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil,
      or_false] at hrom
    obtain ⟨q1, q2⟩ := hrom
    have hz1 := Equiv.Zisk.execute_store_mkj z (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) 0 (aW i.d)
      (ia - pc) xs pc st hx hpc hst _ _ (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2) hwr hnr
    rw [← q1, ← zstep_run zrom z pc hpc] at hz1
    have eia : pc + (ia - pc) = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
    generalize hz1s : ({ z with regs := _, mem := _ } : Equiv.Zisk.ZState) = z1 at hz1
    have hx1 : z1.regs.get? _root_.Zisk.Register.Xs = some xs := by
      subst hz1s; simp [Std.ExtDHashMap.get?_insert, hx]
    have hpc1 : z1.regs.get? _root_.Zisk.Register.PC = some ia := by
      subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
    have hst1 : z1.regs.get? _root_.Zisk.Register.Step = some (BitVec.addInt st 1) := by
      subst hz1s; simp [Std.ExtDHashMap.get?_insert]
    have hze1 : z1.regs.get? _root_.Zisk.Register.Zerror = some false := by
      subst hz1s; simp [Std.ExtDHashMap.get?_insert, hze]
    have hm1 : z1.mem = Equiv.memWrite (n := aW i.d) z.mem (regv xs i.rs1.toNat + 0).toNat
        ((regv xs i.rs2.toNat).setWidth (8 * aW i.d)) := by subst hz1s; rfl
    have hz2 := Equiv.Zisk.execute_mkj z1 (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure i.rd.toNat
      (by omega) (pc + 4 - ia) (pc + 4 - ia) xs ia (BitVec.addInt st 1) hx1 hpc1 hst1 0 0 (Or.inl rfl) (Or.inl rfl)
      (if i.d then 8 else 4)
    rw [← q2, ← zstep_run zrom z1 ia hpc1] at hz2
    simp only [ite_self, Equiv.Zisk.copyb_val] at hz2
    refine ⟨_, 2, .step hz1 (.step hz2 (.done _)), ?_, ?_, ⟨BitVec.addInt (BitVec.addInt st 1) 1, ?_, ?_⟩, hm1,
      FKeep.trans (zstep_fkeep hpc (by rw [q1]; rfl) hz1) (zstep_fkeep hpc1 (by rw [q2]; rfl) hz2)⟩
    · simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
      exact linkRegs_xs _ _ _ _ hx1
    · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
    · simp [Std.ExtDHashMap.get?_insert]
    · simp [Std.ExtDHashMap.get?_insert, hze1]

/-- Setting a register keeps every other register. -/
theorem setX_get_other (s : RVState) (k : Nat) (v : BitVec 64) (r : Register) (hr : isXReg r = false) :
    (setX s k v).regs.get? r = s.regs.get? r := by
  unfold setX; split
  · rfl
  · simp [Std.ExtDHashMap.get?_insert, xreg_beq_false k r hr]

theorem _root_.Equiv.Riscv.WF.setX {s : RVState} (h : WF s) (k : Nat) (v : BitVec 64) : WF (setX s k v) := by
  unfold Equiv.Riscv.setX; split
  · exact h
  · refine ⟨fun r => ?_⟩
    simp only [Std.ExtDHashMap.get?_insert]
    split
    · rfl
    · exact h.all r

set_option maxHeartbeats 16000000 in
/-- sc: sail-riscv's plain store in its place, the word put back, rd then 0:
    a step of the machine; ZisK's store, and the 0 to rd. -/
theorem scstep_m (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ScInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hv2 : getX s i.rs2.toNat = some v2) (hok : ScOk rs i pc v1) (n : Nat) :
    ∃ s' z' k, MStep rs n s s' ∧ ZRun zrom k z z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem ∧ FKeep z z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw0 : ZiskRiscv.readWord s.mem pc'.toNat = some i.encode := readWord_of' hi.h0 hi.h1 hi.h2 hi.h3
  have hd : ∀ fetch : Nat → Option (BitVec 32), fetch pc'.toNat = some i.encode →
      ZiskRiscv.decode fetch pc'.toNat = .subst i.sub i.rd.toNat := fun _ hf => i.decode_m hf
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hr2 : regv xs i.rs2.toNat = v2 := by
    have := getX_regv agree l2; rw [hv2] at this; exact (Option.some.inj this).symm
  obtain ⟨hst', hoff⟩ := hok
  have hzw := hst'.zwrite
  have hzn := hst'.zregs
  -- sail-riscv
  obtain ⟨s1, hrun, hmem, hwf, hpc1, hxs, ho⟩ :=
    sc_riscv s rs hr pc' hpc hi.al hi.x1 hi.x2 i v1 v2 hv1 hv2 hst' n
  -- ZisK
  obtain ⟨ia, hrom⟩ := i.rom_of_spec hi.tr
  obtain ⟨z', k, hz, hx', hzpc', hst'', hzm', hk⟩ := zisk_sc zrom z i xs pc' st ia hx hzpc hst hze
    (by rw [hr1]; simpa using hzw) (by rw [hr1]; simpa using hzn) hrom
  have hmw : ZiskRiscv.writeWord s1.mem pc'.toNat i.encode =
      Equiv.memWrite (n := aW i.d) s.mem v1.toNat (v2.setWidth (8 * aW i.d)) := by
    rw [hmem, writeWord_memWrite _ _ _ _ hw0 _ _ (by omega)]
  have hs2 : (mPost s1 pc'.toNat i.encode i.rd.toNat).mem =
      Equiv.memWrite (n := aW i.d) s.mem v1.toNat (v2.setWidth (8 * aW i.d)) := by
    rw [← hmw]; simp only [mPost, setX]; split <;> rfl
  refine ⟨_, z', k, .subst pc' _ _ _ s1 hpc hw0 hd (hr.with_mem _) hrun rfl, hz, ?_,
    by rw [hs2]; exact memWrite_ramOnly _ _ _ hzw, hk⟩
  have hready1 : StepReady s1 rs := (hr.with_mem _).keep hwf ho
  refine ⟨?_, ⟨_, hx', ?_⟩, ⟨pc' + 4, ?_, hzpc'⟩, hst'', ?_⟩
  · refine (hready1.with_mem (ZiskRiscv.writeWord s1.mem pc'.toNat i.encode)).keep
      ((⟨fun r => hwf.all r⟩ : WF { s1 with mem := ZiskRiscv.writeWord s1.mem pc'.toNat i.encode }).setX _ _) ?_
    intro r h1 _ _ _ _
    exact setX_get_other _ _ _ r h1
  · have a1 : RegsAgree { s1 with mem := ZiskRiscv.writeWord s1.mem pc'.toNat i.encode } xs := by
      intro k hk1 hk2
      show getX s1 k = _
      rw [hxs k hk2]; exact agree k hk1 hk2
    exact regs_after0 _ xs i.rd.toNat 0 ld a1
  · simp only [mPost]
    rw [setX_get_other _ _ _ _ rfl]; exact hpc1
  · rw [hs2, hzm', hr1, hr2, show v1 + 0 = v1 from by bv_decide]
    exact Equiv.normIn_memWrite hzm _ _

end Equiv.Sim
