/-
  sail-riscv's decoder on Zbs: bclr, bext, binv, bset (ZBS_RTYPE) and their
  immediate forms (ZBS_IOP), a clause each, Zbs enabled through misa's B
  bit. One lemma per op, each in its own module (Equiv/DecodeZbs/) so that
  they check in parallel; here, the two dispatchers.
-/
import Equiv.DecodeZbs.Bclr
import Equiv.DecodeZbs.Bext
import Equiv.DecodeZbs.Binv
import Equiv.DecodeZbs.Bset
import Equiv.DecodeZbs.Bclri
import Equiv.DecodeZbs.Bexti
import Equiv.DecodeZbs.Binvi
import Equiv.DecodeZbs.Bseti

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A Zbs register word decodes to ZBS_RTYPE with its fields. -/
theorem decode_zbs_r (s : RVState) (w : BitVec 32) (op : brop_zbs) (hop : w.extractLsb 6 0 = 0x33#7)
    (hf7 : w.extractLsb 31 25 = zbsF7 op) (hf3 : w.extractLsb 14 12 = zbsF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBS_RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          op)) s := by
  cases op
  · exact decode_zbs_bclr s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbs_bext s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbs_binv s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbs_bset s w hop hf7 hf3 hp sec hsec mv hmv hmB

/-- A Zbs immediate word decodes to ZBS_IOP with its fields. -/
theorem decode_zbs_i (s : RVState) (w : BitVec 32) (op : biop_zbs) (hop : w.extractLsb 6 0 = 0x13#7)
    (hf6 : w.extractLsb 31 26 = zbsiF6 op) (hf3 : w.extractLsb 14 12 = zbsiF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBS_IOP (w.extractLsb 25 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op
  · exact decode_zbs_bclri s w hop hf6 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbs_bexti s w hop hf6 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbs_binvi s w hop hf6 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbs_bseti s w hop hf6 hf3 hp sec hsec mv hmv hmB

end Equiv.Riscv
