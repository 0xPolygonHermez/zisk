/-
  sail-riscv's decoder on the vector opcodes (OP-V 0x57, OP-VE 0x77):
  ILLEGAL, the profile having no vector extension. A lemma per opcode and
  funct3 in Equiv/Decode/Vec/, one per module so that they check in
  parallel; the extension checks and the vtype fields they read in
  RiscvDecodeVecBase.lean.
-/
import Equiv.Decode.Vec.V57_0
import Equiv.Decode.Vec.V57_1
import Equiv.Decode.Vec.V57_2
import Equiv.Decode.Vec.V57_3
import Equiv.Decode.Vec.V57_4
import Equiv.Decode.Vec.V57_5
import Equiv.Decode.Vec.V57_6
import Equiv.Decode.Vec.V57_7
import Equiv.Decode.Vec.V77_0
import Equiv.Decode.Vec.V77_1
import Equiv.Decode.Vec.V77_2
import Equiv.Decode.Vec.V77_3
import Equiv.Decode.Vec.V77_4
import Equiv.Decode.Vec.V77_5
import Equiv.Decode.Vec.V77_6
import Equiv.Decode.Vec.V77_7

namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A word of a vector opcode decodes to ILLEGAL. -/
theorem decode_vec (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x57#7 ∨ w.extractLsb 6 0 = 0x77#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (vt : RegisterType vtype) (hvt : s.regs.get? vtype = some vt) (hok : VtypeOk vt)
    (l : RegisterType vl) (hl : s.regs.get? vl = some l) (st : RegisterType vstart) (hst : s.regs.get? vstart = some st) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  have h3 : w.extractLsb 14 12 = 0#3 ∨ w.extractLsb 14 12 = 1#3 ∨ w.extractLsb 14 12 = 2#3 ∨
      w.extractLsb 14 12 = 3#3 ∨ w.extractLsb 14 12 = 4#3 ∨ w.extractLsb 14 12 = 5#3 ∨
      w.extractLsb 14 12 = 6#3 ∨ w.extractLsb 14 12 = 7#3 := by bv_decide
  rcases hop with ho | ho
  · rcases h3 with h | h | h | h | h | h | h | h
    · exact decode_vec57_0 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_1 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_2 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_3 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_4 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_5 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_6 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec57_7 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
  · rcases h3 with h | h | h | h | h | h | h | h
    · exact decode_vec77_0 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_1 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_2 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_3 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_4 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_5 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_6 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst
    · exact decode_vec77_7 s w ⟨ho, h⟩ hp sec hsec mv hmv hA hB ms hms vt hvt hok l hl st hst

end Equiv.Riscv
