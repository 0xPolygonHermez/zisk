/* EVM support: the jump destination analysis of a contract's bytecode. */

/* jump_dest: write at a the bitmap of the bytecode at b, of the length at
 * EXTRA_PARAMS_ADDR: bit i is set when byte i is a JUMPDEST (0x5b) that is
 * an instruction, not the data of a PUSH1..PUSH32 (0x60..0x7f), which
 * carries opcode - 0x5f bytes of it. ceil(count / 64) words; c = 0.
 *
 * The machine walks the bytecode a word at a time, skipping the words a
 * PUSH covers entirely (precompiles/helpers/src/jump_dest/bitmap.rs); the
 * bitmap is the same, and the model walks it a byte at a time. The walk
 * reads no byte inside a PUSH's data, as the machine reads no such word. */
union clause zisk_op = ZOP_JUMP_DEST : unit
function clause zisk_op_of_code (0xc0) = Some(ZOP_JUMP_DEST())
function clause zisk_exec_mem (ZOP_JUMP_DEST(), a, b, _) = {
  let count = unsigned(read_mem(EXTRA_PARAMS_ADDR, 8));
  assert(count > 0, "jump_dest of no bytes: the guest must skip the call");
  var word : bits(64) = zeros();
  var skip : int = 0;
  foreach (i from 0 to (count - 1)) {
    if i > 0 & i % 64 == 0 then {
      write_mem(a + to_bits(64, (i / 64 - 1) * 8), 8, word);
      word = zeros()
    };
    if skip > 0 then skip = skip - 1
    else {
      let op = read_mem(b + to_bits(64, i), 1);
      if op == 0x5b then word[i % 64] = bitone
      else if (0x60 <=_u op) & (op <=_u 0x7f) then skip = unsigned(op) - 95  /* op - 0x5f */
    }
  };
  write_mem(a + to_bits(64, ((count - 1) / 64) * 8), 8, word);
  Some((zeros(), false))
}
