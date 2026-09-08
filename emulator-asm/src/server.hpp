#ifndef EMULATOR_ASM_SERVER_HPP
#define EMULATOR_ASM_SERVER_HPP

#include <stdint.h>

// ROM histogram output sizing, known before the emulation starts (see server_setup)
extern uint64_t histogram_size; // Total bytes of the output: control header + both multiplicity tables
extern uint64_t rom_length;     // Instruction multiplicity counters, one per ROM instruction
extern uint64_t frops_length;   // FROPS multiplicity counters, one per FROPS table row

void server_setup (void);
void server_reset_fast (void);
void server_reset_slow (void);
void server_reset_trace (void);
void server_run (void);
void server_cleanup (void);
void server_signal_handler (void);

#endif // EMULATOR_ASM_SERVER_HPP