/-
  Both models, side by side.

  The equivalence proofs relate sail-riscv's `execute` on RISC-V
  instructions to sail-zisk's `zisk_execute` on the ZisK instructions the
  transpiler emits for them. This module only imports the two generated
  models and names the functions the proofs will relate, so that a change
  to either backend's output breaks the build here first.
-/
import Zisk
import LeanRV64D

#check @Zisk.Functions.zisk_execute
#check @LeanRV64D.Functions.execute
