#include <stdint.h>
#include <stddef.h>
#include "zisklib.h"
#include "emit.h"
static uint8_t g_out[32]; static uint8_t g_in[8]={0}; static volatile uint32_t s=1;
int main(void){ ziskos_keccak(g_in, 0, g_out); emit32(g_out); s=g_out[0]; return 0; }
