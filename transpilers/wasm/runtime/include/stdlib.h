#ifndef NITRO_STDLIB_H
#define NITRO_STDLIB_H
#include <stddef.h>
#define EXIT_FAILURE 1
void *malloc(size_t);
void free(void *);
_Noreturn void exit(int);
#endif
