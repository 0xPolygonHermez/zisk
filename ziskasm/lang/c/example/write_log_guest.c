// write_log (zkvm_io.h) guest: logs text through the inline form and, from
// write_log_thunk.c, through the thunk, and outputs a value computed between the
// logs. Run with `ziskemu --write-log` it prints
//   hello, log
//   i=0
//   i=1
//   i=2
//   sum > 5
//   through the thunk
// and otherwise nothing; the output (the sum, 21) is the same either way.
#include "zkvm_io.h"

void log_through_thunk(const uint8_t* utf8_bytes, size_t utf8_len);

static const char HELLO[] = "hello, log\n";
static const char BIG[] = "sum > 5\n";
static const char THUNK[] = "through the thunk\n";

int main(void) {
    write_log((const uint8_t*)HELLO, sizeof HELLO - 1);
    write_log((const uint8_t*)HELLO, 0); /* empty: prints nothing */
    uint32_t sum = 0;
    char line[] = "i=0\n";
    for (int i = 0; i < 3; i++) {
        sum += 7 * i;
        line[2] = (char)('0' + i);
        write_log((const uint8_t*)line, sizeof line - 1);
    }
    if (sum > 5) write_log((const uint8_t*)BIG, sizeof BIG - 1);
    log_through_thunk((const uint8_t*)THUNK, sizeof THUNK - 1);
    write_output((const uint8_t*)&sum, sizeof sum);
    return 0;
}
