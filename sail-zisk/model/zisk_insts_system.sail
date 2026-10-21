/* System operations: publishing an output, and ending in error. */

/* pubout: c = b. The BIOS uses it to publish an output word; the
 * emulator gives it no side effect beyond c, and neither does the model. */
union clause zisk_op = ZOP_PUBOUT : unit
function clause zisk_exec (ZOP_PUBOUT(), _, b) = (b, false)

/* halt: c = 0. The program ends in error; the instruction carries is_end,
 * and zisk_execute records the error and its code, since an op here is a
 * pure function of a and b (Emu: opc_halt). */
union clause zisk_op = ZOP_HALT : unit
function clause zisk_exec (ZOP_HALT(), _, _) = (zeros(), false)
