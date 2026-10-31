/* Internal operations: Main computes them itself (OpType::Internal). */

/* flag: c = 0 and flag set. `call` uses it to take its jump. */
union clause zisk_op = ZOP_FLAG : unit
function clause zisk_op_of_code (0x00) = Some(ZOP_FLAG())
function clause zisk_exec (ZOP_FLAG(), _, _) = (zeros(), true)

/* copyb: c = b. Used for loads, stores and register moves. */
union clause zisk_op = ZOP_COPYB : unit
function clause zisk_op_of_code (0x01) = Some(ZOP_COPYB())
function clause zisk_exec (ZOP_COPYB(), _, b) = (b, false)
