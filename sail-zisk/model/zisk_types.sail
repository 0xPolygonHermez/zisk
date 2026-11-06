/* ZisK instruction operand types: register indices, sources and stores.
 *
 * Split a/b enums make the asymmetry a type error rather than a runtime
 * panic: STEP is a-only, IND is b-only.
 */

type regidx = bits(6)

enum a_src  = { A_C, A_MEM, A_IMM, A_STEP, A_REG }
enum b_src  = { B_C, B_MEM, B_IMM, B_IND,  B_REG }
enum c_store = { ST_NONE, ST_MEM, ST_IND, ST_REG }
