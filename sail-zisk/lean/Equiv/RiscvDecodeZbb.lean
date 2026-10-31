/-
  sail-riscv's decoder on Zbb's register ops (ZBB_RTYPE): andn, orn, xnor,
  max, maxu, min, minu, rol, ror, a clause each; rolw and rorw
  (ZBB_RTYPEW); rori and roriw. One lemma per op, each in
  its own module (Equiv/Decode/Zbb/) so that they check in parallel; here,
  the dispatcher.
-/
import Equiv.Decode.Zbb.Andn
import Equiv.Decode.Zbb.Orn
import Equiv.Decode.Zbb.Xnor
import Equiv.Decode.Zbb.Max
import Equiv.Decode.Zbb.Maxu
import Equiv.Decode.Zbb.Min
import Equiv.Decode.Zbb.Minu
import Equiv.Decode.Zbb.Rol
import Equiv.Decode.Zbb.Ror
import Equiv.Decode.Zbb.Rolw
import Equiv.Decode.Zbb.Rorw
import Equiv.Decode.Zbb.Rori
import Equiv.Decode.Zbb.Roriw

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A Zbb register word decodes to ZBB_RTYPE with its fields. -/
theorem decode_zbb_r (s : RVState) (w : BitVec 32) (op : brop_zbb) (hop : w.extractLsb 6 0 = 0x33#7)
    (hf7 : w.extractLsb 31 25 = zbbF7 op) (hf3 : w.extractLsb 14 12 = zbbF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBB_RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          op)) s := by
  cases op
  · exact decode_zbb_andn s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_orn s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_xnor s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_max s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_maxu s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_min s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_minu s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_rol s w hop hf7 hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_ror s w hop hf7 hf3 hp sec hsec mv hmv hmB

/-- A Zbb word rotation decodes to ZBB_RTYPEW with its fields. -/
theorem decode_zbb_w (s : RVState) (w : BitVec 32) (op : bropw_zbb) (hop : w.extractLsb 6 0 = 0x3B#7)
    (hf : w.extractLsb 31 25 = 0x30#7) (hf3 : w.extractLsb 14 12 = zbbWF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBB_RTYPEW (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          op)) s := by
  cases op
  · exact decode_zbb_rolw s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_rorw s w hop hf hf3 hp sec hsec mv hmv hmB

end Equiv.Riscv
