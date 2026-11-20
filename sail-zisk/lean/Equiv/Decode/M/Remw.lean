/- sail-riscv's decoder: `decode_remw`, on its own so that it checks in parallel (RiscvDecodeM.lean). -/
import Equiv.RiscvDecodeMBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
theorem decode_remw (s : RVState) (w : BitVec 32) (u : Bool)
    (hop : w.extractLsb 6 0 = 0x3B#7) (hf3 : w.extractLsb 14 12 = (if u then 7#3 else 6#3))
    (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.REMW (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), u)) s := by
  have h12 : w.extractLsb 12 12 = (if u then 1#1 else 0#1) := by
    cases u <;> simp only [Bool.false_eq_true, if_false, if_true] at hf3 ⊢ <;> bv_decide
  cases u <;> simp only [Bool.false_eq_true, if_false, if_true] at hf3 h12 <;>
  · unfold ext_decode
    delta encdec_backwards
    repeat skip_p
    try dsimp only
    rw [interp_bind, run_bind']
    try dsimp only
    m_tail

end Equiv.Riscv
