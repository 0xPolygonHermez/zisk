/-
  sail-riscv's decoder on A's AMOs, each op and width a lemma in its own
  module (Equiv/Decode/Amo/) so that they check in parallel.
-/
import Equiv.Decode.Amo.SwapD
import Equiv.Decode.Amo.SwapW
import Equiv.Decode.Amo.AddD
import Equiv.Decode.Amo.AddW
import Equiv.Decode.Amo.XorD
import Equiv.Decode.Amo.XorW
import Equiv.Decode.Amo.AndD
import Equiv.Decode.Amo.AndW
import Equiv.Decode.Amo.OrD
import Equiv.Decode.Amo.OrW
import Equiv.Decode.Amo.MinD
import Equiv.Decode.Amo.MinW
import Equiv.Decode.Amo.MaxD
import Equiv.Decode.Amo.MaxW
import Equiv.Decode.Amo.MinuD
import Equiv.Decode.Amo.MinuW
import Equiv.Decode.Amo.MaxuD
import Equiv.Decode.Amo.MaxuW
