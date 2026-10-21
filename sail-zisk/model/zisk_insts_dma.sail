/* DMA operations: copying, comparing and filling memory, byte by byte.
 *
 * Each moves `count` bytes at once, whatever their alignment. The machine
 * splits an unaligned run into a head, aligned words and a tail
 * (core/src/operations/dma_*.rs); that is how it proves the operation, not
 * what the operation does, so the model leaves it out. The plain forms take
 * the count from EXTRA_PARAMS_ADDR, where the guest wrote it; the x forms
 * from the instruction's extended argument.
 */

let EXTRA_PARAMS_ADDR : bits(64) = 0x0000_0000_a040_0f00   /* SYS_ADDR + 0x0F00 */

/* memmove: the bytes [src, src + count) end up at [dst, dst + count), as
 * if through a buffer, even when the ranges overlap (Mem::memcpy). */
val dma_copy : (bits(64), bits(64), int) -> unit
function dma_copy(dst, src, count) =
  if unsigned(dst) <= unsigned(src) then
    foreach (i from 0 to (count - 1))
      write_mem(dst + to_bits(64, i), 1, read_mem(src + to_bits(64, i), 1))
  else
    foreach (i from (count - 1) downto 0)
      write_mem(dst + to_bits(64, i), 1, read_mem(src + to_bits(64, i), 1))

/* The difference of the first bytes that differ, a's minus b's, as a
 * signed 64-bit value; 0 when all `count` bytes agree (Mem::memcmp). */
val dma_compare : (bits(64), bits(64), int) -> bits(64)
function dma_compare(a, b, count) = {
  var r : bits(64) = zeros();
  var done : bool = false;
  foreach (i from 0 to (count - 1)) {
    if not_bool(done) then {
      let x = unsigned(read_mem(a + to_bits(64, i), 1));
      let y = unsigned(read_mem(b + to_bits(64, i), 1));
      if x != y then { r = to_bits(64, x - y); done = true }
    }
  };
  r
}

/* dma_memcpy: copy count bytes from b to a; c = a. */
union clause zisk_op = ZOP_DMA_MEMCPY : unit
function clause zisk_op_of_code (0xd0) = Some(ZOP_DMA_MEMCPY())
function clause zisk_exec_mem (ZOP_DMA_MEMCPY(), a, b, _) = {
  dma_copy(a, b, unsigned(read_mem(EXTRA_PARAMS_ADDR, 8)));
  Some((a, false))
}

/* dma_xmemcpy: the same, with the count in the extended argument. */
union clause zisk_op = ZOP_DMA_XMEMCPY : unit
function clause zisk_op_of_code (0xd6) = Some(ZOP_DMA_XMEMCPY())
function clause zisk_exec_mem (ZOP_DMA_XMEMCPY(), a, b, ext) = {
  dma_copy(a, b, unsigned(ext));
  Some((a, false))
}

/* dma_memcmp: compare count bytes at a and b; c = the first difference. */
union clause zisk_op = ZOP_DMA_MEMCMP : unit
function clause zisk_op_of_code (0xd1) = Some(ZOP_DMA_MEMCMP())
function clause zisk_exec_mem (ZOP_DMA_MEMCMP(), a, b, _) =
  Some((dma_compare(a, b, unsigned(read_mem(EXTRA_PARAMS_ADDR, 8))), false))

/* dma_xmemcmp: the same, with the count in the extended argument. */
union clause zisk_op = ZOP_DMA_XMEMCMP : unit
function clause zisk_op_of_code (0xd7) = Some(ZOP_DMA_XMEMCMP())
function clause zisk_exec_mem (ZOP_DMA_XMEMCMP(), a, b, ext) =
  Some((dma_compare(a, b, unsigned(ext)), false))

/* dma_xmemset: fill b bytes at a with the extended argument's low byte;
 * c = a. */
union clause zisk_op = ZOP_DMA_XMEMSET : unit
function clause zisk_op_of_code (0xd9) = Some(ZOP_DMA_XMEMSET())
function clause zisk_exec_mem (ZOP_DMA_XMEMSET(), a, b, ext) = {
  foreach (i from 0 to (unsigned(b) - 1))
    write_mem(a + to_bits(64, i), 1, ext[7 .. 0]);
  Some((a, false))
}
