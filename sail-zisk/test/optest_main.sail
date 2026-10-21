/* Runs the per-operation checks: `op_checks` (build/optest/op_checks.sail,
 * generated) compares zisk_exec with the emulator's op functions on every
 * test vector, prints each mismatch, and returns how many there were. */

val main : unit -> unit
function main() = {
  let n = op_checks();
  if n == 0 then print_endline("OK: every operation agrees with the emulator on every vector")
  else print_endline(concat_str("FAIL: mismatches: ", dec_str(n)))
}
