#ifndef EMULATOR_ASM_SERVER_HPP
#define EMULATOR_ASM_SERVER_HPP

void server_setup (void);
void server_reset_fast (void);
void server_reset_slow (void);
void server_reset_trace (void);
void server_run (void);
void server_cleanup (void);

#endif // EMULATOR_ASM_SERVER_HPP