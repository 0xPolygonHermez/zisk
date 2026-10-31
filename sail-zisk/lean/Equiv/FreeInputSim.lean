/-
  The free input read by a load (contract § 2): an 8-byte load of
  INPUT_ADDR. sail-riscv reads the 8 bytes there, the machine's mirror of
  the free input; sail-zisk's read_mem returns its FreeInput register. The
  two are the same value when the mirror holds the free input and ZisK's
  register the machine's (FcallRel).
-/
import Equiv.X0Sim
import Equiv.ZStepKeep

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- An 8-byte read of INPUT_ADDR: the free input. -/
theorem read_mem_free (s : ZState) (f : BitVec 64) (hf : s.regs.get? FreeInput = some f) :
    (read_mem INPUT_ADDR 8).run s = .ok f s := by
  have hrb : readable INPUT_ADDR 8 = true := by decide
  have hnr : hits_registers INPUT_ADDR 8 = false := by decide
  simp only [read_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hrb, hnr, if_true, run_bind',
    run_pure', Bool.not_false]
  try dsimp only
  simp only [beq_self_eq_true, Bool.and_self, if_true, run_bind', readReg_run s FreeInput f hf, run_pure']
  simp

set_option maxHeartbeats 4000000 in
/-- A load as the spec builds it, of the free input: rd gets it, through
    op; on to pc + 4. -/
theorem execute_load_free (s : ZState) (ra : a_src × BitVec 64) (op : zisk_op) (hop : PureOp op)
    (rd : Nat) (hd : rd < 32) (off : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va : BitVec 64) (ha : ASrc ra xs va) (hva : va + off = INPUT_ADDR)
    (f : BitVec 64) (hf : s.regs.get? FreeInput = some f) :
    let r := zisk_exec op va f
    (zisk_execute (Transpiler.mk ra (.B_IND, off) op (Transpiler.storeReg rd) 4 4 (w := 8))).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd r.1).insert Zc r.1).insert Zflag r.2).insert
          Step (BitVec.addInt st 1)).insert PC (pc + 4) } := by
  intro r
  have hm' := hop.mem
  have hp := hop.notPre
  have hh := hop.notHalt ()
  have ld : (rd : Int) < 40 := by omega
  have hrf : ∀ a, a = INPUT_ADDR → (read_mem a 8).run s = .ok f s := by
    intro a ha; rw [ha]; exact read_mem_free s f hf
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩ <;> by_cases h0 : rd = 0 <;>
  · (try subst hv1)
    (try have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, Transpiler.storeReg, h0, source_a, source_b, rX, wX,
      store_c, value_to_store, next_pc, hm', hp, linkRegs, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
      writeReg_run, ld, reg_index, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [hrf]; try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *]))
    all_goals first
      | (simp (disch := omega) [r, _root_.Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend, *]; done)
      | (rw [← hva, Equiv.Sim.getElem!_int]; congr 1; rw [getElem!_pos xs _ (by omega)]; rfl)

end Equiv.Zisk

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

set_option maxHeartbeats 4000000 in
/-- One step of each on `ld` of the free input, any registers: sail-riscv
    reads the 8 bytes at INPUT_ADDR, f, the free input's mirror; ZisK's
    load the FreeInput register, f too. The states stay related. -/
theorem ldfree_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LdInstr) (next : Option (BitVec 32))
    (hi : LdtAt s rs pc zrom i next)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hva : v1 + i.imm.signExtend 64 = _root_.Zisk.Functions.INPUT_ADDR)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + i.imm.signExtend 64)) 8 = some r)
    (hrb : r.attributes.readable = true) (hal : SplitOk r (v1 + i.imm.signExtend 64) 8)
    (f : BitVec 64) (hbytes : Equiv.memRead s.mem 8 (v1 + i.imm.signExtend 64).toNat = some f)
    (hzf : z.regs.get? _root_.Zisk.Register.FreeInput = some f) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem ∧ FKeep z z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  obtain ⟨h14, h13⟩ := ld_f3 _ hf3
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have hr1 : Equiv.Zisk.regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  obtain ⟨s', z', h1, h2, h3, h4, h5⟩ : ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc' + 4) ∧
      s'.mem = s.mem := by
    refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
      (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, false, 8)) ?_ i.rd.toNat i.rd.isLt f (pc' + 4) ?_ zrom ?_ n
    · intro t htp ⟨w, hw'⟩ hmt _
      have := decode_load0 t _ hop (by rw [h13, h14]; decide) htp w hw'
      rw [himm, hr1x, hrdx, h13, h14] at this; exact this
    · intro t he htm hmr hsec hwf hnext _
      rw [execute_load_eq, insert_same t _ _ hnext]
      obtain ⟨sv, hsv, hpm⟩ := hr.pmm
      have hv : Equiv.memRead t.mem 8 _ = some f := htm ▸ hbytes
      exact execute_ld t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
        r hm hrb hal f hv
    · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
      have ha := Equiv.Zisk.srcA_ASrc xs _ l1
      rw [hr1] at ha
      have hex := Equiv.Zisk.execute_load_free z _ (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure i.rd.toNat i.rd.isLt _
        xs pc' st hx hzpc hst v1 ha hva f hzf
      refine ⟨_, hex, ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert],
        by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
      simp only [Std.ExtDHashMap.get?_insert]
      simp [linkRegs_xs _ xs _ _ hx, Equiv.Zisk.copyb_val]
  refine ⟨s', z', h1, h2, h3, h4, h5, zstep_fkeep hzpc ?_ h2⟩
  rw [i.zisk_of_spec' hi.tr]; rfl

end Equiv.Sim
