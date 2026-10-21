/* Comparison operators on bitvectors.
 *
 * Sail's standard library leaves these to each model; sail-riscv defines
 * the same set in its own prelude. Naming matches theirs so the two
 * models read alike.
 */

default Order dec
$include <prelude.sail>

infix 4 <_u
infix 4 <_s
infix 4 >=_u
infix 4 >=_s

val operator <_u  : forall 'n. (bits('n), bits('n)) -> bool
val operator <_s  : forall 'n, 'n > 0. (bits('n), bits('n)) -> bool
val operator >=_u : forall 'n. (bits('n), bits('n)) -> bool
val operator >=_s : forall 'n, 'n > 0. (bits('n), bits('n)) -> bool

function operator <_u  (x, y) = unsigned(x) < unsigned(y)
function operator <_s  (x, y) = signed(x)   < signed(y)
function operator >=_u (x, y) = unsigned(x) >= unsigned(y)
function operator >=_s (x, y) = signed(x)   >= signed(y)

/* Wrappers over the Sail builtins, with the names sail-riscv uses.
 * `zeros()` takes its width implicitly from context. */

val zero_extend : forall 'n 'm, 'm >= 'n. (bits('n), int('m)) -> bits('m)
function zero_extend(x, n) = sail_zero_extend(x, n)

val sign_extend : forall 'n 'm, 'm >= 'n. (bits('n), int('m)) -> bits('m)
function sign_extend(x, n) = sail_sign_extend(x, n)

val zeros : forall 'n, 'n >= 0. implicit('n) -> bits('n)
function zeros(n) = sail_zeros(n)

/* Bitvector operators sail-riscv's prelude defines and Sail's library leaves
 * to each model. No arithmetic right shift here: the library's
 * sail_arith_shiftright maps to BitVec.rotateRight in the Lean backend, a
 * rotation, so the model shifts arithmetically through sign_extend instead. */

overload operator - = {sub_bits}
overload operator ^ = {xor_vec}
overload ~ = {not_vec}
infixl 7 <<
infixl 7 >>
overload operator << = {sail_shiftleft}
overload operator >> = {sail_shiftright}

infix 4 <=_u
infix 4 <=_s

val operator <=_u : forall 'n. (bits('n), bits('n)) -> bool
val operator <=_s : forall 'n, 'n > 0. (bits('n), bits('n)) -> bool

function operator <=_u (x, y) = unsigned(x) <= unsigned(y)
function operator <=_s (x, y) = signed(x)   <= signed(y)

/* An integer as a bitvector, truncated to the width: sail-riscv's
 * to_bits_truncate. */
val to_bits : forall 'l, 'l >= 0. (implicit('l), int) -> bits('l)
function to_bits(l, n) = get_slice_int(l, n, 0)
