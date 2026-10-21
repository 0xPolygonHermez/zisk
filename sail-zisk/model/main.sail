/* The C emulator's entry point: run the program, and trace it.
 *
 * Only the C build includes this file (`make emu`); the Lean build leaves it
 * out, since a proof has no use for a main loop or for printing. The trace
 * format is the one tools/sail-difftest writes for ziskemu, so the two runs
 * compare with a plain diff. Per step:
 *
 *   <step> <pc it ran at> <c> <flag> [x<i>=<value> for each register changed]
 *
 * then how the run ended, `halt`'s error and exit code,
 *
 *   end <error> <code>
 *
 * and the output region as 32 words:
 *
 *   out <k> <word at OUTPUT_ADDR + 8k>
 *
 * The initial state is ziskemu's: pc at ROM_ENTRY, everything else zero.
 * Memory comes from the command line (`-b addr,file`, see mem.args).
 */

$include <string.sail>

let ROM_ENTRY   : bits(64) = 0x0000_0000_0000_1000
let OUTPUT_ADDR : bits(64) = 0x0000_0000_a041_0000
let MAX_STEPS   : int      = 10000000

val flag_str : bool -> string
function flag_str(f) = if f then "1" else "0"

val trace_step : (int, bits(64), vector(40, bits(64))) -> unit
function trace_step(n, pc, before) = {
  var line = concat_str(dec_str(n), concat_str(" ", bits_str(pc)));
  line = concat_str(line, concat_str(" ", bits_str(Zc)));
  line = concat_str(line, concat_str(" ", flag_str(Zflag)));
  foreach (r from 1 to 39) {
    if Xs[r] != before[r] then
      line = concat_str(line, concat_str(concat_str(" x", dec_str(r)), concat_str("=", bits_str(Xs[r]))))
  };
  print_endline(line)
}

val main : unit -> unit
function main() = {
  PC    = ROM_ENTRY;
  Zc    = zeros();
  Zflag = false;
  Step  = zeros();
  Zerror      = false;
  Zerror_code = zeros();
  foreach (r from 0 to 39) { Xs[r] = zeros() };

  var n : int = 0;
  var done : bool = false;
  while not_bool(done) do {
    assert(n < MAX_STEPS, "program did not end within MAX_STEPS");
    let pc = PC;
    let before = Xs;
    done = zisk_step();
    trace_step(n, pc, before);
    n = n + 1
  };

  print_endline(concat_str(concat_str("end ", flag_str(Zerror)), concat_str(" ", bits_str(Zerror_code))));
  foreach (k from 0 to 31) {
    let w = read_mem(OUTPUT_ADDR + 8 * k, 8);
    print_endline(concat_str(concat_str("out ", dec_str(k)), concat_str(" ", bits_str(w))))
  }
}
