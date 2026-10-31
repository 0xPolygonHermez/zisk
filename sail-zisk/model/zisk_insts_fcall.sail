/* Free input calls: asking the host for a value, and reading the answer.
 *
 * A guest gets values it cannot compute cheaply — an inverse, a square root,
 * the input length — from the host, through fcall (core/src/fcall.rs), and
 * then checks them itself. Nothing in the proof relies on the host answering
 * truthfully, so the model does not say how it answers: the answers are an
 * oracle, the fcall tape, in memory from FCALL_TAPE on. Each fcall takes the
 * tape's next entry,
 *
 *   <n> <word 0> ... <word n-1>
 *
 * as its result, and the parameters the guest passed (fcall_param) do not
 * matter to the model. tools/sail-difftest records the tape from ziskemu's
 * own answers; a proof takes it as part of the initial state, like the
 * input.
 *
 * The emulator keeps the result in InstContext::fcall. The model keeps where
 * it is on the tape: FcallBase, FcallSize and FcallGot, which is the index of
 * the current word plus one, as result_got is. FreeInput, what a read of
 * INPUT_ADDR returns, is that word, or 0 once the result is used up.
 */

let FCALL_TAPE : bits(64) = 0x0002_0000_0000_0000

/* The result's word at index `got - 1`, or 0 past its end. */
val fcall_current : unit -> bits(64)
function fcall_current() =
  if unsigned(FcallGot) >= 1 & unsigned(FcallGot) <= unsigned(FcallSize)
  then __read_ram(64, 8, zeros(), FcallBase + to_bits(64, (unsigned(FcallGot) - 1) * 8))
  else zeros()

/* fcall_param: pass a parameter (a words, at b if a > 1, else b itself);
 * c = b. The parameters go to the host, which the model leaves out. */
union clause zisk_op = ZOP_FCALL_PARAM : unit
function clause zisk_op_of_code (0xf6) = Some(ZOP_FCALL_PARAM())
function clause zisk_exec_mem (ZOP_FCALL_PARAM(), _, b, _) = Some((b, false))

/* fcall: call host function a; c = b. Its result is the tape's next
 * entry, and the free input its first word. */
union clause zisk_op = ZOP_FCALL : unit
function clause zisk_op_of_code (0xf7) = Some(ZOP_FCALL())
function clause zisk_exec_mem (ZOP_FCALL(), _, b, _) = {
  let n = __read_ram(64, 8, zeros(), FcallNext);
  FcallBase = FcallNext + 8;
  FcallSize = n;
  FcallGot  = 0x0000_0000_0000_0001;
  FcallNext = FcallNext + 8 + (n << 3);
  FreeInput = fcall_current();
  Some((b, false))
}

/* fcall_get: move the free input to the result's next word; c = b. */
union clause zisk_op = ZOP_FCALL_GET : unit
function clause zisk_op_of_code (0xf8) = Some(ZOP_FCALL_GET())
function clause zisk_exec_mem (ZOP_FCALL_GET(), _, b, _) = {
  assert(FcallSize != zeros(), "fcall_get with no fcall result");
  assert(unsigned(FcallGot) <= unsigned(FcallSize), "fcall_get past the fcall result");
  FcallGot  = FcallGot + 1;
  FreeInput = fcall_current();
  Some((b, false))
}

/* dma_inputcpy: copy b bytes of the result, from the free input's word on,
 * to a, and move past them; c = a. b is a whole number of words. */
union clause zisk_op = ZOP_DMA_INPUTCPY : unit
function clause zisk_op_of_code (0xd2) = Some(ZOP_DMA_INPUTCPY())
function clause zisk_exec_mem (ZOP_DMA_INPUTCPY(), a, b, _) = {
  let count = unsigned(b);
  assert(count % 8 == 0, "dma_inputcpy of a partial word");
  assert(FcallSize != zeros(), "dma_inputcpy with no fcall result");
  assert(unsigned(FcallGot) >= 1 & unsigned(FcallGot) - 1 + count / 8 <= unsigned(FcallSize),
         "dma_inputcpy past the fcall result");
  let src = FcallBase + to_bits(64, (unsigned(FcallGot) - 1) * 8);
  foreach (i from 0 to (count - 1))
    write_mem(a + to_bits(64, i), 1, __read_ram(64, 1, zeros(), src + to_bits(64, i)));
  FcallGot  = FcallGot + to_bits(64, count / 8);
  FreeInput = fcall_current();
  Some((a, false))
}
