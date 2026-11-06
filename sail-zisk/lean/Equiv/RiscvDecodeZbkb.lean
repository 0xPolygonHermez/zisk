/-
  sail-riscv's decoder on Zbkb's packs (pack, packh, packw) and Zbb's
  extensions (sext.b, sext.h, zext.h). One lemma per form, each in its own
  module (Equiv/Decode/Zbkb/) so that they check in parallel; pack with rs2
  x0 goes through zext.h's RV32 clause first. Here, the dispatcher.
-/
import Equiv.Decode.Zbkb.Pack
import Equiv.Decode.Zbkb.Pack0
import Equiv.Decode.Zbkb.Packh
import Equiv.Decode.Zbkb.Packw
import Equiv.Decode.Zbkb.Sextb
import Equiv.Decode.Zbkb.Sexth
import Equiv.Decode.Zbkb.Zexth

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A pack or packh word decodes to ZBKB_RTYPE with its fields. -/
theorem decode_zbkb_r (s : RVState) (w : BitVec 32) (op : brop_zbkb) (hop : w.extractLsb 6 0 = 0x33#7)
    (hf : w.extractLsb 31 25 = 0x04#7) (hf3 : w.extractLsb 14 12 = zbkbF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBKB_RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          op)) s := by
  cases op
  · by_cases h0 : w.extractLsb 24 20 = 0#5
    · exact decode_zbkb_pack0 s w hop hf hf3 h0 hp sec hsec mv hmv hmB
    · exact decode_zbkb_pack s w hop hf hf3 h0 hp sec hsec mv hmv hmB
  · exact decode_zbkb_packh s w hop hf hf3 hp sec hsec mv hmv hmB

def sextF12 : extop_zbb → BitVec 12
  | .SEXTB => 0x604 | .SEXTH => 0x605 | .ZEXTH => 0x080

/-- A sext.b or sext.h word decodes to ZBB_EXTOP with its fields. -/
theorem decode_zbb_sext (s : RVState) (w : BitVec 32) (op : extop_zbb) (hz : op ≠ .ZEXTH)
    (hop : w.extractLsb 6 0 = 0x13#7) (hf : w.extractLsb 31 20 = sextF12 op) (hf3 : w.extractLsb 14 12 = 1#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBB_EXTOP (.Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op
  · exact decode_zbb_sextb s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_sexth s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact absurd rfl hz

end Equiv.Riscv
