/* One check: zisk_exec on (a, b) against the emulator's (c, flag). Prints a
 * mismatch and counts it. */

$include <string.sail>

val check_op : (string, zisk_op, bits(64), bits(64), bits(64), bool) -> int
function check_op(name, op, a, b, c, flag) = {
  let (c', flag') = zisk_exec(op, a, b);
  if c' == c & flag' == flag then 0
  else {
    print_endline(concat_str(name, concat_str(" a=", concat_str(bits_str(a), concat_str(" b=",
      concat_str(bits_str(b), concat_str(": model c=", concat_str(bits_str(c'),
      concat_str(", emulator c=", bits_str(c))))))))));
    1
  }
}

