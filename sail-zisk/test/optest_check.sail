/* One check: zisk_exec on (a, b) against the emulator's (c, flag); and,
 * for an m32 op, that a's high half does not matter, since the circuit
 * zeroes it before the op sees it (pil/operations.pil). Prints a mismatch
 * and counts it. */

$include <string.sail>

val check_op : (string, zisk_op, bits(64), bits(64), bits(64), bool) -> int
function check_op(name, op, a, b, c, flag) = {
  let (c', flag') = zisk_exec(op, a, b);
  var n : int = 0;
  if not_bool(c' == c & flag' == flag) then {
    print_endline(concat_str(name, concat_str(" a=", concat_str(bits_str(a), concat_str(" b=",
      concat_str(bits_str(b), concat_str(": model c=", concat_str(bits_str(c'),
      concat_str(", emulator c=", bits_str(c))))))))));
    n = n + 1
  };
  if zisk_op_attrs(op).m32 then {
    let (cm, flagm) = zisk_exec(op, zero_extend(a[31 .. 0], 64), b);
    if not_bool(cm == c' & flagm == flag') then {
      print_endline(concat_str(name, concat_str(" is m32 but depends on a's high half: a=",
        concat_str(bits_str(a), concat_str(" b=", concat_str(bits_str(b),
        concat_str(" gives c=", concat_str(bits_str(c'),
        concat_str(", with a's high half zeroed c=", bits_str(cm))))))))));
      n = n + 1
    }
  };
  n
}

