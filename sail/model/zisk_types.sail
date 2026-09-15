/* ZisK machine types: state, instruction record, sources and stores. */

type regidx = bits(5)

/* --- Architectural state ------------------------------------------- */

register PC    : bits(64)   /* program counter                         */
register Zc    : bits(64)   /* c register: result of the previous step */
register Zflag : bool       /* flag register: one bit, not 64          */
register Step  : bits(64)   /* execution step counter                  */

/* --- Sources and stores --------------------------------------------- *
 * Split a/b enums make the asymmetry a type error rather than a runtime
 * panic: STEP is a-only, IND is b-only.
 */

enum a_src  = { A_C, A_MEM, A_IMM, A_STEP, A_REG }
enum b_src  = { B_C, B_MEM, B_IMM, B_IND,  B_REG }
enum c_store = { ST_NONE, ST_MEM, ST_IND, ST_REG }
