/* The ZisK execution step.
 *
 * Every instruction has the same shape: (c, flag) = op(a, b). There is
 * one step function, not one per opcode -- the per-opcode content lives
 * entirely in zisk_exec. Mirrors Emu::step in emulator/src/emu.rs.
 */

function source_a (inst : zisk_inst) -> bits(64) =
  match inst.a_src {
    A_C    => Zc,                      /* result of the previous step */
    A_REG  => rX(inst.a_imm[4 .. 0]),
    A_MEM  => read_mem(inst.a_imm, 8),
    A_IMM  => inst.a_imm,
    A_STEP => Step
  }

function source_b (inst : zisk_inst, a : bits(64)) -> bits(64) =
  match inst.b_src {
    B_C   => Zc,
    B_REG => rX(inst.b_imm[4 .. 0]),
    B_MEM => read_mem(inst.b_imm, 8),
    B_IMM => inst.b_imm,
    B_IND => zero_extend(read_mem(a + inst.b_imm, inst.ind_width), 64)
  }

/* store_pc selects pc + jmp_off2 over c, which is how JAL/JALR record a
 * return address without a second instruction. */
function value_to_store (inst : zisk_inst, c : bits(64)) -> bits(64) =
  if inst.store_pc then PC + inst.jmp_off2 else c

function store_c (inst : zisk_inst, a : bits(64), c : bits(64)) -> unit = {
  let v = value_to_store(inst, c);
  match inst.store {
    ST_NONE => (),
    ST_REG  => wX(inst.store_off[4 .. 0], v),
    ST_MEM  => write_mem(inst.store_off, 8, v),
    /* Bind the width ONCE: each projection of an existentially-typed
       field yields a fresh type variable, so two reads of inst.ind_width
       cannot be proven equal. */
    ST_IND  => { let w = inst.ind_width;
                 write_mem(a + inst.store_off, w, truncate(v, 8 * w)) }
  }
}

function next_pc (inst : zisk_inst, c : bits(64), flag : bool) -> bits(64) =
  if inst.set_pc then c + inst.jmp_off1        /* indirect (JALR)  */
  else if flag   then PC + inst.jmp_off1       /* branch taken     */
  else                PC + inst.jmp_off2       /* fall through     */

/* Returns true when the program has ended. */
function zisk_step () -> bool = {
  let inst = rom_fetch(PC);
  let a    = source_a(inst);
  let b    = source_b(inst, a);
  let (c, flag) = zisk_exec(inst.op, a, b);
  store_c(inst, a, c);
  Zc    = c;
  Zflag = flag;
  Step  = Step + 1;
  if inst.is_end then true
  else { PC = next_pc(inst, c, flag); false }
}
