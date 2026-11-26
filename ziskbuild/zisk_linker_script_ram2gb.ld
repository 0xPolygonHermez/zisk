/* ZisK guest linker script for a prover built with its `ram_2gb` feature: the heap runs to the
   end of the 2 GB RAM window (0x1_4000_0000). Everything else is zisk_linker_script.ld, which
   the linker finds through the library search path (-L <dir of this script>). */
ZISK_RAM_SIZE = 0x80000000;
INCLUDE zisk_linker_script.ld
