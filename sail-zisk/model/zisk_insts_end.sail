/* The end of the operations: every family has added its clauses. */

function clause zisk_op_of_code (_) = None()

/* An op with a zisk_exec_mem clause has none here, and zisk_execute never
 * asks this function about it. */
function clause zisk_exec (_, _, _) = (zeros(), false)

/* Every other op is a pure one, a clause of zisk_exec. */
function clause zisk_exec_mem (_, _, _, _) = None()

end zisk_op
end zisk_exec
end zisk_exec_mem
end zisk_op_of_code
