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
