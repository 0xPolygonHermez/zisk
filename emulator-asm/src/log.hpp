#ifndef EMULATOR_ASM_LOG_HPP
#define EMULATOR_ASM_LOG_HPP

#if defined(__GNUC__) || defined(__clang__)
__attribute__((format(printf, 1, 2)))
#endif
void asm_printf(const char *format, ...);

#if defined(__GNUC__) || defined(__clang__)
__attribute__((format(printf, 1, 2)))
#endif
void asm_raw_printf(const char *format, ...);

#endif // EMULATOR_ASM_LOG_HPP