/-
  sail-riscv's decoder on the instructions the simulation needs: an R-type
  word decodes to RTYPE with its fields.

  sail-riscv's `encdec_backwards` is one definition of some 28,000 lines, a
  chain of clauses, one per encoding: each a guard on the word's bits, and
  some reading registers (`currentlyEnabled`). Its equation lemma is too big
  to generate, so the proofs unfold it with `delta` and peel the clauses off
  the front one at a time (`peel`): a guard refuted by `bv_decide` from the
  word's fields, an extension the profile leaves out read as false, a `pure`
  stepped over. At the instruction's own clause the guard holds, its fields
  are read back, and the rest of the decoder is discarded. Each decoding
  takes a minute or two to check: the ten R-type ops about twenty minutes.
-/
import Equiv.RiscvMem

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- An extension the profile leaves out, which reads no register. -/
theorem ce_run (s : RVState) (e : extension) (h : currentlyEnabled e = pure false) :
    (interpretSailEffects (currentlyEnabled e)).run s = .ok false s := by rw [h]; rfl

/-- Zicfilp, which reads the landing-pad enable: off in machine mode's mseccfg. -/
theorem ce_zicfilp (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (v : RegisterType mseccfg) (hv : s.regs.get? mseccfg = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zicfilp)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, get_xLPE, readReg_run _ _ _ hp, readReg_run _ _ _ hv]

/-- A clause's guard is false for the word. -/
macro "guard_false" : tactic => `(tactic| (
  intro h
  simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq] at h
  first
  | bv_decide
  | (simp_all (config := { decide := true }) [encdec_uop_backwards_matches]; done)))

/-- One step through the decoder's chain of clauses. -/
macro "peel" : tactic => `(tactic| first
  | (rw [if_neg]; rotate_left; focus guard_false)
  | (rw [interp_pure, run_pure']; try dsimp only)
  | (rw [ce_run]; rotate_left; (focus (simp [currentlyEnabled, hartSupports]; done)); try dsimp only)
  | (rw [ce_zicfilp _ (by assumption) _ (by assumption)]; try dsimp only)
  | (rw [interp_bind, run_bind']; try dsimp only))

/-- funct3 and funct7 of each R-type op (RISC-V's base encoding). -/
def rtypeF3 : rop → BitVec 3
  | .ADD => 0 | .SUB => 0 | .SLL => 1 | .SLT => 2 | .SLTU => 3
  | .XOR => 4 | .SRL => 5 | .SRA => 5 | .OR => 6 | .AND => 7

def rtypeF7 : rop → BitVec 7
  | .SUB => 0x20 | .SRA => 0x20 | _ => 0


/-! ## `bv_decide`'s enum encodings, made once here

  `bv_decide` defines an enum's encoding (`iop.enumToBitVec`, ...) the first
  time a module needs it. The decode lemmas are each in their own module
  (Equiv/Decode/), so that they check in parallel; two that both made one
  could not be imported together. Made here, which they all import, each is
  made once. -/

theorem enum_rop (x : rop) (h0 : x ≠ .ADD) (h1 : x ≠ .SUB) (h2 : x ≠ .SLL) (h3 : x ≠ .SLT) (h4 : x ≠ .SLTU) (h5 : x ≠ .XOR) (h6 : x ≠ .SRL) (h7 : x ≠ .SRA) (h8 : x ≠ .OR) : x = .AND := by bv_decide
theorem enum_iop (x : iop) (h0 : x ≠ .ADDI) (h1 : x ≠ .SLTI) (h2 : x ≠ .SLTIU) (h3 : x ≠ .XORI) (h4 : x ≠ .ORI) : x = .ANDI := by bv_decide
theorem enum_sop (x : sop) (h0 : x ≠ .SLLI) (h1 : x ≠ .SRLI) : x = .SRAI := by bv_decide
theorem enum_bop (x : bop) (h0 : x ≠ .BEQ) (h1 : x ≠ .BNE) (h2 : x ≠ .BLT) (h3 : x ≠ .BGE) (h4 : x ≠ .BLTU) : x = .BGEU := by bv_decide
theorem enum_uop (x : uop) (h0 : x ≠ .LUI) : x = .AUIPC := by bv_decide
theorem enum_ropw (x : ropw) (h0 : x ≠ .ADDW) (h1 : x ≠ .SUBW) (h2 : x ≠ .SLLW) (h3 : x ≠ .SRLW) : x = .SRAW := by bv_decide
theorem enum_sopw (x : sopw) (h0 : x ≠ .SLLIW) (h1 : x ≠ .SRLIW) : x = .SRAIW := by bv_decide
theorem enum_brop_zbs (x : brop_zbs) (h0 : x ≠ .BCLR) (h1 : x ≠ .BEXT) (h2 : x ≠ .BINV) : x = .BSET := by bv_decide
theorem enum_biop_zbs (x : biop_zbs) (h0 : x ≠ .BCLRI) (h1 : x ≠ .BEXTI) (h2 : x ≠ .BINVI) : x = .BSETI := by bv_decide

/-! ## The chain, a clause at a time, by its head

`peel_r` rewrites (`rw`, `dsimp`) through the whole rest of the decoder at
every clause, a cost quadratic in the clauses. `skip_r` instead applies, by
`Eq.trans`, an equation whose left side unification matches against the
term's head alone: a clause that decodes nothing, and the chain goes on. -/

/-- A clause of the decoder's chain that decodes nothing: its guard false,
    or its body none; the chain goes on, the state as it was. -/
theorem skip_ite {α β : Type} (c : Bool) (A : SailM (Option α)) (k : Option α → SailM β) (s : RVState)
    (h : c = true → (interpretSailEffects A).run s = .ok none s) :
    (interpretSailEffects ((if c = true then A else pure none) >>= k)).run s =
      (interpretSailEffects (k none)).run s := by
  cases c
  · rfl
  · simp only [interp_bind, run_bind', if_true, h rfl]

/-- A clause of any shape that decodes nothing. -/
theorem skip_none {α β : Type} (m : SailM (Option α)) (k : Option α → SailM β) (s : RVState)
    (h : (interpretSailEffects m).run s = .ok none s) :
    (interpretSailEffects (m >>= k)).run s = (interpretSailEffects (k none)).run s := by
  rw [interp_bind, run_bind', h]

/-- `peel`'s steps, a clause at a time by its head: a clause whose guard
    `guard_false` refutes, or whose body `peel` runs to none. -/
macro "skip_p" : tactic => `(tactic| first
  | (refine (skip_ite _ _ _ _ ?h).trans ?_
     case h => intro hc; exfalso; revert hc; guard_false)
  | (refine (skip_none _ _ _ ?h).trans ?_
     case h => (try dsimp only); (repeat peel); first | done | rfl))

end Equiv.Riscv
