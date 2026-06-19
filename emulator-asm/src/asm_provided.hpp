#ifndef EMULATOR_ASM_ASM_PROVIDED_HPP
#define EMULATOR_ASM_ASM_PROVIDED_HPP

#include <stdint.h>

/**************************/
/* Assembly-provided code */
/**************************/

// This is the emulator assembly code start function, which will execute the code in the ROM until
// it ends, and generate the trace in the output trace memory.
// It is called from C to start the execution of the assembly code.
void emu_start(void);

// These functions are implemented in assembly and provide access to configuration parameters used
// to generate the assembly code, and that in some cases must match the C main program configuration
void write_ro_init_data(void);
void write_rw_init_data(void);
uint64_t get_rom_length(void);
uint64_t get_gen_method(void); // Must match the C main program provided argument
uint64_t get_precompile_results(void);

// These variables are updated by the assembly code to provide information about the execution
// status and trace generation, accessed by C to generate the response to the client
extern uint64_t MEM_STEP; // Current step, i.e. number of executed instructions, updated by assembly at every step or at the end of every chunk, depending on the generation method
extern uint64_t MEM_END; // Indicates the end of execution
extern uint64_t MEM_ERROR; // Indicates an error during execution
extern uint64_t MEM_TRACE_ADDRESS; // Address of the trace memory
extern uint64_t MEM_CHUNK_ADDRESS; // Address of the current chunk
extern uint64_t MEM_CHUNK_START_STEP; // Step at which the current chunk started

#endif // EMULATOR_ASM_ASM_PROVIDED_HPP